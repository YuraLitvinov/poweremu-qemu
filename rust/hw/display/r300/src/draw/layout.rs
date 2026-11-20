//! Texture layout and border colours (`r300_tex_layout`,
//! `r300_tex_level_dims`, `r300_border_color`).

use crate::types::{R300TexDesc, TEXDIM_2D, TEXDIM_3D, TEXDIM_CUBE};

fn pot_ceil(v: u32) -> u32 {
    let mut p = 1u32;
    while p < v {
        p <<= 1;
    }
    p
}

/// Width, height and depth (slices; 6 faces for a cube) of mip level `l`.
pub fn tex_level_dims(td: &R300TexDesc, l: u32) -> (u32, u32, u32) {
    let shifted = |v: u32| v.checked_shr(l).unwrap_or(0).max(1);
    let depth = match td.dim {
        TEXDIM_CUBE => 6,
        TEXDIM_3D => shifted(td.depth),
        _ => 1,
    };
    (shifted(td.width), shifted(td.height), depth)
}

/// Fills the layout fields of `td` from its size, format, dim and levels;
/// `bpp` is bytes per texel (per 4x4 block for DXT, flagged by `dxt`).
///
/// The hardware's addressing, as Mesa's `r300_texture_desc.c` lays out linear
/// (untiled) textures: 32-byte rows, power-of-two heights for mipmapped, 3D
/// and cube textures, levels one after another.  Little-endian guests only.
pub fn tex_layout(td: &mut R300TexDesc, bpp: u32, dxt: bool, pitch_en: bool) {
    let pot_rows = td.levels > 1 || td.dim != TEXDIM_2D;
    let mut off = 0u32;
    for l in 0..(td.levels as usize).min(td.lvl_off.len()) {
        let (w, mut h, d) = tex_level_dims(td, l as u32);
        if pot_rows {
            h = pot_ceil(h);
        }
        let (mut pitch, mut rows);
        if dxt {
            pitch = (w.div_ceil(4) * bpp + 31) & !31;
            rows = h.div_ceil(4);
        } else {
            // Linear textures: rows padded to 32 bytes (Mesa's rule).
            // Micro-tiled ones (TX_OFFSET bit 3) as a little-endian guest's
            // card lays them out, measured: 2x2, 3x2, 4x4 -> 16 bytes; 8x8 ->
            // 32; 16x16 -> 64, so rows are aligned to 16 bytes.  The micro
            // tile is 8 pixels wide for 8bpp (8 bytes) and 16 bytes wide for
            // 16..64bpp.
            let mt = if bpp == 1 { 7 } else { 15 };
            // TX_OFFSET micro tile 2: 4x4 pixels (16bpp)
            let square = td.tile & 0x10 != 0;
            let micro = td.tile & 0x8 != 0;
            pitch = if micro { (w * bpp + mt) & !mt } else { (w * bpp + 31) & !31 };
            if square {
                pitch = (w * bpp + 4 * bpp - 1) & !(4 * bpp - 1);
            }
            rows = h;
            if square {
                rows = (rows + 3) & !3;
            } else if micro {
                // The driver also rounds the height up to the micro tile's
                // height (r300_get_pixel_alignment: 4 rows for 8bpp, 2 for
                // 16..64bpp), so a 1-row level takes 2 rows and every level
                // after it sits that much further on.
                let th = if bpp == 1 {
                    4
                } else if bpp <= 8 {
                    2
                } else {
                    1
                };
                rows = (rows + th - 1) & !(th - 1);
            }
        }
        if l == 0 && pitch_en {
            pitch = td.pitch_bytes;
        }
        td.lvl_off[l] = off;
        td.lvl_pitch[l] = pitch;
        td.lvl_rows[l] = rows;
        off = off.wrapping_add(pitch.wrapping_mul(rows).wrapping_mul(d));
    }
    td.size_bytes = off;
}

fn un_bits(v: u32, shift: u32, n: u32) -> f32 {
    ((v >> shift) & ((1 << n) - 1)) as f32 / ((1u32 << n) - 1) as f32
}

fn sn_bits(v: u32, shift: u32, n: u32) -> f32 {
    let x = ((((v >> shift) & ((1 << n) - 1)) << (32 - n)) as i32) >> (32 - n);
    (x as f32 / ((1u32 << (n - 1)) - 1) as f32).max(-1.0)
}

/// A binary16 value as a float.
pub(crate) fn half_to_float(h: u16) -> f32 {
    let (s, e, m) = ((h >> 15) & 1, u32::from((h >> 10) & 0x1F), u32::from(h & 0x3FF));
    // 2^k for k in the normal f32 range, exactly.
    let pow2 = |k: i32| f32::from_bits(((k + 127) as u32) << 23);
    let f = match e {
        0 => m as f32 * pow2(-24),
        31 if m != 0 => f32::NAN,
        31 => f32::INFINITY,
        _ => (1.0 + m as f32 / 1024.0) * pow2(e as i32 - 15),
    };
    if s != 0 {
        -f
    } else {
        f
    }
}

/// A `TX_BORDER_COLOR` dword as the unit's XYZW before the swizzle: packed
/// like a texel of the unit's format (8888 for texels wider than 32 bits; DXT
/// as B8G8R8A8).
pub fn border_color(fmt: u32, v: u32) -> [f32; 4] {
    let mut c = [0.0f32; 4];
    match fmt & 0x1F {
        0x00 => c[0] = un_bits(v, 0, 8),
        0x01 => c[0] = un_bits(v, 0, 16),
        0x02 => {
            c[0] = un_bits(v, 0, 4);
            c[1] = un_bits(v, 4, 4);
        }
        0x03 => {
            c[0] = un_bits(v, 0, 8);
            c[1] = un_bits(v, 8, 8);
        }
        0x04 => {
            c[0] = un_bits(v, 0, 16);
            c[1] = un_bits(v, 16, 16);
        }
        0x05 => {
            c[0] = un_bits(v, 0, 2);
            c[1] = un_bits(v, 2, 3);
            c[2] = un_bits(v, 5, 3);
        }
        0x06 => {
            c[0] = un_bits(v, 0, 5);
            c[1] = un_bits(v, 5, 6);
            c[2] = un_bits(v, 11, 5);
        }
        0x07 => {
            c[0] = un_bits(v, 0, 5);
            c[1] = un_bits(v, 5, 5);
            c[2] = un_bits(v, 10, 6);
        }
        0x08 => {
            c[0] = un_bits(v, 0, 10);
            c[1] = un_bits(v, 10, 11);
            c[2] = un_bits(v, 21, 11);
        }
        0x09 => {
            c[0] = un_bits(v, 0, 11);
            c[1] = un_bits(v, 11, 11);
            c[2] = un_bits(v, 22, 10);
        }
        0x0A => {
            for (i, ch) in c.iter_mut().enumerate() {
                *ch = un_bits(v, 4 * i as u32, 4);
            }
        }
        0x0B => {
            for (i, ch) in c.iter_mut().take(3).enumerate() {
                *ch = un_bits(v, 5 * i as u32, 5);
            }
            c[3] = un_bits(v, 15, 1);
        }
        0x0D => {
            for (i, ch) in c.iter_mut().take(3).enumerate() {
                *ch = un_bits(v, 10 * i as u32, 10);
            }
            c[3] = un_bits(v, 30, 2);
        }
        // DXT: B8G8R8A8
        0x0F..=0x11 => {
            c[0] = un_bits(v, 16, 8);
            c[1] = un_bits(v, 8, 8);
            c[2] = un_bits(v, 0, 8);
            c[3] = un_bits(v, 24, 8);
        }
        // signed formats: R8G8B8A8_SNORM
        0x12 => {
            for (i, ch) in c.iter_mut().enumerate() {
                *ch = sn_bits(v, 8 * i as u32, 8);
            }
        }
        // R16G16_SNORM
        0x16 | 0x17 => {
            c[0] = sn_bits(v, 0, 16);
            c[1] = sn_bits(v, 16, 16);
        }
        // R16G16_FLOAT
        0x18 | 0x19 => {
            c[0] = half_to_float(v as u16);
            c[1] = half_to_float((v >> 16) as u16);
        }
        0x1B => c[0] = f32::from_bits(v),
        // 8888
        _ => {
            for (i, ch) in c.iter_mut().enumerate() {
                *ch = un_bits(v, 8 * i as u32, 8);
            }
        }
    }
    c
}
