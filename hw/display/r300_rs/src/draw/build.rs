//! Draw assembly: a `3D_DRAW_*` packet plus the register state to a draw
//! packet (`r300_draw_build`, `r300_draw_build_indexed`).

use super::assemble::{assemble, Class, Prov};
use super::emit::{s312, Build};
use super::fetch::{Fetch, ReadFn};
use super::gpu_vs;
use super::misc::{aa_positions, aa_samples, index_at, vc_swap, Arrays};
use super::pvsrec;
use super::regs::*;
use super::route::{output_layout, rs_route, rs_values, vtx_colors};
use super::setup::{bits_to_float, sc_bias, set_depth, set_textures, set_uniforms, viewport_xform};
use super::{ColorDesc, DrawPacket, CULL_FACE_CW_MASK, WARN_FLOW, WARN_PRIM, WARN_PVS, WARN_RTFMT};
use crate::pvs::{fmax, fmin, FlowControl, Inputs, Outputs, Program, ProgramDesc, Unsupported, NUM_OUTPUTS};
use crate::state::{R300State, PVS_CONST_START, PVS_MEM_VECS};
use crate::us::{cb_view, num_targets, out_fmt, to_glsl, CbView, FsDesc, R300Vertex};

/// Index buffer from memory (`INDX_BUFFER` after a `DRAW_INDX_2` with no
/// inline indices), already fetched.
pub struct Indices<'a> {
    /// Raw dwords as fetched (`VC_SWAP` is applied here).
    pub dw: &'a [u32],
}

/// Builds a draw from a `3D_DRAW_VBUF_2` (0x34), `3D_DRAW_IMMD_2` (0x35) or
/// `3D_DRAW_INDX_2` (0x36) body `d`.  Errors when the draw cannot be assembled
/// at all; smaller omissions are flagged in the packet's `warn`.
pub fn draw_build(
    st: &R300State,
    arr: &Arrays,
    opcode: u32,
    d: &[u32],
    read: &mut ReadFn,
) -> Result<DrawPacket, &'static str> {
    let Some(&vf) = d.first() else {
        return Err("empty draw packet");
    };
    let payload = &d[1..];
    let n = vf >> 16;
    if n == 0 {
        return Err("no vertices");
    }
    let order: Vec<u32> = if opcode == 0x36 {
        // DRAW_INDX_2, inline
        let i32_indices = (vf >> 11) & 1 != 0;
        (0..n).map(|i| index_at(payload, i32_indices, i)).collect()
    } else {
        // DRAW_VBUF_2, DRAW_IMMD_2
        (0..n).collect()
    };
    core(st, arr, vf, &order, if opcode == 0x35 { Some(payload) } else { None }, read)
}

/// The same for `DRAW_INDX_2` whose indices come from memory: `vf` is the
/// packet's `VAP_VF_CNTL` and `idx` the dwords `INDX_BUFFER` pointed at, read as
/// they lie (`VC_SWAP` is applied here, as the card's vertex cache does for all
/// fetched data).
pub fn draw_build_indexed(
    st: &R300State,
    arr: &Arrays,
    vf: u32,
    idx: &Indices,
    read: &mut ReadFn,
) -> Result<DrawPacket, &'static str> {
    let n = vf >> 16;
    let swap = st.reg(VAP_CNTL_STATUS) & 3;
    let i32_indices = (vf >> 11) & 1 != 0;
    if n == 0 {
        return Err("no vertices");
    }
    let needed = if i32_indices { u64::from(n) } else { u64::from(n).div_ceil(2) };
    if needed > idx.dw.len() as u64 {
        return Err("index buffer shorter than the draw");
    }
    let swapped: Vec<u32> = idx.dw.iter().map(|&w| vc_swap(w, swap)).collect();
    let order: Vec<u32> = (0..n).map(|i| index_at(&swapped, i32_indices, i)).collect();
    core(st, arr, vf, &order, None, read)
}

fn clamp01(v: f32) -> f32 {
    fmin(fmax(v, 0.0), 1.0)
}

/// Everything after vertex ordering: `order` holds the source vertex of each
/// drawn vertex, `immd` is the `DRAW_IMMD_2` vertex data.
fn core(
    st: &R300State,
    arr: &Arrays,
    vf: u32,
    order: &[u32],
    immd: Option<&[u32]>,
    read: &mut ReadFn,
) -> Result<DrawPacket, &'static str> {
    let mut pkt = DrawPacket::default();
    let mut n = order.len() as u32;
    let mut prim = vf & 0xF;
    let pitch_reg = st.reg(RB3D_COLORPITCH0);
    let cntl0 = st.reg(VAP_PVS_CODE_CNTL_0);
    let cc = st.reg(VAP_PVS_CONST_CNTL);
    let bypass = (st.reg(VAP_CNTL_STATUS) >> 8) & 1 != 0;
    let (tl, br) = (st.reg(SC_SCISSORS_TL), st.reg(SC_SCISSORS_BR));
    let clip = st.reg(VAP_CLIP_CNTL);
    let fogsel = st.reg(GB_SELECT) & 7;
    let fog = st.reg(FG_FOG_BLEND) & 1 != 0;
    let fog_scale = bits_to_float(st.reg(GA_FOG_SCALE));
    let fog_off = bits_to_float(st.reg(GA_FOG_OFFSET));
    let bias = sc_bias(st);

    // Colour buffer
    pkt.rt_gpu_addr = st.reg(RB3D_COLOROFFSET0) & !0x1F;
    pkt.rt_pitch = pitch_reg & 0x3FFE;
    pkt.rt_format = (pitch_reg >> 21) & 0xF;
    let (view, bpp) = cb_view(pkt.rt_format, st.reg(US_OUT_FMT_0));
    pkt.rt_view = view;
    pkt.rt_bpp = bpp;
    if (bpp == 1 && (pitch_reg >> 19) & 3 != 0) || (bpp == 2 && ((pitch_reg >> 19) & 3) >= 2) {
        pkt.warn |= WARN_RTFMT; // swap across pixels: not done
    }
    if view == CbView::None {
        return Err("colour buffer format not supported");
    }
    // Render targets B-D (multiple render targets or multiwrites)
    pkt.num_cb = num_targets(st) as u32;
    for k in 1..pkt.num_cb as usize {
        let pr = st.reg(RB3D_COLORPITCH0 + 4 * k as u32);
        let (cview, cbpp) = {
            let format = (pr >> 21) & 0xF;
            cb_view(format, out_fmt(st, k))
        };
        pkt.cb[k] = ColorDesc {
            gpu_addr: st.reg(RB3D_COLOROFFSET0 + 4 * k as u32) & !0x1F,
            pitch: pr & 0x3FFE,
            format: (pr >> 21) & 0xF,
            bpp: cbpp,
            view: cview as u32,
        };
        if cview == CbView::None || pkt.cb[k].pitch == 0 {
            pkt.warn |= WARN_RTFMT;
            pkt.num_cb = k as u32; // bind the ones before it
            break;
        }
    }
    let lo = |v: u32| (v & 0x1FFF).saturating_sub(bias);
    let lo_hi = |v: u32| ((v >> 13) & 0x1FFF).saturating_sub(bias);
    let hi = |v: u32| if (v & 0x1FFF) >= bias { (v & 0x1FFF) - bias + 1 } else { 0 };
    let hi_hi = |v: u32| if ((v >> 13) & 0x1FFF) >= bias { ((v >> 13) & 0x1FFF) - bias + 1 } else { 0 };
    pkt.scissor = [lo(tl), lo_hi(tl), hi(br), hi_hi(br)];
    {
        // The DRM clips with up to four cliprects (SC_CLIPRECT_TL/BR_0..3)
        // chosen by SC_CLIP_RULE: bit (in0 | in1<<1 | in2<<2 | in3<<3) says
        // whether a pixel inside exactly those rectangles is drawn.  Use the
        // bounding box of the rectangles that take part in a passing
        // combination; a rule that passes pixels outside every rectangle
        // (index 0) clips nothing.
        let rule = st.reg(SC_CLIP_RULE) & 0xFFFF;
        let outside_ok = rule & 1 != 0;
        let (mut bx0, mut by0, mut bx1, mut by1) = (u32::MAX, u32::MAX, 0u32, 0u32);
        let mut k = 0;
        while k < 4 && !outside_ok {
            let used = (0..16).any(|idx| (idx >> k) & 1 != 0 && (rule >> idx) & 1 != 0);
            if used {
                let ctl = st.reg(0x43B0 + 8 * k);
                let cbr = st.reg(0x43B4 + 8 * k);
                bx0 = bx0.min(lo(ctl));
                by0 = by0.min(lo_hi(ctl));
                bx1 = bx1.max(hi(cbr));
                by1 = by1.max(hi_hi(cbr));
            }
            k += 1;
        }
        if !outside_ok && bx0 != u32::MAX {
            pkt.scissor[0] = pkt.scissor[0].max(bx0);
            pkt.scissor[1] = pkt.scissor[1].max(by0);
            pkt.scissor[2] = pkt.scissor[2].min(bx1);
            pkt.scissor[3] = pkt.scissor[3].min(by1);
        } else if !outside_ok {
            pkt.scissor[2] = 0; // nothing passes
            pkt.scissor[3] = 0;
        }
    }
    pkt.rt_width = pkt.rt_pitch;
    // an open scissor (Linux) says nothing about the buffer: it is as tall as
    // the screen
    pkt.rt_height = if st.le_screen_h != 0 && ((br >> 13) & 0x1FFF) >= 0x1F00 { st.le_screen_h } else { pkt.scissor[3] };
    if pkt.rt_width == 0 || pkt.rt_height == 0 {
        return Err("empty colour buffer");
    }

    pkt.aa_samples = aa_samples(st);
    pkt.aa_pos = aa_positions(st);

    // Viewport depth range and z clipping, once per draw (see
    // DrawPacket::depth_min).
    {
        let vte = st.reg(VAP_VTE_CNTL);
        let zs_en = (vte >> 4) & 1 != 0;
        let zs = if zs_en { bits_to_float(st.reg(SE_VPORT_XSCALE + 16)) } else { 1.0 };
        let zo = if (vte >> 5) & 1 != 0 { bits_to_float(st.reg(SE_VPORT_XSCALE + 20)) } else { 0.0 };
        pkt.depth_dx = (st.reg(VAP_CNTL) >> 22) & 1 != 0;
        if zs_en && (vte >> 8) & 1 == 0 {
            pkt.depth_min = if pkt.depth_dx { zo } else { zo - zs };
            pkt.depth_max = zo + zs;
        } else {
            pkt.depth_min = 0.0; // z is already the depth (or not transformed)
            pkt.depth_max = 1.0;
        }
        pkt.depth_min = clamp01(pkt.depth_min);
        pkt.depth_max = clamp01(pkt.depth_max);
        pkt.depth_clip = (st.reg(VAP_CLIP_CNTL) >> 16) & 1 == 0;
    }

    pkt.uniforms = set_uniforms(st);
    set_textures(st, &mut pkt.uniforms, &mut pkt.tex, &mut pkt.warn);
    pkt.depth = set_depth(st, &mut pkt.uniforms, &mut pkt.warn);
    // Mesa gives a window's colour buffer a wider row pitch than its depth
    // buffer (64 against 32 pixels for a 32 pixel window).  The depth buffer
    // only has to cover what is drawn, so when the scissor fits inside the depth
    // pitch the render area is that wide; wider than the pitch the draw cannot
    // use the depth buffer.
    if pkt.depth.attach && pkt.depth.pitch != 0 && pkt.depth.pitch < pkt.rt_width && pkt.scissor[2] <= pkt.depth.pitch {
        pkt.rt_width = pkt.depth.pitch;
    }
    // Also when the masked colour target is the depth buffer's own memory (Mesa
    // blits packed depth/stencil as ARGB8888 words, then draws stencil ops with
    // that same buffer bound as colour): the colour store would write the old
    // word back over the stencil and depth the same invocation just updated.
    if pkt.depth.attach
        && (pkt.depth.pitch > pkt.rt_pitch || pkt.rt_gpu_addr == pkt.depth.gpu_addr)
        && pkt.depth.pitch >= pkt.rt_pitch
        && pkt.scissor[2] <= pkt.depth.pitch
        && pkt.num_cb == 1
        && pkt.aa_samples == 1
        && st.reg(RB3D_COLOR_CHANNEL_MASK) & 0xF == 0
    {
        pkt.rt_dummy = true;
        pkt.rt_pitch = pkt.depth.pitch;
        pkt.rt_width = pkt.depth.pitch;
    }

    let mut f = Fetch {
        st,
        arr,
        read,
        immd,
        vtx_size: st.reg(VAP_VTX_SIZE) & 0x7F,
        swap: st.reg(VAP_CNTL_STATUS) & 3,
        warn: 0,
        why: None,
    };

    // Vertex program
    let mut consts = vec![[0.0f32; 4]; 256];
    for (i, c) in consts.iter_mut().enumerate() {
        let vec = PVS_CONST_START + (cc & 0xFF) as usize + i;
        if vec >= PVS_MEM_VECS {
            break;
        }
        for (k, v) in c.iter_mut().enumerate() {
            *v = f32::from_bits(st.pvs_mem[vec * 4 + k]);
        }
        if vec + 1 >= PVS_MEM_VECS {
            break;
        }
    }
    let mut flow = FlowControl { opc: st.reg(VAP_PVS_FLOW_CNTL_OPC), ..Default::default() };
    for i in 0..16 {
        flow.addrs[i] = st.reg(VAP_PVS_FLOW_CNTL_ADDRS_0 + 4 * i as u32);
        flow.loops[i] = st.reg(VAP_PVS_FLOW_CNTL_LOOP_INDEX_0 + 4 * i as u32);
    }
    let prog = Program::new(ProgramDesc {
        code: &st.pvs_mem[..],
        first_inst: (cntl0 & 0x3FF) as usize,
        last_inst: ((cntl0 >> 20) & 0x3FF) as usize,
        consts: &consts,
        max_const: ((cc >> 16) & 0xFF) as i32,
        flow,
    });

    let lay = output_layout(st);
    let route = rs_route(st, &mut pkt.warn);
    let mut desc = FsDesc { route: route.route, vs_gpu: None };

    if prim != 8 && !bypass && !pvsrec::wanted(n) && gpu_vs::wanted(st, prim, false, bypass) {
        if let gpu_vs::Outcome::Built { vs } =
            gpu_vs::build(st, &mut f, &prog, &consts, &lay, &route, &mut pkt, order, prim, fog, fogsel, clip)?
        {
            pkt.warn |= f.warn;
            desc.vs_gpu = Some(&vs);
            pkt.glsl = to_glsl(st, &desc, st.glsl_flags)?;
            return Ok(pkt);
        }
    }

    // Rectangle lists: three corners a rectangle, the card completes the
    // parallelogram (v0 + v2 - v1) and draws it as a quad.
    let rects = prim == 8;
    let nsrc = if rects { (n / 3 * 4) as usize } else { n as usize };

    // Transform every vertex in draw order (cheap next to the GPU work).
    let mut xv = vec![R300Vertex::default(); nsrc];
    let mut outs: Vec<Outputs> = vec![[[0.0; 4]; NUM_OUTPUTS]; nsrc];
    let rec = !rects && !bypass && pvsrec::wanted(n);
    let mut cin: Vec<Inputs> = Vec::new();
    // Fetch every vertex first, then run the vertex program over all of them at
    // once (Program::run_batch splits a big batch across worker threads).
    // oidx[k] is the output slot of the k-th vertex; a rectangle's fourth corner
    // is derived from the other three once their outputs exist.
    let mut ins: Vec<Inputs> = Vec::new();
    let mut oidx: Vec<u32> = Vec::new();
    let mut corners: Vec<usize> = Vec::new();
    let mut j = 0usize;
    for (i, &src) in order.iter().enumerate() {
        if j >= nsrc {
            break;
        }
        let Some(input) = f.vertex(src) else {
            return Err(f.why.unwrap_or("vertex data out of range"));
        };
        if rec {
            cin.push(input);
        }
        if bypass {
            outs[j] = input;
        } else {
            ins.push(input);
            oidx.push(j as u32);
        }
        if rects && i % 3 == 2 {
            j += 1;
            corners.push(j);
        }
        j += 1;
    }
    if !ins.is_empty() {
        let u = prog.run_batch(&ins, &mut outs, &oidx);
        if u.0 & Unsupported::FLOW.0 != 0 {
            pkt.warn |= WARN_FLOW;
        }
        if u.0 & !Unsupported::FLOW.0 != 0 {
            pkt.warn |= WARN_PVS;
        }
    }
    for &j in &corners {
        let (a, mid, c) = (outs[j - 3], outs[j - 2], outs[j - 1]);
        for (o, out) in outs[j].iter_mut().enumerate() {
            *out = [0, 1, 2, 3].map(|k| a[o][k] + c[o][k] - mid[o][k]);
        }
    }
    if rec {
        pvsrec::record(&prog, &consts, n, &cin, &outs);
    }
    for j in 0..nsrc {
        let out = &outs[j];
        xv[j].pos = viewport_xform(st, pkt.rt_width, pkt.rt_height, pkt.depth_dx, &out[lay.pos]);
        let cols = vtx_colors(&lay, out);
        rs_values(st, &lay, &route, out, &cols, None, &mut xv[j].v);
        if fog {
            let src = match fogsel {
                0..=3 => lay.color[fogsel as usize].map_or(0.0, |s| out[s][3]),
                4 => out[lay.pos][3],
                _ => xv[j].pos[2] / if xv[j].pos[3] != 0.0 { xv[j].pos[3] } else { 1.0 },
            };
            xv[j].aux[0] = src * fog_scale + fog_off;
        }
        // User clip planes (VAP_CLIP_CNTL.UCP_ENA_n): the distance of the
        // clip-space position from plane n in PVS memory 1024 + n.
        for k in 0..6 {
            let base = (crate::state::PVS_UCP_START + k) * 4;
            let pl = [0, 1, 2, 3].map(|i| f32::from_bits(st.pvs_mem[base + i]));
            let cp = &out[lay.pos];
            xv[j].ucp[k] = if (clip >> k) & 1 != 0 {
                cp[0] * pl[0] + cp[1] * pl[1] + cp[2] * pl[2] + cp[3] * pl[3]
            } else {
                1.0
            };
        }
    }
    if rects {
        prim = 13;
        n = nsrc as u32;
    }
    pkt.warn |= f.warn;

    // Primitive assembly
    let colctl = st.reg(GA_COLOR_CONTROL);
    let back_colors = st.reg(VAP_OUTPUT_VTX_FMT_0) & (3 << 3) != 0;
    let mut fancy_colors = (0..4).any(|c| (colctl >> (4 * c)) & 3 != 2 || (colctl >> (4 * c + 2)) & 3 != 2);
    fancy_colors |= back_colors;
    let poly = st.reg(GA_POLY_MODE);
    let (lc, sc, ps, mm) =
        (st.reg(GA_LINE_CNTL), st.reg(GA_LINE_STIPPLE_CONFIG), st.reg(GA_POINT_SIZE), st.reg(GA_POINT_MINMAX));
    let stip_reset = sc & 3;
    let mut b = Build {
        st,
        lay: &lay,
        route: &route,
        outs: &outs,
        xv: &xv,
        n,
        rt_width: pkt.rt_width,
        rt_height: pkt.rt_height,
        colctl,
        fancy_colors,
        back_colors,
        solid: [
            s312(st.reg(GA_SOLID_RG) >> 16),
            s312(st.reg(GA_SOLID_RG)),
            s312(st.reg(GA_SOLID_BA) >> 16),
            s312(st.reg(GA_SOLID_BA)),
        ],
        poly_mode: if poly & 3 == 1 { poly } else { 0 },
        cull: 0,
        line_hw: (lc & 0xFFFF) as f32 / 12.0,
        line_end: (lc >> 16) & 3,
        stip_scale: bits_to_float(sc & !3),
        stip_reset,
        stip_acc: if stip_reset != 0 { 0.0 } else { (st.reg(GA_LINE_STIPPLE_VALUE) & 0xFF_FFFF) as f32 },
        point_hw: ((ps >> 16) & 0xFFFF) as f32 / 12.0,
        point_hh: (ps & 0xFFFF) as f32 / 12.0,
        point_min: (mm & 0xFFFF) as f32 / 6.0,
        point_max: (mm >> 16) as f32 / 6.0,
        point_psize: lay.psize.is_some(),
        tri: Vec::new(),
        line: Vec::new(),
    };
    // The setup unit culls polygons only (triangles, fans, strips, quads, quad
    // strips, polygons).  The host culls filled triangles; the CPU does it when
    // polygon mode turns them into lines or points.
    pkt.cull = if (1u32.checked_shl(prim).unwrap_or(0) & 0xE0F0) != 0 { st.reg(SU_CULL_MODE) & 7 } else { 0 };
    if b.poly_mode != 0 && pkt.cull != 0 {
        b.cull = pkt.cull;
        pkt.cull &= CULL_FACE_CW_MASK;
    }

    let asm = assemble(prim, n).filter(|a| !a.indices.is_empty());
    let Some(asm) = asm else {
        if prim != 0 {
            pkt.warn |= WARN_PRIM;
        }
        return Err("primitive type not supported");
    };
    match asm.class {
        Class::Triangles => {
            for (ix, prov) in asm.indices.as_chunks::<3>().0.iter().zip(&asm.prov) {
                b.triangle(&[ix[0], ix[1], ix[2]], prov);
            }
        }
        Class::Lines => {
            if b.stip_reset == 2 {
                b.stip_acc = 0.0;
            }
            for (ix, prov) in asm.indices.as_chunks::<2>().0.iter().zip(&asm.prov) {
                b.line(ix[0], ix[1], Some(prov), false);
            }
        }
        Class::Points => {
            for (&i, prov) in asm.indices.iter().zip(&asm.prov) {
                b.point(i, Some(prov), false);
            }
        }
    }
    let (tri, line) = (std::mem::take(&mut b.tri), std::mem::take(&mut b.line));
    if tri.is_empty() && !line.is_empty() {
        pkt.num_verts = line.len() as u32;
        pkt.verts = line;
        pkt.prim_class = 1;
    } else {
        pkt.num_verts = tri.len() as u32;
        pkt.num_line_verts = line.len() as u32;
        pkt.verts = tri;
        pkt.verts.extend(line);
        pkt.prim_class = 0;
    }
    pkt.glsl = to_glsl(st, &desc, st.glsl_flags)?;
    let _: &[Prov] = &asm.prov;
    Ok(pkt)
}
