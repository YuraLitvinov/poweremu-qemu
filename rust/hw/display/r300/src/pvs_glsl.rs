//! R300-family PVS programs as GLSL (`r300_pvs_glsl.c`).
//!
//! [`to_glsl`] turns a decoded program into the text of
//!
//! ```glsl
//! void r300_pvs(vec4 IN[32], inout vec4 OUT[32])
//! ```
//!
//! that does what [`crate::pvs::Runner::run`] does for one vertex.  The
//! program's constants are read through a function `vec4 pvs_const(int i)`
//! that the consumer defines (from a buffer or a uniform block); indices past
//! the program's `max_const` are never requested, as in the interpreter.  `OUT`
//! must be zero-filled by the caller (outputs the program does not write keep
//! their previous contents).
//!
//! Only straight-line programs are handled; anything the translator does not do
//! is reported in the error and the caller keeps running the interpreter for
//! that draw.  The semantics are those of the interpreter, instruction by
//! instruction; where the two differ the interpreter is the reference.

use core::fmt::Write;

use crate::pvs::{
    dst_mode, me, ve, AddrMode, DstType, Program, Src, SrcType, MAX_INSTS, NUM_ALT_TEMPS, NUM_INPUTS,
    NUM_OUTPUTS, NUM_TEMPS,
};

/// Why a program cannot be translated: a combination of the `FLOW`, `PRED`
/// and `OPCODE` flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlslUnsupported(pub u32);

impl GlslUnsupported {
    /// Loops, jumps or subroutines.
    pub const FLOW: u32 = 1 << 0;
    /// R5xx predication.
    pub const PRED: u32 = 1 << 1;
    /// An opcode the interpreter does not know.
    pub const OPCODE: u32 = 1 << 2;
}

const COMP: [char; 4] = ['x', 'y', 'z', 'w'];

/// A `vec4` expression for the register an operand names, before swizzle, abs
/// and negate.
fn reg_expr(prog: &Program, s: &Src) -> String {
    const ZERO: &str = "vec4(0.0)";
    let i = u32::from(s.offset);
    let max_const = prog.desc.max_const;
    if s.mode == AddrMode::Absolute {
        let in_range = match s.ty {
            SrcType::Temp => (i as usize) < NUM_TEMPS,
            SrcType::Input => (i as usize) < NUM_INPUTS,
            SrcType::Const => i as i32 <= max_const,
            SrcType::Alt => (i as usize) < NUM_ALT_TEMPS,
        };
        if !in_range {
            return ZERO.into();
        }
        return match s.ty {
            SrcType::Temp => format!("T[{i}]"),
            SrcType::Input => format!("IN[{i}]"),
            SrcType::Const => format!("pvs_const({i})"),
            SrcType::Alt => format!("A[{i}]"),
        };
    }
    // Relative to A0.sel (mode 1) or to the loop index, which is 0 outside the
    // loops this translator leaves out.
    let ri = if s.mode == AddrMode::A0 { format!("({i} + a0[{}])", s.sel & 3) } else { i.to_string() };
    match s.ty {
        SrcType::Temp => format!("({ri} >= 0 && {ri} < {NUM_TEMPS} ? T[{ri}] : {ZERO})"),
        SrcType::Input => format!("({ri} >= 0 && {ri} < {NUM_INPUTS} ? IN[{ri}] : {ZERO})"),
        SrcType::Const => format!("({ri} >= 0 && {ri} <= {max_const} ? pvs_const({ri}) : {ZERO})"),
        SrcType::Alt => format!("({ri} >= 0 && {ri} < {NUM_ALT_TEMPS} ? A[{ri}] : {ZERO})"),
    }
}

/// A swizzled source operand: declares `vec4 name` in the current block.
fn emit_src(out: &mut String, prog: &Program, s: &Src, name: &str) {
    let reg = reg_expr(prog, s);
    if s.plain {
        let _ = writeln!(out, "    vec4 {name} = {reg};");
        return;
    }
    let _ = write!(out, "    vec4 {name}_r = {reg};\n    vec4 {name} = vec4(");
    for i in 0..4 {
        let sel = s.swz[i];
        let e = match sel {
            0..=3 => format!("{name}_r.{}", COMP[usize::from(sel)]),
            5 => "1.0".into(),
            _ => "0.0".into(),
        };
        let _ = write!(
            out,
            "{}{}{}{e})",
            if i > 0 { ", " } else { "" },
            if s.neg[i] != 0 { "-" } else { "" },
            if s.absmask != 0xffff_ffff { "abs(" } else { "(" },
        );
    }
    out.push_str(");\n");
}

/// Math engine: statements that set `vec4 r` from the scalars `a`, `b` and
/// `c`.  False for an opcode the interpreter does not know.
fn emit_math(out: &mut String, op: u32, a: &str, b: &str, c: &str, r: &str) -> bool {
    const FM: &str = "3.4028235e38";
    let scalar = match op {
        me::EXP_BASE2_DX => {
            let _ = writeln!(
                out,
                "    {r} = vec4(exp2(floor({a})), {a} > 128.0 ? 0.0 : {a} - floor({a}), exp2({a}), 1.0);"
            );
            return true;
        }
        me::LOG_BASE2_DX => {
            let _ = write!(
                out,
                "    if ({a} == 0.0) {{ {r} = vec4(-{FM}, 1.0, -{FM}, 1.0); }}\n    else {{ int e_; float m_ = frexp(abs({a}), e_); {r} = vec4(float(e_ - 1), m_ * 2.0, log2(abs({a})), 1.0); }}\n"
            );
            return true;
        }
        me::LIGHT_COEFF_DX => {
            let _ = writeln!(
                out,
                "    {r} = vec4(1.0, max({b}, 0.0), {b} > 0.0 ? pvs_pow_ff(max({a}, 0.0), clamp({c}, -128.0, 128.0)) : 0.0, 1.0);"
            );
            return true;
        }
        me::EXP_BASEE_FF => format!("exp({a})"),
        me::POWER_FUNC_FF => format!("pvs_pow_ff({a}, {c})"),
        me::RECIP_DX => format!("({a} == 0.0 ? {FM} : 1.0 / {a})"),
        me::RECIP_FF => format!("({a} == 0.0 ? 0.0 : 1.0 / {a})"),
        me::RECIP_SQRT_DX => format!("({a} == 0.0 ? {FM} : 1.0 / sqrt(abs({a})))"),
        me::RECIP_SQRT_FF => format!("({a} == 0.0 ? 0.0 : 1.0 / sqrt(abs({a})))"),
        me::MULTIPLY => format!("({a} * {b})"),
        me::EXP_BASE2_FULL_DX => format!("exp2({a})"),
        me::LOG_BASE2_FULL_DX => format!("({a} == 0.0 ? -{FM} : log2(abs({a})))"),
        me::POWER_FUNC_FF_CLAMP_B => format!("({a} < {b} ? 0.0 : pvs_pow_ff({a}, {c}))"),
        me::POWER_FUNC_FF_CLAMP_B1 => format!("({a} < {b} ? 0.0 : {a} > 1.0 ? 1.0 : pvs_pow_ff({a}, {c}))"),
        me::POWER_FUNC_FF_CLAMP_01 => format!("({a} <= 0.0 ? 0.0 : {a} > 1.0 ? 1.0 : pvs_pow_ff({a}, {c}))"),
        me::SIN => format!("sin(clamp({a}, -3.14159274, 3.14159274))"),
        me::COS => format!("cos(clamp({a}, -3.14159274, 3.14159274))"),
        me::LOG_BASE2_IEEE => format!("log2(abs({a}))"),
        me::RECIP_IEEE => format!("(1.0 / {a})"),
        me::RECIP_SQRT_IEEE => format!("(1.0 / sqrt(abs({a})))"),
        me::NOP => "0.0".into(),
        _ => return false,
    };
    let _ = writeln!(out, "    {r} = vec4({scalar});");
    true
}

/// Vector engine: statements that set `vec4 r` from the vec4s `a`, `b` and
/// `cc`.  False for an opcode the interpreter does not know.
fn emit_vector(out: &mut String, op: u32, r: &str) -> bool {
    let rhs: &str = match op {
        ve::DOT_PRODUCT => "vec4(a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w)",
        ve::DISTANCE_VECTOR => "vec4(1.0, a.y * b.y, a.z, b.w)",
        ve::MULTIPLY_CLAMP => "vec4(cc.w < a.w * b.w ? cc.w : cc.x >= a.x * b.x ? cc.x : a.x * b.x)",
        ve::NOP => "vec4(0.0)",
        ve::MULTIPLY => "a * b",
        ve::ADD => "a + b",
        ve::MULTIPLY_ADD => "a * b + cc",
        ve::FRACTION => "a - floor(a)",
        ve::MAXIMUM => "max(a, b)",
        ve::MINIMUM => "min(a, b)",
        ve::SET_GREATER_THAN_EQUAL => "vec4(greaterThanEqual(a, b))",
        ve::SET_LESS_THAN => "vec4(lessThan(a, b))",
        ve::MULTIPLYX2_ADD => "2.0 * (a * b) + cc",
        ve::FLT2FIX_DX => "floor(a)",
        ve::FLT2FIX_DX_RND => "floor(a + 0.5)",
        ve::COND_MUX_EQ => {
            "vec4(a.x == 0.0 ? b.x : cc.x, a.y == 0.0 ? b.y : cc.y, a.z == 0.0 ? b.z : cc.z, a.w == 0.0 ? b.w : cc.w)"
        }
        ve::COND_MUX_GT => {
            "vec4(a.x > 0.0 ? b.x : cc.x, a.y > 0.0 ? b.y : cc.y, a.z > 0.0 ? b.z : cc.z, a.w > 0.0 ? b.w : cc.w)"
        }
        ve::COND_MUX_GTE => {
            "vec4(a.x >= 0.0 ? b.x : cc.x, a.y >= 0.0 ? b.y : cc.y, a.z >= 0.0 ? b.z : cc.z, a.w >= 0.0 ? b.w : cc.w)"
        }
        ve::SET_GREATER_THAN => "vec4(greaterThan(a, b))",
        ve::SET_EQUAL => "vec4(equal(a, b))",
        ve::SET_NOT_EQUAL => "vec4(notEqual(a, b))",
        _ => return false,
    };
    let _ = writeln!(out, "    {r} = {rhs};");
    true
}

/// A write of `vec4 r` under the destination fields of `d0`.
fn emit_write(out: &mut String, d0: u32, r: &str) {
    let ty = DstType::from_d0(d0);
    let mask = (d0 >> 20) & 0xF;
    let mode = dst_mode(d0);
    let offset = (d0 >> 13) & 0x7F;
    let repl = ty == DstType::OutReplX;
    let (arr, lim) = match ty {
        DstType::A0 => {
            for (i, c) in COMP.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    let _ = writeln!(out, "    a0.{c} = int(clamp(floor({r}.{c}), -256.0, 255.0));");
                }
            }
            return;
        }
        DstType::Temp => ("T", NUM_TEMPS),
        DstType::Out | DstType::OutReplX => ("OUT", NUM_OUTPUTS),
        DstType::Alt => ("A", NUM_ALT_TEMPS),
        DstType::Input | DstType::Invalid => return,
    };
    if mask == 0 {
        return;
    }
    let src_comp = |i: usize| COMP[if repl { 0 } else { i }];
    if mode == AddrMode::Absolute {
        if offset as usize >= lim {
            return;
        }
        for i in (0..4).filter(|i| mask & (1 << i) != 0) {
            let _ = writeln!(out, "    {arr}[{offset}].{} = {r}.{};", COMP[i], src_comp(i));
        }
        return;
    }
    // Relative destination: A0.sel (mode 1) or the loop index (0 here).
    if mode == AddrMode::A0 {
        let _ = writeln!(out, "    {{ int di_ = {offset} + a0[{}]; if (di_ >= 0 && di_ < {lim}) {{", (d0 >> 29) & 3);
    } else {
        let _ = writeln!(out, "    {{ int di_ = {offset}; if (di_ >= 0 && di_ < {lim}) {{");
    }
    for i in (0..4).filter(|i| mask & (1 << i) != 0) {
        let _ = writeln!(out, "        {arr}[di_].{} = {r}.{};", COMP[i], src_comp(i));
    }
    out.push_str("    } }\n");
}

const PRELUDE: &str = "float pvs_pow_ff(float base, float e)\n\
{\n\
\x20   if (base == 0.0) return e < 0.0 ? uintBitsToFloat(0x7f800000u) : 0.0;\n\
\x20   if (e == 0.0) return 1.0;\n\
\x20   if (base < 0.0) return -pow(-base, e);\n\
\x20   return pow(base, e);\n\
}\n";

/// Translates `prog` to GLSL, or says why it cannot.
pub fn to_glsl(prog: &Program) -> Result<String, GlslUnsupported> {
    let mut bad = 0;
    let mut out = String::from(PRELUDE);
    if prog.desc.flow.opc != 0 {
        bad |= GlslUnsupported::FLOW;
    }
    let _ = write!(out, "void r300_pvs(vec4 IN[{NUM_INPUTS}], inout vec4 OUT[{NUM_OUTPUTS}])\n{{\n");
    if prog.ntemps != 0 {
        let n = prog.ntemps;
        let _ = write!(out, "    vec4 T[{n}];\n    for (int i_ = 0; i_ < {n}; i_++) {{ T[i_] = vec4(0.0); }}\n");
    }
    if prog.uses_alt {
        let _ = write!(
            out,
            "    vec4 A[{NUM_ALT_TEMPS}];\n    for (int i_ = 0; i_ < {NUM_ALT_TEMPS}; i_++) {{ A[i_] = vec4(0.0); }}\n"
        );
    }
    out.push_str("    ivec4 a0 = ivec4(0);\n");

    let mut pc = prog.desc.first_inst;
    while bad == 0 && pc <= prog.desc.last_inst && pc < MAX_INSTS {
        bad |= emit_inst(&mut out, prog, pc);
        pc += 1;
    }
    out.push_str("}\n");
    if bad != 0 {
        return Err(GlslUnsupported(bad));
    }
    Ok(out)
}

/// One instruction; the unsupported flags it raises (0 if translated).
fn emit_inst(out: &mut String, prog: &Program, pc: usize) -> u32 {
    let ins = &prog.dec[pc];
    let d0 = ins.d[0];
    let op = d0 & 0x3F;
    let dual = ins.is_dual();

    if (d0 >> 26) & 1 != 0 {
        return GlslUnsupported::PRED;
    }
    let _ = writeln!(out, "    {{ /* {pc} */");
    emit_src(out, prog, &ins.src[0], "a");
    emit_src(out, prog, &ins.src[1], "b");
    if dual {
        out.push_str("    vec4 cc = vec4(0.0);\n");
    } else {
        emit_src(out, prog, &ins.src[2], "cc");
    }
    out.push_str("    vec4 r;\n");
    if ins.is_math() {
        if !emit_math(out, op, "a.w", "b.w", "cc.w", "r") {
            return GlslUnsupported::OPCODE;
        }
        if (d0 >> 25) & 1 != 0 {
            out.push_str("    r = clamp(r, 0.0, 1.0);\n");
        }
        emit_write(out, d0, "r");
    } else {
        let vop = match (ins.is_macro(), op & 1) {
            (true, 1) => ve::MULTIPLYX2_ADD,
            (true, _) => ve::MULTIPLY_ADD,
            _ => op,
        };
        if !emit_vector(out, vop, "r") {
            return GlslUnsupported::OPCODE;
        }
        if (d0 >> 24) & 1 != 0 {
            out.push_str("    r = clamp(r, 0.0, 1.0);\n");
        }
        if dual {
            let bad = emit_dual_math(out, prog, ins.d[3], &ins.src[2], d0);
            if bad != 0 {
                return bad;
            }
        } else {
            emit_write(out, d0, "r");
        }
    }
    out.push_str("    }\n");
    0
}

/// The math half of a dual-issue instruction: the third source dword `s`
/// describes a math op writing ATRM 0-3.
fn emit_dual_math(out: &mut String, prog: &Program, s: u32, src: &Src, d0: u32) -> u32 {
    let mop = ((s >> 21) & 0xF) | (((s >> 2) & 1) << 4);
    let (xs, ys) = ((s >> 13) & 7, (s >> 16) & 7);
    let operand = |sel: u32| match sel {
        0..=3 => format!("dv.{}", COMP[sel as usize]),
        5 => "1.0".to_string(),
        _ => "0.0".to_string(),
    };
    let wrap = if s & (1 << 3) != 0 { "abs(" } else { "(" };
    let _ = write!(out, "    vec4 dv = {};\n    vec4 mr;\n", reg_expr(prog, src));
    let _ = writeln!(out, "    float dx = {wrap}{}), dy = {wrap}{});", operand(xs), operand(ys));
    if s & (1 << 25) != 0 {
        out.push_str("    dx = -dx;\n");
    }
    if s & (1 << 26) != 0 {
        out.push_str("    dy = -dy;\n");
    }
    if !emit_math(out, mop, "dx", "dy", "dy", "mr") {
        return GlslUnsupported::OPCODE;
    }
    if (d0 >> 25) & 1 != 0 {
        out.push_str("    mr = clamp(mr, 0.0, 1.0);\n");
    }
    emit_write(out, d0, "r"); // vector result first...
    let c = COMP[((s >> 27) & 3) as usize];
    let _ = writeln!(out, "    A[{}].{c} = mr.{c};", (s >> 19) & 3);
    0
}
