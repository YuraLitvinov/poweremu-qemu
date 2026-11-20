//! Types shared with the C side.  `#[repr(C)]` mirrors must match
//! `hw/display/r300/r300_draw.h`; `tests/layout.rs` checks them against
//! `tests/golden/layout.txt`, which `tools/gen_golden.sh` generates from the
//! header.

use core::ptr;

/// Maximum number of mip levels in a texture (`R300_TEX_MAX_LEVELS`: 2048 .. 1).
pub const TEX_MAX_LEVELS: usize = 12;

/// `TX_FORMAT1.TEX_COORD_TYPE`.
pub const TEXDIM_2D: u32 = 0;
pub const TEXDIM_3D: u32 = 1;
pub const TEXDIM_CUBE: u32 = 2;

/// How the renderer sees a texture (`R300_TEXK_*`).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TexKind {
    /// 32bpp view; the shader reverses the bytes.
    Rgba8 = 0,
    /// X8 view.
    R8 = 1,
    /// Y8X8 view; the shader swaps the bytes.
    Rg8 = 2,
    /// 16bpp packed: decoded to RGBA8 on the CPU.
    Convert16 = 3,
    /// Uint view of the texel dwords, decoded by the shader.
    Raw = 4,
    /// Compressed blocks, copied as they lie.
    Dxt1 = 5,
    Dxt3 = 6,
    Dxt5 = 7,
}

impl TexKind {
    /// Converts the raw `kind` field.  The C code treated every value it did
    /// not know like a one-byte-per-texel copy, which is what [`TexKind::Raw`]
    /// gets, so unknown values map there.
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::Rgba8,
            1 => Self::R8,
            2 => Self::Rg8,
            3 => Self::Convert16,
            5 => Self::Dxt1,
            6 => Self::Dxt3,
            7 => Self::Dxt5,
            _ => Self::Raw,
        }
    }
}

/// `R300TexDesc`: a bound texture unit.  Field order and types are the C
/// struct's; the C comments in `r300_draw.h` describe each field.
#[repr(C)]
#[derive(Debug)]
pub struct R300TexDesc {
    pub bound: bool,
    pub gpu_addr: u32,
    pub width: u32,
    pub height: u32,
    pub endian: u32,
    pub le: bool,
    pub tile: u32,
    pub pitch_bytes: u32,
    pub format: u32,
    pub kind: u32,
    pub view_bpp: u32,
    pub filter0: u32,
    pub filter1: u32,
    pub host_data: *mut u8,
    pub dim: u32,
    pub depth: u32,
    pub levels: u32,
    pub lvl_off: [u32; TEX_MAX_LEVELS],
    pub lvl_pitch: [u32; TEX_MAX_LEVELS],
    pub lvl_rows: [u32; TEX_MAX_LEVELS],
    pub size_bytes: u32,
}

impl Default for R300TexDesc {
    fn default() -> Self {
        Self {
            bound: false,
            gpu_addr: 0,
            width: 0,
            height: 0,
            endian: 0,
            le: false,
            tile: 0,
            pitch_bytes: 0,
            format: 0,
            kind: 0,
            view_bpp: 0,
            filter0: 0,
            filter1: 0,
            host_data: ptr::null_mut(),
            dim: 0,
            depth: 0,
            levels: 0,
            lvl_off: [0; TEX_MAX_LEVELS],
            lvl_pitch: [0; TEX_MAX_LEVELS],
            lvl_rows: [0; TEX_MAX_LEVELS],
            size_bytes: 0,
        }
    }
}
