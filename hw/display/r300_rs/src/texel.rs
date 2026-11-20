//! Texel conversion for the renderers (`r300_texel.c` + `r300_texel_le.c`).
//!
//! A guest texture level is repacked with tight rows into something the host
//! GPU can sample: 16bpp packed formats are decoded to RGBA8, DXT blocks and
//! the plain 8/16/32bpp formats are copied row by row.

use crate::types::TexKind;

/// Decodes one 16bpp texel as the card sees it to `[r, g, b, a]` bytes.
///
/// `fmt` is `TX_FORMAT1 & 0x1F`; formats other than the four packed ones are
/// X16 (the high byte in the first channel).
pub fn decode16(fmt: u32, texel: u16) -> [u8; 4] {
    let v = u32::from(texel);
    // Scales an n-bit channel value to 0..=255 the way the card does.
    let scale = |value: u32, max: u32| (value * 255 / max) as u8;
    match fmt {
        // Z5Y6X5
        0x6 => [scale(v & 31, 31), scale((v >> 5) & 63, 63), scale(v >> 11, 31), 255],
        // Z6Y5X5
        0x7 => [scale(v & 31, 31), scale((v >> 5) & 31, 31), scale(v >> 10, 63), 255],
        // W4Z4Y4X4
        0xA => [
            ((v & 15) * 17) as u8,
            (((v >> 4) & 15) * 17) as u8,
            (((v >> 8) & 15) * 17) as u8,
            ((v >> 12) * 17) as u8,
        ],
        // W1Z5Y5X5
        0xB => [
            scale(v & 31, 31),
            scale((v >> 5) & 31, 31),
            scale((v >> 10) & 31, 31),
            if v >> 15 != 0 { 255 } else { 0 },
        ],
        // X16
        _ => [(v >> 8) as u8, 0, 0, 255],
    }
}

/// What to convert: the unit's texture kind and format, and the byte pitch of
/// the source rows (rows of blocks for DXT).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelSpec {
    pub kind: TexKind,
    pub format: u32,
    pub src_pitch: u32,
}

/// Why a level could not be converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TexelError {
    /// The source slice ends before the last row.
    SourceTooShort { needed: usize, have: usize },
    /// The destination slice is not exactly [`LevelGeometry::out_len`] bytes.
    BadOutputLen { expected: usize, have: usize },
    /// The level is so large that its size does not fit in `usize`/`u32`.
    TooLarge,
}

/// Row counts and byte sizes of one converted level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelGeometry {
    /// Rows converted (rows of 4x4 blocks for DXT).
    pub rows: usize,
    /// Bytes read from each source row.
    pub in_row: usize,
    /// Bytes written per row: the `bpr` the renderer uploads with.
    pub out_row: usize,
}

impl LevelGeometry {
    /// Size of the converted level in bytes.
    pub fn out_len(&self) -> usize {
        self.rows * self.out_row
    }

    /// Bytes of the source that conversion reads, counted from its start.
    pub fn src_len(&self, src_pitch: u32) -> usize {
        match self.rows {
            0 => 0,
            n => (n - 1) * src_pitch as usize + self.in_row,
        }
    }
}

impl LevelSpec {
    /// Geometry of a `w` x `h` texel level.
    pub fn geometry(&self, w: u32, h: u32) -> Result<LevelGeometry, TexelError> {
        let (w, h) = (w as usize, h as usize);
        let geo = match self.kind {
            TexKind::Convert16 => LevelGeometry { rows: h, in_row: w * 2, out_row: w * 4 },
            TexKind::Dxt1 | TexKind::Dxt3 | TexKind::Dxt5 => {
                let block = if self.kind == TexKind::Dxt1 { 8 } else { 16 };
                let row = w.div_ceil(4) * block;
                LevelGeometry { rows: h.div_ceil(4), in_row: row, out_row: row }
            }
            // RGBA8, R8, RG8: bytes as they lie.  RAW is never converted by
            // the renderers; the C code gave it one byte per texel.
            kind => {
                let bpp = match kind {
                    TexKind::Rgba8 => 4,
                    TexKind::Rg8 => 2,
                    _ => 1,
                };
                LevelGeometry { rows: h, in_row: w * bpp, out_row: w * bpp }
            }
        };
        let fits = u32::try_from(geo.out_row).is_ok() && geo.rows.checked_mul(geo.out_row).is_some();
        if fits {
            Ok(geo)
        } else {
            Err(TexelError::TooLarge)
        }
    }
}

/// Converts one `w` x `h` level of `src` into `out`, which must hold exactly
/// `spec.geometry(w, h)?.out_len()` bytes.
pub fn convert_level(
    spec: &LevelSpec,
    src: &[u8],
    w: u32,
    h: u32,
    out: &mut [u8],
) -> Result<(), TexelError> {
    let geo = spec.geometry(w, h)?;
    let needed = geo.src_len(spec.src_pitch);
    if src.len() < needed {
        return Err(TexelError::SourceTooShort { needed, have: src.len() });
    }
    if out.len() != geo.out_len() {
        return Err(TexelError::BadOutputLen { expected: geo.out_len(), have: out.len() });
    }
    if geo.rows == 0 || geo.in_row == 0 {
        return Ok(());
    }
    let pitch = spec.src_pitch as usize;
    let src_rows = (0..geo.rows).map(|y| &src[y * pitch..][..geo.in_row]);
    let out_rows = out.chunks_exact_mut(geo.out_row);
    for (src_row, out_row) in src_rows.zip(out_rows) {
        if spec.kind == TexKind::Convert16 {
            let (texels, _) = src_row.as_chunks::<2>();
            let (pixels, _) = out_row.as_chunks_mut::<4>();
            for (texel, pixel) in texels.iter().zip(pixels) {
                *pixel = decode16(spec.format, u16::from_le_bytes(*texel));
            }
        } else {
            out_row.copy_from_slice(src_row);
        }
    }
    Ok(())
}

/// A converted level and the row size it was packed with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Level {
    pub data: Vec<u8>,
    /// Bytes per row (per row of blocks for DXT).
    pub bytes_per_row: u32,
}

/// Convenience wrapper that allocates the output.
pub fn level_bytes(spec: &LevelSpec, src: &[u8], w: u32, h: u32) -> Result<Level, TexelError> {
    let geo = spec.geometry(w, h)?;
    let mut data = vec![0; geo.out_len()];
    convert_level(spec, src, w, h, &mut data)?;
    Ok(Level { data, bytes_per_row: geo.out_row as u32 })
}
