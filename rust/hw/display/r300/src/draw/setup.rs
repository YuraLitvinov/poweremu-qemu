//! Per-draw state: uniforms, textures, depth buffer and viewport.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use super::layout::{border_color, tex_layout};
use super::regs::*;
use super::{DepthDesc, WARN_DEPTH, WARN_TEXFMT};
use crate::pvs::Vec4;
use crate::state::R300State;
use crate::types::{R300TexDesc, TexKind, TEXDIM_2D, TEXDIM_3D, TEXDIM_CUBE, TEX_MAX_LEVELS};
use crate::us::{
    float24, tex_raw_bpp, R300FSUniforms, NUM_CONSTS, NUM_TEX_UNITS, TEXF_GAMMA, TEXF_POT_ROWS, ZFMT_ENDIAN_MASK,
    ZFMT_Z16,
};

pub(crate) fn bits_to_float(v: u32) -> f32 {
    f32::from_bits(v)
}

static BIAS: AtomicU32 = AtomicU32::new(1440);

/// Scissor and clip rectangle coordinates carry a bias so they stay positive:
/// the Linux driver (and Apple's) uses 1440.  The Windows driver (ati2mtag)
/// uses 1088: its clip rectangles for sprites it crops (the Start button's flag
/// and label) only line up with its vertex positions that way, and read with
/// 1440 they are empty, which clips every fragment.  Once the guest has sent a
/// 3D_DRAW_IMMD packet (only the Windows driver does; Mesa never does) a
/// rectangle coordinate in 1000..1439 means a 1088 bias; the first one seen
/// switches to 1088 for good.  Without the IMMD condition this misfired on
/// Linux (windows partly off-screen have slightly negative clip coordinates,
/// 1440 - n).  WORKAROUND, narrow and evidenced.
///
/// Like the C code, the choice is process-wide state: once 1088, always 1088.
pub fn sc_bias(st: &R300State) -> u32 {
    static OFF: OnceLock<bool> = OnceLock::new();
    let off = *OFF.get_or_init(|| std::env::var_os("R300_NO_BIAS1088").is_some());
    if BIAS.load(Ordering::Relaxed) == 1440 && st.win_immd && !off {
        for i in 0..8 {
            let v = st.reg(SC_CLIPRECT_TL_0 + 4 * i);
            for k in 0..2 {
                let c = (v >> (13 * k)) & 0x1FFF;
                if (1000..1440).contains(&c) {
                    BIAS.store(1088, Ordering::Relaxed);
                }
            }
        }
    }
    BIAS.load(Ordering::Relaxed)
}

/// Resets the process-wide scissor bias (for tests).
pub fn reset_sc_bias() {
    BIAS.store(1440, Ordering::Relaxed);
}

pub fn set_uniforms(st: &R300State) -> R300FSUniforms {
    let mut u = R300FSUniforms::default();
    let outfmt = st.reg(US_OUT_FMT_0);
    let bc = st.reg(RB3D_BLEND_COLOR);
    let bias = sc_bias(st) as i32;

    for i in 0..NUM_CONSTS {
        for c in 0..4 {
            u.consts[i][c] = float24(st.reg(PFS_PARAM_0_X + 16 * i as u32 + 4 * c as u32));
        }
    }
    // RB3D_BLEND_COLOR byte n belongs to stored component n of the colour
    // buffer, i.e. to the channel US_OUT_FMT's select n names (0 A, 1 R, 2 G, 3
    // B); Mesa pre-swaps the colour for buffers that are not BGRA.
    for n in 0..4 {
        let sel = (outfmt >> (8 + 2 * n)) & 3;
        u.blend_color[if sel == 0 { 3 } else { sel as usize - 1 }] = ((bc >> (8 * n)) & 0xFF) as f32 / 255.0;
    }
    u.cblend = st.reg(RB3D_CBLEND);
    u.ablend = st.reg(RB3D_ABLEND);
    u.chanmask = st.reg(RB3D_COLOR_CHANNEL_MASK) & 0xF;
    u.alpha_func = st.reg(FG_ALPHA_FUNC);
    for n in 0..4 {
        u.out_sel[n] = (outfmt >> (8 + 2 * n)) & 3;
    }
    let pitch0 = st.reg(RB3D_COLORPITCH0);
    u.rt_swap32 = u32::from((pitch0 >> 19) & 3 == 2);
    u.rt_endian = (pitch0 >> 19) & 3;
    u.clip_rule = st.reg(SC_CLIP_RULE) & 0xFFFF;
    for i in 0..4u32 {
        let tl = st.reg(SC_CLIPRECT_TL_0 + 8 * i);
        let br = st.reg(SC_CLIPRECT_TL_0 + 8 * i + 4);
        u.cliprect[i as usize] = [
            (tl & 0x1FFF) as i32 - bias,
            ((tl >> 13) & 0x1FFF) as i32 - bias,
            (br & 0x1FFF) as i32 - bias,
            ((br >> 13) & 0x1FFF) as i32 - bias,
        ];
    }
    u.rop = st.reg(RB3D_ROPCNTL);
    // Fog: colours and the constant factor are 10-bit fractions.
    for c in 0..3 {
        u.fog_color[c] = (st.reg(FG_FOG_COLOR_R + 4 * c as u32) & 0x3FF) as f32 / 1023.0;
    }
    u.fog_color[3] = (st.reg(FG_FOG_FACTOR) & 0x3FF) as f32 / 1023.0;
    u.fog_blend = st.reg(FG_FOG_BLEND) & 7;
    u.depth_src = st.reg(FG_DEPTH_SRC) & 1;
    // Polygon offset (SU_POLY_OFFSET_*): scale multiplies the depth slope per
    // 1/12-pixel subpixel and offset is in units of 2^-24 of the depth range.
    // Mesa programs a 16-bit Z buffer's scale 16 times larger, so the slope
    // there is in 2^-20 units; set_depth adjusts for Z16.
    u.poly_en = st.reg(SU_POLY_OFFSET_ENABLE) & 7;
    for i in 0..2u32 {
        u.poly_offset[2 * i as usize] = bits_to_float(st.reg(SU_POLY_OFFSET_FRONT_SCALE + 8 * i)) / 12.0;
        u.poly_offset[2 * i as usize + 1] = bits_to_float(st.reg(SU_POLY_OFFSET_FRONT_SCALE + 8 * i + 4)) / 16_777_216.0;
    }
    u
}

fn texfmt_log(k: u32, fmt: u32, kind: TexKind, f0: u32, f1: u32, f2: u32, off: u32) {
    static SEEN: OnceLock<Option<Mutex<Vec<u32>>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| std::env::var_os("PPCGPU_TEXFMT_LOG").map(|_| Mutex::new(Vec::new())));
    let Some(seen) = seen else {
        return;
    };
    let mut seen = seen.lock().unwrap_or_else(|p| p.into_inner());
    if seen.len() >= 256 {
        return;
    }
    let key = (fmt << 24) ^ (f0 & 0x3FF_FFFF) ^ (f1 << 3) ^ (off & 0x1F);
    if !seen.contains(&key) {
        seen.push(key);
        eprintln!(
            "TEXFMT unit {k} fmt {fmt:02x} kind {} {}x{} levels {} f0 {f0:08x} f1 {f1:08x} f2 {f2:08x} off {off:08x}",
            kind as u32,
            (f0 & 0x7FF) + 1,
            ((f0 >> 11) & 0x7FF) + 1,
            ((f0 >> 26) & 0xF) + 1
        );
    }
}

/// Fills the texture descriptors and the per-unit uniforms.
pub fn set_textures(st: &R300State, u: &mut R300FSUniforms, tex: &mut [R300TexDesc; NUM_TEX_UNITS], warn: &mut u32) {
    let enable = st.reg(TX_ENABLE);
    for (k, t) in tex.iter_mut().enumerate() {
        let ku = k as u32;
        let f0 = st.reg(TX_FORMAT0_0 + 4 * ku);
        let f1 = st.reg(TX_FORMAT1_0 + 4 * ku);
        let f2 = st.reg(TX_FORMAT2_0 + 4 * ku);
        let off = st.reg(TX_OFFSET_0 + 4 * ku);
        let fmt = f1 & 0x1F;

        *t = R300TexDesc::default();
        // swizzle: R 14:12, G 17:15, B 20:18, A 11:9
        u.tex_swz[k] = [(f1 >> 12) & 7, (f1 >> 15) & 7, (f1 >> 18) & 7, (f1 >> 9) & 7];
        if enable & (1 << k) == 0 {
            continue;
        }
        let (kind, bpp, dxt) = match fmt {
            // X8
            0x0 => (TexKind::R8, 1, false),
            // Y8X8
            0x3 => (TexKind::Rg8, 2, false),
            0x6 | 0x7 | 0xA | 0xB => (TexKind::Convert16, 2, false),
            // W8Z8Y8X8
            0xC => (TexKind::Rgba8, 4, false),
            // The card hands the shader DXT1 texels as B,G,R,A and DXT3/5 as
            // A,B,G,R (Tiger's swizzles R=Z,G=Y,B=X and R=W,G=Z,B=Y,A=X); the
            // host decodes to R,G,B,A, so reorder before the swizzle.
            0xF => (TexKind::Dxt1, 8, true),
            0x10 => (TexKind::Dxt3, 16, true),
            0x11 => (TexKind::Dxt5, 16, true),
            _ => {
                let bpp = tex_raw_bpp(fmt);
                if bpp == 0 {
                    *warn |= WARN_TEXFMT;
                    continue;
                }
                t.view_bpp = bpp.max(4);
                (TexKind::Raw, bpp, false)
            }
        };
        t.kind = kind as u32;
        texfmt_log(ku, fmt, kind, f0, f1, f2, off);
        t.bound = true;
        t.gpu_addr = off & !0x1F;
        t.le = true;
        // A little-endian guest's words are already the card's: endian 2 undoes
        // the big-endian view.
        t.endian = 2;
        t.tile = off & 0x1C;
        t.width = (f0 & 0x7FF) + 1;
        t.height = ((f0 >> 11) & 0x7FF) + 1;
        t.dim = (f1 >> 25) & 3;
        if t.dim > TEXDIM_CUBE {
            t.dim = TEXDIM_2D;
        }
        t.depth = if t.dim == TEXDIM_3D { 1 << ((f0 >> 22) & 0xF) } else { 1 };
        {
            // NUM_LEVELS counts the levels below the base, up to 1x1x1
            let big = t.width.max(t.height).max(t.depth);
            let mut chain = 1;
            while big >> chain != 0 {
                chain += 1;
            }
            t.levels = (((f0 >> 26) & 0xF) + 1).min(chain).min(TEX_MAX_LEVELS as u32);
        }
        if f0 >> 31 != 0 {
            // TX_PITCH_EN: level 0 pitch in texels
            let texels = (f2 & 0x3FFF) + 1;
            t.pitch_bytes = if dxt { texels.div_ceil(4) * bpp } else { texels * bpp };
        }
        tex_layout(t, bpp, dxt, f0 >> 31 != 0);
        t.pitch_bytes = t.lvl_pitch[0];
        t.format = fmt;
        t.filter0 = st.reg(TX_FILTER0_0 + 4 * ku);
        t.filter1 = st.reg(TX_FILTER1_0 + 4 * ku);
        u.tex_info[k][0] = 1;
        // RAW: the uint view's elements per row, as the renderer binds it
        u.tex_info[k][1] = if kind == TexKind::Raw { t.pitch_bytes / t.view_bpp } else { 0 };
        u.tex_info[k][2] = t.width;
        u.tex_info[k][3] = t.height;

        u.tex_border[k] = border_color(fmt, st.reg(TX_BORDER_COLOR_0 + 4 * ku));
        {
            let bias = ((((t.filter1 >> 3) & 0x3FF) << 22) as i32) >> 22;
            let minl = (t.filter0 >> 17) & 0xF;
            u.tex_lod[k] = [
                bias as f32 / 32.0,
                minl.min(t.levels - 1) as f32,
                (t.levels - 1) as f32,
                ((t.filter0 >> 13) & 3) as f32,
            ];
        }
        u.tex_dim[k][0] = t.dim;
        u.tex_dim[k][1] = t.depth;
        u.tex_dim[k][2] = t.pitch_bytes;
        // Only the shader-decoded (raw) path reads the level table: keep the
        // common draw cheap.
        if kind == TexKind::Raw {
            for l in 0..(t.levels as usize).min(TEX_MAX_LEVELS) {
                u.tex_lvl[k][l / 4][l % 4] = t.lvl_off[l];
                u.tex_lvl[k][3 + l / 4][l % 4] = t.lvl_pitch[l];
                u.tex_lvl[k][6 + l / 4][l % 4] = t.lvl_pitch[l].wrapping_mul(t.lvl_rows[l]);
            }
        }
        // Mesa gives channel i of the format (storage order) SIGNED_W, _Z, _Y,
        // _X: a 1- or 2-channel format sets only the top bits, so component j
        // (X first) is signed when bit 5 + 3 - j is set.
        u.tex_dim[k][3] = ((f1 >> 5) & 1)
            | (((f1 >> 6) & 1) << 1)
            | (((f1 >> 7) & 1) << 2)
            | (((f1 >> 8) & 1) << 3)
            | if (f1 >> 21) & 1 != 0 { TEXF_GAMMA } else { 0 }
            | if t.levels > 1 || t.dim != TEXDIM_2D { TEXF_POT_ROWS } else { 0 };
    }
}

/// The depth/stencil buffer (`ZB_*`) and its uniforms.
pub fn set_depth(st: &R300State, u: &mut R300FSUniforms, warn: &mut u32) -> DepthDesc {
    let cntl = st.reg(ZB_CNTL);
    let fmt = st.reg(ZB_FORMAT) & 0xF;
    let pitch = st.reg(ZB_DEPTHPITCH);
    let mut z = DepthDesc::default();
    // STENCIL_ENABLE or Z_ENABLE; Z_WRITE_ENABLE alone does nothing.
    if cntl & 3 == 0 {
        return z;
    }
    z.attach = true;
    z.gpu_addr = st.reg(ZB_DEPTHOFFSET) & !0x1F;
    z.pitch = pitch & 0x3FFC;
    z.format = fmt;
    z.bpp = if fmt == 2 { 4 } else { 2 };
    if fmt > 2 {
        *warn |= WARN_DEPTH; // reserved format: as Z24S8
        z.bpp = 4;
    } else if fmt == 1 {
        *warn |= WARN_DEPTH; // 13E3 float: kept as 16-bit unorm
    }
    u.zinfo[0] = cntl;
    u.zinfo[1] = st.reg(ZB_ZSTENCILCNTL);
    u.zinfo[2] = st.reg(ZB_STENCILREFMASK);
    u.zinfo[3] = ((pitch >> 19) & ZFMT_ENDIAN_MASK) | if z.bpp == 2 { ZFMT_Z16 } else { 0 };
    if z.bpp == 2 {
        u.poly_offset[0] /= 16.0;
        u.poly_offset[2] /= 16.0;
    }
    z
}

/// Clip-space position `c` to the viewport of a `width` x `height` target.
pub fn viewport_xform(st: &R300State, width: u32, height: u32, depth_dx: bool, c: &Vec4) -> Vec4 {
    let vte = st.reg(VAP_VTE_CNTL);
    let (w_px, h_px) = (width as f32, height as f32);
    let mut vp = [0.0f32; 6];
    for (i, v) in vp.iter_mut().enumerate() {
        *v = if (vte >> i) & 1 != 0 {
            bits_to_float(st.reg(SE_VPORT_XSCALE + 4 * i as u32))
        } else if i % 2 != 0 {
            0.0
        } else {
            1.0
        };
    }
    // vp: xscale, xoffset, yscale, yoffset, zscale, zoffset
    if (vte >> 8) & 1 == 0 {
        // Perspective divide by the VTE: stay linear in clip space so the host
        // clips and interpolates exactly as the card would.
        let w = c[3];
        [
            (2.0 * vp[0] / w_px) * c[0] + (2.0 * vp[1] / w_px - 1.0) * w,
            -(2.0 * vp[2] / h_px) * c[1] + (1.0 - 2.0 * vp[3] / h_px) * w,
            // clip-space z in Vulkan's 0..w clip volume; the viewport depth
            // range (depth_min/max) does the rest
            if depth_dx { c[2] } else { 0.5 * (c[2] + w) },
            w,
        ]
    } else {
        let (wx, wy) = (vp[0] * c[0] + vp[1], vp[2] * c[1] + vp[3]);
        [2.0 * wx / w_px - 1.0, 1.0 - 2.0 * wy / h_px, vp[4] * c[2] + vp[5], 1.0]
    }
}
