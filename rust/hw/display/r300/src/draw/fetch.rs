//! Vertex fetch through the programmable stream controller (PSC).

use super::misc::{vc_swap, Arrays};
use super::regs::*;
use super::WARN_VTXFMT;
use crate::draw::layout::half_to_float;
use crate::pvs::{Inputs, Vec4, NUM_INPUTS};
use crate::state::R300State;

/// Reads guest GPU memory (VRAM or GART) for vertex arrays and indices: fills
/// `dst` from `gpu_addr`, or returns false if the range is not mapped.
pub type ReadFn<'a> = dyn FnMut(u32, &mut [u8]) -> bool + 'a;

pub struct Fetch<'a> {
    pub st: &'a R300State,
    pub arr: &'a Arrays,
    pub read: &'a mut ReadFn<'a>,
    /// Immediate vertex data, if any.
    pub immd: Option<&'a [u32]>,
    /// `VAP_VTX_SIZE`, dwords.
    pub vtx_size: u32,
    /// `VAP_CNTL_STATUS.VC_SWAP`.
    pub swap: u32,
    pub warn: u32,
    /// First fetch failure.
    pub why: Option<&'static str>,
}

fn psc_dwords(ty: u32) -> usize {
    const N: [usize; 16] = [1, 2, 3, 4, 1, 1, 1, 2, 1, 1, 8, 1, 2, 0, 0, 0];
    N[(ty & 15) as usize]
}

impl Fetch<'_> {
    /// Fetch dwords for PSC element `k` of vertex `vtx` starting at `cursor`.
    fn dwords(&mut self, k: usize, vtx: u32, cursor: u32, count: usize) -> Option<[u32; 8]> {
        let mut out = [0u32; 8];
        if let Some(immd) = self.immd {
            let at = u64::from(vtx) * u64::from(self.vtx_size) + u64::from(cursor);
            if at + count as u64 > immd.len() as u64 {
                self.why = Some("immediate vertex data shorter than VAP_VTX_SIZE says");
                return None;
            }
            // CP data: already in order.
            out[..count].copy_from_slice(&immd[at as usize..at as usize + count]);
            return Some(out);
        }
        if self.arr.count == 0 {
            self.why = Some("vertex element with no 3D_LOAD_VBPNTR array");
            return None;
        }
        // More elements than arrays: the rest are interleaved in the last one
        // (the caller keeps the cursor running for them).
        let a = &self.arr.a[k.min(self.arr.count - 1)];
        let addr = a.addr.wrapping_add(vtx.wrapping_mul(a.stride_dw).wrapping_add(cursor).wrapping_mul(4));
        let mut bytes = [0u8; 32];
        if !(self.read)(addr, &mut bytes[..count * 4]) {
            self.why = Some("vertex array outside VRAM and the GART");
            return None;
        }
        let (chunks, _) = bytes.as_chunks::<4>();
        for (w, b) in out.iter_mut().zip(chunks).take(count) {
            *w = vc_swap(u32::from_le_bytes(*b), self.swap);
        }
        Some(out)
    }

    /// The input vertex memory for one vertex, through the PSC.  `None` when
    /// the data cannot be fetched ([`Fetch::why`] says why).
    pub fn vertex(&mut self, vtx: u32) -> Option<Inputs> {
        let mut input: Inputs = [[0.0; 4]; NUM_INPUTS];
        let mut cursor = 0u32;
        for k in 0..16usize {
            let reg = self.st.reg(VAP_PROG_STREAM_CNTL_0 + 4 * (k as u32 / 2));
            let ext = self.st.reg(VAP_PROG_STREAM_CNTL_EXT_0 + 4 * (k as u32 / 2));
            let e = (reg >> (16 * (k % 2))) & 0xFFFF;
            let x = (ext >> (16 * (k % 2))) & 0xFFFF;
            let (ty, skip, dst) = (e & 0xF, (e >> 4) & 0xF, ((e >> 8) & 0x1F) as usize);
            let n = psc_dwords(ty);

            if self.immd.is_none() && k < self.arr.count {
                cursor = 0; // each array starts afresh
            }
            cursor += skip;
            if n == 0 {
                self.warn |= WARN_VTXFMT;
            } else {
                let w = self.dwords(k, vtx, cursor, n)?;
                if ty == 10 {
                    // FLOAT_8: two FLOAT_4 vectors at DST_VEC_LOC and the next
                    store(x, &decode_element(3, false, false, &w), &mut input[dst]);
                    let mut hi = [0u32; 8];
                    hi[..4].copy_from_slice(&w[4..]);
                    store(x, &decode_element(3, false, false, &hi), &mut input[(dst + 1) % NUM_INPUTS]);
                } else {
                    store(x, &decode_element(ty, (e >> 14) & 1 != 0, (e >> 15) & 1 != 0, &w), &mut input[dst]);
                }
            }
            cursor += n as u32;
            if e & (1 << 13) != 0 {
                break; // LAST_VEC
            }
        }
        Some(input)
    }
}

fn decode_element(ty: u32, sgn: bool, norm: bool, w: &[u32; 8]) -> Vec4 {
    let mut c = [0.0, 0.0, 0.0, 1.0];
    match ty {
        // FLOAT_1..4
        0..=3 => {
            for i in 0..=ty as usize {
                c[i] = f32::from_bits(w[i]);
            }
        }
        // BYTE x4, D3DCOLOR
        4 | 5 => {
            for (i, ch) in c.iter_mut().enumerate() {
                let b = (w[0] >> (8 * i)) & 0xFF;
                let mut x = if sgn { f32::from(b as u8 as i8) } else { b as f32 };
                if norm || ty == 5 {
                    x = if sgn { (x / 127.0).max(-1.0) } else { x / 255.0 };
                }
                *ch = x;
            }
            if ty == 5 {
                // stored B,G,R,A
                c.swap(0, 2);
            }
        }
        // SHORT_2, SHORT_4
        6 | 7 => {
            for i in 0..if ty == 6 { 2 } else { 4 } {
                let h = (w[i / 2] >> (16 * (i % 2))) & 0xFFFF;
                let mut x = if sgn { f32::from(h as u16 as i16) } else { h as f32 };
                if norm {
                    x = if sgn { (x / 32767.0).max(-1.0) } else { x / 65535.0 };
                }
                c[i] = x;
            }
        }
        // VECTOR_3_TTT: 10:10:10, VECTOR_3_EET: 11:11:10
        8 | 9 => {
            const SH: [[u32; 3]; 2] = [[0, 10, 20], [0, 11, 22]];
            const NB: [[u32; 3]; 2] = [[10, 10, 10], [11, 11, 10]];
            let t = (ty - 8) as usize;
            for i in 0..3 {
                let n = NB[t][i];
                let b = (w[0] >> SH[t][i]) & ((1 << n) - 1);
                let mut x = if sgn { (((b << (32 - n)) as i32) >> (32 - n)) as f32 } else { b as f32 };
                if norm {
                    let m = ((1u32 << if sgn { n - 1 } else { n }) - 1) as f32;
                    x = if sgn { (x / m).max(-1.0) } else { x / m };
                }
                c[i] = x;
            }
        }
        // FLT16_2, FLT16_4
        11 | 12 => {
            for i in 0..if ty == 11 { 2 } else { 4 } {
                c[i] = half_to_float(((w[i / 2] >> (16 * (i % 2))) & 0xFFFF) as u16);
            }
        }
        _ => {}
    }
    c
}

/// PSC element's EXT swizzle and write mask into one input vector.
fn store(x: u32, c: &Vec4, dst: &mut Vec4) {
    for (i, d) in dst.iter_mut().enumerate() {
        if (x >> (12 + i)) & 1 != 0 {
            let s = (x >> (3 * i)) & 7;
            *d = match s {
                0..=3 => c[s as usize],
                5 => 1.0,
                _ => 0.0,
            };
        }
    }
}
