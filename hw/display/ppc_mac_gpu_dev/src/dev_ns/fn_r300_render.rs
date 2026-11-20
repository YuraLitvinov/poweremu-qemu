//! Stages of an R300 draw ("r300_render"), split out of the original single function.
use super::*;

/// Draw-packet warning bits and the message logged (once) for each, in report order.
const DRAW_WARNINGS: [(u32, &str); 7] = [
    (0, "vertex program uses unimplemented features"),
    (1, "depth format 13E3 or reserved: treated as integer Z"),
    (2, "texture format not implemented yet"),
    (5, "colour buffer endian swap across pixels ignored"),
    (3, "vertex format not implemented"),
    (6, "too many interpolants"),
    (7, "vertex program flow control ran away"),
];

/// Reports the warnings the draw builder flagged on `pkt`.
pub fn r300_report_draw_warnings(pkt: &R300DrawPacket) {
    for (bit, message) in DRAW_WARNINGS {
        if pkt.warn & (1 << bit) != 0 {
            r300_warn_once(message, None);
        }
    }
}

/// Largest texture the GART read path will copy into host memory.
const MAX_GART_TEXTURE_BYTES: uint64_t = 1 << 30;

/// Binds the draw's textures: those in VRAM are made coherent with the renderer; those in
/// GART/AGP memory are copied into host memory. A texture that cannot be read is unbound,
/// and its slot in the shader's texture info is cleared.
pub fn r300_prepare_textures(
    s: &mut PPCMacGPUFields,
    pkt: &mut R300DrawPacket,
    vram: crate::src::qemu::Vram,
) {
    for unit in 0..pkt.tex.len() {
        if pkt.tex[unit].bound && !r300_bind_texture(s, &mut pkt.tex[unit], vram) {
            pkt.uniforms.tex_info[unit][0] = 0;
        }
    }
}

/// Makes one bound texture available to the renderer. Returns false (and unbinds it) if it
/// is in neither VRAM nor readable GART memory.
fn r300_bind_texture(
    s: &mut PPCMacGPUFields,
    td: &mut R300TexDesc,
    vram: crate::src::qemu::Vram,
) -> bool {
    let tlen = td.size_bytes as uint64_t;
    let ga = td.gpu_addr;
    if r300_to_vram(s, &mut td.gpu_addr, tlen) {
        let start = td.gpu_addr as uint64_t;
        gpu_vram_access(s, start, start.wrapping_add(tlen), false);
        if crate::src::qemu::env_flag(c"PPCGPU_TEX_LOG") {
            r300_trace_vram_texture(s, td, ga, tlen, vram);
        }
        return true;
    }
    if !r300_read_gart_texture(s, td, ga, tlen) {
        r300_warn_once("texture outside VRAM and the GART", None);
        td.bound = false;
        return false;
    }
    if crate::src::qemu::env_flag(c"PPCGPU_GART_LOG") {
        r300_trace_gart_texture(td, ga, tlen);
    }
    static SAID: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
    if !SAID.get() {
        SAID.set(true);
        crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: texture in AGP/GART memory at {:08x} ({}x{}), read through the GART\n", ((ga) as u32), ((td.width) as u32), ((td.height) as u32)));
    }
    true
}

/// Copies a texture from GART/AGP guest memory at `ga` into a fresh host buffer in
/// `td.host_data`. On failure the buffer is released and `host_data` is left null.
fn r300_read_gart_texture(
    s: &mut PPCMacGPUFields,
    td: &mut R300TexDesc,
    ga: uint32_t,
    tlen: uint64_t,
) -> bool {
    td.host_data = ::core::ptr::null_mut();
    let mut ok = false;
    if tlen != 0 && tlen <= MAX_GART_TEXTURE_BYTES {
        if let Some(mut buf) = crate::src::qemu::try_alloc_bytes(tlen as usize) {
            if r300_read_gpu_bytes(s, ga, &mut buf) {
                td.host_data = crate::src::qemu::c_malloc_copy(&buf);
                ok = !td.host_data.is_null();
            }
        }
    }
    if !ok {
        crate::src::qemu::c_free_slot(&mut td.host_data);
    }
    ok
}

impl R300DrawPacket {
    /// A draw packet with every field cleared, ready for the draw builder to fill in.
    pub fn empty() -> Self {
        R300DrawPacket {
        rt_gpu_addr: 0,
        rt_pitch: 0,
        rt_width: 0,
        rt_height: 0,
        rt_format: 0,
        rt_bpp: 0,
        rt_dummy: false,
        harvest_hint: false,
        depth_min: 0.,
        depth_max: 0.,
        depth_clip: false,
        depth_dx: false,
        rt_view: 0,
        scissor: [0; 4],
        num_cb: 0,
        cb: [R300ColorDesc {
            gpu_addr: 0,
            pitch: 0,
            format: 0,
            bpp: 0,
            view: 0,
        }; 4],
        depth: R300DepthDesc {
            attach: false,
            gpu_addr: 0,
            pitch: 0,
            bpp: 0,
            format: 0,
        },
        cull: 0,
        aa_samples: 0,
        aa_pos: [[0.; 2]; 6],
        glsl: ::core::ptr::null_mut::<::core::ffi::c_char>(),
        uniforms: R300FSUniforms {
            consts: [[0.; 4]; 32],
            blend_color: [0.; 4],
            tex_swz: [[0; 4]; 16],
            tex_info: [[0; 4]; 16],
            cblend: 0,
            ablend: 0,
            chanmask: 0,
            alpha_func: 0,
            out_sel: [0; 4],
            rt_swap32: 0,
            clip_rule: 0,
            rt_endian: 0,
            rop: 0,
            cliprect: [[0; 4]; 4],
            zinfo: [0; 4],
            zpass_count: 0,
            poly_en: 0,
            pad2: [0; 2],
            tex_border: [[0.; 4]; 16],
            tex_lod: [[0.; 4]; 16],
            tex_dim: [[0; 4]; 16],
            fog_color: [0.; 4],
            fog_blend: 0,
            depth_src: 0,
            pad3: [0; 2],
            poly_offset: [0.; 4],
            tex_addr: [[0; 4]; 16],
            tex_lvl: [[[0; 4]; 9]; 16],
        },
        tex: [R300TexDesc {
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
            host_data: ::core::ptr::null_mut::<uint8_t>(),
            dim: 0,
            depth: 0,
            levels: 0,
            lvl_off: [0; 12],
            lvl_pitch: [0; 12],
            lvl_rows: [0; 12],
            size_bytes: 0,
        }; 16],
        verts: ::core::ptr::null_mut::<R300Vertex>(),
        gpu_vs: false,
        gpu_data: ::core::ptr::null_mut::<uint32_t>(),
        gpu_words: 0,
        num_verts: 0,
        num_line_verts: 0,
        prim_class: 0,
        warn: 0,
    }
    }
}

impl R300DrawPacket {
    /// Every built vertex, including the extra line vertices.
    pub fn vertices_with_lines(&self) -> &[R300Vertex] {
        if self.verts.is_null() {
            return &[];
        }
        let n = self.num_verts.wrapping_add(self.num_line_verts) as usize;
        // SAFETY: the draw builder allocates `num_verts + num_line_verts` vertices.
        unsafe { core::slice::from_raw_parts(self.verts, n) }
    }
}

/// A colour buffer that lives in GART/AGP memory. The draw renders into a scratch area at the
/// end of VRAM instead; afterwards the touched rows are copied back to the guest pages and
/// the scratch area's previous contents are restored.
pub struct GartColourBuffer {
    guest_addr: uint32_t,
    scratch: uint32_t,
    len: uint64_t,
    first_row: uint32_t,
    end_row: uint32_t,
    saved: Vec<u8>,
}

impl GartColourBuffer {
    /// Rows of the render target the draw can touch, from the vertices' y extent and the scissor.
    fn touched_rows(pkt: &R300DrawPacket) -> (uint32_t, uint32_t, (f32, f32)) {
        let (mut min_y, mut max_y) = if pkt.gpu_vs { (-1.0f32, 1.0f32) } else { (1e30f32, -1e30f32) };
        for v in pkt.vertices_with_lines() {
            min_y = min_y.min(v.pos[1]);
            max_y = max_y.max(v.pos[1]);
        }
        let height = pkt.rt_height as f32;
        let top = (1.0 - max_y) * height / 2.0;
        let bottom = (1.0 - min_y) * height / 2.0;
        let first = if top > 0.0 { top as uint32_t } else { 0 };
        let end = if bottom > 0.0 { bottom.ceil() as uint32_t } else { 0 };
        (first.max(pkt.scissor[1]), end.min(pkt.scissor[3]).min(pkt.rt_height), (min_y, max_y))
    }

    /// Redirects the packet's colour buffer (at guest address `guest_addr`) to scratch VRAM.
    /// Returns None if that is not possible (the caller warns and drops the draw).
    pub fn begin(
        s: &mut PPCMacGPUFields,
        pkt: &mut R300DrawPacket,
        guest_addr: uint32_t,
        rt_len: uint64_t,
        vram: crate::src::qemu::Vram,
    ) -> Option<Self> {
        let pitch_bytes = pkt.rt_pitch.wrapping_mul(pkt.rt_bpp);
        pkt.rt_gpu_addr = guest_addr;
        let (first_row, end_row, y_extent) = Self::touched_rows(pkt);
        if crate::src::qemu::env_flag(c"PPCGPU_GART_LOG") {
            r300_trace_gart_render_target(pkt, guest_addr, y_extent, first_row, end_row);
        }
        if pkt.aa_samples != 1
            || pitch_bytes == 0
            || end_row <= first_row
            || rt_len > s.vram_size / 4
        {
            return None;
        }
        let mut saved = crate::src::qemu::try_alloc_bytes(rt_len as usize)?;
        let scratch = (s.vram_size.wrapping_sub(rt_len) & !0xfff) as uint32_t;
        gpu_vram_access(s, scratch as uint64_t, (scratch as uint64_t).wrapping_add(rt_len), true);
        crate::src::qemu::renderer_flush_sync(s);
        vram.read_bytes(scratch as u64, &mut saved);
        let mut contents = vec![0u8; rt_len as usize];
        if !r300_read_gpu_bytes(s, guest_addr, &mut contents) {
            contents.fill(0);
        }
        vram.write_bytes(scratch as u64, &contents);
        gpu_vram_dirty(s, scratch as uint64_t, rt_len);
        pkt.rt_gpu_addr = scratch;
        Some(GartColourBuffer { guest_addr, scratch, len: rt_len, first_row, end_row, saved })
    }

    /// Copies the rows the draw touched from the scratch area back to the guest's GART pages.
    pub fn write_back(
        &self,
        s: &mut PPCMacGPUFields,
        pitch_bytes: uint32_t,
        vram: crate::src::qemu::Vram,
    ) {
        let mut offset = self.first_row as uint64_t * pitch_bytes as uint64_t;
        let end = self.end_row as uint64_t * pitch_bytes as uint64_t;
        while offset < end {
            let guest = (self.guest_addr as uint64_t).wrapping_add(offset) as uint32_t;
            let page = gpu_agp_page(s, guest);
            if page.is_null() {
                r300_warn_once("GART colour buffer page not mapped", None);
                break;
            }
            let in_page = 0x1000 - (guest & 0xfff) as uint64_t;
            let n = (end - offset).min(in_page) as usize;
            let mut chunk = vec![0u8; n];
            vram.read_bytes(self.scratch as u64 + offset, &mut chunk);
            if !crate::src::qemu::agp_page_write(s, page.wrapping_add((guest & 0xfff) as usize), &chunk) {
                r300_warn_once("GART colour buffer page not writable", None);
                break;
            }
            offset += n as uint64_t;
        }
    }

    /// Puts the scratch area's previous contents back.
    pub fn restore(self, s: &mut PPCMacGPUFields, vram: crate::src::qemu::Vram) {
        vram.write_bytes(self.scratch as u64, &self.saved[..self.len as usize]);
        gpu_vram_dirty(s, self.scratch as uint64_t, self.len);
    }
}

/// Loads the vertex array pointers (AOS) the draw will fetch from.
pub fn r300_load_vertex_arrays(s: &mut PPCMacGPUFields) {
    const AOS_WORDS: usize = 1 + 3 * 8;
    let mut aos = [0 as uint32_t; AOS_WORDS];
    for (k, word) in aos.iter_mut().enumerate() {
        *word = r300_reg(s.r3_ref(), 0x20c0 + 4 * k as uint32_t);
    }
    crate::src::qemu::r300_load_arrays(&mut s.r3_arrays, &aos);
}

/// Builds the draw packet from the CP state (indexed if `idx` is non-null). Returns whether
/// it worked and the builder's error message (a static C string) when it did not.
pub fn r300_build_draw_packet(
    s: &mut PPCMacGPUFields,
    pkt: &mut R300DrawPacket,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
    idx: Option<&[uint32_t]>,
) -> (bool, *const ::core::ffi::c_char) {
    crate::src::qemu::r300_build_packet(s, pkt, opcode, d, body_dw, idx)
}

/// Marks the draw's colour buffers dirty for the display and fixes the present pitch when
/// the draw targets the scanout surface with a different stride.
pub fn r300_mark_draw_dirty(s: &mut PPCMacGPUFields, pkt: &R300DrawPacket) {
            let len: uint64_t = (pkt.rt_pitch as uint64_t)
                .wrapping_mul(pkt.rt_bpp as uint64_t)
                .wrapping_mul(pkt.rt_height as uint64_t);
            if s.renderer_ref().set_dirty_source.is_none() {
                gpu_vram_dirty(s, pkt.rt_gpu_addr as uint64_t, len);
                let mut k_1: uint32_t = 1 as uint32_t;
                while k_1 < pkt.num_cb {
                    gpu_vram_dirty(
                        s,
                        pkt.cb[k_1 as usize].gpu_addr as uint64_t,
                        (pkt.cb[k_1 as usize].pitch as uint64_t)
                            .wrapping_mul(pkt.cb[k_1 as usize].bpp as uint64_t)
                            .wrapping_mul(pkt.rt_height as uint64_t),
                    );
                    k_1 = k_1.wrapping_add(1);
                }
            }
            if pkt.rt_gpu_addr == s.disp.offset
                && pkt.rt_pitch.wrapping_mul(pkt.rt_bpp) != s.disp.stride
            {
                gpu_set_present_pitch(s, pkt.rt_pitch.wrapping_mul(pkt.rt_bpp));
            }
}

/// Emulates the colour-buffer/Z-buffer fast clear (CBZB): fills the depth buffer rows the
/// draw covered with the cleared value, after the renderer drew the colour part.
pub fn r300_emulate_cbzb_clear(
    s: &mut PPCMacGPUFields,
    pkt: &R300DrawPacket,
    vram: crate::src::qemu::Vram,
) {

            let zpitch_reg: uint32_t = r300_reg(s.r3_ref(), 0x4f24 as uint32_t);
            let mut zo_0: uint32_t = r300_reg(s.r3_ref(), 0x4f20 as uint32_t) & !(0xf as uint32_t);
            let pitch_b_0: uint32_t =
                (zpitch_reg & 0x3ffc as uint32_t).wrapping_mul(4 as uint32_t);
            let x0: uint32_t = pkt.scissor[0 as ::core::ffi::c_int as usize];
            let y0: uint32_t = pkt.scissor[1 as ::core::ffi::c_int as usize];
            let mut x1: uint32_t = pkt.scissor[2 as ::core::ffi::c_int as usize];
            let mut y1: uint32_t = pkt.scissor[3 as ::core::ffi::c_int as usize];
            let v_2: uint32_t = r300_swap_mode(
                r300_reg(s.r3_ref(), 0x4f28 as uint32_t),
                zpitch_reg as ::core::ffi::c_uint >> 19 as ::core::ffi::c_int
                    & 3 as ::core::ffi::c_uint,
            );
            x1 = {
                let mut _a79: uint32_t = x1 as uint32_t;
                let mut _b80: uint32_t = (pitch_b_0 as uint32_t).wrapping_div(4 as uint32_t);
                if _a79 < _b80 {
                    _a79 as uint32_t
                } else {
                    _b80 as uint32_t
                }
            };
            if pkt.rt_height != 0 {
                y1 = {
                    let mut _a81: uint32_t = y1 as uint32_t;
                    let mut _b82: uint32_t = pkt.rt_height as uint32_t;
                    if _a81 < _b82 {
                        _a81 as uint32_t
                    } else {
                        _b82 as uint32_t
                    }
                };
            }
            if !crate::src::qemu::getenv(c"PPCGPU_CBZB_LOG").is_null() {
                crate::src::logs::err(format_args!("CBZB fill zo {:08x} pitch {} rows {}..{} cols {}..{} value {:08x} rt {:08x} rt_pitch {} rt_h {}\n", ((zo_0) as u32), ((pitch_b_0) as u32), ((y0) as u32), ((y1) as u32), ((x0) as u32), ((x1) as u32), ((v_2) as u32), ((pkt.rt_gpu_addr) as u32), ((pkt.rt_pitch) as u32), ((pkt.rt_height) as u32)));
            }
            if pitch_b_0 != 0
                && x1 > x0
                && y1 > y0
                && r300_to_vram(
                    s,
                    &mut zo_0,
                    (pitch_b_0 as uint64_t).wrapping_mul(y1 as uint64_t),
                ) as ::core::ffi::c_int
                    != 0
            {
                crate::src::qemu::renderer_flush_sync(s);
                let row_bytes: Vec<u8> = (x0..x1).flat_map(|_| v_2.to_le_bytes()).collect();
                for y_0 in y0..y1 {
                    let off = (zo_0 as uint64_t)
                        .wrapping_add((y_0 as uint64_t).wrapping_mul(pitch_b_0 as uint64_t))
                        .wrapping_add(x0 as uint64_t * 4);
                    vram.write_bytes(off, &row_bytes);
                }
                gpu_vram_dirty(
                    s,
                    (zo_0 as uint64_t)
                        .wrapping_add((y0 as uint64_t).wrapping_mul(pitch_b_0 as uint64_t)),
                    (y1.wrapping_sub(y0) as uint64_t).wrapping_mul(pitch_b_0 as uint64_t),
                );
            }
        }

/// Sets the state the draw builder reads about the renderer and the screen.
pub fn r300_prepare_draw_state(s: &mut PPCMacGPUFields) {
    s.r3_mut().glsl_flags = if s.renderer.is_null() { 0 } else { s.renderer_ref().r300_glsl_flags };
    if s.disp.height == 0 {
        ppc_mac_gpu_update_display_mode(s);
    }
    s.r3_mut().le_screen_h = s.disp.height;
}

/// Maps the draw's colour buffers into VRAM. The main one may live in GART memory, in which
/// case it is redirected to scratch VRAM and returned as a `GartColourBuffer`. Err names the
/// buffer that could not be mapped.
pub fn r300_map_colour_buffers(
    s: &mut PPCMacGPUFields,
    pkt: &mut R300DrawPacket,
    vram: crate::src::qemu::Vram,
) -> Result<Option<GartColourBuffer>, &'static str> {
    let ns = pkt.aa_samples;
    let guest_addr = pkt.rt_gpu_addr;
    let rt_len = (pkt.rt_pitch as uint64_t)
        .wrapping_mul(pkt.rt_bpp as uint64_t)
        .wrapping_mul(pkt.rt_height as uint64_t)
        .wrapping_mul(ns as uint64_t);
    let mut gart = None;
    if !r300_to_vram(s, &mut pkt.rt_gpu_addr, rt_len) {
        gart = Some(
            GartColourBuffer::begin(s, pkt, guest_addr, rt_len, vram)
                .ok_or("colour buffer outside VRAM")?,
        );
    }
    let mut k_0: uint32_t = 1 as uint32_t;
    while k_0 < pkt.num_cb {
        let c: &mut R300ColorDesc = &mut pkt.cb[(k_0) as usize];
        if !r300_to_vram(
            s,
            &mut c.gpu_addr,
            (c.pitch as uint64_t)
                .wrapping_mul(c.bpp as uint64_t)
                .wrapping_mul(pkt.rt_height as uint64_t)
                .wrapping_mul(ns as uint64_t),
        ) {
            r300_warn_once("render target B-D outside VRAM", None);
            pkt.num_cb = k_0;
            break;
        } else {
            k_0 = k_0.wrapping_add(1);
        }
    }
    if !crate::src::qemu::getenv(c"PPCGPU_TEX_LOG").is_null() {
        crate::src::logs::err(format_args!("DRAWRT vram+{:x} pitch {} h {} fmt {} cbs {}\n", ((pkt.rt_gpu_addr) as u32), ((pkt.rt_pitch) as u32), ((pkt.rt_height) as u32), ((pkt.rt_format) as u32), ((pkt.num_cb) as u32)));
    }

    Ok(gart)
}

/// Decides the CBZB fast-clear case, converts Z-buffer contents when needed, and maps the
/// depth buffer into VRAM (dropping the attachment if it cannot be). Returns whether this
/// draw is a CBZB clear.
pub fn r300_map_depth_buffer(s: &mut PPCMacGPUFields, pkt: &mut R300DrawPacket) -> bool {
    let ns = pkt.aa_samples;
    let cbzb_clear: bool = r300_reg(s.r3_ref(), 0x4f1c as uint32_t) & 0x20 as uint32_t
        != 0 as uint32_t
        && crate::src::qemu::getenv(c"PPCGPU_NO_CBZB").is_null();
    if cbzb_clear {
        pkt.depth.attach = 0 as ::core::ffi::c_int != 0;
    }
    if s.r3_zconv as ::core::ffi::c_int != 0
        && (pkt.depth.attach as ::core::ffi::c_int != 0
            || r300_reg(s.r3_ref(), 0x4f00 as uint32_t) & 7 as uint32_t != 0)
    {
        r300_zconv(s, 0 as ::core::ffi::c_int != 0);
        s.r3_zconv = 0 as ::core::ffi::c_int != 0;
    }
    if pkt.depth.attach {
        let zo: uint32_t = pkt.depth.gpu_addr;
        if !r300_to_vram(
            s,
            &mut pkt.depth.gpu_addr,
            (pkt.depth.pitch as uint64_t)
                .wrapping_mul(pkt.depth.bpp as uint64_t)
                .wrapping_mul(pkt.rt_height as uint64_t)
                .wrapping_mul(ns as uint64_t),
        ) {
            r300_warn_once("depth buffer outside VRAM", None);
            pkt.depth.attach = 0 as ::core::ffi::c_int != 0;
        } else {
            if s.r3_zb_offset != zo {
                s.r3_zb_offset = zo;
                s.r3_zb_height = 0 as uint32_t;
            }
            s.r3_zb_height = {
                let mut _a77: uint32_t = s.r3_zb_height as uint32_t;
                let mut _b78: uint32_t = pkt.rt_height as uint32_t;
                if _a77 > _b78 {
                    _a77 as uint32_t
                } else {
                    _b78 as uint32_t
                }
            };
            s.r3_zb_ns = ns;
        }
    }

    cbzb_clear
}

/// Remembers which surface is multisampled so a later resolve knows its sample count.
pub fn r300_note_multisampling(s: &mut PPCMacGPUFields, pkt: &R300DrawPacket) {
    let ns = pkt.aa_samples;
    if ns > 1 as uint32_t {
        static said_0: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
        if !said_0.get() {
            said_0.set(1 as ::core::ffi::c_int != 0);
            crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: first multisampled draw: {} samples (GB_MSPOS {:08x} {:08x}) into {:06x}\n", ((ns) as u32), ((r300_reg(s.r3_ref(), 0x4010 as uint32_t)) as u32), ((r300_reg(s.r3_ref(), 0x4014 as uint32_t)) as u32), ((pkt.rt_gpu_addr) as u32)));
        }
        s.r3_cb_aa_off = pkt.rt_gpu_addr;
        s.r3_cb_aa_ns = ns;
    } else if s.r3_cb_aa_off == pkt.rt_gpu_addr {
        s.r3_cb_aa_ns = 1 as uint32_t;
    }
}

/// Hands the built packet to the renderer under the big lock, then does the follow-up the
/// draw needs: CBZB clear, GART write-back and scratch restore, dirty marking and tracing.
pub fn r300_submit_draw(
    s: &mut PPCMacGPUFields,
    pkt: &mut R300DrawPacket,
    gart_rt: &mut Option<GartColourBuffer>,
    cbzb_clear: bool,
    vram: crate::src::qemu::Vram,
) {
    if !s.renderer.is_null() && s.renderer_ref().draw_r300.is_some() {
        let _bql = crate::src::qemu::BqlGuard::acquire();
        static sync_each: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
        if sync_each.get() < 0 as ::core::ffi::c_int {
            sync_each.set(crate::src::qemu::env_flag(c"R300_SYNC") as ::core::ffi::c_int);
        }
        pkt.harvest_hint = s.vram_dev_dirty as ::core::ffi::c_int != 0
            || s.kick_epoch != s.last_draw_epoch;
        s.vram_dev_dirty = 0 as ::core::ffi::c_int != 0;
        s.last_draw_epoch = s.kick_epoch;
        let rr: ::core::ffi::c_int = crate::src::qemu::renderer_draw_r300(s, &vram, pkt);
        if !s.r3_dump.is_null() && g_r300_arm_rt.get() != 0 && pkt.rt_gpu_addr == g_r300_arm_rt.get() {
            crate::src::logs::err(format_args!("   renderer -> {}\n", ((rr) as i32)));
        }
        if cbzb_clear && rr >= 0 && s.renderer_ref().flush_sync.is_some() {
            r300_emulate_cbzb_clear(s, pkt, vram);
        }
        if let Some(gart) = gart_rt.take() {
            if rr >= 0 as ::core::ffi::c_int && s.renderer_ref().flush_sync.is_some() {
                crate::src::qemu::renderer_flush_sync(s);
                gart.write_back(s, pkt.rt_pitch.wrapping_mul(pkt.rt_bpp), vram);
            }
            gart.restore(s, vram);
        }
        if rr < 0 as ::core::ffi::c_int {
            r300_warn_once("renderer rejected a draw", None);
        } else {
            if sync_each.get() != 0 && s.renderer_ref().flush_sync.is_some() {
                crate::src::qemu::renderer_flush_sync(s);
            }
            r300_trace_armed_steps(s, pkt, vram);
            r300_snapshot_textures(s, pkt, vram);
            r300_mark_draw_dirty(s, pkt);
        }
    }
}
