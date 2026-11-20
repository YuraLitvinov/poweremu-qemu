//! R300-family fragment shader unit (US) to GLSL (`r300_us.c`).
//!
//! Program layout (`US_CONFIG`, `US_CODE_OFFSET`, `US_CODE_ADDR_0..3`): up to
//! four nodes, stored so that the last node is always in `CODE_ADDR_3`.  Each
//! node runs a block of texture instructions and then a block of ALU
//! instructions; starts are relative to the offsets in `US_CODE_OFFSET`.
//!
//! ALU instructions are split into an RGB half and an alpha half, each with
//! three source addresses (temporaries 0-31, constants 32-63), an operand
//! select per argument, a pre-subtract, an op, an output modifier and a clamp.
//! See AMD "R3xx 3D Registers", `US_ALU_{RGB,ALPHA}_{ADDR,INST}`.
//!
//! The generated source (Vulkan GLSL 4.50) holds a pass-through vertex shader
//! for post-transform vertices and a fragment shader that runs the R300
//! program, then does the alpha test, blending and colour-buffer packing in
//! the shader.  This is a 1:1 translation of the C code: the GLSL it produces
//! is byte-for-byte the C translator's.

use core::fmt::Write;

use crate::state::R300State;
use crate::us_text::{
    US_PRELUDE, US_PRELUDE2, US_PRELUDE_COMMON, US_PRELUDE_VS, US_PRELUDE_Z, US_RAW_SSBO, US_RAW_TEX,
};

pub const NUM_TEMPS: usize = 32;
pub const NUM_CONSTS: usize = 32;
pub const MAX_ALU: usize = 64;
pub const MAX_TEX: usize = 32;
pub const NUM_TEX_UNITS: usize = 16;
pub const NUM_VARYINGS: usize = 10;
/// Render targets A-D.
pub const MAX_TARGETS: usize = 4;

const US_CONFIG: u32 = 0x4600;
const US_CODE_OFFSET: u32 = 0x4608;
const US_CODE_ADDR_0: u32 = 0x4610;
const US_TEX_INST_0: u32 = 0x4620;
const US_ALU_RGB_ADDR_0: u32 = 0x46C0;
const US_ALU_ALPHA_ADDR_0: u32 = 0x47C0;
const US_ALU_RGB_INST_0: u32 = 0x48C0;
const US_ALU_ALPHA_INST_0: u32 = 0x49C0;
const US_OUT_FMT_0: u32 = 0x46A4;
const TX_FILTER0_0: u32 = 0x4400;
const TX_FORMAT1_0: u32 = 0x44C0;
const TX_ENABLE: u32 = 0x4104;
const RB3D_CCTL: u32 = 0x4E00;
const RB3D_COLORPITCH0: u32 = 0x4E38;
const VAP_CLIP_CNTL: u32 = 0x221C;

/// Resource bindings of the generated GLSL (descriptor set 0).
pub mod bind {
    /// UBO `R300FSUniforms` (fragment).
    pub const UNIFORMS: u32 = 0;
    /// SSBO uint: `ZB_ZPASS` counter (fragment).
    pub const ZPASS: u32 = 1;
    /// SSBO `R300Vertex[]` (vertex).
    pub const VERTS: u32 = 2;
    /// UBO vec4: multisample clip-space shift.
    pub const MS: u32 = 3;
    /// SSBO uint[]: VRAM (`GLSL_VRAM_SSBO`).
    pub const VRAM: u32 = 4;
    /// SSBO uint[]: GART texels (`GLSL_VRAM_SSBO`).
    pub const AUX: u32 = 5;
    /// Input attachments: colour buffers, then Z.
    pub const FB0: u32 = 8;
    /// Combined image samplers, one per unit.
    pub const TEX0: u32 = 16;
    pub const COUNT: u32 = 32;
}

/// `to_glsl` flag: raw units read VRAM from a storage buffer, not a uint
/// texture view.
pub const GLSL_VRAM_SSBO: u32 = 1 << 0;
/// `to_glsl` flag: colour and depth buffers are storage images (coherent,
/// inside a pixel interlock), not input attachments: the fragment shader reads
/// what earlier fragments of its own draw wrote.
pub const GLSL_FB_IMAGE: u32 = 1 << 1;

/// A vertex as the device hands it to the renderer: `pos` is in clip space
/// with y up, `v[k]` are the values the rasterizer interpolates into fragment
/// temporaries (see [`FsDesc::route`]).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct R300Vertex {
    pub pos: [f32; 4],
    pub v: [[f32; 4]; NUM_VARYINGS],
    /// x: fog value (`GB_SELECT.FOG_SELECT`, `GA_FOG_*`).
    pub aux: [f32; 4],
    /// User clip plane distances (`VAP_CLIP_CNTL`).
    pub ucp: [f32; 8],
}

/// Must match uniform block `R300FSUniforms` in the generated GLSL (std140).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct R300FSUniforms {
    pub consts: [[f32; 4]; NUM_CONSTS],
    pub blend_color: [f32; 4],
    /// R,G,B,A selects: 0-3 XYZW, 4 zero, 5 one.
    pub tex_swz: [[u32; 4]; NUM_TEX_UNITS],
    /// x: 1 bound, y: raw->XYZW (0 as is, 1 .abgr, 2 .grba), z, w: width,
    /// height in texels.
    pub tex_info: [[u32; 4]; NUM_TEX_UNITS],
    pub cblend: u32,
    pub ablend: u32,
    pub chanmask: u32,
    pub alpha_func: u32,
    /// `US_OUT_FMT_0` C0..C3 (0 A, 1 R, 2 G, 3 B).
    pub out_sel: [u32; 4],
    /// Stored bytes are C3,C2,C1,C0.
    pub rt_swap32: u32,
    pub clip_rule: u32,
    /// `RB3D_COLORPITCH0.COLOR_ENDIAN`.
    pub rt_endian: u32,
    /// `RB3D_ROPCNTL`.
    pub rop: u32,
    /// `SC_CLIPRECT` x0, y0, x1, y1 (inclusive).
    pub cliprect: [[i32; 4]; 4],
    /// Depth/stencil: `ZB_CNTL`, `ZB_ZSTENCILCNTL`, `ZB_STENCILREFMASK`, and
    /// `ZFMT_*` (layout of the buffer bound after the colour buffers).
    pub zinfo: [u32; 4],
    /// Count Z-pass samples (`ZB_ZPASS_*`).
    pub zpass_count: u32,
    /// `SU_POLY_OFFSET_ENABLE`.
    pub poly_en: u32,
    pub pad2: [u32; 2],
    /// `TX_BORDER_COLOR`, XYZW before the swizzle.
    pub tex_border: [[f32; 4]; NUM_TEX_UNITS],
    /// LOD bias, finest level (`MAX_MIP_LEVEL`), coarsest level, mip filter
    /// (0 none, 1 nearest, 2 linear).
    pub tex_lod: [[f32; 4]; NUM_TEX_UNITS],
    /// `TEXDIM_*`, level-0 depth, level-0 pitch in bytes, `TEXF_*`.
    pub tex_dim: [[u32; 4]; NUM_TEX_UNITS],
    /// `FG_FOG_COLOR_*`, w: `FG_FOG_FACTOR`.
    pub fog_color: [f32; 4],
    /// `FG_FOG_BLEND`.
    pub fog_blend: u32,
    /// `FG_DEPTH_SRC`.
    pub depth_src: u32,
    pub pad3: [u32; 2],
    /// Front scale, offset, back scale, offset, in units of the [0,1] depth
    /// range.
    pub poly_offset: [f32; 4],
    /// x: byte address of a raw unit's texels, y: 0 in VRAM, 1 in the aux
    /// buffer (`GLSL_VRAM_SSBO` only).
    pub tex_addr: [[u32; 4]; NUM_TEX_UNITS],
    /// The card's layout of each level of a raw unit's texture
    /// (`tex_layout`): group g (0 byte offset, 1 row pitch, 2 face/slice
    /// bytes) of level l is `[g * 3 + l / 4][l % 4]`.
    pub tex_lvl: [[[u32; 4]; 9]; NUM_TEX_UNITS],
}

impl Default for R300FSUniforms {
    fn default() -> Self {
        Self {
            consts: [[0.0; 4]; NUM_CONSTS],
            blend_color: [0.0; 4],
            tex_swz: [[0; 4]; NUM_TEX_UNITS],
            tex_info: [[0; 4]; NUM_TEX_UNITS],
            cblend: 0,
            ablend: 0,
            chanmask: 0,
            alpha_func: 0,
            out_sel: [0; 4],
            rt_swap32: 0,
            clip_rule: 0,
            rt_endian: 0,
            rop: 0,
            cliprect: [[0; 4]; 4],
            zinfo: [0; 4],
            zpass_count: 0,
            poly_en: 0,
            pad2: [0; 2],
            tex_border: [[0.0; 4]; NUM_TEX_UNITS],
            tex_lod: [[0.0; 4]; NUM_TEX_UNITS],
            tex_dim: [[0; 4]; NUM_TEX_UNITS],
            fog_color: [0.0; 4],
            fog_blend: 0,
            depth_src: 0,
            pad3: [0; 2],
            poly_offset: [0.0; 4],
            tex_addr: [[0; 4]; NUM_TEX_UNITS],
            tex_lvl: [[[0; 4]; 9]; NUM_TEX_UNITS],
        }
    }
}

/// `tex_dim[k].w`: component j (storage order) is `TX_FORMAT1` bit 5 + j.
pub const TEXF_SIGNED_X: u32 = 1 << 0;
pub const TEXF_GAMMA: u32 = 1 << 4;
/// Heights round up to a power of two.
pub const TEXF_POT_ROWS: u32 = 1 << 5;

/// `ZB_DEPTHPITCH.DEPTHENDIAN`.
pub const ZFMT_ENDIAN_MASK: u32 = 3;
/// 16-bit Z, no stencil.
pub const ZFMT_Z16: u32 = 1 << 2;

/// What the translator needs besides the US registers themselves.
#[derive(Clone, Debug)]
pub struct FsDesc<'a> {
    /// Temp <- varying index, or -1.
    pub route: [i8; NUM_TEMPS],
    /// The whole vertex stage as GLSL (vertex program on the GPU), if any.
    pub vs_gpu: Option<&'a str>,
}

/// Colour buffer layout, from `RB3D_COLORPITCH0.COLORFORMAT` and
/// `US_OUT_FMT_0.OUT_FMT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CbView {
    /// Unsupported combination.
    None = 0,
    /// ARGB8888 with C4_8: float RGBA8 view.
    Rgba8,
    /// 1-byte pixels, uint views ...
    R8u,
    R16u,
    R32u,
    Rg32u,
    Rgba32u,
}

/// Texture formats (`TX_FORMAT1.TXFORMAT`) the shader decodes itself from the
/// texel words, read through a uint view of guest memory: returns the texel
/// size in bytes, or 0 for the formats sampled through a float texture (X8,
/// X16, Y8X8, the 16-bit colour formats, W8Z8Y8X8, DXT) and for ones not
/// implemented.  The uint view's element is 4 bytes for texels of up to 4
/// bytes, else the texel.
///
/// The card's view of a texel dword follows the rule the 32-bit formats
/// established: dword = `TXO_ENDIAN` swap of the big-endian dword in VRAM
/// (VRAM holds the guest CPU's side of its byte-swapping aperture), and the
/// texel at the lowest address is in the low bits.  GART copies are
/// byte-reversed per dword on upload so the same rule holds.
pub fn tex_raw_bpp(txformat: u32) -> u32 {
    match txformat & 0x1F {
        // Y4X4, Z3Y3X2
        0x02 | 0x05 => 1,
        // X16 (16-bit depth): 16 bits of precision, not 8; CxV8U8, VYUY, YVYU;
        // 16 MPEG, 16F
        0x01 | 0x12 | 0x14 | 0x15 | 0x16 | 0x18 => 2,
        // Y16X16, 11/11/10, 2/10/10/10; AVYU, 16_16 MPEG, 16F x2, 32F
        0x04 | 0x08 | 0x09 | 0x0D | 0x13 | 0x17 | 0x19 | 0x1B => 4,
        // W16Z16Y16X16, 16F x4, 32F x2
        0x0E | 0x1A | 0x1C => 8,
        // 32F x4
        0x1D => 16,
        _ => 0,
    }
}

/// The colour buffer view and bytes per pixel for a colour format and
/// `US_OUT_FMT`.
pub fn cb_view(colorformat: u32, outfmt: u32) -> (CbView, u32) {
    let of = outfmt & 0x1F;
    match colorformat & 0xF {
        // I8
        9 => (CbView::R8u, 1),
        // 1555, 565, UV88, 4444
        3 | 4 | 13 | 15 => (CbView::R16u, 2),
        // ARGB8888.  C4_10 (Tiger's desktop) still stores 8 bits a channel
        // here: the buffer format decides the layout, and 10-bit storage is
        // the R5xx ARGB2101010.  Other OUT_FMTs are Mesa's aliases (RG16,
        // R32F, RG16F) of a 32-bit pixel.
        6 => (if of <= 2 { CbView::Rgba8 } else { CbView::R32u }, 4),
        // ARGB16161616
        10 => (CbView::Rg32u, 8),
        // ARGB32323232
        7 => (CbView::Rgba32u, 16),
        // Reserved, YUV 4:2:2
        _ => (CbView::None, 4),
    }
}

/// `TX_FORMAT1.TXFORMAT` 16F..32F x4: the card cannot filter them.
fn tex_is_float(fmt: u32) -> bool {
    (0x18..=0x1D).contains(&fmt)
}

/// R300 US constants are 24-bit floats (1 sign, 7 exponent, 16 mantissa).
pub fn float24(v: u32) -> f32 {
    let mant = v & 0xFFFF;
    let exp = (v >> 16) & 0x7F;
    let negative = (v >> 23) & 1 != 0;
    if exp == 0 && mant == 0 {
        return if negative { -0.0 } else { 0.0 };
    }
    // Bias 63; the top exponent is a large finite value, not infinity.  Both
    // factors are exact in f32, so the product is too.
    let scale = f32::from_bits((exp + 127 - 63) << 23);
    let f = (1.0 + mant as f32 / 65536.0) * scale;
    if negative {
        -f
    } else {
        f
    }
}

#[derive(Clone, Copy, Default)]
struct Node {
    alu_start: usize,
    alu_count: usize,
    tex_start: usize,
    tex_count: usize,
}

/// Decodes the node list.
fn nodes(st: &R300State) -> Vec<Node> {
    let config = st.reg(US_CONFIG);
    let off = st.reg(US_CODE_OFFSET);
    let n = (config & 3) as usize + 1;
    let (alu_off, tex_off) = ((off & 0x3F) as usize, ((off >> 13) & 0x1F) as usize);
    (0..n)
        .map(|i| {
            let a = st.reg(US_CODE_ADDR_0 + 4 * (4 - n + i) as u32);
            Node {
                alu_start: alu_off + (a & 0x3F) as usize,
                alu_count: ((a >> 6) & 0x3F) as usize + 1,
                tex_start: tex_off + ((a >> 12) & 0x1F) as usize,
                // FIRST_NODE_HAS_TEX clear.
                tex_count: if i == 0 && config & (1 << 3) == 0 { 0 } else { ((a >> 17) & 0x1F) as usize + 1 },
            }
        })
        .collect()
}

// ---- source expressions -------------------------------------------------

fn addr_expr(addr: u32) -> String {
    if addr & 32 != 0 {
        format!("u.consts[{}]", addr & 31)
    } else {
        format!("t[{}]", addr & 31)
    }
}

/// RGB operand select (`US_ALU_RGB_INST.SEL_*`) as a vec3 expression.
fn rgb_sel(sel: u32) -> String {
    const COMP: [&str; 4] = ["rgb", "rrr", "ggg", "bbb"];
    match sel {
        0..=11 => format!("cs{}.{}", sel / 4, COMP[(sel % 4) as usize]),
        12..=14 => format!("vec3(as{}.a)", sel - 12),
        15..=18 => format!("ps.{}", COMP[(sel - 15) as usize]),
        19 => "vec3(psa)".into(),
        20 => "vec3(0.0)".into(),
        21 => "vec3(1.0)".into(),
        22 => "vec3(0.5)".into(),
        23..=25 => format!("cs{}.gbr", sel - 23),
        26..=28 => format!("cs{}.brg", sel - 26),
        _ => format!("vec3(as{0}.a, cs{0}.b, cs{0}.g)", sel - 29),
    }
}

/// Alpha operand select (`US_ALU_ALPHA_INST.SEL_*`) as a float expression.
fn alpha_sel(sel: u32) -> String {
    const COMP: [char; 3] = ['r', 'g', 'b'];
    match sel {
        0..=8 => format!("cs{}.{}", sel / 3, COMP[(sel % 3) as usize]),
        9..=11 => format!("as{}.a", sel - 9),
        12..=14 => format!("ps.{}", COMP[(sel - 12) as usize]),
        15 => "psa".into(),
        16 => "0.0".into(),
        17 => "1.0".into(),
        _ => "0.5".into(),
    }
}

fn apply_mod(expr: &str, m: u32) -> String {
    match m {
        1 => format!("(-{expr})"),
        2 => format!("abs({expr})"),
        3 => format!("(-abs({expr}))"),
        _ => format!("({expr})"),
    }
}

fn omod(m: u32) -> &'static str {
    ["", " * 2.0", " * 4.0", " * 8.0", " * 0.5", " * 0.25", " * 0.125", ""][(m & 7) as usize]
}

fn presub_rgb(op: u32) -> &'static str {
    ["1.0 - 2.0 * cs0.rgb", "cs1.rgb - cs0.rgb", "cs1.rgb + cs0.rgb", "1.0 - cs0.rgb"][(op & 3) as usize]
}

fn presub_alpha(op: u32) -> &'static str {
    ["1.0 - 2.0 * as0.a", "as1.a - as0.a", "as1.a + as0.a", "1.0 - as0.a"][(op & 3) as usize]
}

/// A 3-bit write mask as swizzle letters.
fn mask_str(m: u32) -> String {
    [(1, 'r'), (2, 'g'), (4, 'b')].iter().filter(|(bit, _)| m & bit != 0).map(|&(_, c)| c).collect()
}

// ---- instructions -------------------------------------------------------

/// The `r300_texu()` arguments for a unit whose texels the shader decodes.
fn texu_args(st: &R300State, unit: u32) -> String {
    let f1 = st.reg(TX_FORMAT1_0 + 4 * unit);
    let f0 = st.reg(TX_FILTER0_0 + 4 * unit);
    let fmt = f1 & 0x1F;
    // filt: bit 0 magnify linear, bit 1 minify linear, S wrap 6:4, T wrap 10:8,
    // R wrap 14:12 (TX_FILTER0 CLAMP_S/T/R)
    let mut filt = ((f0 & 7) << 4) | (((f0 >> 3) & 7) << 8) | (((f0 >> 6) & 7) << 12);
    if !tex_is_float(fmt) {
        filt |= u32::from((f0 >> 9) & 3 != 1) | (u32::from((f0 >> 11) & 3 != 1) << 1);
    }
    // A little-endian guest's texel words are already the card's: endian 2
    // undoes the big-endian view the shader starts from (the same rule as
    // R300TexDesc.endian in set_textures).  The 32-bit float formats
    // 0x1B..0x1D are left as the register says: Mesa also uses them as raw
    // copy sources (RGBA8 and luminance uploads and read-backs go through a
    // float staging texture), and those paths only come out right with the old
    // rule (see work fix_piglit_texture_upload_copy_formats), so real 32F
    // textures stay wrong for now.
    format!(
        "0x{fmt:x}u, {}u, 2u, 0x{filt:x}u, {}",
        tex_raw_bpp(fmt),
        if (f1 >> 22) & 1 != 0 { "true" } else { "false" }
    )
}

fn unit_is_raw(st: &R300State, unit: u32) -> bool {
    (st.reg(TX_ENABLE) >> unit) & 1 != 0 && tex_raw_bpp(st.reg(TX_FORMAT1_0 + 4 * unit)) != 0
}

/// `TX_FORMAT1.TEX_COORD_TYPE` of an enabled unit: 0 2D, 1 3D, 2 cube.
fn unit_dim(st: &R300State, unit: u32) -> usize {
    let d = (st.reg(TX_FORMAT1_0 + 4 * unit) >> 25) & 3;
    if (st.reg(TX_ENABLE) >> unit) & 1 != 0 && d <= 2 {
        d as usize
    } else {
        0
    }
}

/// Border handling of a unit sampled through a float texture: per axis 1 =
/// clamp to border, 2 = mirror once to border (S bits 1:0, T 3:2, R 5:4), bit
/// 6 linear filtering.  The Metal sampler clamps to transparent black and
/// `r300_tex` adds `TX_BORDER_COLOR` for the missing coverage (Metal has no
/// arbitrary border colours).
fn unit_border(st: &R300State, unit: u32) -> u32 {
    let f0 = st.reg(TX_FILTER0_0 + 4 * unit);
    let dim = unit_dim(st, unit);
    if dim == 2 {
        return 0; // cube maps never reach a border
    }
    let axes = if dim == 1 { 3 } else { 2 };
    let mut bm = 0u32;
    let lin = (f0 >> 9) & 3 != 1 || (f0 >> 11) & 3 != 1;
    for a in 0..axes {
        let m = (f0 >> (3 * a)) & 7;
        let mut v = match m {
            6 => 1,
            7 => 2,
            _ => 0,
        };
        // Half-border clamps (GL_CLAMP, GL_MIRROR_CLAMP_EXT): a nearest fetch
        // is the edge texel, but a linear fetch blends the edge with the
        // border colour, as clamp to border does.  The sampler clamps to the
        // edge for these, so the border mixes in rather than adds: field
        // value 3, with bits 7..9 marking the mirror variant (the coordinate
        // is clamped to [0, 1] first, after abs() for the mirror).
        if lin && (m == 4 || m == 5) {
            v = 3;
            if m == 5 {
                bm |= 128 << a; // mirror first (bits 7..9 per axis)
            }
        }
        bm |= v << (2 * a);
    }
    if bm != 0 && lin {
        bm |= 64;
    }
    if std::env::var_os("PPCGPU_BORDER_LOG").is_some() {
        eprintln!("BORDER unit {unit} TX_FILTER0 {f0:08x} -> bm {bm:x} (lin {})", u32::from(lin));
    }
    bm
}

/// The texture unit a fetch instruction samples.  The ID in the instruction is
/// relative to the texture offset in `US_CODE_OFFSET` (bits 17:13), modulo 16.
/// Mesa leaves that offset at 0, so its IDs read as absolute; the Windows
/// driver instead loads one fetch instruction (ID 0) into slot N and runs it
/// with offset N, to select texture unit N without rewriting the program.
/// (Seen with the Catalyst 10.2 driver: slot 7 and 23 with unit 7 enabled,
/// slot 3 and 19 with unit 3, ID always 0.  Read literally, it samples the
/// unbound unit 0 and the result is black.)
fn tex_unit(st: &R300State, inst: u32) -> u32 {
    (((inst >> 11) & 0xF) + ((st.reg(US_CODE_OFFSET) >> 13) & 0x1F)) & 0xF
}

fn emit_tex(out: &mut String, st: &R300State, inst: u32, units_used: &mut u32) -> Result<(), &'static str> {
    let (src, dst) = (inst & 0x1F, (inst >> 6) & 0x1F);
    let (unit, op) = (tex_unit(st, inst), (inst >> 15) & 7);
    let (proj, bias) = match op {
        // NOP
        0 => return Ok(()),
        // KIL
        2 => {
            let _ = writeln!(out, "    if (any(lessThan(t[{src}], vec4(0.0)))) r300_dead = true;");
            return Ok(());
        }
        // LD
        1 => ("false", "0.0".to_string()),
        // TXP
        3 => ("true", "0.0".to_string()),
        // TXB: bias in the source's w
        4 => ("false", format!("t[{src}].w")),
        _ => return Err("unknown texture instruction"),
    };
    if unit_is_raw(st, unit) {
        let _ = writeln!(
            out,
            "    t[{dst}] = r300_texu(tex{unit}, {unit}u, t[{src}], {proj}, {bias}, {});",
            texu_args(st, unit)
        );
    } else {
        let _ = writeln!(
            out,
            "    t[{dst}] = r300_tex(tex{unit}, {unit}u, t[{src}], {proj}, {bias}, {}u);",
            unit_border(st, unit)
        );
    }
    *units_used |= 1 << unit;
    Ok(())
}

fn emit_alu(out: &mut String, st: &R300State, i: usize) {
    let i4 = 4 * i as u32;
    let ra = st.reg(US_ALU_RGB_ADDR_0 + i4);
    let aa = st.reg(US_ALU_ALPHA_ADDR_0 + i4);
    let ri = st.reg(US_ALU_RGB_INST_0 + i4);
    let ai = st.reg(US_ALU_ALPHA_INST_0 + i4);
    let (rop, aop) = ((ri >> 23) & 0xF, (ai >> 23) & 0xF);

    let _ = writeln!(out, "    {{ // alu {i}: rgb {ra:08x}/{ri:08x} alpha {aa:08x}/{ai:08x}");
    for k in 0..3 {
        let _ = write!(out, "        vec4 cs{k} = {};", addr_expr((ra >> (6 * k)) & 0x3F));
        let _ = writeln!(out, " vec4 as{k} = {};", addr_expr((aa >> (6 * k)) & 0x3F));
    }
    let _ = writeln!(
        out,
        "        vec3 ps = {}; float psa = {};",
        presub_rgb((ri >> 21) & 3),
        presub_alpha((ai >> 21) & 3)
    );

    // RGB arguments A, B, C
    for k in 0..3u32 {
        let e = apply_mod(&rgb_sel((ri >> (7 * k)) & 0x1F), (ri >> (7 * k + 5)) & 3);
        let _ = writeln!(out, "        vec3 r{} = {e};", char::from(b'A' + k as u8));
    }
    for k in 0..3u32 {
        let e = apply_mod(&alpha_sel((ai >> (7 * k)) & 0x1F), (ai >> (7 * k + 5)) & 3);
        let _ = writeln!(out, "        float a{} = {e};", char::from(b'A' + k as u8));
    }

    // The dot product feeds both halves (alpha OP_DP takes it).
    let dp = match rop {
        2 => "dot(rA, rB) + aA * aB",
        3 => "rA.r * rB.r + rA.g * rB.g + rC.b",
        _ => "dot(rA, rB)",
    };
    let _ = writeln!(out, "        float dp = {dp};");

    // Alpha result first: OP_SOP hands it to the RGB half.
    let ar = match aop {
        0 => "aA * aB + aC",
        1 => "dp",
        2 => "min(aA, aB)",
        3 => "max(aA, aB)",
        5 => "aC > 0.5 ? aA : aB",
        6 => "aC >= 0.0 ? aA : aB",
        7 => "fract(aA)",
        8 => "exp2(aA)",
        9 => "log2(aA)",
        10 => "1.0 / aA",
        11 => "inversesqrt(aA)",
        _ => "0.0",
    };
    let _ = writeln!(out, "        float ar = {ar};");
    let rr = match rop {
        0 => "rA * rB + rC",
        1..=3 => "vec3(dp)",
        4 => "min(rA, rB)",
        5 => "max(rA, rB)",
        7 => "mix(rB, rA, greaterThan(rC, vec3(0.5)))",
        8 => "mix(rB, rA, greaterThanEqual(rC, vec3(0.0)))",
        9 => "fract(rA)",
        10 => "vec3(ar)",
        _ => "vec3(0.0)",
    };
    let _ = writeln!(out, "        vec3 rr = {rr};");
    let _ = writeln!(out, "        rr = rr{}; ar = ar{};", omod((ri >> 27) & 7), omod((ai >> 27) & 7));
    if (ri >> 30) & 1 != 0 {
        out.push_str("        rr = clamp(rr, 0.0, 1.0);\n");
    }
    if (ai >> 30) & 1 != 0 {
        out.push_str("        ar = clamp(ar, 0.0, 1.0);\n");
    }

    // Writes: temporaries, the output FIFO of render target TARGET, and (alpha
    // OMASK_W) the fragment's depth.
    let m = mask_str((ra >> 23) & 7);
    if !m.is_empty() {
        let _ = writeln!(out, "        t[{}].{m} = rr.{m};", (ra >> 18) & 0x1F);
    }
    let m = mask_str((ra >> 26) & 7);
    if !m.is_empty() {
        let _ = writeln!(out, "        oc[{}].{m} = rr.{m};", (ra >> 29) & 3);
    }
    if (aa >> 23) & 1 != 0 {
        let _ = writeln!(out, "        t[{}].a = ar;", (aa >> 18) & 0x1F);
    }
    if (aa >> 24) & 1 != 0 {
        let _ = writeln!(out, "        oc[{}].a = ar;", (aa >> 25) & 3);
    }
    if (aa >> 27) & 1 != 0 {
        out.push_str("        ow = ar;\n");
    }
    out.push_str("    }\n");
}

/// Render targets the active program writes (bit n: target n), and whether it
/// writes depth (`OMASK_W`).
fn targets_written(st: &R300State) -> (u32, bool) {
    let (mut mask, mut writes_w) = (0u32, false);
    for node in nodes(st) {
        for a in node.alu_start..node.alu_start + node.alu_count {
            if a >= MAX_ALU {
                break;
            }
            let ra = st.reg(US_ALU_RGB_ADDR_0 + 4 * a as u32);
            let aa = st.reg(US_ALU_ALPHA_ADDR_0 + 4 * a as u32);
            if (ra >> 26) & 7 != 0 {
                mask |= 1 << ((ra >> 29) & 3);
            }
            if (aa >> 24) & 1 != 0 {
                mask |= 1 << ((aa >> 25) & 3);
            }
            if (aa >> 27) & 1 != 0 {
                writes_w = true;
            }
        }
    }
    (mask, writes_w)
}

/// `US_OUT_FMT` of render target `k` (target A's for a multiwrite target left
/// UNUSED).
pub fn out_fmt(st: &R300State, k: usize) -> u32 {
    let f = st.reg(US_OUT_FMT_0 + 4 * k as u32);
    // multiwrite targets without their own format use target A's
    if k != 0 && f & 0x1F == 15 {
        st.reg(US_OUT_FMT_0)
    } else {
        f
    }
}

/// Colour buffers a draw binds: 1 + the highest render target (B-D) the
/// program writes whose `US_OUT_FMT` is not UNUSED, or `RB3D_CCTL`'s
/// `NUM_MULTIWRITES` (target A replicated), whichever is larger.
pub fn num_targets(st: &R300State) -> usize {
    let (written, _) = targets_written(st);
    let mw = ((st.reg(RB3D_CCTL) >> 5) & 3) as usize + 1;
    let mut n = 1;
    for k in 1..MAX_TARGETS {
        if (written >> k) & 1 != 0 && st.reg(US_OUT_FMT_0 + 4 * k as u32) & 0x1F != 15 {
            n = k + 1;
        }
    }
    n.max(mw)
}

// ---- colour buffers -----------------------------------------------------

fn tf(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

/// The card's word(s): swaps are their own inverse.
fn word_load(bpp: u32, e: u32) -> String {
    match bpp {
        1 => "uint w = fb;".into(),
        2 if e == 1 || e == 2 => "uint w = ((fb >> 8) & 0xffu) | ((fb & 0xffu) << 8);".into(),
        2 => "uint w = fb & 0xffffu;".into(),
        4 => format!("uint w = r300_cswap(fb, {e}u);"),
        8 => format!("uvec2 w = uvec2(r300_cswap(fb.x, {e}u), r300_cswap(fb.y, {e}u));"),
        _ => format!(
            "uvec4 w = uvec4(r300_cswap(fb.x, {e}u), r300_cswap(fb.y, {e}u), r300_cswap(fb.z, {e}u), r300_cswap(fb.w, {e}u));"
        ),
    }
}

/// Pack: the word(s), then the same swap back.
fn word_store(bpp: u32, e: u32) -> String {
    match bpp {
        1 => "return w & 0xffu;".into(),
        2 if e == 1 || e == 2 => "return ((w >> 8) & 0xffu) | ((w & 0xffu) << 8);".into(),
        2 => "return w & 0xffffu;".into(),
        4 => format!("return r300_cswap(w, {e}u);"),
        8 => format!("return uvec2(r300_cswap(w.x, {e}u), r300_cswap(w.y, {e}u));"),
        _ => format!(
            "return uvec4(r300_cswap(w.x, {e}u), r300_cswap(w.y, {e}u), r300_cswap(w.z, {e}u), r300_cswap(w.w, {e}u));"
        ),
    }
}

/// The GLSL that unpacks and packs one colour buffer's pixel words, per
/// colour format `cf` and `US_OUT_FMT` `of`.  Returns (unpack, pack, stored
/// components of C0..C3 the buffer holds).
fn cb_codec(cf: u32, of: u32, sg: [bool; 4]) -> (String, String, u32) {
    let s = |n: usize| tf(sg[n]);
    let mut stored = 0xF;
    let (ub, pb) = match cf {
        // I8 stores C2 (Mesa r300_translate_out_fmt)
        9 => {
            stored = 0x4;
            (format!("h.z = r300_upk(w, 0u, 8u, {});", s(2)), format!("uint w = r300_pk(h.z, 0u, 8u, {});", s(2)))
        }
        // C_5_6_5
        4 => (
            "h = vec4(r300_upk(w, 0u, 5u, false), r300_upk(w, 5u, 6u, false), r300_upk(w, 11u, 5u, false), 1.0);".into(),
            "uint w = r300_pk(h.x, 0u, 5u, false) | r300_pk(h.y, 5u, 6u, false) | r300_pk(h.z, 11u, 5u, false);".into(),
        ),
        // C_1_5_5_5
        3 => (
            "h = vec4(r300_upk(w, 0u, 5u, false), r300_upk(w, 5u, 5u, false), r300_upk(w, 10u, 5u, false), r300_upk(w, 15u, 1u, false));".into(),
            "uint w = r300_pk(h.x, 0u, 5u, false) | r300_pk(h.y, 5u, 5u, false) | r300_pk(h.z, 10u, 5u, false) | r300_pk(h.w, 15u, 1u, false);".into(),
        ),
        // C4_4
        15 => (
            "h = vec4(r300_upk(w, 0u, 4u, false), r300_upk(w, 4u, 4u, false), r300_upk(w, 8u, 4u, false), r300_upk(w, 12u, 4u, false));".into(),
            "uint w = r300_pk(h.x, 0u, 4u, false) | r300_pk(h.y, 4u, 4u, false) | r300_pk(h.z, 8u, 4u, false) | r300_pk(h.w, 12u, 4u, false);".into(),
        ),
        // UV88: C2 low byte, C0 high (Mesa); C_16 / C_16_FP: C0
        13 => {
            if of == 3 {
                stored = 0x1;
                (format!("h.x = r300_upk(w, 0u, 16u, {});", s(0)), format!("uint w = r300_pk(h.x, 0u, 16u, {});", s(0)))
            } else if of == 16 {
                stored = 0x1;
                ("h.x = r300_h16(w);".into(), "uint w = r300_ph16(h.x);".into())
            } else {
                stored = 0x5;
                (
                    format!("h.z = r300_upk(w, 0u, 8u, {}); h.x = r300_upk(w, 8u, 8u, {});", s(2), s(0)),
                    format!("uint w = r300_pk(h.z, 0u, 8u, {}) | r300_pk(h.x, 8u, 8u, {});", s(2), s(0)),
                )
            }
        }
        // 32-bit pixels other than C4_8
        6 => match of {
            3 => {
                stored = 0x1;
                (format!("h.x = r300_upk(w, 0u, 16u, {});", s(0)), format!("uint w = r300_pk(h.x, 0u, 16u, {});", s(0)))
            }
            16 => {
                stored = 0x1;
                ("h.x = r300_h16(w);".into(), "uint w = r300_ph16(h.x);".into())
            }
            17 => {
                stored = 0x3;
                (
                    "h.x = r300_h16(w); h.y = r300_h16(w >> 16);".into(),
                    "uint w = r300_ph16(h.x) | (r300_ph16(h.y) << 16);".into(),
                )
            }
            19 => {
                stored = 0x1;
                ("h.x = uintBitsToFloat(w);".into(), "uint w = floatBitsToUint(h.x);".into())
            }
            // C2_16
            _ => {
                stored = 0x3;
                (
                    format!("h.x = r300_upk(w, 0u, 16u, {}); h.y = r300_upk(w, 16u, 16u, {});", s(0), s(1)),
                    format!("uint w = r300_pk(h.x, 0u, 16u, {}) | r300_pk(h.y, 16u, 16u, {});", s(0), s(1)),
                )
            }
        },
        // 64-bit pixels
        10 => {
            if of == 18 {
                (
                    "h = vec4(r300_h16(w.x), r300_h16(w.x >> 16), r300_h16(w.y), r300_h16(w.y >> 16));".into(),
                    "uvec2 w = uvec2(r300_ph16(h.x) | (r300_ph16(h.y) << 16), r300_ph16(h.z) | (r300_ph16(h.w) << 16));".into(),
                )
            } else if of == 20 {
                // RG32F in an ARGB16161616 buffer stores C0 and C2 (Mesa
                // r300_translate_out_fmt: "stores XZ")
                stored = 0x5;
                (
                    "h.x = uintBitsToFloat(w.x); h.z = uintBitsToFloat(w.y);".into(),
                    "uvec2 w = uvec2(floatBitsToUint(h.x), floatBitsToUint(h.z));".into(),
                )
            } else {
                // C4_16
                (
                    format!(
                        "h = vec4(r300_upk(w.x, 0u, 16u, {}), r300_upk(w.x, 16u, 16u, {}), r300_upk(w.y, 0u, 16u, {}), r300_upk(w.y, 16u, 16u, {}));",
                        s(0), s(1), s(2), s(3)
                    ),
                    format!(
                        "uvec2 w = uvec2(r300_pk(h.x, 0u, 16u, {}) | r300_pk(h.y, 16u, 16u, {}), r300_pk(h.z, 0u, 16u, {}) | r300_pk(h.w, 16u, 16u, {}));",
                        s(0), s(1), s(2), s(3)
                    ),
                )
            }
        }
        // 7: 128-bit pixels, C4_32_FP (C2_32_FP fills two)
        _ => {
            if of == 20 {
                stored = 0x3;
                (
                    "h.x = uintBitsToFloat(w.x); h.y = uintBitsToFloat(w.y);".into(),
                    "uvec4 w = uvec4(floatBitsToUint(h.x), floatBitsToUint(h.y), 0u, 0u);".into(),
                )
            } else {
                ("h = uintBitsToFloat(w);".into(), "uvec4 w = floatBitsToUint(h);".into())
            }
        }
    };
    (ub, pb, stored)
}

fn emit_cb(out: &mut String, st: &R300State, k: usize, flags: u32) {
    let pitch = st.reg(RB3D_COLORPITCH0 + 4 * k as u32);
    let (cf, e) = ((pitch >> 21) & 0xF, (pitch >> 19) & 3);
    let ofr = out_fmt(st, k);
    let of = ofr & 0x1F;
    let (view, bpp) = cb_view(cf, of);
    // The R300 blends and clamps only its 8-bit-per-channel formats.
    let fp = (16..=21).contains(&of);
    let blend = view == CbView::Rgba8 || cf == 3 || cf == 4 || cf == 15 || cf == 9 || (cf == 13 && of == 0);

    let (sel, swap) = if k == 0 {
        ("u.out_sel".to_string(), "u.rt_swap32".to_string())
    } else {
        (
            format!("uvec4({}u, {}u, {}u, {}u)", (ofr >> 8) & 3, (ofr >> 10) & 3, (ofr >> 12) & 3, (ofr >> 14) & 3),
            format!("{}u", u32::from(e == 2)),
        )
    };
    let sg = [0, 1, 2, 3].map(|n| (ofr >> (16 + n)) & 1 != 0);
    let image = flags & GLSL_FB_IMAGE != 0;
    let fb = bind::FB0 + k as u32;

    if view == CbView::Rgba8 || view == CbView::None {
        let fbdecl = if image {
            format!(
                "layout(set = 0, binding = {fb}, rgba8) coherent uniform image2D fbin{k};\n#define R300_FB{k}_LOAD imageLoad(fbin{k}, ivec2(gl_FragCoord.xy))\n"
            )
        } else {
            format!(
                "layout(input_attachment_index = {k}, set = 0, binding = {fb}) uniform subpassInput fbin{k};\n#define R300_FB{k}_LOAD subpassLoad(fbin{k})\n"
            )
        };
        let _ = write!(
            out,
            "#define r300_cb{k}_t vec4\n#define r300_cb{k}_o vec4\n#define R300_CB{k}_OUT(x) (x)\n\
             #define R300_CB{k}_CLAMP true\n#define R300_CB{k}_BLEND true\n\
             #define R300_CB{k}_SEL {sel}\n\
             {fbdecl}\
             vec4 r300_cb{k}_unpack(vec4 fb) {{ return r300_unpack(fb, {sel}, {swap}); }}\n\
             vec4 r300_cb{k}_pack(vec4 c) {{ return r300_pack(c, {sel}, {swap}); }}\n\n"
        );
        return;
    }
    let ty = match view {
        CbView::Rg32u => "uvec2",
        CbView::Rgba32u => "uvec4",
        _ => "uint",
    };
    let load = match view {
        CbView::Rg32u => ".xy",
        CbView::Rgba32u => "",
        _ => ".x",
    };
    // Outputs are uvec4 like the input attachment: Metal wants a fetched
    // attachment's input and output types to agree.
    let pad = match view {
        CbView::Rg32u => "uvec4(x, 0u, 0u)",
        CbView::Rgba32u => "(x)",
        _ => "uvec4(x, 0u, 0u, 0u)",
    };
    let fbdecl = if image {
        let fmt = match bpp {
            1 => "r8ui",
            2 => "r16ui",
            8 => "rg32ui",
            16 => "rgba32ui",
            _ => "r32ui",
        };
        format!(
            "layout(set = 0, binding = {fb}, {fmt}) coherent uniform uimage2D fbin{k};\n#define R300_FB{k}_LOAD imageLoad(fbin{k}, ivec2(gl_FragCoord.xy)){load}\n"
        )
    } else {
        format!(
            "layout(input_attachment_index = {k}, set = 0, binding = {fb}) uniform usubpassInput fbin{k};\n#define R300_FB{k}_LOAD subpassLoad(fbin{k}){load}\n"
        )
    };
    let _ = write!(
        out,
        "// colour buffer {k}: format {cf}, US_OUT_FMT {ofr:08x}, endian {e}, {bpp} bytes\n\
         #define r300_cb{k}_t {ty}\n#define r300_cb{k}_o uvec4\n#define R300_CB{k}_OUT(x) {pad}\n\
         #define R300_CB{k}_CLAMP {}\n#define R300_CB{k}_BLEND {}\n\
         #define R300_CB{k}_SEL {sel}\n\
         {fbdecl}",
        tf(!fp),
        tf(blend)
    );

    let swb = word_load(bpp, e);
    let (ub, pb, stored) = cb_codec(cf, of, sg);
    let back = word_store(bpp, e);
    let _ = write!(
        out,
        "vec4 r300_cb{k}_unpack(r300_cb{k}_t fb)\n\
         {{\n    {swb}\n    vec4 h = vec4(0.0, 0.0, 0.0, 1.0);\n    {ub}\n\
         \x20   return r300_from_hw_m(h, {sel}, {stored}u);\n}}\n\
         r300_cb{k}_t r300_cb{k}_pack(vec4 c)\n\
         {{\n    vec4 h = r300_to_hw(c, {sel});\n    {pb}\n    {back}\n}}\n\n"
    );
}

/// Blend, ROP and write mask for colour buffer `k` from shader output `src`
/// (`None`: the target is not written).
fn emit_target(out: &mut String, k: usize, src: Option<usize>) {
    let Some(src) = src else {
        let _ = writeln!(out, "    o.c{k} = fb{k};      // target {k}: not written");
        return;
    };
    let _ = write!(
        out,
        "    {{   // target {k}\n\
         \x20       /* fixed-point buffers blend the clamped fragment colour */\n\
         \x20       vec4 s = R300_CB{k}_CLAMP ? clamp(oc[{src}], 0.0, 1.0) : oc[{src}];\n\
         \x20       vec4 d = r300_cb{k}_unpack(fb{k});\n\
         \x20       vec4 res = R300_CB{k}_CLAMP ? clamp(s, 0.0, 1.0) : s;\n\
         \x20       if (R300_CB{k}_BLEND && (u.cblend & 1u) != 0u) {{\n\
         \x20           uint ab = (u.cblend & 2u) != 0u ? u.ablend : u.cblend;\n\
         \x20           uint cs = (u.cblend >> 16) & 63u, cd = (u.cblend >> 24) & 63u;\n\
         \x20           uint as_ = (ab >> 16) & 63u, ad = (ab >> 24) & 63u;\n\
         \x20           for (uint c = 0u; c < 3u; c++)\n\
         \x20               res[c] = r300_combine(u.cblend, s[c], r300_bfactor(cs, s, d, u.blend_color, c),\n\
         \x20                                     d[c], r300_bfactor(cd, s, d, u.blend_color, c));\n\
         \x20           res.a = r300_combine(ab, s.a, r300_bfactor(as_, s, d, u.blend_color, 3u),\n\
         \x20                                d.a, r300_bfactor(ad, s, d, u.blend_color, 3u));\n\
         \x20           if (keep) res = d;\n\
         \x20       }}\n\
         \x20       if ((u.rop & 4u) != 0u)\n\
         \x20           res = r300_cb{k}_unpack(r300_rop(r300_cb{k}_pack(R300_CB{k}_CLAMP ? clamp(s, 0.0, 1.0) : s),\n\
         \x20                                           fb{k}, (u.rop >> 8) & 15u));\n\
         \x20       /* RB3D_COLOR_CHANNEL_MASK: one bit per stored component, through the buffer's channel selects */\n\
         \x20       uint cm = r300_lmask(u.chanmask, R300_CB{k}_SEL);\n\
         \x20       res = vec4((cm & 1u) != 0u ? res.r : d.r, (cm & 2u) != 0u ? res.g : d.g,\n\
         \x20                  (cm & 4u) != 0u ? res.b : d.b, (cm & 8u) != 0u ? res.a : d.a);\n\
         \x20       o.c{k} = r300_cb{k}_pack(res);\n\
         \x20   }}\n"
    );
}

/// Builds the GLSL source for the current US program.  It compiles as three
/// shaders by the macro defined: `R300_VS` (vertex), `R300_FS` (colour buffers
/// only) or `R300_FS_Z` (colour buffers plus the depth/stencil buffer as a
/// uint colour attachment after them: R32Uint for Z24S8, R16Uint for Z16).
/// Colour buffer k is output location k and input attachment k
/// ([`num_targets`] of them); the fragment shaders read the buffers' current
/// contents through the input attachments (framebuffer fetch).  The error is
/// for programs not yet translated.  Every register the source depends on is
/// folded into the text, so the string is its own cache key.
pub fn to_glsl(st: &R300State, desc: &FsDesc, flags: u32) -> Result<String, &'static str> {
    let nodes = nodes(st);
    let mut units = 0u32;
    let (written, writes_w) = targets_written(st);
    let nt = num_targets(st);
    let mw = ((st.reg(RB3D_CCTL) >> 5) & 3) as usize + 1;
    let ucp = st.reg(VAP_CLIP_CNTL) & 0x3F;

    let mut body = String::new();
    for (i, nd) in nodes.iter().enumerate() {
        if nd.alu_start + nd.alu_count > MAX_ALU || nd.tex_start + nd.tex_count > MAX_TEX {
            return Err("US program runs past instruction memory");
        }
        let _ = writeln!(
            body,
            "    // node {i}: tex {}+{} alu {}+{}",
            nd.tex_start, nd.tex_count, nd.alu_start, nd.alu_count
        );
        for k in 0..nd.tex_count {
            let inst = st.reg(US_TEX_INST_0 + 4 * (nd.tex_start + k) as u32);
            emit_tex(&mut body, st, inst, &mut units)?;
        }
        for k in 0..nd.alu_count {
            emit_alu(&mut body, st, nd.alu_start + k);
        }
    }

    let mut sb = String::new();
    let _ = writeln!(sb, "{US_PRELUDE_COMMON}// flags {flags:x}");

    // Vertex stage: post-transform vertices as they are (shifted by the sample
    // offset when multisampling), plus the user clip plane distances when
    // VAP_CLIP_CNTL enables any.
    if let Some(vs) = desc.vs_gpu {
        // the vertex program runs here: the C side generated the whole stage
        let _ = write!(sb, "#ifdef R300_VS\n{vs}\n#else\n\n");
    } else {
        let _ = write!(
            sb,
            "#ifdef R300_VS\n{US_PRELUDE_VS}{}\
             void main()\n\
             {{\n\
             \x20   R300Vertex x = vb[gl_VertexIndex];\n\
             \x20   gl_Position = vec4(x.pos.xy + ms.xy * x.pos.w, x.pos.zw);\n\
             \x20   v0 = x.v[0]; v1 = x.v[1]; v2 = x.v[2]; v3 = x.v[3];\n\
             \x20   v4 = x.v[4]; v5 = x.v[5]; v6 = x.v[6]; v7 = x.v[7];\n\
             \x20   v8 = x.v[8]; v9 = x.v[9]; aux = x.aux;\n\
             {}\
             }}\n\n\
             #else\n\n",
            if ucp != 0 { "out gl_PerVertex { vec4 gl_Position; float gl_ClipDistance[6]; };\n" } else { "" },
            if ucp != 0 { "    for (int i = 0; i < 6; i++) gl_ClipDistance[i] = x.ucp[i];\n" } else { "" },
        );
    }

    // Discards only mark the invocation dead; main() discards once the
    // attachment interlock (if any) has been left.
    sb.push_str("bool r300_dead = false;\n");
    let raw = if flags & GLSL_VRAM_SSBO != 0 { US_RAW_SSBO } else { US_RAW_TEX };
    let _ = write!(sb, "{US_PRELUDE}{raw}{US_PRELUDE2}");
    for k in 0..nt {
        emit_cb(&mut sb, st, k, flags);
    }
    sb.push_str(US_PRELUDE_Z);

    // Fragment outputs: colour buffers 0..nt-1, then the depth buffer.
    let (mut outs, mut fbp, mut fbarg, mut fbin, mut fbout, mut zsel) =
        (String::new(), String::new(), String::new(), String::new(), String::new(), String::new());
    for k in 0..nt {
        let _ = write!(outs, "r300_cb{k}_t c{k}; ");
        let _ = write!(fbp, "{}r300_cb{k}_t fb{k}", if k > 0 { ", " } else { "" });
        let _ = write!(fbarg, ", fb{k}");
        let _ = writeln!(fbin, "    r300_cb{k}_t fb{k} = R300_FB{k}_LOAD;");
        if flags & GLSL_FB_IMAGE != 0 {
            let _ = writeln!(fbout, "r300_cb{k}_o o_c{k};");
        } else {
            let _ = writeln!(fbout, "layout(location = {k}) out r300_cb{k}_o o_c{k};");
        }
        let _ = writeln!(zsel, "    o_c{k} = R300_CB{k}_OUT(pass ? c.c{k} : fb{k});");
    }
    let _ = write!(sb, "struct R300FOut {{ {outs}}};\n{fbout}\n");

    // Texture units: unit k at binding TEX0 + k.
    const DIMS: [&str; 3] = ["sampler2D", "sampler3D", "samplerCube"];
    for k in 0..NUM_TEX_UNITS as u32 {
        if units & (1 << k) == 0 {
            continue;
        }
        if unit_is_raw(st, k) {
            if flags & GLSL_VRAM_SSBO != 0 {
                let _ = writeln!(sb, "#define tex{k} {k}u");
            } else {
                let _ = writeln!(sb, "layout(set = 0, binding = {}) uniform usampler2D tex{k};", bind::TEX0 + k);
            }
        } else {
            let _ = writeln!(
                sb,
                "layout(set = 0, binding = {}) uniform {} tex{k};",
                bind::TEX0 + k,
                DIMS[unit_dim(st, k)]
            );
        }
    }
    let _ = write!(
        sb,
        "\n#define R300_WRITES_W {}\n\
         R300FOut r300_shade({fbp}, inout float ow)\n\
         {{\n\
         \x20   /* SC_CLIP_RULE: a 16-entry truth table over which of the four\n\
         \x20      clip rectangles contain the pixel. */\n\
         \x20   {{\n\
         \x20       ivec2 p = ivec2(gl_FragCoord.xy);\n\
         \x20       uint idx = 0u;\n\
         \x20       for (uint i = 0u; i < 4u; i++) {{\n\
         \x20           ivec4 r = u.cliprect[i];\n\
         \x20           if (p.x >= r.x && p.y >= r.y && p.x <= r.z && p.y <= r.w) idx |= 1u << i;\n\
         \x20       }}\n\
         \x20       if (((u.clip_rule >> idx) & 1u) == 0u) r300_dead = true;\n\
         \x20   }}\n\
         \x20   vec4 t[32];\n\
         \x20   for (int i = 0; i < 32; i++) t[i] = vec4(0.0);\n\
         \x20   vec4 vin[10] = vec4[10](v0, v1, v2, v3, v4, v5, v6, v7, v8, v9);\n",
        tf(writes_w)
    );
    for (k, &route) in desc.route.iter().enumerate() {
        if route >= 0 {
            let _ = writeln!(sb, "    t[{k}] = vin[{route}];");
        }
    }
    // FACE emulation (gl_FrontFacing): Mesa 22.3.6 negates the hardware FACE
    // input (1 back, 0 front) in the first hardware input register after the
    // colours.  Nothing routes that register on R300, so feed it from the
    // rasterised facing here: the first input register is FACE when no colour
    // is routed to it.
    if desc.route[0] < 0 {
        sb.push_str("    t[0] = vec4(gl_FrontFacing ? 0.0 : 1.0, 0.0, 0.0, 0.0);\n");
    }
    let _ = write!(
        sb,
        "    vec4 oc[4] = vec4[4](vec4(0.0, 0.0, 0.0, 1.0), vec4(0.0, 0.0, 0.0, 1.0),\n\
         \x20                        vec4(0.0, 0.0, 0.0, 1.0), vec4(0.0, 0.0, 0.0, 1.0));\n{body}"
    );
    sb.push_str(
        "    /* FG: fog, then the alpha test on render target A. */\n\
         \x20   if ((u.fog_blend & 1u) != 0u) {\n\
         \x20       float f = r300_fogf(u.fog_blend, aux.x, u.fog_color.w);\n\
         \x20       for (int k = 0; k < 4; k++) oc[k].rgb = mix(u.fog_color.rgb, oc[k].rgb, f);\n\
         \x20   }\n\
         \x20   if (!r300_alpha_pass(u.alpha_func, clamp(oc[0].a, 0.0, 1.0))) r300_dead = true;\n\
         \x20   bool keep = r300_discard_src(u.cblend, clamp(oc[0], 0.0, 1.0));\n\
         \x20   R300FOut o;\n",
    );
    for k in 0..nt {
        let src = if mw > 1 && k < mw {
            Some(0)
        } else if (written >> k) & 1 != 0 || k == 0 {
            Some(k)
        } else {
            None
        };
        emit_target(&mut sb, k, src);
    }
    let mut stores = String::new();
    let zdecl = if flags & GLSL_FB_IMAGE != 0 {
        // The buffers are written here, inside the critical section, by the
        // invocations that were not discarded.
        stores.push_str("    if (!r300_dead) {\n");
        for k in 0..nt {
            let _ = writeln!(stores, "        imageStore(fbin{k}, ivec2(gl_FragCoord.xy), o_c{k});");
        }
        stores.push_str("#ifdef R300_FS_Z\n        imageStore(zin, ivec2(gl_FragCoord.xy), o_z);\n#endif\n    }\n");
        let z = bind::FB0 + nt as u32;
        format!(
            "#ifdef R300_Z16\n\
             layout(set = 0, binding = {z}, r16ui) coherent uniform uimage2D zin;\n\
             #else\n\
             layout(set = 0, binding = {z}, r32ui) coherent uniform uimage2D zin;\n\
             #endif\n\
             uvec4 o_z;\n\
             #define R300_ZIN_LOAD imageLoad(zin, ivec2(gl_FragCoord.xy)).x\n"
        )
    } else {
        format!(
            "layout(input_attachment_index = {nt}, set = 0, binding = {}) uniform usubpassInput zin;\n\
             layout(location = {nt}) out uvec4 o_z;\n\
             #define R300_ZIN_LOAD subpassLoad(zin).x\n",
            bind::FB0 + nt as u32
        )
    };
    let fbarg = &fbarg[2..];
    let _ = write!(
        sb,
        "    return o;\n\
         }}\n\
         \n\
         #ifdef R300_FS_Z\n\
         {zdecl}\
         #endif\n\
         \n\
         void main()\n\
         {{\n\
         {fbin}\
         \x20   float ow = gl_FragCoord.z;\n\
         #ifndef R300_FS_Z\n\
         \x20   R300FOut c = r300_shade({fbarg}, ow);\n\
         \x20   if (u.zpass_count != 0u && !r300_dead) atomicAdd(zpass, 1u);\n"
    );
    for k in 0..nt {
        let _ = writeln!(sb, "    o_c{k} = R300_CB{k}_OUT(c.c{k});");
    }
    let _ = write!(
        sb,
        "#else\n\
         \x20   uint zb = R300_ZIN_LOAD;\n\
         \x20   bool front = gl_FrontFacing;\n\
         \x20   /* SU_POLY_OFFSET_*: slope in depth per pixel */\n\
         \x20   float slope = max(abs(dFdx(gl_FragCoord.z)), abs(dFdy(gl_FragCoord.z)));\n\
         \x20   R300FOut c = r300_shade({fbarg}, ow);\n\
         \x20   /* FG_DEPTH_SRC: the program's OMASK_W output replaces the depth */\n\
         \x20   float fz = (u.depth_src != 0u && R300_WRITES_W) ? ow : gl_FragCoord.z;\n\
         \x20   if (front ? (u.poly_en & 1u) != 0u : (u.poly_en & 2u) != 0u)\n\
         \x20       fz += front ? u.poly_offset.x * slope + u.poly_offset.y\n\
         \x20                   : u.poly_offset.z * slope + u.poly_offset.w;\n\
         \x20   bool pass;\n\
         \x20   o_z = uvec4(r300_ztest(zb, fz, front, pass), 0u, 0u, 0u);\n\
         {zsel}\
         #endif\n\
         {stores}\
         \x20   /*R300_END*/\n\
         \x20   if (r300_dead) discard;\n\
         }}\n\
         #endif\n"
    );
    Ok(sb)
}

/// Disassembles the active US program, for traces.
pub fn disasm(st: &R300State) -> String {
    let nodes = nodes(st);
    let mut sb = String::new();
    let _ = writeln!(
        sb,
        "US config {:08x} offset {:08x}, {} node(s)",
        st.reg(US_CONFIG),
        st.reg(US_CODE_OFFSET),
        nodes.len()
    );
    for (i, nd) in nodes.iter().enumerate() {
        let _ = writeln!(sb, " node {i}");
        for k in 0..nd.tex_count {
            let t = st.reg(US_TEX_INST_0 + 4 * (nd.tex_start + k) as u32);
            let _ = writeln!(
                sb,
                "  tex{:<2} op{} t{} <- unit{} t{} (raw {t:08x})",
                nd.tex_start + k,
                (t >> 15) & 7,
                (t >> 6) & 0x1F,
                tex_unit(st, t),
                t & 0x1F
            );
        }
        for k in 0..nd.alu_count {
            let a = (nd.alu_start + k) as u32;
            let _ = writeln!(
                sb,
                "  alu{a:<2} rgb {:08x} {:08x}  alpha {:08x} {:08x}",
                st.reg(US_ALU_RGB_ADDR_0 + 4 * a),
                st.reg(US_ALU_RGB_INST_0 + 4 * a),
                st.reg(US_ALU_ALPHA_ADDR_0 + 4 * a),
                st.reg(US_ALU_ALPHA_INST_0 + 4 * a)
            );
        }
    }
    sb
}
