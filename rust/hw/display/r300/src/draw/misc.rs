//! Small helpers of the draw module: vertex array pointers, index streams,
//! multisampling, and content hashes.

use std::sync::OnceLock;

use crate::state::R300State;

pub const MAX_ARRAYS: usize = 16;

const GB_MSPOS0: u32 = 0x4010;
const GB_MSPOS1: u32 = 0x4014;
const GB_AA_CONFIG: u32 = 0x4020;

/// One vertex array of a `3D_LOAD_VBPNTR`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArrayDesc {
    /// GPU address.
    pub addr: u32,
    /// Dwords per element.
    pub size_dw: u32,
    /// Dwords between elements.
    pub stride_dw: u32,
}

/// `3D_LOAD_VBPNTR` state: one entry per vertex array.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arrays {
    pub count: usize,
    pub a: [ArrayDesc; MAX_ARRAYS],
}

/// Decodes a `3D_LOAD_VBPNTR` (type-3 0x2F) body.
pub fn load_vbpntr(d: &[u32]) -> Arrays {
    let mut arr = Arrays::default();
    let Some(&first) = d.first() else {
        return arr;
    };
    let n = (first & 0x1F) as usize;
    let mut i = 0;
    while i < n && i < MAX_ARRAYS {
        let (pair, half) = (i / 2, i % 2);
        let base = 1 + pair * 3;
        if base + 1 + half >= d.len() {
            break;
        }
        let fmt = d[base] >> (16 * half);
        arr.a[i] = ArrayDesc { size_dw: fmt & 0x7F, stride_dw: (fmt >> 8) & 0x7F, addr: d[base + 1 + half] };
        i += 1;
    }
    arr.count = i;
    arr
}

/// Index `i` (0-based) of a `DRAW_INDX_2` index stream: 16-bit indices are
/// packed two per dword, the first in the low half.  Past the end reads 0.
pub fn index_at(dw: &[u32], i32_indices: bool, i: u32) -> u32 {
    let i = i as usize;
    if i32_indices {
        dw.get(i).copied().unwrap_or(0)
    } else {
        dw.get(i / 2).map_or(0, |w| (w >> (16 * (i & 1))) & 0xFFFF)
    }
}

/// The byte swap the card's vertex cache applies to fetched data
/// (`VAP_CNTL_STATUS.VC_SWAP`).
pub fn vc_swap(v: u32, mode: u32) -> u32 {
    match mode & 3 {
        1 => ((v & 0x00FF_00FF) << 8) | ((v >> 8) & 0x00FF_00FF),
        2 => v.swap_bytes(),
        3 => v.rotate_left(16),
        _ => v,
    }
}

/// `GB_AA_CONFIG`'s sample count (1 without antialiasing).
pub fn aa_samples(st: &R300State) -> u32 {
    const N: [u32; 4] = [2, 3, 4, 6];
    let aa = st.reg(GB_AA_CONFIG);
    if aa & 1 != 0 {
        N[((aa >> 1) & 3) as usize]
    } else {
        1
    }
}

/// `GB_MSPOS`' sample positions as offsets from the pixel centre, in pixels
/// (twelfths of a pixel from the pixel's corner).
pub fn aa_positions(st: &R300State) -> [[f32; 2]; 6] {
    let mut v = u64::from(st.reg(GB_MSPOS0) & 0xFF_FFFF) | u64::from(st.reg(GB_MSPOS1) & 0xFF_FFFF) << 24;
    let mut pos = [[0.0; 2]; 6];
    for p in &mut pos {
        *p = [((v & 0xF) as f32 - 6.0) / 12.0, (((v >> 4) & 0xF) as f32 - 6.0) / 12.0];
        v >>= 8;
    }
    pos
}

/// Byte offset of sample `sample` of pixel (x, y) in a multisampled R300
/// colour/depth buffer (`ns` = 2 or 4 samples) as the guest driver computes it
/// for CPU access: 4x4-pixel micro tiles holding all samples, in 8-row bands,
/// `pitch_px` pixels wide, `bpp` 2 or 4.  `None` for other sample counts.
pub fn msaa_offset(x: u32, y: u32, ns: u32, pitch_px: u32, bpp: u32, sample: u32) -> Option<u32> {
    if (ns != 2 && ns != 4) || sample >= ns {
        return None;
    }
    let sh = if bpp == 2 { 1 } else { 2 }; // log2(bytes per sample)
    let pitch = pitch_px.wrapping_mul(ns); // in samples, as the driver keeps it
    let base = 32u32.wrapping_mul(
        (pitch / ns / 4)
            .wrapping_mul(ns * sh * 2)
            .wrapping_mul(y >> 3)
            .wrapping_add((((x >> 2) << 1) | ((y >> 2) & 1)) * ns * sh),
    );
    let mut idx = if ns == 2 {
        (((y >> 1) & 1) << 4) | (((x >> 1) & 1) << 3) | (sample << 2)
    } else {
        (((((x >> 2) & 1) ^ ((y >> 1) & 1)) & 1) << 5) | (((x >> 1) & 1) << 4) | (sample << 2)
    };
    idx |= ((y & 1) << 1) | (x & 1);
    Some(base.wrapping_add(idx << sh))
}

const M: u64 = 0x9E37_79B9_7F4A_7C15;

/// 64-bit content hash, for caches of textures copied out of guest memory.
pub fn hash_bytes(p: &[u8]) -> u64 {
    let mut h = (p.len() as u64).wrapping_mul(M);
    let (words, rest) = p.as_chunks::<8>();
    for w in words {
        h = (h ^ u64::from_le_bytes(*w)).wrapping_mul(M);
        h ^= h >> 29;
    }
    for &b in rest {
        h = (h ^ u64::from(b)).wrapping_mul(M);
    }
    h ^ (h >> 32)
}

/// Identity of a texture's texels for the renderers' caches.  Hashing every
/// byte of a mip chain on every draw cost a third of the render thread in
/// Doom 3 (megabytes of DXT per draw), so large chains are sampled: both ends
/// plus a spread of words across the rest.  The driver uploads whole
/// textures, which changes every sampled stretch; `$R300_TEX_FULLHASH=1`
/// hashes everything.
pub fn hash_tex(p: &[u8]) -> u64 {
    static FULL: OnceLock<bool> = OnceLock::new();
    let n = p.len();
    if *FULL.get_or_init(|| std::env::var_os("R300_TEX_FULLHASH").is_some()) || n <= 32768 {
        return hash_bytes(p);
    }
    let mut h = hash_bytes(&p[..8192]) ^ hash_bytes(&p[n - 8192..]).wrapping_mul(3);
    let step = (n / 2048) & !7; // 2048 spots, 8-byte aligned
    let mut i = 8192;
    while i + 8 <= n - 8192 {
        let v = u64::from_le_bytes(p[i..i + 8].try_into().expect("8 bytes"));
        h = (h ^ v).wrapping_mul(M);
        h ^= h >> 29;
        i += step;
    }
    h ^ n as u64
}
