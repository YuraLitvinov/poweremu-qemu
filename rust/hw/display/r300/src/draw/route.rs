//! Output vertex layout and rasterizer routing (`RS_IP`, `RS_INST`).

use super::regs::*;
use super::WARN_VARYINGS;
use crate::pvs::{Outputs, Vec4};
use crate::state::R300State;
use crate::us::{NUM_TEMPS, NUM_VARYINGS};

/// Where a texture interpolant scalar comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalSlot {
    /// The rasterizer's own (s, t): a point sprite or line stipple.
    Generated,
    /// An output vertex slot.
    Slot(usize),
}

/// Which PVS output slots hold what (`VAP_OUTPUT_VTX_FMT_*`).
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub pos: usize,
    /// Point size (x).
    pub psize: Option<usize>,
    pub color: [Option<usize>; 4],
    /// Texture interpolants as one scalar list: (slot, component).  A unit
    /// whose `GB_ENABLE` source is ST and that has no vertex components
    /// contributes the rasterizer's own (s, t) (point sprite, line stipple) at
    /// its place in the list.
    pub scal: Vec<(ScalSlot, usize)>,
}

pub fn output_layout(st: &R300State) -> Layout {
    let f0 = st.reg(VAP_OUTPUT_VTX_FMT_0);
    let f1 = st.reg(VAP_OUTPUT_VTX_FMT_1);
    let mut l = Layout::default();
    let mut slot = 0;
    let mut next = || {
        slot += 1;
        slot - 1
    };
    l.pos = next(); // always present
    if f0 & (1 << 16) != 0 {
        l.psize = Some(next());
    }
    for c in 0..4 {
        if f0 & (1 << (1 + c)) != 0 {
            l.color[c] = Some(next());
        }
    }
    for t in 0..8u32 {
        let n = ((f1 >> (3 * t)) & 7) as usize;
        if n != 0 {
            let s = next();
            for i in 0..n {
                if l.scal.len() < 32 {
                    l.scal.push((ScalSlot::Slot(s), i));
                }
            }
        } else if (st.reg(GB_ENABLE) >> (16 + 2 * t)) & 3 == 1 {
            for i in 0..2 {
                if l.scal.len() < 32 {
                    l.scal.push((ScalSlot::Generated, i));
                }
            }
        }
    }
    l
}

#[derive(Clone, Copy, Debug)]
pub struct Vary {
    pub is_tex: bool,
    pub ip: u32,
}

/// `RS_INST` as varyings and the temp routing for the fragment program.
#[derive(Clone, Debug)]
pub struct Route {
    pub vary: Vec<Vary>,
    /// Fragment temp <- varying index, or -1.
    pub route: [i8; NUM_TEMPS],
}

pub fn rs_route(st: &R300State, warn: &mut u32) -> Route {
    let count = (st.reg(RS_INST_COUNT) & 0xF) + 1;
    let mut r = Route { vary: Vec::new(), route: [-1; NUM_TEMPS] };
    for i in 0..count {
        let inst = st.reg(RS_INST_0 + 4 * i);
        for pass in 0..2 {
            let tex = pass == 0;
            let write = if tex { (inst >> 3) & 1 } else { (inst >> 14) & 1 } != 0;
            let ip = if tex { inst & 7 } else { (inst >> 11) & 7 };
            let addr = if tex { (inst >> 6) & 0x1F } else { (inst >> 17) & 0x1F };
            if !write {
                continue;
            }
            if r.vary.len() == NUM_VARYINGS {
                *warn |= WARN_VARYINGS;
                continue;
            }
            r.route[addr as usize] = r.vary.len() as i8;
            r.vary.push(Vary { is_tex: tex, ip });
        }
    }
    r
}

/// Per component: 0-3 = channel, -1 = 0.0, -2 = 1.0.  Formats the table does
/// not list read channel 0 four times, as the C table did.
pub(crate) const COL_FMT: [[i8; 4]; 16] = [
    [0, 1, 2, 3],
    [0, 1, 2, -1],
    [0, 1, 2, -2],
    [0, 0, 0, 0],
    [-1, -1, -1, 3],
    [-1, -1, -1, -1],
    [-1, -1, -1, -2],
    [0, 0, 0, 0],
    [-2, -2, -2, 3],
    [-2, -2, -2, -1],
    [-2, -2, -2, -2],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
];

fn rs_col_fmt(fmt: u32, c: &Vec4, i: usize) -> f32 {
    match COL_FMT[(fmt & 15) as usize][i] {
        s if s >= 0 => c[s as usize],
        -2 => 1.0,
        _ => 0.0,
    }
}

/// The four rasterized colours of a vertex (RS colour pointer 0-3).
pub fn vtx_colors(l: &Layout, out: &Outputs) -> [Vec4; 4] {
    let mut cols = [[0.0; 4]; 4];
    for (cp, col) in cols.iter_mut().enumerate() {
        if let Some(slot) = l.color[cp] {
            *col = out[slot];
        }
    }
    cols
}

/// Interpolant values of one vertex: texture coordinates from the output
/// vertex, colours from `cols` ([`vtx_colors`], or substituted for flat
/// shading and back faces), and the rasterizer's generated (s, t) (point
/// sprites, line stipple) past the vertex's own texture components.  Only the
/// first `route.vary.len()` entries of `v` are written.
pub fn rs_values(
    st: &R300State,
    l: &Layout,
    route: &Route,
    out: &Outputs,
    cols: &[Vec4; 4],
    sprite: Option<[f32; 2]>,
    v: &mut [Vec4; NUM_VARYINGS],
) {
    for (k, vary) in route.vary.iter().enumerate() {
        let ip = st.reg(RS_IP_0 + 4 * vary.ip);
        if vary.is_tex {
            let ptr = (ip & 0x3F) as usize;
            for (i, out_v) in v[k].iter_mut().enumerate() {
                let sel = (ip >> (13 + 3 * i)) & 7;
                let mut x = 0.0;
                if sel < 4 {
                    let s = ptr + sel as usize;
                    match l.scal.get(s) {
                        Some((ScalSlot::Generated, comp)) => x = sprite.map_or(0.0, |sp| sp[*comp]),
                        Some((ScalSlot::Slot(slot), comp)) => x = out[*slot][*comp],
                        None => {
                            if let Some(sp) = sprite {
                                // Point sprite: the rasterizer generates (s, t).
                                let c = s - l.scal.len();
                                x = match c {
                                    0 | 1 => sp[c],
                                    3 => 1.0,
                                    _ => 0.0,
                                };
                            }
                        }
                    }
                } else if sel == 5 {
                    x = 1.0;
                }
                *out_v = x;
            }
        } else {
            let cp = ((ip >> 6) & 7) as usize;
            let c = if cp < 4 { cols[cp] } else { [0.0; 4] };
            let rm = st.reg(GA_ROUND_MODE);
            // Vertex colours are clamped to [0, 1] unless GA_ROUND_MODE
            // selects the FP20 range (GL_CLAMP_VERTEX_COLOR).
            let mut cc = [0.0; 4];
            for i in 0..4 {
                let keep = (rm >> if i < 3 { 4 } else { 5 }) & 1 != 0;
                cc[i] = if keep { c[i] } else { fmin(fmax(c[i], 0.0), 1.0) };
            }
            for (i, out_v) in v[k].iter_mut().enumerate() {
                *out_v = rs_col_fmt((ip >> 9) & 0xF, &cc, i);
            }
        }
    }
}

use crate::pvs::{fmax, fmin};
