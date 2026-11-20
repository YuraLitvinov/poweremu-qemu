use super::*;
/// Renders one R300 draw: builds a draw packet from the CP state, maps its buffers into
/// VRAM, hands it to the renderer and does the bookkeeping the guest relies on afterwards.
/// `idx` is non-null for an indexed draw. The stages live in `fn_r300_render.rs`.
pub fn r300_render(
    s: &mut PPCMacGPUState,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
    idx: Option<&[uint32_t]>,
) {
    let mut pkt = R300DrawPacket::empty();
    let vram = crate::src::qemu::Vram::of(s);

    r300_drawlog_draw(s, opcode, d, body_dw);
    r300_trace_draw_inputs(s, opcode, d, body_dw);
    if opcode != 0x35 {
        r300_load_vertex_arrays(s);
    }
    r300_trace_render_target(s);
    r300_prepare_draw_state(s);

    let (build_ok, err) = r300_build_draw_packet(s, &mut pkt, opcode, d, body_dw, idx);
    r300_trace_built_packet(s, &pkt, opcode, d, body_dw, build_ok, err);
    if !build_ok {
        r300_report_build_failure(s, opcode, d, body_dw, err);
        return;
    }
    r300_report_draw_warnings(&pkt);

    let mut gart_rt = match r300_map_colour_buffers(s, &mut pkt, vram) {
        Ok(gart) => gart,
        Err(what) => {
            r300_warn_once(what, None);
            r300_draw_release(&mut pkt);
            return;
        }
    };
    r300_prepare_textures(s, &mut pkt, vram);
    r300_note_formats(s, &pkt);
    r300_note_features(s, &pkt);
    r300_dump_draw_assets(s, &pkt, vram);
    let cbzb_clear = r300_map_depth_buffer(s, &mut pkt);
    pkt.uniforms.zpass_count = s.r3_zpass_active as uint32_t;
    r300_trace_zpass(s, &pkt);

    if r300_reg(s.r3_ref(), 0x4e88) & 1 != 0 {
        // The draw only resolves a multisampled buffer: no renderer submission.
        r300_aa_resolve(s, &pkt);
    } else {
        r300_note_multisampling(s, &pkt);
        if !s.renderer.is_null() && s.renderer_ref().draw_r300.is_some() {
            r300_submit_draw(s, &mut pkt, &mut gart_rt, cbzb_clear, vram);
        }
    }
    if let Some(gart) = gart_rt.take() {
        gart.restore(s, vram);
    }
    r300_draw_release(&mut pkt);
    s.regs.stall_draws = s.regs.stall_draws.wrapping_add(1);
}
/// True while the R300 ring dump is active (see `qemu::ringdump_active`).
pub fn r300_ringdump() -> bool {
    crate::src::qemu::ringdump_active()
}
/// Read callback of the surface watch region. `opaque` is the device state (see
/// `state_of_opaque`); bytes are read big-endian from VRAM at the watch base plus `addr`.
/// Bytes past the end of VRAM read as zero.
pub unsafe extern "C" fn r300_watch_read(
    opaque: *mut ::core::ffi::c_void,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return 0;
    };
    let vram = crate::src::qemu::Vram::of(s);
    let base = (s.r300_watch_base as uint64_t).wrapping_add(addr);
    gpu_sync_at(
        s,
        (0x4000 as uint32_t).wrapping_add(GPU_WHY_DISPLAY as ::core::ffi::c_int as uint32_t),
    );
    let mut v: uint64_t = 0;
    for i in 0..size {
        let byte = vram.get8(base.wrapping_add(i as uint64_t)).unwrap_or(0);
        v = (v << 8) | byte as uint64_t;
    }
    let fresh11 = s.r300_watch_logged;
    s.r300_watch_logged = s.r300_watch_logged.wrapping_add(1);
    if fresh11 < 64 as uint32_t {
        crate::src::logs::qemu(format_args!("r300 surfwatch: rd{} 0x{:x} = 0x{:x}\n", ((size) as u32), (((s.r300_watch_base as hwaddr).wrapping_add(addr) as ::core::ffi::c_uint) as u32), ((v) as u64)));
    }
    return v;
}
/// Write callback of the surface watch region: stores the `size` low bytes of `val` big-endian
/// at the watch base plus `addr`. Bytes past the end of VRAM are dropped.
pub unsafe extern "C" fn r300_watch_write(
    opaque: *mut ::core::ffi::c_void,
    addr: hwaddr,
    val: uint64_t,
    size: ::core::ffi::c_uint,
) {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return;
    };
    let vram = crate::src::qemu::Vram::of(s);
    let base = (s.r300_watch_base as uint64_t).wrapping_add(addr);
    for i in 0..size {
        // Shifts of 64 or more (access sizes past 8 bytes) yield zero instead of panicking.
        let sh = 8u32.wrapping_mul(size.wrapping_sub(1).wrapping_sub(i));
        let byte = val.checked_shr(sh).unwrap_or(0) as uint8_t;
        vram.put8(base.wrapping_add(i as uint64_t), byte);
    }
    let fresh10 = s.r300_watch_logged;
    s.r300_watch_logged = s.r300_watch_logged.wrapping_add(1);
    if fresh10 < 64 as uint32_t {
        crate::src::logs::qemu(format_args!("r300 surfwatch: wr{} 0x{:x} = 0x{:x}\n", ((size) as u32), (((s.r300_watch_base as hwaddr).wrapping_add(addr) as ::core::ffi::c_uint) as u32), ((val) as u64)));
    }
}
/// Byte offset of one sample's pixel in an MSAA-tiled surface. Same arithmetic as the
/// driver's r300_msaa_offset; `!0` for an unsupported sample count or sample index.
fn zconv_msaa_offset(x: u32, y: u32, ns: u32, pitch_px: u32, bpp: u32, sample: u32) -> u32 {
    if (ns != 2 && ns != 4) || sample >= ns {
        return !0;
    }
    let sh: u32 = if bpp == 2 { 1 } else { 2 };
    let pitch = pitch_px.wrapping_mul(ns);
    let tile_row = (pitch / ns / 4)
        .wrapping_mul(ns)
        .wrapping_mul(sh)
        .wrapping_mul(2)
        .wrapping_mul(y >> 3);
    let tile_col = (((x >> 2) << 1) | ((y >> 2) & 1)).wrapping_mul(ns).wrapping_mul(sh);
    let base = 32u32.wrapping_mul(tile_row.wrapping_add(tile_col));
    let mut idx = if ns == 2 {
        (((y >> 1) & 1) << 4) | (((x >> 1) & 1) << 3) | (sample << 2)
    } else {
        ((((x >> 2) & 1) ^ ((y >> 1) & 1)) << 5) | (((x >> 1) & 1) << 4) | (sample << 2)
    };
    idx |= ((y & 1) << 1) | (x & 1);
    base.wrapping_add(idx << sh)
}

/// Copies one 4-byte pixel from `src` at byte `s_at` to `dst` at byte `d_at`. Offsets that
/// fall outside either buffer are skipped rather than panicking.
fn zconv_copy_pixel(src: &[u8], s_at: u64, dst: &mut [u8], d_at: u64) {
    let span = |at: u64| usize::try_from(at).ok().and_then(|a| a.checked_add(4).map(|e| a..e));
    if let (Some(from), Some(to)) = (span(s_at).and_then(|r| src.get(r)), span(d_at)) {
        if let Some(to) = dst.get_mut(to) {
            to.copy_from_slice(from);
        }
    }
}

pub fn r300_zconv(s: &mut PPCMacGPUState, to_card: bool) {
    let vram = crate::src::qemu::Vram::of(s);
    let off: uint32_t = s.r3_zconv_off;
    let pitch: uint32_t = s.r3_zconv_pitch;
    let rows: uint32_t = s.r3_zconv_rows;
    let ns: uint32_t = s.r3_zconv_ns;
    let lin: uint64_t = (pitch as uint64_t)
        .wrapping_mul(4 as uint64_t)
        .wrapping_mul(rows as uint64_t);
    let card: uint64_t = lin.wrapping_mul(ns as uint64_t);
    if rows == 0 || (off as uint64_t).wrapping_add(card) > s.vram_size {
        return;
    }
    gpu_sync_at(
        s,
        (0x4000 as uint32_t).wrapping_add(GPU_WHY_DISPLAY as ::core::ffi::c_int as uint32_t),
    );
    // `vr` is the card's current bytes; `tmp` is the converted result, written back whole.
    let (Some(mut vr), Some(mut tmp)) = (
        crate::src::qemu::try_alloc_bytes(card as usize),
        crate::src::qemu::try_alloc_bytes(card as usize),
    ) else {
        return;
    };
    if !vram.read_bytes(off as uint64_t, &mut vr) {
        return;
    }
    if s.r3_zconv_rows_ns {
        for y in 0..rows {
            for k in 0..ns {
                let row_base = (y as uint64_t)
                    .wrapping_mul(ns as uint64_t)
                    .wrapping_add(k as uint64_t)
                    .wrapping_mul(pitch as uint64_t)
                    .wrapping_mul(4 as uint64_t);
                for x in 0..pitch {
                    let row = row_base.wrapping_add((x as uint64_t).wrapping_mul(4));
                    let sample = zconv_msaa_offset(x, y, ns, pitch, 4, k) as uint64_t;
                    // Direction is picked once per pixel: gather rows into the tiled layout
                    // for to_card, scatter the tiled layout back into rows otherwise.
                    if to_card {
                        zconv_copy_pixel(&vr, row, &mut tmp, sample);
                    } else {
                        zconv_copy_pixel(&vr, sample, &mut tmp, row);
                    }
                }
            }
        }
        vram.write_bytes(off as uint64_t, &tmp);
    } else if to_card {
        for y in 0..rows {
            for x in 0..pitch {
                let linear = (y as uint64_t)
                    .wrapping_mul(pitch as uint64_t)
                    .wrapping_add(x as uint64_t)
                    .wrapping_mul(4 as uint64_t);
                for k in 0..ns {
                    let sample = zconv_msaa_offset(x, y, ns, pitch, 4, k) as uint64_t;
                    zconv_copy_pixel(&vr, linear, &mut tmp, sample);
                }
            }
        }
        vram.write_bytes(off as uint64_t, &tmp);
    } else {
        // Scatter back: the linear layout fills the first `lin` bytes; the rest stays zero.
        for y in 0..rows {
            for x in 0..pitch {
                let linear = (y as uint64_t)
                    .wrapping_mul(pitch as uint64_t)
                    .wrapping_add(x as uint64_t)
                    .wrapping_mul(4 as uint64_t);
                let sample = zconv_msaa_offset(x, y, ns, pitch, 4, 0) as uint64_t;
                zconv_copy_pixel(&vr, sample, &mut tmp, linear);
            }
        }
        vram.write_bytes(off as uint64_t, &tmp);
    }
    gpu_vram_dirty(s, off as uint64_t, card);
    static logged: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
    let fresh12 = logged.get();
    logged.set(logged.get() + 1);
    if fresh12 < 6 as ::core::ffi::c_int {
        crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: depth at 0x{:x} ({}x{}, {} samples) {} the card's layout for a CPU readback\n", ((s.r3_zconv_off) as u32), ((pitch) as u32), ((rows) as u32), ((ns) as u32), crate::src::qemu::cstr(if to_card as ::core::ffi::c_int != 0 {
                c"to".as_ptr()
            } else {
                c"back from".as_ptr()
            })));
    }
}
pub fn r300_surface_changed(s: &mut PPCMacGPUState) {
    let lo: uint32_t = s.r300_surf[1 as ::core::ffi::c_int as usize];
    let hi: uint32_t = s.r300_surf[2 as ::core::ffi::c_int as usize];
    let pitch: uint32_t = r300_reg(s.r3_ref(), 0x4f24 as uint32_t) & 0x3ffc as uint32_t;
    let zo: uint32_t = r300_reg(s.r3_ref(), 0x4f20 as uint32_t) & !(0x1f as uint32_t);
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    if s.r3_zconv {
        if lo == s.r3_zconv_off {
            return;
        }
        r300_zconv(s, 0 as ::core::ffi::c_int != 0);
        s.r3_zconv = 0 as ::core::ffi::c_int != 0;
    }
    if s.r3.is_null()
        || hi <= lo
        || zo < fb_base
        || lo != zo.wrapping_sub(fb_base)
        || s.r3_zb_offset != zo
        || r300_reg(s.r3_ref(), 0x4f10 as uint32_t) & 0xf as uint32_t != 2 as uint32_t
        || pitch == 0
        || s.r3_zb_height == 0
    {
        return;
    }
    let mut ns: uint32_t = s.r3_zb_ns;
    s.r3_zconv_rows_ns = ns >= 2 as uint32_t;
    if ns < 2 as uint32_t {
        ns = hi
            .wrapping_sub(lo)
            .wrapping_add(1 as uint32_t)
            .wrapping_add(
                pitch
                    .wrapping_mul(2 as uint32_t)
                    .wrapping_mul(s.r3_zb_height),
            )
            .wrapping_div(
                pitch
                    .wrapping_mul(4 as uint32_t)
                    .wrapping_mul(s.r3_zb_height),
            );
    }
    if ns != 2 as uint32_t && ns != 4 as uint32_t {
        return;
    }
    s.r3_zconv_off = lo;
    s.r3_zconv_pitch = pitch;
    s.r3_zconv_ns = ns;
    s.r3_zconv_rows = {
        let mut _a133: uint32_t = s.r3_zb_height as uint32_t;
        let mut _b134: uint32_t = (hi as uint32_t)
            .wrapping_sub(lo as uint32_t)
            .wrapping_add(1 as uint32_t)
            .wrapping_div(
                (pitch as uint32_t)
                    .wrapping_mul(4 as uint32_t)
                    .wrapping_mul(ns as uint32_t),
            );
        if _a133 < _b134 {
            _a133 as uint32_t
        } else {
            _b134 as uint32_t
        }
    };
    r300_zconv(s, 1 as ::core::ffi::c_int != 0);
    s.r3_zconv = 1 as ::core::ffi::c_int != 0;
}
pub fn r300_surfwatch_update(s: &mut PPCMacGPUState) {
    static enabled: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    let lo: uint32_t = s.r300_surf[1 as ::core::ffi::c_int as usize];
    let hi: uint32_t = s.r300_surf[2 as ::core::ffi::c_int as usize];
    if enabled.get() < 0 as ::core::ffi::c_int {
        enabled.set(crate::src::qemu::env_flag(c"R300_SURFWATCH") as ::core::ffi::c_int);
    }
    if enabled.get() == 0 {
        return;
    }
    if s.r300_watch_on {
        crate::src::qemu::mr_del_subregion(&mut s.vram_bar, &mut s.r300_watch);
        crate::src::qemu::mr_unparent(&mut s.r300_watch);
        s.r300_watch_on = false;
    }
    if hi > lo && (hi as uint64_t) < s.vram_size {
        crate::src::qemu::r300_watch_region_init(
            s,
            &r300_watch_ops,
            hi.wrapping_sub(lo).wrapping_add(1 as uint32_t) as uint64_t,
        );
        crate::src::qemu::mr_add_subregion_overlap(
            &mut s.vram_bar,
            lo as hwaddr,
            &mut s.r300_watch,
            10 as ::core::ffi::c_int,
        );
        s.r300_watch_base = lo;
        s.r300_watch_on = 1 as ::core::ffi::c_int != 0;
        crate::src::logs::qemu(format_args!("r300 surfwatch: surface0 0x{:x}-0x{:x} info {:08x}\n", ((lo) as u32), ((hi) as u32), ((s.r300_surf[3 as ::core::ffi::c_int as usize]) as u32)));
    }
}
