//! R300-family Programmable Vertex Shader (PVS) interpreter
//! (`r300_pvs.c`).
//!
//! The PVS executes 128-bit instructions (one opcode/destination dword and
//! three source dwords).  Encodings and opcode semantics follow the AMD
//! "R5xx Acceleration" guide, section 7.5 (v1.5: 7.5.7 vector ops, 7.5.8 math
//! ops), restricted to what R3xx/R4xx implement.  Math ops read the W channel
//! of their operands after swizzle: A.w is the argument or base, B.w a clamp,
//! C.w an exponent.
//!
//! The device runs vertex programs on the host CPU (or translates them to GLSL,
//! see [`crate::pvs_glsl`]).  A program is decoded once per draw
//! ([`Program::new`]) and then run for each vertex.

use std::env;
use std::ops::BitOr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// R3xx/R4xx code memory, in instructions.
pub const MAX_INSTS: usize = 256;
pub const NUM_INPUTS: usize = 32;
pub const NUM_OUTPUTS: usize = 32;
pub const NUM_TEMPS: usize = 128;
pub const NUM_ALT_TEMPS: usize = 20;

/// One vec4 register.
pub type Vec4 = [f32; 4];
/// The input vertex memory (IVM).
pub type Inputs = [Vec4; NUM_INPUTS];
/// The output vertex memory (OVM).
pub type Outputs = [Vec4; NUM_OUTPUTS];

/// Vector engine opcodes.
pub mod ve {
    pub const NOP: u32 = 0;
    pub const DOT_PRODUCT: u32 = 1;
    pub const MULTIPLY: u32 = 2;
    pub const ADD: u32 = 3;
    pub const MULTIPLY_ADD: u32 = 4;
    pub const DISTANCE_VECTOR: u32 = 5;
    pub const FRACTION: u32 = 6;
    pub const MAXIMUM: u32 = 7;
    pub const MINIMUM: u32 = 8;
    pub const SET_GREATER_THAN_EQUAL: u32 = 9;
    pub const SET_LESS_THAN: u32 = 10;
    pub const MULTIPLYX2_ADD: u32 = 11;
    pub const MULTIPLY_CLAMP: u32 = 12;
    pub const FLT2FIX_DX: u32 = 13;
    pub const FLT2FIX_DX_RND: u32 = 14;
    // R5xx, harmless to implement.
    pub const COND_MUX_EQ: u32 = 23;
    pub const COND_MUX_GT: u32 = 24;
    pub const COND_MUX_GTE: u32 = 25;
    pub const SET_GREATER_THAN: u32 = 26;
    pub const SET_EQUAL: u32 = 27;
    pub const SET_NOT_EQUAL: u32 = 28;
}

/// Math engine opcodes.
pub mod me {
    pub const NOP: u32 = 0;
    pub const EXP_BASE2_DX: u32 = 1;
    pub const LOG_BASE2_DX: u32 = 2;
    pub const EXP_BASEE_FF: u32 = 3;
    pub const LIGHT_COEFF_DX: u32 = 4;
    pub const POWER_FUNC_FF: u32 = 5;
    pub const RECIP_DX: u32 = 6;
    pub const RECIP_FF: u32 = 7;
    pub const RECIP_SQRT_DX: u32 = 8;
    pub const RECIP_SQRT_FF: u32 = 9;
    pub const MULTIPLY: u32 = 10;
    pub const EXP_BASE2_FULL_DX: u32 = 11;
    pub const LOG_BASE2_FULL_DX: u32 = 12;
    pub const POWER_FUNC_FF_CLAMP_B: u32 = 13;
    pub const POWER_FUNC_FF_CLAMP_B1: u32 = 14;
    pub const POWER_FUNC_FF_CLAMP_01: u32 = 15;
    pub const SIN: u32 = 16;
    pub const COS: u32 = 17;
    pub const LOG_BASE2_IEEE: u32 = 18;
    pub const RECIP_IEEE: u32 = 19;
    pub const RECIP_SQRT_IEEE: u32 = 20;
}

/// Flags for things the interpreter does not implement (0 when fully handled).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unsupported(pub u32);

impl Unsupported {
    pub const NONE: Self = Self(0);
    /// Flow control ran past its step limit.
    pub const FLOW: Self = Self(1 << 0);
    /// R5xx predication.
    pub const PRED: Self = Self(1 << 1);
    /// Unknown opcode.
    pub const OPCODE: Self = Self(1 << 2);
    /// Relative destination address.
    pub const RELDST: Self = Self(1 << 3);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for Unsupported {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// What a source operand reads (`s & 3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SrcType {
    Temp,
    Input,
    Const,
    Alt,
}

/// Where a destination writes (`(d0 >> 8) & 0xF`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DstType {
    Temp,
    A0,
    Out,
    /// Like `Out`, but every written channel takes the result's X.
    OutReplX,
    Alt,
    Input,
    /// Not a valid destination: the write is dropped.
    Invalid,
}

impl DstType {
    pub fn from_d0(d0: u32) -> Self {
        match (d0 >> 8) & 0xF {
            0 => Self::Temp,
            1 => Self::A0,
            2 => Self::Out,
            3 => Self::OutReplX,
            4 => Self::Alt,
            5 => Self::Input,
            _ => Self::Invalid,
        }
    }
}

/// ADDR_MODE of an operand or destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrMode {
    Absolute,
    /// Relative to A0.ADDR_SEL.
    A0,
    /// Relative to the loop index.
    Loop,
    /// Reserved encoding: addressed like `Absolute`, but counted as relative
    /// when the program is analysed (as the C code did).
    Reserved,
}

impl AddrMode {
    fn from_bits(low: u32, high: u32) -> Self {
        match (low & 1) | (high & 1) << 1 {
            0 => Self::Absolute,
            1 => Self::A0,
            2 => Self::Loop,
            _ => Self::Reserved,
        }
    }

    pub fn is_relative(self) -> bool {
        self != Self::Absolute
    }
}

/// Destination address mode of an instruction's first dword.
pub fn dst_mode(d0: u32) -> AddrMode {
    AddrMode::from_bits(d0 >> 31, d0 >> 12)
}

/// A source operand, decoded once per draw instead of once per vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Src {
    pub ty: SrcType,
    pub mode: AddrMode,
    /// A0 component (`s >> 29`).
    pub sel: u8,
    /// Component select: 0-3 channel, 5 one, else zero.
    pub swz: [u8; 4],
    pub offset: u16,
    /// Identity swizzle, no abs, no negate: use the register as it is.
    pub plain: bool,
    /// `0x7fffffff` with abs, else `0xffffffff`.
    pub absmask: u32,
    /// `0x80000000` where negated.
    pub neg: [u32; 4],
}

impl Src {
    pub fn decode(s: u32) -> Self {
        let ty = match s & 3 {
            0 => SrcType::Temp,
            1 => SrcType::Input,
            2 => SrcType::Const,
            _ => SrcType::Alt,
        };
        let absmask = if s & (1 << 3) != 0 { 0x7fff_ffff } else { 0xffff_ffff };
        let mut src = Src {
            ty,
            mode: AddrMode::from_bits(s >> 4, s >> 31),
            sel: ((s >> 29) & 7) as u8,
            swz: [0; 4],
            offset: ((s >> 5) & 0xFF) as u16,
            plain: absmask == 0xffff_ffff,
            absmask,
            neg: [0; 4],
        };
        for i in 0..4 {
            src.swz[i] = ((s >> (13 + 3 * i)) & 7) as u8;
            src.neg[i] = if s & (1 << (25 + i)) != 0 { 0x8000_0000 } else { 0 };
            src.plain = src.plain && usize::from(src.swz[i]) == i && src.neg[i] == 0;
        }
        src
    }
}

/// A decoded instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inst {
    /// The raw instruction.
    pub d: [u32; 4],
    pub src: [Src; 3],
}

impl Inst {
    pub fn decode(d: [u32; 4]) -> Self {
        Self { d, src: [Src::decode(d[1]), Src::decode(d[2]), Src::decode(d[3])] }
    }

    /// A math engine instruction.
    pub fn is_math(&self) -> bool {
        (self.d[0] >> 6) & 1 != 0
    }

    /// A two-clock MAD / M2X_ADD macro.
    pub fn is_macro(&self) -> bool {
        (self.d[0] >> 7) & 1 != 0
    }

    /// A vector op with a math op issued alongside; the third source dword
    /// then describes that math op.
    pub fn is_dual(&self) -> bool {
        !self.is_math() && !self.is_macro() && (self.d[0] >> 28) & 1 != 0
    }
}

impl Default for Inst {
    fn default() -> Self {
        Self::decode([0; 4])
    }
}

/// Flow control (AMD R5xx guide 7.5.5): `VAP_PVS_FLOW_CNTL_OPC` (2 bits per
/// instruction: 0 none, 1 JUMP, 2 LOOP, 3 JSR), `_ADDRS` and `_LOOP_INDEX`.
/// All zero for straight-line programs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlowControl {
    pub opc: u32,
    pub addrs: [u32; 16],
    pub loops: [u32; 16],
}

/// What a draw programs: the raw code, constants and flow control.
#[derive(Clone, Copy, Debug)]
pub struct ProgramDesc<'a> {
    /// PVS code memory, 4 dwords per instruction.
    pub code: &'a [u32],
    /// `VAP_PVS_CODE_CNTL_0.PVS_FIRST_INST`.
    pub first_inst: usize,
    /// `VAP_PVS_CODE_CNTL_0.PVS_LAST_INST`.
    pub last_inst: usize,
    /// Constant memory, from `CONST_BASE_OFFSET`.
    pub consts: &'a [Vec4],
    /// Reads beyond this return 0.
    pub max_const: i32,
    pub flow: FlowControl,
}

/// A decoded program, ready to run.
#[derive(Clone, Debug)]
pub struct Program<'a> {
    pub desc: ProgramDesc<'a>,
    /// Decoded instructions, indexed by their address.
    pub dec: Vec<Inst>,
    /// Temporaries a vertex can read before writing: zeroed per vertex.
    pub ntemps: usize,
    /// The program reads or writes the ATRM registers.
    pub uses_alt: bool,
    /// Input vertex slots the program reads (all of them when addressed
    /// relatively).
    pub in_mask: u32,
}

impl<'a> Program<'a> {
    /// Decodes the program once.  Code that `desc.code` does not hold reads as
    /// zero (a NOP).
    pub fn new(desc: ProgramDesc<'a>) -> Self {
        let mut dec = vec![Inst::default(); MAX_INSTS];
        let (mut nt, mut all, mut alt, mut inrel, mut imask) = (0usize, false, false, false, 0u32);
        let last = desc.last_inst.min(MAX_INSTS - 1);
        for (pc, slot) in dec.iter_mut().enumerate().take(last + 1).skip(desc.first_inst) {
            let mut d = [0u32; 4];
            if let Some(words) = desc.code.get(pc * 4..pc * 4 + 4) {
                d.copy_from_slice(words);
            }
            let ins = Inst::decode(d);
            let d0 = d[0];
            // Which temporaries can the program touch?  A relative address
            // could reach any of them.
            for sr in &ins.src {
                match sr.ty {
                    SrcType::Alt => alt = true,
                    SrcType::Input if sr.mode.is_relative() => inrel = true,
                    SrcType::Input if usize::from(sr.offset) < NUM_INPUTS => imask |= 1 << sr.offset,
                    SrcType::Temp if sr.mode.is_relative() => all = true,
                    SrcType::Temp => nt = nt.max(usize::from(sr.offset) + 1),
                    _ => {}
                }
            }
            // An ATRM destination, or a dual-issue math half that writes one.
            if DstType::from_d0(d0) == DstType::Alt || ins.is_dual() {
                alt = true;
            }
            if DstType::from_d0(d0) == DstType::Temp {
                if dst_mode(d0).is_relative() {
                    all = true;
                } else {
                    nt = nt.max(((d0 >> 13) & 0x7F) as usize + 1);
                }
            }
            *slot = ins;
        }
        Self {
            desc,
            dec,
            ntemps: if all || nt > NUM_TEMPS { NUM_TEMPS } else { nt },
            uses_alt: alt,
            in_mask: if inrel { 0xffff_ffff } else { imask },
        }
    }

    /// A runner holding the register file, for running many vertices.
    pub fn runner(&self) -> Runner<'_, 'a> {
        Runner { prog: self, regs: Regs::zeroed(), al: 0, unsup: Unsupported::NONE }
    }

    /// Runs the program for one vertex.  Outputs the program does not write
    /// keep their previous contents.
    pub fn run(&self, input: &Inputs, out: &mut Outputs) -> Unsupported {
        self.runner().run(input, out)
    }

    /// Runs the program for `ins.len()` vertices: vertex `i` reads `ins[i]` and
    /// writes `outs[oidx[i]]`.  Large batches are split across worker threads;
    /// the results are exactly those of the same number of [`Program::run`]
    /// calls, provided `oidx` holds each index at most once.  Returns the OR of
    /// their flags.
    ///
    /// # Panics
    /// If `oidx` is shorter than `ins` or holds an index outside `outs`.
    pub fn run_batch(&self, ins: &[Inputs], outs: &mut [Outputs], oidx: &[u32]) -> Unsupported {
        let n = ins.len();
        assert!(oidx.len() >= n, "one output index per vertex");
        let workers = if n < MIN_BATCH { 1 } else { worker_count() };
        let next = AtomicUsize::new(0);

        // A worker claims chunks of vertices and returns the results it made;
        // the outputs are written back afterwards, so that no two threads need
        // `&mut` access to the same array.
        let work = |outs: &[Outputs]| {
            let mut runner = self.runner();
            let mut unsup = Unsupported::NONE;
            let mut done: Vec<(usize, Vec<Outputs>)> = Vec::new();
            loop {
                let i0 = next.fetch_add(CHUNK, Ordering::Relaxed);
                if i0 >= n {
                    break;
                }
                let i1 = (i0 + CHUNK).min(n);
                let mut chunk: Vec<Outputs> = (i0..i1).map(|i| outs[oidx[i] as usize]).collect();
                for (i, out) in (i0..i1).zip(&mut chunk) {
                    unsup = unsup | runner.run(&ins[i], out);
                }
                done.push((i0, chunk));
            }
            (unsup, done)
        };

        let results: Vec<_> = if workers == 1 {
            vec![work(outs)]
        } else {
            let shared: &[Outputs] = outs;
            thread::scope(|scope| {
                let handles: Vec<_> = (0..workers).map(|_| scope.spawn(|| work(shared))).collect();
                handles.into_iter().map(|h| h.join().expect("PVS worker panicked")).collect()
            })
        };

        let mut unsup = Unsupported::NONE;
        for (u, done) in results {
            unsup = unsup | u;
            for (i0, chunk) in done {
                for (k, out) in chunk.into_iter().enumerate() {
                    outs[oidx[i0 + k] as usize] = out;
                }
            }
        }
        unsup
    }
}

/// Smaller batches cost more to hand out than to run.
const MIN_BATCH: usize = 256;
const CHUNK: usize = 64;
/// Worker threads at most, counting the calling thread.
const MAX_THREADS: usize = 8;

/// Total threads for a batch: `R300_PVS_THREADS` if set (1 = serial), else
/// the CPU count, at most [`MAX_THREADS`].
fn worker_count() -> usize {
    let want = match env::var("R300_PVS_THREADS") {
        Ok(v) => v.trim().parse().unwrap_or(0),
        Err(_) => thread::available_parallelism().map_or(1, |n| n.get()).min(MAX_THREADS),
    };
    want.clamp(1, MAX_THREADS)
}

#[derive(Clone)]
struct Regs {
    temp: [Vec4; NUM_TEMPS],
    alt: [Vec4; NUM_ALT_TEMPS],
    a0: [i32; 4],
}

impl Regs {
    fn zeroed() -> Self {
        Self { temp: [[0.0; 4]; NUM_TEMPS], alt: [[0.0; 4]; NUM_ALT_TEMPS], a0: [0; 4] }
    }
}

/// One active LOOP.
struct LoopFrame {
    /// Its FLOW_CNTL instruction.
    fc: usize,
    /// Iterations left.
    count: u32,
    /// The enclosing loop's index.
    saved_al: i32,
}

/// Runs vertices of one program, reusing the register file.
pub struct Runner<'p, 'a> {
    prog: &'p Program<'a>,
    regs: Regs,
    /// Fixed-point loop index of the innermost loop.
    al: i32,
    unsup: Unsupported,
}

const ZERO4: Vec4 = [0.0; 4];

impl Runner<'_, '_> {
    /// Runs the program for one vertex.
    pub fn run(&mut self, input: &Inputs, out: &mut Outputs) -> Unsupported {
        let prog = self.prog;
        // Only the registers the program can read need to start at zero (see
        // `Program::new`).
        self.regs.temp[..prog.ntemps].fill(ZERO4);
        if prog.uses_alt {
            self.regs.alt = [ZERO4; NUM_ALT_TEMPS];
        }
        self.regs.a0 = [0; 4];
        self.unsup = Unsupported::NONE;
        self.al = 0;

        let flow = &prog.desc.flow;
        // Active loops (innermost last) and subroutine returns, as indices of
        // their FLOW_CNTL instruction.
        let (mut loops, mut jsr): (Vec<LoopFrame>, Vec<usize>) = (Vec::new(), Vec::new());
        let mut steps = 0u32;
        let mut pc = prog.desc.first_inst;
        while pc <= prog.desc.last_inst && pc < MAX_INSTS {
            steps += 1;
            if steps > 65536 {
                self.unsup = self.unsup | Unsupported::FLOW; // runaway loop
                break;
            }
            self.exec(&prog.dec[pc], input, out);
            pc = if flow.opc != 0 { self.next_pc(flow, pc, &mut loops, &mut jsr) } else { pc + 1 };
        }
        self.unsup
    }

    fn mem(&self, ty: SrcType, index: i32, input: &Inputs) -> Vec4 {
        let Ok(i) = usize::try_from(index) else {
            return ZERO4;
        };
        let found = match ty {
            SrcType::Temp => self.regs.temp.get(i),
            SrcType::Input => input.get(i),
            SrcType::Const if index <= self.prog.desc.max_const => self.prog.desc.consts.get(i),
            SrcType::Const => None,
            SrcType::Alt => self.regs.alt.get(i),
        };
        found.copied().unwrap_or(ZERO4)
    }

    /// ADDR_MODE: absolute, relative to A0.ADDR_SEL, or relative to the loop
    /// index (R5xx guide, PVS source and destination operands).
    fn rel(&self, mode: AddrMode, sel: u32, offset: u32) -> i32 {
        match mode {
            AddrMode::A0 => offset as i32 + self.regs.a0[(sel & 3) as usize],
            AddrMode::Loop => offset as i32 + self.al,
            _ => offset as i32,
        }
    }

    /// A source operand with swizzle, abs and negate applied.
    fn src(&self, s: &Src, input: &Inputs) -> Vec4 {
        let index = if s.mode == AddrMode::Absolute {
            i32::from(s.offset)
        } else {
            self.rel(s.mode, u32::from(s.sel), u32::from(s.offset))
        };
        let v = self.mem(s.ty, index, input);
        if s.plain {
            return v;
        }
        let t = [v[0], v[1], v[2], v[3], 0.0, 1.0, 0.0, 0.0];
        let mut o = ZERO4;
        for i in 0..4 {
            let bits = (t[usize::from(s.swz[i])].to_bits() & s.absmask) ^ s.neg[i];
            o[i] = f32::from_bits(bits);
        }
        o
    }

    fn write(&mut self, d0: u32, r: &Vec4, out: &mut Outputs) {
        let ty = DstType::from_d0(d0);
        let mask = (d0 >> 20) & 0xF;
        let rel = self.rel(dst_mode(d0), d0 >> 29, (d0 >> 13) & 0x7F);
        let index = usize::try_from(rel).unwrap_or(usize::MAX);
        let dst = match ty {
            DstType::Temp => self.regs.temp.get_mut(index),
            DstType::A0 => {
                for (i, a0) in self.regs.a0.iter_mut().enumerate() {
                    if mask & (1 << i) != 0 {
                        let f = r[i].floor();
                        *a0 = if f < -256.0 {
                            -256
                        } else if f > 255.0 {
                            255
                        } else {
                            f as i32
                        };
                    }
                }
                return;
            }
            DstType::Out | DstType::OutReplX => out.get_mut(index),
            DstType::Alt => self.regs.alt.get_mut(index),
            DstType::Input | DstType::Invalid => return,
        };
        let Some(dst) = dst else {
            return;
        };
        for i in 0..4 {
            if mask & (1 << i) != 0 {
                dst[i] = if ty == DstType::OutReplX { r[0] } else { r[i] };
            }
        }
    }

    /// Math engine: scalar operands A, B, C (the `.w` channels).
    fn math(&mut self, op: u32, a: f32, b: f32, c: f32) -> Vec4 {
        let x = match op {
            me::EXP_BASE2_DX => {
                return [a.floor().exp2(), if a > 128.0 { 0.0 } else { a - a.floor() }, a.exp2(), 1.0];
            }
            me::LOG_BASE2_DX => {
                if a == 0.0 {
                    return [-f32::MAX, 1.0, -f32::MAX, 1.0];
                }
                let (m, e) = frexp(a.abs()); // m in [0.5, 1)
                return [(e - 1) as f32, m * 2.0, a.abs().log2(), 1.0];
            }
            me::LIGHT_COEFF_DX => {
                let z = if b > 0.0 { pow_ff(fmax(a, 0.0), fmin(fmax(c, -128.0), 128.0)) } else { 0.0 };
                return [1.0, fmax(b, 0.0), z, 1.0];
            }
            me::EXP_BASEE_FF => a.exp(),
            me::POWER_FUNC_FF => pow_ff(a, c),
            me::RECIP_DX => {
                if a == 0.0 {
                    f32::MAX
                } else {
                    1.0 / a
                }
            }
            me::RECIP_FF => {
                if a == 0.0 {
                    0.0
                } else {
                    1.0 / a
                }
            }
            me::RECIP_SQRT_DX => {
                if a == 0.0 {
                    f32::MAX
                } else {
                    1.0 / a.abs().sqrt()
                }
            }
            me::RECIP_SQRT_FF => {
                if a == 0.0 {
                    0.0
                } else {
                    1.0 / a.abs().sqrt()
                }
            }
            me::MULTIPLY => a * b,
            me::EXP_BASE2_FULL_DX => a.exp2(),
            me::LOG_BASE2_FULL_DX => {
                if a == 0.0 {
                    -f32::MAX
                } else {
                    a.abs().log2()
                }
            }
            me::POWER_FUNC_FF_CLAMP_B => {
                if a < b {
                    0.0
                } else {
                    pow_ff(a, c)
                }
            }
            me::POWER_FUNC_FF_CLAMP_B1 => {
                if a < b {
                    0.0
                } else if a > 1.0 {
                    1.0
                } else {
                    pow_ff(a, c)
                }
            }
            me::POWER_FUNC_FF_CLAMP_01 => {
                if a <= 0.0 {
                    0.0
                } else if a > 1.0 {
                    1.0
                } else {
                    pow_ff(a, c)
                }
            }
            me::SIN | me::COS => {
                // Inputs outside [-pi, pi] clamp: sin 0, cos -1.
                let t = fmin(fmax(a, -core::f32::consts::PI), core::f32::consts::PI);
                if op == me::SIN {
                    t.sin()
                } else {
                    t.cos()
                }
            }
            me::LOG_BASE2_IEEE => a.abs().log2(),
            me::RECIP_IEEE => 1.0 / a,
            me::RECIP_SQRT_IEEE => 1.0 / a.abs().sqrt(),
            me::NOP => 0.0,
            _ => {
                self.unsup = self.unsup | Unsupported::OPCODE;
                0.0
            }
        };
        [x; 4]
    }

    fn vector(&mut self, op: u32, a: &Vec4, b: &Vec4, c: &Vec4) -> Vec4 {
        let per = |f: &dyn Fn(usize) -> f32| [f(0), f(1), f(2), f(3)];
        let flag = |t: bool| if t { 1.0 } else { 0.0 };
        match op {
            ve::DOT_PRODUCT => [a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]; 4],
            ve::DISTANCE_VECTOR => [1.0, a[1] * b[1], a[2], b[3]],
            ve::MULTIPLY_CLAMP => {
                let x = if c[3] < a[3] * b[3] {
                    c[3]
                } else if c[0] >= a[0] * b[0] {
                    c[0]
                } else {
                    a[0] * b[0]
                };
                [x; 4]
            }
            ve::NOP => ZERO4,
            ve::MULTIPLY => per(&|i| a[i] * b[i]),
            ve::ADD => per(&|i| a[i] + b[i]),
            ve::MULTIPLY_ADD => per(&|i| a[i] * b[i] + c[i]),
            ve::FRACTION => per(&|i| a[i] - a[i].floor()),
            ve::MAXIMUM => per(&|i| fmax(a[i], b[i])),
            ve::MINIMUM => per(&|i| fmin(a[i], b[i])),
            ve::SET_GREATER_THAN_EQUAL => per(&|i| flag(a[i] >= b[i])),
            ve::SET_LESS_THAN => per(&|i| flag(a[i] < b[i])),
            ve::MULTIPLYX2_ADD => per(&|i| 2.0 * (a[i] * b[i]) + c[i]),
            ve::FLT2FIX_DX => per(&|i| a[i].floor()),
            ve::FLT2FIX_DX_RND => per(&|i| (a[i] + 0.5).floor()),
            ve::COND_MUX_EQ => per(&|i| if a[i] == 0.0 { b[i] } else { c[i] }),
            ve::COND_MUX_GT => per(&|i| if a[i] > 0.0 { b[i] } else { c[i] }),
            ve::COND_MUX_GTE => per(&|i| if a[i] >= 0.0 { b[i] } else { c[i] }),
            ve::SET_GREATER_THAN => per(&|i| flag(a[i] > b[i])),
            ve::SET_EQUAL => per(&|i| flag(a[i] == b[i])),
            ve::SET_NOT_EQUAL => per(&|i| flag(a[i] != b[i])),
            _ => {
                self.unsup = self.unsup | Unsupported::OPCODE;
                ZERO4
            }
        }
    }

    /// One instruction.
    fn exec(&mut self, ins: &Inst, input: &Inputs, out: &mut Outputs) {
        let d0 = ins.d[0];
        let op = d0 & 0x3F;
        let dual = ins.is_dual();

        if (d0 >> 26) & 1 != 0 {
            self.unsup = self.unsup | Unsupported::PRED;
        }
        let a = self.src(&ins.src[0], input);
        let b = self.src(&ins.src[1], input);
        // Dual: no third operand.
        let c = if dual { ZERO4 } else { self.src(&ins.src[2], input) };

        if ins.is_math() {
            let mut r = self.math(op, a[3], b[3], c[3]);
            if (d0 >> 25) & 1 != 0 {
                saturate(&mut r);
            }
            self.write(d0, &r, out);
            return;
        }

        // A macro is a two-clock MAD/M2X_ADD with three distinct temporaries.
        let vop = match (ins.is_macro(), op & 1) {
            (true, 1) => ve::MULTIPLYX2_ADD,
            (true, _) => ve::MULTIPLY_ADD,
            _ => op,
        };
        let mut r = self.vector(vop, &a, &b, &c);
        if (d0 >> 24) & 1 != 0 {
            saturate(&mut r);
        }

        if dual {
            // The third source dword describes a math op writing ATRM 0-3.
            let s = ins.d[3];
            let mop = ((s >> 21) & 0xF) | (((s >> 2) & 1) << 4);
            // Addressed like the third source operand, which `ins.src[2]` decodes.
            let sd = &ins.src[2];
            let v = self.mem(sd.ty, self.rel(sd.mode, u32::from(sd.sel), u32::from(sd.offset)), input);
            let select = |sel: u32| match sel {
                0..=3 => v[sel as usize],
                5 => 1.0,
                _ => 0.0,
            };
            let (mut x, mut y) = (select((s >> 13) & 7), select((s >> 16) & 7));
            if s & (1 << 3) != 0 {
                x = x.abs();
                y = y.abs();
            }
            if s & (1 << 25) != 0 {
                x = -x;
            }
            if s & (1 << 26) != 0 {
                y = -y;
            }
            let mut mr = self.math(mop, x, y, y);
            if (d0 >> 25) & 1 != 0 {
                saturate(&mut mr);
            }
            self.write(d0, &r, out); // vector result first...
            let comp = ((s >> 27) & 3) as usize;
            self.regs.alt[((s >> 19) & 3) as usize][comp] = mr[comp]; // ...then math
            return;
        }
        self.write(d0, &r, out);
    }

    /// The instruction after `pc`, following the flow control instructions
    /// (AMD R5xx guide 7.5.5; `VAP_PVS_FLOW_CNTL_*`), once `pc` has run.  A
    /// loop's end and a subroutine's return are live only while that loop or
    /// call is.
    ///
    /// ADDRS: 7:0 activation instruction (the last before the redirect), 15:8
    /// JUMP/JSR target or LOOP count, 23:16 last instruction of the loop or
    /// subroutine, 31:24 loop start or return address.  LOOP_INDEX: 7:0
    /// initial loop index, 15:8 signed step.
    fn next_pc(&mut self, fc: &FlowControl, pc: usize, loops: &mut Vec<LoopFrame>, jsr: &mut Vec<usize>) -> usize {
        if let Some(top) = loops.last_mut() {
            let k = top.fc;
            if ((fc.addrs[k] >> 16) & 0xFF) as usize == pc {
                self.al = self.al.wrapping_add(i32::from((fc.loops[k] >> 8) as u8 as i8));
                top.count -= 1;
                if top.count != 0 {
                    return ((fc.addrs[k] >> 24) & 0xFF) as usize;
                }
                self.al = top.saved_al;
                loops.pop();
                return pc + 1;
            }
        }
        if let Some(&k) = jsr.last()
            && ((fc.addrs[k] >> 16) & 0xFF) as usize == pc
        {
            jsr.pop();
            return ((fc.addrs[k] >> 24) & 0xFF) as usize;
        }
        for k in 0..16 {
            let op = (fc.opc >> (2 * k)) & 3;
            let a = fc.addrs[k];
            if op == 0 || (a & 0xFF) as usize != pc {
                continue;
            }
            return match op {
                // JUMP
                1 => ((a >> 8) & 0xFF) as usize,
                // LOOP
                2 => {
                    if (a >> 8) & 0xFF == 0 || loops.len() == 16 {
                        return pc + 1;
                    }
                    loops.push(LoopFrame { fc: k, count: (a >> 8) & 0xFF, saved_al: self.al });
                    self.al = (fc.loops[k] & 0xFF) as i32;
                    ((a >> 24) & 0xFF) as usize
                }
                // JSR
                _ => {
                    if jsr.len() == 16 {
                        return pc + 1;
                    }
                    jsr.push(k);
                    ((a >> 8) & 0xFF) as usize
                }
            };
        }
        pc + 1
    }
}

/// `fmaxf`: a NaN operand yields the other one; when the operands compare equal
/// (including `-0.0` and `+0.0`) the first is returned.  C leaves the sign of a
/// zero result open and compilers differ, so this fixes it for the port.
pub(crate) fn fmax(a: f32, b: f32) -> f32 {
    if b > a || a.is_nan() {
        b
    } else {
        a
    }
}

/// `fminf`, with the tie and NaN rules of [`fmax`].
pub(crate) fn fmin(a: f32, b: f32) -> f32 {
    if b < a || a.is_nan() {
        b
    } else {
        a
    }
}

fn saturate(r: &mut Vec4) {
    for v in r {
        *v = fmin(fmax(*v, 0.0), 1.0);
    }
}

/// `base ** e` with the PVS special cases, in the documented order of
/// detection.
pub(crate) fn pow_ff(base: f32, e: f32) -> f32 {
    if base == 0.0 {
        // The math engine never produces an infinity: RECIP and LOG return the
        // largest finite value for a zero operand, and Mesa's lowering (select
        // * result) relies on that: 0 * INF would be NaN.
        return if e < 0.0 { f32::MAX } else { 0.0 };
    }
    if e == 0.0 {
        return 1.0;
    }
    if base < 0.0 {
        return -(-base).powf(e);
    }
    base.powf(e)
}

/// C's `frexpf`: `x = m * 2^e` with `m` in [0.5, 1) (zero, infinity and NaN
/// come back unchanged with `e = 0`).
pub(crate) fn frexp(x: f32) -> (f32, i32) {
    if x == 0.0 || !x.is_finite() {
        return (x, 0);
    }
    // A subnormal is scaled into the normal range first.
    let (x, bias) = if x.abs() < f32::MIN_POSITIVE { (x * 16_777_216.0, -24) } else { (x, 0) };
    let bits = x.to_bits();
    let e = ((bits >> 23) & 0xFF) as i32 - 126;
    (f32::from_bits((bits & 0x807F_FFFF) | (126 << 23)), e + bias)
}

/// One line of disassembly of an instruction, for traces.
pub fn disasm_inst(d: &[u32; 4]) -> String {
    const DST_TYPE: [&str; 8] = ["t", "a0", "o", "ox", "at", "i", "?", "?"];
    const SRC_TYPE: [&str; 4] = ["t", "i", "c", "at"];
    const SWZ: &[u8; 8] = b"xyzw01??";
    let d0 = d[0];
    let chan = |bit: u32, name: &'static str| if (d0 >> bit) & 1 != 0 { name } else { "" };
    let engine = if (d0 >> 6) & 1 != 0 {
        "ME"
    } else if (d0 >> 7) & 1 != 0 {
        "MACRO"
    } else {
        "VE"
    };
    let dual = if (d0 >> 6) & 1 == 0 && (d0 >> 28) & 1 != 0 { "+dual" } else { "" };
    let mut line = format!(
        "{engine}{dual} op{:<2} {}{}.{}{}{}{} <-",
        d0 & 0x3F,
        DST_TYPE[((d0 >> 8) & 7) as usize],
        (d0 >> 13) & 0x7F,
        chan(20, "x"),
        chan(21, "y"),
        chan(22, "z"),
        chan(23, "w"),
    );
    for &s in &d[1..] {
        let sw: String = (0..4)
            .map(|i| {
                let c = SWZ[((s >> (13 + 3 * i)) & 7) as usize];
                // Upper case = negated.
                char::from(if s & (1 << (25 + i)) != 0 { c.to_ascii_uppercase() } else { c })
            })
            .collect();
        line += &format!(
            " {}{}[{}{}].{sw}",
            if s & 8 != 0 { "|" } else { "" },
            SRC_TYPE[(s & 3) as usize],
            (s >> 5) & 0xFF,
            if s & 16 != 0 { "+a0" } else { "" },
        );
    }
    line
}
