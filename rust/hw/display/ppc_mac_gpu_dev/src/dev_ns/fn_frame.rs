use super::*;
pub fn frame_tracker_init(ft: &mut FrameTracker) {
    *ft = crate::src::qemu::zeroed_frame_tracker();
    ft.dump_limit = 5 as uint32_t;
}
pub fn frame_tracker_new_frame(ft: &mut FrameTracker) {
    ft.frame_id = ft.frame_id.wrapping_add(1);
    ft.pass_id = 0 as uint32_t;
    ft.event_count = 0 as uint32_t;
    ft.has_compositor_rt = 0 as ::core::ffi::c_int != 0;
    ft.has_window_rt = 0 as ::core::ffi::c_int != 0;
    ft.has_present = 0 as ::core::ffi::c_int != 0;
    ft.has_fallback_2d = 0 as ::core::ffi::c_int != 0;
    ft.has_host_upload = 0 as ::core::ffi::c_int != 0;
    ft.has_blit_upload = 0 as ::core::ffi::c_int != 0;
    ft.has_drag_copy = 0 as ::core::ffi::c_int != 0;
    ft.drag_observed = 0 as ::core::ffi::c_int != 0;
    ft.has_mixed_surface_owner = 0 as ::core::ffi::c_int != 0;
    ft.surface_state_count = 0 as uint32_t;
    ft.dumped = 0 as ::core::ffi::c_int != 0;
}
pub fn frame_tracker_request_dump(
    ft: &mut FrameTracker,
    frames: uint32_t,
) {
    if frames > ft.force_dump_remaining {
        ft.force_dump_remaining = frames;
    }
}
pub fn frame_tracker_note_drag(
    ft: &mut FrameTracker,
    dump_frames: uint32_t,
) {
    ft.drag_observed = 1 as ::core::ffi::c_int != 0;
    frame_tracker_request_dump(ft, dump_frames);
}
pub fn frame_tracker_surface_state(
    ft: &mut FrameTracker,
    offset: uint32_t,
    pitch: uint32_t,
    sc: SurfaceClass,
) -> Option<&mut SurfaceFrameState> {
    let used = ft.surface_state_count as usize;
    if let Some(i) = ft.surface_states[..used]
        .iter()
        .position(|st| st.valid && st.offset == offset)
    {
        let st = &mut ft.surface_states[i];
        if pitch != 0 {
            st.pitch = pitch;
        }
        st.surface_class = sc;
        return Some(st);
    }
    if used >= ft.surface_states.len() {
        return None;
    }
    ft.surface_state_count = ft.surface_state_count.wrapping_add(1);
    let st = &mut ft.surface_states[used];
    *st = SurfaceFrameState {
        valid: true,
        offset,
        pitch,
        surface_class: sc,
        owner_mask: 0,
        qe_presents: 0,
        cpu_presents: 0,
        drag_copies: 0,
        host_uploads: 0,
        blit_uploads: 0,
        renders: 0,
    };
    Some(st)
}
pub fn frame_tracker_surface_has_owner(
    ft: &FrameTracker,
    offset: uint32_t,
    owner_mask: uint32_t,
) -> bool {
    ft.surface_states[..ft.surface_state_count as usize]
        .iter()
        .any(|st| st.valid && st.offset == offset && st.owner_mask & owner_mask != 0)
}
pub fn frame_tracker_mark_surface_owner(
    ft: &mut FrameTracker,
    offset: uint32_t,
    pitch: uint32_t,
    sc: SurfaceClass,
    type_0: PassEventType,
    dst_offset: uint32_t,
    from_srt: bool,
) {
    let Some(st) = frame_tracker_surface_state(ft, offset, pitch, sc) else {
        return;
    };
    match type_0 as ::core::ffi::c_uint {
        4 => {
            if from_srt {
                st.owner_mask |= SURF_OWNER_QE_PRESENT as ::core::ffi::c_int as uint32_t;
                st.qe_presents = st.qe_presents.wrapping_add(1);
            } else if dst_offset == 0 as uint32_t {
                st.owner_mask |= SURF_OWNER_CPU_PRESENT as ::core::ffi::c_int as uint32_t;
                st.cpu_presents = st.cpu_presents.wrapping_add(1);
            }
        }
        5 => {
            if dst_offset == 0 as uint32_t {
                st.owner_mask |= SURF_OWNER_CPU_PRESENT as ::core::ffi::c_int as uint32_t;
                st.cpu_presents = st.cpu_presents.wrapping_add(1);
            }
        }
        8 => {
            st.owner_mask |= SURF_OWNER_DRAG_COPY as ::core::ffi::c_int as uint32_t;
            st.drag_copies = st.drag_copies.wrapping_add(1);
        }
        6 => {
            st.owner_mask |= SURF_OWNER_HOST_UPLOAD as ::core::ffi::c_int as uint32_t;
            st.host_uploads = st.host_uploads.wrapping_add(1);
        }
        7 => {
            st.owner_mask |= SURF_OWNER_BLIT_UPLOAD as ::core::ffi::c_int as uint32_t;
            st.blit_uploads = st.blit_uploads.wrapping_add(1);
        }
        0 | 2 | 3 => {
            st.owner_mask |= SURF_OWNER_RENDER as ::core::ffi::c_int as uint32_t;
            st.renders = st.renders.wrapping_add(1);
        }
        _ => {}
    }
    if st.owner_mask & SURF_OWNER_QE_PRESENT as ::core::ffi::c_int as uint32_t != 0
        && (st.owner_mask & SURF_OWNER_CPU_PRESENT as ::core::ffi::c_int as uint32_t != 0
            || st.owner_mask & SURF_OWNER_DRAG_COPY as ::core::ffi::c_int as uint32_t != 0)
    {
        ft.has_mixed_surface_owner = true;
    }
}
pub fn frame_tracker_owner_string(mask: uint32_t) -> String {
    let entries: [(uint32_t, &str); 6] = [
        (SURF_OWNER_QE_PRESENT as uint32_t, "qe_present"),
        (SURF_OWNER_CPU_PRESENT as uint32_t, "cpu_present"),
        (SURF_OWNER_DRAG_COPY as uint32_t, "drag_copy"),
        (SURF_OWNER_HOST_UPLOAD as uint32_t, "host_upload"),
        (SURF_OWNER_BLIT_UPLOAD as uint32_t, "blit_upload"),
        (SURF_OWNER_RENDER as uint32_t, "render"),
    ];
    let names: Vec<&str> = entries
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, name)| *name)
        .collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join("|")
    }
}
pub fn frame_tracker_record_detail(
    ft: &mut FrameTracker,
    type_0: PassEventType,
    surface_offset: uint32_t,
    pitch: uint32_t,
    src_offset: uint32_t,
    dst_offset: uint32_t,
    w: uint16_t,
    h: uint16_t,
    src_x: uint16_t,
    src_y: uint16_t,
    dst_x: uint16_t,
    dst_y: uint16_t,
    move_x: int16_t,
    move_y: int16_t,
    from_srt: bool,
) {
    if ft.event_count >= 512 as uint32_t {
        return;
    }
    let sc: SurfaceClass = classify_surface(surface_offset, pitch, dst_offset);
    let fresh1 = ft.event_count;
    ft.event_count = ft.event_count.wrapping_add(1);
    let e = &mut ft.events[fresh1 as usize];
    e.type_0 = type_0;
    e.pass_id = ft.pass_id;
    e.surface_offset = surface_offset;
    e.pitch = pitch;
    e.src_offset = src_offset;
    e.dst_offset = dst_offset;
    e.width = w;
    e.height = h;
    e.src_x = src_x;
    e.src_y = src_y;
    e.dst_x = dst_x;
    e.dst_y = dst_y;
    e.move_x = move_x;
    e.move_y = move_y;
    e.surface_class = sc;
    e.from_srt = from_srt;
    match type_0 as ::core::ffi::c_uint {
        0 | 2 => {
            if sc as ::core::ffi::c_uint
                == SURF_COMPOSITOR_RT as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                ft.has_compositor_rt = 1 as ::core::ffi::c_int != 0;
            }
            if sc as ::core::ffi::c_uint
                == SURF_WINDOW_RT as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                ft.has_window_rt = 1 as ::core::ffi::c_int != 0;
            }
        }
        4 => {
            ft.has_present = 1 as ::core::ffi::c_int != 0;
        }
        5 => {
            ft.has_fallback_2d = 1 as ::core::ffi::c_int != 0;
        }
        6 => {
            ft.has_host_upload = 1 as ::core::ffi::c_int != 0;
        }
        7 => {
            ft.has_blit_upload = 1 as ::core::ffi::c_int != 0;
        }
        8 => {
            ft.has_drag_copy = 1 as ::core::ffi::c_int != 0;
            ft.drag_observed = 1 as ::core::ffi::c_int != 0;
        }
        _ => {}
    }
    frame_tracker_mark_surface_owner(ft, surface_offset, pitch, sc, type_0, dst_offset, from_srt);
}
pub fn frame_tracker_record(
    ft: &mut FrameTracker,
    type_0: PassEventType,
    surface_offset: uint32_t,
    pitch: uint32_t,
    src_offset: uint32_t,
    dst_offset: uint32_t,
    w: uint16_t,
    h: uint16_t,
    dst_x: uint16_t,
    dst_y: uint16_t,
    from_srt: bool,
) {
    frame_tracker_record_detail(
        ft,
        type_0,
        surface_offset,
        pitch,
        src_offset,
        dst_offset,
        w,
        h,
        0 as uint16_t,
        0 as uint16_t,
        dst_x,
        dst_y,
        0 as int16_t,
        0 as int16_t,
        from_srt,
    );
}
pub fn frame_tracker_next_pass(ft: &mut FrameTracker) {
    ft.pass_id = ft.pass_id.wrapping_add(1);
}
pub fn frame_tracker_dump(ft: &mut FrameTracker) {
    let forced: bool = ft.force_dump_remaining > 0 as uint32_t;
    if ft.dumped as ::core::ffi::c_int != 0 || ft.event_count == 0 as uint32_t {
        return;
    }
    if ft.frame_id > ft.dump_limit && !forced {
        return;
    }
    ft.dumped = 1 as ::core::ffi::c_int != 0;
    if forced {
        ft.force_dump_remaining = ft.force_dump_remaining.wrapping_sub(1);
    }
    crate::src::logs::err(format_args!("\n[FRAME_SUMMARY] frame={} events={} comp_rt={} win_rt={} present={} fallback={} host_upload={} blit_upload={} drag_copy={} drag={} mixed_owner={} forced={} remaining={}\n", ((ft.frame_id) as u32), ((ft.event_count) as u32), ((ft.has_compositor_rt as ::core::ffi::c_int) as i32), ((ft.has_window_rt as ::core::ffi::c_int) as i32), ((ft.has_present as ::core::ffi::c_int) as i32), ((ft.has_fallback_2d as ::core::ffi::c_int) as i32), ((ft.has_host_upload as ::core::ffi::c_int) as i32), ((ft.has_blit_upload as ::core::ffi::c_int) as i32), ((ft.has_drag_copy as ::core::ffi::c_int) as i32), ((ft.drag_observed as ::core::ffi::c_int) as i32), ((ft.has_mixed_surface_owner as ::core::ffi::c_int) as i32), ((forced as ::core::ffi::c_int) as i32), ((ft.force_dump_remaining) as u32)));
    let mut last_pass: uint32_t = 4294967295 as uint32_t;
    let mut i: uint32_t = 0 as uint32_t;
    while i < ft.event_count {
        let e = &ft.events[i as usize];
        if e.pass_id != last_pass {
            last_pass = e.pass_id;
            crate::src::logs::err(format_args!("[FRAME_PASS] frame={} pass={}\n", ((ft.frame_id) as u32), ((e.pass_id) as u32)));
        }
        crate::src::logs::err(format_args!("  [FRAME_PASS] frame={} pass={} event={} surface_id=0x{:x} pitch={} class={} src_off=0x{:x} dst_off=0x{:x} {}x{} src=({},{}) dst=({},{}) move=({},{}) srt={}\n", ((ft.frame_id) as u32), ((e.pass_id) as u32), pass_event_name(e.type_0), ((e.surface_offset) as u32), ((e.pitch) as u32), surface_class_name(e.surface_class), ((e.src_offset) as u32), ((e.dst_offset) as u32), ((e.width as ::core::ffi::c_int) as u32), ((e.height as ::core::ffi::c_int) as u32), ((e.src_x as ::core::ffi::c_int) as u32), ((e.src_y as ::core::ffi::c_int) as u32), ((e.dst_x as ::core::ffi::c_int) as u32), ((e.dst_y as ::core::ffi::c_int) as u32), ((e.move_x as ::core::ffi::c_int) as i32), ((e.move_y as ::core::ffi::c_int) as i32), ((e.from_srt as ::core::ffi::c_int) as i32)));
        i = i.wrapping_add(1);
    }
    let mut i_0: uint32_t = 0 as uint32_t;
    while i_0 < ft.surface_state_count {
        let st = &ft.surface_states[i_0 as usize];
        if !(!st.valid || st.owner_mask == SURF_OWNER_NONE as ::core::ffi::c_int as uint32_t)
        {
            let owner_buf = frame_tracker_owner_string(st.owner_mask);
            crate::src::logs::err(format_args!("[FRAME_SURFACE] frame={} surface=0x{:x} pitch={} class={} owners={} qe={} cpu={} drag={} host={} blit={} render={}\n", ((ft.frame_id) as u32), ((st.offset) as u32), ((st.pitch) as u32), surface_class_name(st.surface_class), owner_buf, ((st.qe_presents as ::core::ffi::c_int) as u32), ((st.cpu_presents as ::core::ffi::c_int) as u32), ((st.drag_copies as ::core::ffi::c_int) as u32), ((st.host_uploads as ::core::ffi::c_int) as u32), ((st.blit_uploads as ::core::ffi::c_int) as u32), ((st.renders as ::core::ffi::c_int) as u32)));
        }
        i_0 = i_0.wrapping_add(1);
    }
    crate::src::logs::err(format_args!("[FRAME_END] frame={}\n\n", ((ft.frame_id) as u32)));
}
pub fn frame_tracker_record_2d_event(
    type_0: PassEventType,
    surface_offset: uint32_t,
    pitch_bytes: uint32_t,
    src_offset: uint32_t,
    dst_offset: uint32_t,
    src_x: uint32_t,
    src_y: uint32_t,
    dst_x: uint32_t,
    dst_y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    from_srt: bool,
) {
    let Some(ft) = crate::src::qemu::frame_tracker_global() else {
        return;
    };
    frame_tracker_record_detail(
        ft,
        type_0,
        surface_offset,
        pitch_bytes.wrapping_div(4 as uint32_t),
        src_offset,
        dst_offset,
        w as uint16_t,
        h as uint16_t,
        src_x as uint16_t,
        src_y as uint16_t,
        dst_x as uint16_t,
        dst_y as uint16_t,
        0 as int16_t,
        0 as int16_t,
        from_srt,
    );
    if (type_0 as ::core::ffi::c_uint
        == PASS_EVENT_PRESENT as ::core::ffi::c_int as ::core::ffi::c_uint
        || type_0 as ::core::ffi::c_uint
            == PASS_EVENT_FALLBACK_2D as ::core::ffi::c_int as ::core::ffi::c_uint)
        && dst_offset == 0 as uint32_t
        && w >= 800 as uint32_t
        && h >= 400 as uint32_t
    {
        frame_tracker_dump(ft);
        frame_tracker_new_frame(ft);
    }
}
