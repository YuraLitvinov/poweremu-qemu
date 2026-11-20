//! Primitive emission: expands assembled primitives into the vertices the
//! renderer draws (points and wide lines as triangles, polygon mode, culling).

use std::sync::OnceLock;

use super::assemble::Prov;
use super::regs::*;
use super::route::{rs_values, vtx_colors, Layout, Route};
use super::setup::bits_to_float;
use super::{CULL_BACK, CULL_FRONT, FACE_CW};
use crate::pvs::{fmax, fmin, Outputs, Vec4};
use crate::state::R300State;
use crate::us::R300Vertex;

pub struct Build<'a> {
    pub st: &'a R300State,
    pub lay: &'a Layout,
    pub route: &'a Route,
    /// Per source vertex.
    pub outs: &'a [Outputs],
    /// Per source vertex.
    pub xv: &'a [R300Vertex],
    pub n: u32,
    pub rt_width: u32,
    pub rt_height: u32,

    /// `GA_COLOR_CONTROL`.
    pub colctl: u32,
    /// Flat/solid shading or two-sided colours.
    pub fancy_colors: bool,
    /// Output colours 2/3 are back-face 0/1.
    pub back_colors: bool,
    /// `GA_SOLID_*`.
    pub solid: Vec4,
    /// `GA_POLY_MODE`, 0 when off.
    pub poly_mode: u32,
    /// `SU_CULL_MODE` for CPU culling, else 0.
    pub cull: u32,
    /// Half line width, pixels.
    pub line_hw: f32,
    /// `GA_LINE_CNTL.END_TYPE`.
    pub line_end: u32,
    /// `GA_LINE_STIPPLE_CONFIG.STIPPLE_SCALE`.
    pub stip_scale: f32,
    pub stip_reset: u32,
    /// Stipple accumulator, subpixels.
    pub stip_acc: f32,
    /// `GA_POINT_SIZE`, half, pixels.
    pub point_hw: f32,
    pub point_hh: f32,
    /// `GA_POINT_MINMAX` diameters, pixels.
    pub point_min: f32,
    pub point_max: f32,
    /// Per-vertex point size.
    pub point_psize: bool,

    pub tri: Vec<R300Vertex>,
    pub line: Vec<R300Vertex>,
}

/// SU_CULL_MODE: the card's winding is the one seen on screen with row 0 at
/// the top (as Mesa's r300 maps gallium's `front_ccw` straight to
/// `FRONT_FACE_CCW`), which is also how the host judges winding for the
/// clip-space positions this module produces (their y is up).
pub fn front_ccw(cull: u32) -> bool {
    cull & FACE_CW == 0
}

impl Build<'_> {
    /// Colours of source vertex `i` as the rasterizer sees them on a face.
    fn face_colors(&self, i: u32, back: bool) -> [Vec4; 4] {
        let mut cols = vtx_colors(self.lay, &self.outs[i as usize]);
        if back && self.back_colors {
            // Two-sided lighting: the setup engine picks colours 2 and 3 for
            // back faces (AMD R5xx guide 7.5).
            for c in 0..2 {
                if self.lay.color[2 + c].is_some() {
                    cols[c] = cols[2 + c];
                }
            }
        }
        cols
    }

    /// One output vertex from source vertex `i`: `prov` is its primitive's
    /// provoking-vertex candidates, `back` its facing, `sprite` the
    /// rasterizer's generated (s, t).
    fn vertex(&self, i: u32, prov: Option<&Prov>, back: bool, sprite: Option<[f32; 2]>) -> R300Vertex {
        let mut o = self.xv[i as usize];
        if !self.fancy_colors && sprite.is_none() {
            return o;
        }
        let mut cols = self.face_colors(i, back);
        if self.fancy_colors {
            let pv = prov.map_or(i, |p| p.v[((self.colctl >> 16) & 3) as usize]);
            let pc = self.face_colors(if pv < self.n { pv } else { i }, back);
            for c in 0..4 {
                let rgb = (self.colctl >> (4 * c)) & 3;
                let a = (self.colctl >> (4 * c + 2)) & 3;
                for k in 0..4 {
                    let mode = if k < 3 { rgb } else { a };
                    if mode == 0 {
                        cols[c][k] = self.solid[k];
                    } else if mode == 1 {
                        cols[c][k] = pc[c][k];
                    }
                }
            }
        }
        rs_values(self.st, self.lay, self.route, &self.outs[i as usize], &cols, sprite, &mut o.v);
        o
    }

    fn ndc_px(&self, p: &Vec4) -> (f32, f32) {
        let w = if p[3] != 0.0 { p[3] } else { 1.0 };
        ((p[0] / w + 1.0) * self.rt_width as f32 * 0.5, (1.0 - p[1] / w) * self.rt_height as f32 * 0.5)
    }

    /// Replaces the x, y of clip-space `p` by pixel position (x, y), keeping w.
    fn px_ndc(&self, x: f32, y: f32, p: &mut Vec4) {
        let w = if p[3] != 0.0 { p[3] } else { 1.0 };
        p[0] = (2.0 * x / self.rt_width as f32 - 1.0) * w;
        p[1] = (1.0 - 2.0 * y / self.rt_height as f32) * w;
    }

    /// A point: a screen-aligned square `GA_POINT_SIZE` wide (width in the high
    /// half, height in the low half, in sixths of a pixel), or the vertex's
    /// point size clamped to `GA_POINT_MINMAX`, with texture coordinates S0..S1
    /// left to right and T0..T1 bottom to top.
    pub fn point(&mut self, i: u32, prov: Option<&Prov>, back: bool) {
        let st = self.st;
        let p = self.xv[i as usize].pos;
        let (mut hw, mut hh) = (self.point_hw, self.point_hh);
        let s0 = bits_to_float(st.reg(GA_POINT_S0));
        let t0 = bits_to_float(st.reg(GA_POINT_T0));
        let s1 = bits_to_float(st.reg(GA_POINT_S1));
        let t1 = bits_to_float(st.reg(GA_POINT_T1));
        const CORNER: [usize; 6] = [0, 1, 2, 0, 2, 3];

        if self.point_psize
            && let Some(slot) = self.lay.psize
        {
            let d = fmin(fmax(self.outs[i as usize][slot][0], self.point_min), self.point_max);
            hw = d * 0.5;
            hh = hw;
        }
        let (cx, cy) = self.ndc_px(&p);
        // corners: top-left, top-right, bottom-right, bottom-left
        let x = [cx - hw, cx + hw, cx + hw, cx - hw];
        let y = [cy - hh, cy - hh, cy + hh, cy + hh];
        let sc = [s0, s1, s1, s0];
        let tc = [t1, t1, t0, t0];
        for &c in &CORNER {
            let mut o = self.vertex(i, prov, back, Some([sc[c], tc[c]]));
            // screen-space square at the vertex's depth
            o.pos[2] = p[2] / if p[3] != 0.0 { p[3] } else { 1.0 };
            o.pos[3] = 1.0;
            self.px_ndc(x[c], y[c], &mut o.pos);
            self.tri.push(o);
        }
    }

    /// A line from source vertex `i0` to `i1`.  The rasterizer stuffs (s, t)
    /// for the texture components past the vertex's own: s runs `GA_LINE_S0`
    /// to S1 along the line, or with `GA_LINE_STIPPLE_CONFIG.STIPPLE_SCALE`
    /// set, is the stipple accumulator (subpixels, 12 a pixel) times the scale,
    /// which the driver's program looks up in its stipple pattern.  Lines wider
    /// than a pixel become quads: offset across the major axis (END_TYPE
    /// horizontal, vertical, square), or perpendicular to the line (computed).
    pub fn line(&mut self, i0: u32, i1: u32, prov: Option<&Prov>, back: bool) {
        let p0 = self.xv[i0 as usize].pos;
        let p1 = self.xv[i1 as usize].pos;
        let (x0, y0) = self.ndc_px(&p0);
        let (x1, y1) = self.ndc_px(&p1);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        let (sp0, sp1);
        if self.stip_scale != 0.0 {
            if self.stip_reset == 1 {
                self.stip_acc = 0.0;
            }
            sp0 = self.stip_acc * self.stip_scale;
            self.stip_acc += len * 12.0;
            sp1 = self.stip_acc * self.stip_scale;
        } else {
            sp0 = bits_to_float(self.st.reg(GA_LINE_S0));
            sp1 = bits_to_float(self.st.reg(GA_LINE_S1));
        }

        if self.line_hw <= 0.75 {
            let a = self.vertex(i0, prov, back, Some([sp0, 0.0]));
            let b = self.vertex(i1, prov, back, Some([sp1, 0.0]));
            self.line.extend([a, b]);
            return;
        }
        let hw = self.line_hw;
        let (ox, oy) = if self.line_end == 3 && len > 0.0 {
            (-dy / len * hw, dx / len * hw)
        } else if dx.abs() >= dy.abs() {
            (0.0, hw) // x-major: widen vertically
        } else {
            (hw, 0.0)
        };
        let mut q = [R300Vertex::default(); 4];
        q[0] = self.vertex(i0, prov, back, Some([sp0, 0.0]));
        q[1] = q[0];
        q[2] = self.vertex(i1, prov, back, Some([sp1, 0.0]));
        q[3] = q[2];
        self.px_ndc(x0 - ox, y0 - oy, &mut q[0].pos);
        self.px_ndc(x0 + ox, y0 + oy, &mut q[1].pos);
        self.px_ndc(x1 + ox, y1 + oy, &mut q[2].pos);
        self.px_ndc(x1 - ox, y1 - oy, &mut q[3].pos);
        for k in [0, 1, 2, 0, 2, 3] {
            self.tri.push(q[k]);
        }
    }

    /// Winding on screen, as the host and `SU_CULL_MODE` judge it.
    fn tri_ccw(&self, a: u32, c: u32, d: u32) -> bool {
        let p = [self.xv[a as usize].pos, self.xv[c as usize].pos, self.xv[d as usize].pos];
        let mut x = [0.0f32; 3];
        let mut y = [0.0f32; 3];
        for k in 0..3 {
            let w = if p[k][3] != 0.0 { p[k][3] } else { 1.0 };
            x[k] = p[k][0] / w;
            y[k] = p[k][1] / w;
        }
        let mut area = (x[1] - x[0]) * (y[2] - y[0]) - (x[2] - x[0]) * (y[1] - y[0]);
        // a vertex behind the eye flips the projected winding
        if (p[0][3] < 0.0) ^ (p[1][3] < 0.0) ^ (p[2][3] < 0.0) {
            area = -area;
        }
        area > 0.0
    }

    pub fn triangle(&mut self, ix: &[u32; 3], prov: &Prov) {
        let mut front = self.tri_ccw(ix[0], ix[1], ix[2]) == front_ccw(self.st.reg(SU_CULL_MODE));
        let mut back = !front;
        // R300_FACING=front|back|flip: debug override of the facing used for
        // culling and the colour choice (told apart from depth and coverage
        // problems).
        static FACING: OnceLock<u8> = OnceLock::new();
        let facing = *FACING.get_or_init(|| match std::env::var("R300_FACING") {
            Ok(e) => {
                let f = match e.as_str() {
                    "front" => 1,
                    "back" => 2,
                    "flip" => 3,
                    _ => 0,
                };
                if f != 0 {
                    eprintln!("ppc-mac-gpu r300: R300_FACING={e}");
                }
                f
            }
            Err(_) => 0,
        });
        match facing {
            1 => (front, back) = (true, false),
            2 => (front, back) = (false, true),
            3 => (front, back) = (!front, !back),
            _ => {}
        }
        if (front && self.cull & CULL_FRONT != 0) || (back && self.cull & CULL_BACK != 0) {
            return;
        }
        // GA_POLY_MODE.FRONT_PTYPE / BACK_PTYPE: 0 points, 1 lines, 2 triangles
        let ptype = if self.poly_mode == 0 {
            2
        } else if front {
            (self.poly_mode >> 4) & 7
        } else {
            (self.poly_mode >> 7) & 7
        };
        match ptype {
            0 => {
                for &i in ix {
                    self.point(i, Some(prov), back);
                }
            }
            1 => {
                for k in 0..3 {
                    if (prov.edges >> k) & 1 != 0 {
                        self.line(ix[k], ix[(k + 1) % 3], Some(prov), back);
                    }
                }
            }
            _ => {
                for &i in ix {
                    let v = self.vertex(i, Some(prov), back, None);
                    self.tri.push(v);
                }
            }
        }
    }
}

/// S3.12 fixed point (`GA_SOLID_*`).
pub fn s312(v: u32) -> f32 {
    f32::from(v as u16 as i16) / 4096.0
}
