//! The vertex stage on the GPU: the vertex program and the vertex-stage math
//! run in the vertex shader ([`crate::pvs_glsl`]) instead of on the CPU.

use core::fmt::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use super::assemble::{assemble, Class};
use super::fetch::Fetch;
use super::regs::*;
use super::route::{Layout, Route, COL_FMT};
use super::setup::bits_to_float;
use super::DrawPacket;
use crate::pvs::{Program, NUM_INPUTS};
use crate::pvs_glsl::to_glsl;
use crate::state::{R300State, PVS_UCP_START};
use crate::us::NUM_VARYINGS;

/// Word layout of the vertex shader's buffer ([`DrawPacket::gpu_data`]):
///
/// * `0..1023`: the 256 constants (vec4), zero past `max_const`
/// * `1024` W, `1025` H, `1026` VTE bit 8 (uint), `1027` first word of the input
///   vertices (uint)
/// * `1028..1033`: viewport xscale, xoffset, yscale, yoffset, zscale, zoffset
/// * `1034` fog scale, `1035` fog offset, `1036` user clip enables (uint)
/// * `1038..1061`: the six user clip planes (vec4)
/// * `1064..`: one source vertex index per vertex drawn, then the input
///   vertices (nin vec4 each)
pub const GPU_IDX_BASE: usize = 1064;

/// Does this draw take the GPU vertex path?  Triangles only, with the colours
/// as they come out of the vertex program and no polygon mode: the CPU path
/// then does nothing to a vertex but copy it, so the shader can do all of it.
/// `R300_PVS_GPU=0` forces the CPU path, 1 the GPU path; the default is on.
pub fn wanted(st: &R300State, prim: u32, rects: bool, bypass: bool) -> bool {
    static MODE: OnceLock<i32> = OnceLock::new();
    let mode = *MODE.get_or_init(|| match std::env::var("R300_PVS_GPU") {
        Ok(e) => e.trim().parse().unwrap_or(0),
        Err(_) => 1,
    });
    if mode == 0 || rects || bypass || (1u32.checked_shl(prim).unwrap_or(0) & 0xE0F0) == 0 {
        return false;
    }
    if st.reg(GA_POLY_MODE) & 3 == 1 {
        return false;
    }
    let colctl = st.reg(GA_COLOR_CONTROL);
    for c in 0..4 {
        if (colctl >> (4 * c)) & 3 != 2 || (colctl >> (4 * c + 2)) & 3 != 2 {
            return false; // flat or solid shading
        }
    }
    st.reg(VAP_OUTPUT_VTX_FMT_0) & (3 << 3) == 0 // two-sided colours
}

/// The vertex stage as GLSL: the vertex program, then what `vtx_colors`,
/// `viewport_xform` and `rs_values` do.
#[allow(clippy::too_many_arguments)]
fn vs_text(
    st: &R300State,
    lay: &Layout,
    route: &Route,
    pvs_glsl: &str,
    nin: usize,
    slots: &[usize],
    fog: bool,
    fogsel: u32,
    clip: u32,
) -> String {
    let mut g = String::new();
    g.push_str(
        "layout(std430, set = 0, binding = 2) readonly buffer R300PVS { uint pvs_data[]; };\n\
         /* ms.xy: the multisample being drawn, as a clip-space shift */\n\
         layout(std140, set = 0, binding = 3) uniform R300MS { vec4 ms; };\n\
         layout(location = 0) out vec4 v0; layout(location = 1) out vec4 v1;\n\
         layout(location = 2) out vec4 v2; layout(location = 3) out vec4 v3;\n\
         layout(location = 4) out vec4 v4; layout(location = 5) out vec4 v5;\n\
         layout(location = 6) out vec4 v6; layout(location = 7) out vec4 v7;\n\
         layout(location = 8) out vec4 v8; layout(location = 9) out vec4 v9;\n\
         layout(location = 10) out vec4 aux;\n",
    );
    if clip & 0x3F != 0 {
        g.push_str("out gl_PerVertex { vec4 gl_Position; float gl_ClipDistance[6]; };\n");
    }
    let _ = write!(
        g,
        "float pvs_f(uint w) {{ return uintBitsToFloat(pvs_data[w]); }}\n\
         vec4 pvs_v4(uint w) {{ return vec4(pvs_f(w), pvs_f(w + 1u), pvs_f(w + 2u), pvs_f(w + 3u)); }}\n\
         vec4 pvs_const(int i) {{ return pvs_v4(uint(i) * 4u); }}\n\
         {pvs_glsl}\
         void main()\n{{\n\
         \x20   uint sv = pvs_data[{GPU_IDX_BASE}u + uint(gl_VertexIndex)];\n\
         \x20   uint vb = pvs_data[1027u] + sv * {}u;\n\
         \x20   vec4 IN[32]; vec4 OUT[32];\n\
         \x20   for (int i = 0; i < 32; i++) {{ IN[i] = vec4(0.0); OUT[i] = vec4(0.0); }}\n",
        nin * 4
    );
    for (j, slot) in slots.iter().enumerate().take(nin) {
        let _ = writeln!(g, "    IN[{slot}] = pvs_v4(vb + {}u);", j * 4);
    }
    let _ = write!(
        g,
        "    r300_pvs(IN, OUT);\n\
         \x20   vec4 c = OUT[{}];\n\
         \x20   float W = pvs_f(1024u), H = pvs_f(1025u);\n\
         \x20   float xs = pvs_f(1028u), xo = pvs_f(1029u), ys = pvs_f(1030u), yo = pvs_f(1031u), zs = pvs_f(1032u), zo = pvs_f(1033u);\n\
         \x20   vec4 p;\n\
         \x20   if (pvs_data[1026u] == 0u) {{\n\
         \x20       /* no perspective divide by the VTE: stay linear in clip space */\n\
         \x20       p = vec4((2.0 * xs / W) * c.x + (2.0 * xo / W - 1.0) * c.w,\n\
         \x20                -(2.0 * ys / H) * c.y + (1.0 - 2.0 * yo / H) * c.w,\n\
         \x20                pvs_data[1037u] != 0u ? c.z : 0.5 * (c.z + c.w), c.w);\n\
         \x20   }} else {{\n\
         \x20       float wx = xs * c.x + xo, wy = ys * c.y + yo;\n\
         \x20       p = vec4(2.0 * wx / W - 1.0, 1.0 - 2.0 * wy / H, zs * c.z + zo, 1.0);\n\
         \x20   }}\n",
        lay.pos
    );
    for cp in 0..4 {
        if let Some(slot) = lay.color[cp] {
            let rm = st.reg(GA_ROUND_MODE);
            let _ = writeln!(g, "    vec4 col{cp} = OUT[{slot}];");
            if rm & 0x10 == 0 {
                let _ = writeln!(g, "    col{cp}.xyz = clamp(col{cp}.xyz, 0.0, 1.0);");
            }
            if rm & 0x20 == 0 {
                let _ = writeln!(g, "    col{cp}.w = clamp(col{cp}.w, 0.0, 1.0);");
            }
        } else {
            let _ = writeln!(g, "    vec4 col{cp} = vec4(0.0);");
        }
    }
    for k in 0..NUM_VARYINGS {
        let Some(vary) = route.vary.get(k) else {
            let _ = writeln!(g, "    v{k} = vec4(0.0);");
            continue;
        };
        let ip = st.reg(RS_IP_0 + 4 * vary.ip);
        let _ = write!(g, "    v{k} = vec4(");
        for i in 0..4 {
            let e = if vary.is_tex {
                let (ptr, sel) = ((ip & 0x3F) as usize, ((ip >> (13 + 3 * i)) & 7) as usize);
                match lay.scal.get(ptr + sel) {
                    // rasterizer-generated (s, t): not a point here
                    Some((super::route::ScalSlot::Generated, _)) if sel < 4 => "0.0".to_string(),
                    Some((super::route::ScalSlot::Slot(slot), comp)) if sel < 4 => {
                        format!("OUT[{slot}].{}", ['x', 'y', 'z', 'w'][*comp])
                    }
                    _ => (if sel == 5 { "1.0" } else { "0.0" }).to_string(),
                }
            } else {
                let cp = ((ip >> 6) & 7) as usize;
                let sl = COL_FMT[((ip >> 9) & 0xF) as usize][i as usize];
                if sl >= 0 && cp < 4 {
                    format!("col{cp}.{}", ['x', 'y', 'z', 'w'][sl as usize])
                } else {
                    (if sl == -2 { "1.0" } else { "0.0" }).to_string()
                }
            };
            let _ = write!(g, "{}{e}", if i > 0 { ", " } else { "" });
        }
        g.push_str(");\n");
    }
    if fog {
        if fogsel < 4 {
            // The C source printed an empty initialiser here when the colour
            // exists, which is not valid GLSL; kept as is so the text matches
            // the C translator's.
            let present = lay.color[fogsel as usize].is_some();
            let _ = writeln!(g, "    float fsrc = {};", if present { "" } else { "0.0" });
            if let Some(slot) = lay.color[fogsel as usize] {
                let _ = writeln!(g, "    fsrc = OUT[{slot}].w;");
            }
        } else if fogsel == 4 {
            g.push_str("    float fsrc = c.w;\n");
        } else {
            g.push_str("    float fsrc = p.z / (p.w != 0.0 ? p.w : 1.0);\n");
        }
        g.push_str("    aux = vec4(fsrc * pvs_f(1034u) + pvs_f(1035u), 0.0, 0.0, 0.0);\n");
    } else {
        g.push_str("    aux = vec4(0.0);\n");
    }
    if clip & 0x3F != 0 {
        for k in 0..6 {
            let _ = writeln!(
                g,
                "    gl_ClipDistance[{k}] = ((pvs_data[1036u] >> {k}u) & 1u) != 0u ? dot(c, pvs_v4({}u)) : 1.0;",
                1038 + 4 * k
            );
        }
    }
    g.push_str("    gl_Position = vec4(p.xy + ms.xy * p.w, p.zw);\n}\n");
    g
}

/// The result of [`build`].
pub enum Outcome {
    /// This draw must take the CPU path (nothing consumed).
    NotApplicable,
    /// The packet's vertex stage runs on the GPU; this is its GLSL.
    Built { vs: String },
}

/// Builds a draw whose vertex stage runs on the GPU: fills the geometry fields
/// of `pkt` (`gpu_data` set, no `verts`).  Errors on a fetch failure.
#[allow(clippy::too_many_arguments)]
pub fn build(
    st: &R300State,
    f: &mut Fetch,
    prog: &Program,
    consts: &[[f32; 4]],
    lay: &Layout,
    route: &Route,
    pkt: &mut DrawPacket,
    order: &[u32],
    prim: u32,
    fog: bool,
    fogsel: u32,
    clip: u32,
) -> Result<Outcome, &'static str> {
    let n = order.len();
    let vte = st.reg(VAP_VTE_CNTL);
    let Ok(glsl) = to_glsl(prog) else {
        return Ok(Outcome::NotApplicable);
    };
    let Some(asm) = assemble(prim, n as u32).filter(|a| !a.indices.is_empty() && a.class == Class::Triangles) else {
        return Ok(Outcome::NotApplicable);
    };
    let nidx = asm.indices.len();
    let slots: Vec<usize> = (0..NUM_INPUTS).filter(|i| (prog.in_mask >> i) & 1 != 0).collect();
    let nin = slots.len();
    let words = GPU_IDX_BASE + nidx + n * nin * 4;
    let mut gd = vec![0u32; words];
    for (i, c) in consts.iter().enumerate().take(256) {
        if i as i32 > prog.desc.max_const {
            break;
        }
        for (k, v) in c.iter().enumerate() {
            gd[i * 4 + k] = v.to_bits();
        }
    }
    gd[1024] = (pkt.rt_width as f32).to_bits();
    gd[1025] = (pkt.rt_height as f32).to_bits();
    gd[1026] = (vte >> 8) & 1;
    gd[1027] = (GPU_IDX_BASE + nidx) as u32;
    for i in 0..6 {
        let w = if (vte >> i) & 1 != 0 {
            bits_to_float(st.reg(SE_VPORT_XSCALE + 4 * i as u32))
        } else if i % 2 != 0 {
            0.0
        } else {
            1.0
        };
        gd[1028 + i] = w.to_bits();
    }
    gd[1034] = st.reg(GA_FOG_SCALE);
    gd[1035] = st.reg(GA_FOG_OFFSET);
    gd[1036] = clip & 0x3F;
    gd[1037] = u32::from(pkt.depth_dx);
    gd[1038..1038 + 24].copy_from_slice(&st.pvs_mem[PVS_UCP_START * 4..PVS_UCP_START * 4 + 24]);
    gd[GPU_IDX_BASE..GPU_IDX_BASE + nidx].copy_from_slice(&asm.indices);
    for (v, &src) in order.iter().enumerate() {
        let Some(input) = f.vertex(src) else {
            return Err(f.why.unwrap_or("vertex data out of range"));
        };
        for (j, &slot) in slots.iter().enumerate() {
            let at = GPU_IDX_BASE + nidx + (v * nin + j) * 4;
            for k in 0..4 {
                gd[at + k] = input[slot][k].to_bits();
            }
        }
    }
    let vs = vs_text(st, lay, route, &glsl, nin, &slots, fog, fogsel, clip);
    static SAID: AtomicBool = AtomicBool::new(false);
    if !SAID.swap(true, Ordering::Relaxed) {
        eprintln!("ppc-mac-gpu r300: vertex programs run on the GPU (R300_PVS_GPU)");
    }
    pkt.gpu_vs = true;
    pkt.gpu_data = gd;
    pkt.num_verts = nidx as u32;
    pkt.prim_class = 0;
    pkt.cull = st.reg(SU_CULL_MODE) & 7;
    Ok(Outcome::Built { vs })
}
