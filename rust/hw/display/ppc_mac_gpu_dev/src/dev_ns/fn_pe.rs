use super::*;
pub fn pe_area_of(
    x0: uint32_t,
    y0: uint32_t,
    x1: uint32_t,
    y1: uint32_t,
    scr_w: uint32_t,
    scr_h: uint32_t,
) -> PEHarmonyArea {
    let w: uint32_t = x1.wrapping_sub(x0);
    let h: uint32_t = y1.wrapping_sub(y0);
    if scr_w == 0 || scr_h == 0 {
        return PE_AREA_UNKNOWN;
    }
    if x0 == 0 as uint32_t && x1 >= scr_w {
        if y1 >= scr_h {
            return PE_AREA_DESKTOP;
        }
        if y0 == 0 as uint32_t && h <= 48 as uint32_t {
            return PE_AREA_MENUBAR;
        }
    }
    if w >= scr_w && h >= scr_h {
        return PE_AREA_DESKTOP;
    }
    return PE_AREA_WINDOW;
}
pub fn pe_area_name(a: PEHarmonyArea) -> &'static str {
    match a as ::core::ffi::c_uint {
        1 => return "desktop",
        2 => return "menubar",
        3 => return "window",
        _ => return "unknown",
    };
}
pub fn pe_area_of_copy(
    surface: uint32_t,
    x0: uint32_t,
    y0: uint32_t,
    x1: uint32_t,
    y1: uint32_t,
    scr_w: uint32_t,
    scr_h: uint32_t,
) -> PEHarmonyArea {
    let mut a: PEHarmonyArea = pe_area_of(x0, y0, x1, y1, scr_w, scr_h);
    let now: int64_t = crate::src::qemu::monotonic_us() as int64_t;
    if a as ::core::ffi::c_uint == PE_AREA_DESKTOP as ::core::ffi::c_int as ::core::ffi::c_uint {
        pe_desktop_surface.set(surface);
        pe_desktop_surface_us.set(now);
    } else if a as ::core::ffi::c_uint
        == PE_AREA_WINDOW as ::core::ffi::c_int as ::core::ffi::c_uint
        && surface == pe_desktop_surface.get()
        && now - pe_desktop_surface_us.get() < 2 as ::core::ffi::c_long * 1000000 as ::core::ffi::c_long
    {
        a = PE_AREA_DESKTOP;
    }
    return a;
}
pub fn pe_window_kind(e: &PEWindow) -> &'static str {
    return pe_area_name(pe_area_of_copy(
        e.surface,
        e.x0,
        e.y0,
        e.x1,
        e.y1,
        e.scr_w,
        e.scr_h,
    ));
}
/// The all-zero `PEWindow`: the state of an unused slot (the C code memset the table to 0).
const PE_WINDOW_EMPTY: PEWindow = PEWindow {
    surface: 0,
    pitch: 0,
    x0: 0,
    y0: 0,
    x1: 0,
    y1: 0,
    fx0: 0,
    fy0: 0,
    fx1: 0,
    fy1: 0,
    scr_w: 0,
    scr_h: 0,
    pieces: 0,
    last_us: 0,
    live: false,
    settled: false,
};

pub fn pe_harmony_mark(
    surface: uint32_t,
    x: uint32_t,
    y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    scr_w: uint32_t,
    scr_h: uint32_t,
) {
    if scr_w == 0 || scr_h == 0 {
        return;
    }
    let cols: ::core::ffi::c_int = scr_w
        .wrapping_add(8 as uint32_t)
        .wrapping_sub(1 as uint32_t)
        .wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
    let rows: ::core::ffi::c_int = scr_h
        .wrapping_add(8 as uint32_t)
        .wrapping_sub(1 as uint32_t)
        .wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
    if cols != pe_harm_cols.get() || rows != pe_harm_rows.get() {
        crate::src::qemu::harm_tiles_replace((cols as usize).wrapping_mul(rows as usize));
        pe_harm_cols.set(cols);
        pe_harm_rows.set(rows);
        pe_harm_resized_us.set(crate::src::qemu::monotonic_us() as int64_t);
    }
    let now: int64_t = crate::src::qemu::monotonic_us() as int64_t;
    let settling: bool = now - pe_harm_resized_us.get()
        < 1000000 as ::core::ffi::c_long * 3 as ::core::ffi::c_long / 2 as ::core::ffi::c_long;
    if settling as ::core::ffi::c_int != 0
        && (w as uint64_t)
            .wrapping_mul(h as uint64_t)
            .wrapping_mul(10 as uint64_t)
            < (scr_w as uint64_t)
                .wrapping_mul(scr_h as uint64_t)
                .wrapping_mul(9 as uint64_t)
    {
        return;
    }
    let a: PEHarmonyArea = pe_area_of_copy(
        surface,
        x,
        y,
        x.wrapping_add(w),
        y.wrapping_add(h),
        scr_w,
        scr_h,
    );
    let mut tx0: ::core::ffi::c_int = 0;
    let mut ty0: ::core::ffi::c_int = 0;
    let mut tx1: ::core::ffi::c_int = 0;
    let mut ty1: ::core::ffi::c_int = 0;
    if a as ::core::ffi::c_uint == PE_AREA_WINDOW as ::core::ffi::c_int as ::core::ffi::c_uint {
        tx0 = x.wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
        ty0 = y.wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
        tx1 = x
            .wrapping_add(w)
            .wrapping_add(8 as uint32_t)
            .wrapping_sub(1 as uint32_t)
            .wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
        ty1 = y
            .wrapping_add(h)
            .wrapping_add(8 as uint32_t)
            .wrapping_sub(1 as uint32_t)
            .wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
    } else {
        tx0 = x
            .wrapping_add(8 as uint32_t)
            .wrapping_sub(1 as uint32_t)
            .wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
        ty0 = y
            .wrapping_add(8 as uint32_t)
            .wrapping_sub(1 as uint32_t)
            .wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
        tx1 = x.wrapping_add(w).wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
        ty1 = y.wrapping_add(h).wrapping_div(8 as uint32_t) as ::core::ffi::c_int;
    }
    tx1 = {
        let mut _a15: ::core::ffi::c_int = tx1 as ::core::ffi::c_int;
        let mut _b16: ::core::ffi::c_int = cols as ::core::ffi::c_int;
        if _a15 < _b16 {
            _a15 as ::core::ffi::c_int
        } else {
            _b16 as ::core::ffi::c_int
        }
    };
    ty1 = {
        let mut _a17: ::core::ffi::c_int = ty1 as ::core::ffi::c_int;
        let mut _b18: ::core::ffi::c_int = rows as ::core::ffi::c_int;
        if _a17 < _b18 {
            _a17 as ::core::ffi::c_int
        } else {
            _b18 as ::core::ffi::c_int
        }
    };
    let whole_screen: bool = (w as uint64_t)
        .wrapping_mul(h as uint64_t)
        .wrapping_mul(10 as uint64_t)
        >= (scr_w as uint64_t)
            .wrapping_mul(scr_h as uint64_t)
            .wrapping_mul(9 as uint64_t);
    // The map was (re)allocated above as cols*rows bytes, or kept from an earlier call.
    let Some(tiles) = crate::src::qemu::harm_tiles_mut() else {
        return;
    };
    for ty in ty0.max(0)..ty1 {
        let row = &mut tiles[ty as usize * cols as usize..][..cols as usize];
        for tile in &mut row[tx0.max(0) as usize..tx1.max(0) as usize] {
            if whole_screen && *tile as ::core::ffi::c_int != PE_AREA_UNKNOWN as ::core::ffi::c_int {
                continue;
            }
            if *tile != a as uint8_t {
                *tile = a as uint8_t;
                pe_harm_gen.set(pe_harm_gen.get().wrapping_add(1));
            }
        }
    }
}
pub fn pe_window_saw_blit(
    surface: uint32_t,
    pitch: uint32_t,
    x: uint32_t,
    y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    scr_w: uint32_t,
    scr_h: uint32_t,
) {
    let on_level = crate::src::qemu::env_first_byte(c"PPCGPU_WINDOWS");
    let on = crate::src::qemu::env_flag(c"PPCGPU_WINDOWS");
    if w == 0 || h == 0 {
        return;
    }
    pe_harmony_mark(surface, x, y, w, h, scr_w, scr_h);
    let now: int64_t = crate::src::qemu::monotonic_us() as int64_t;
    if on && on_level.is_some_and(|b| b as i8 as ::core::ffi::c_int >= '2' as i32) {
        crate::src::logs::err(format_args!("PEBLIT surface={:06x} pitch={} at=({},{}) {}x{} screen={}x{}\n", ((surface) as u32), ((pitch) as u32), ((x) as u32), ((y) as u32), ((w) as u32), ((h) as u32), ((scr_w) as u32), ((scr_h) as u32)));
    }
    let mut windows_guard = pe_windows.lock().unwrap_or_else(|e| e.into_inner());
    let windows = &mut *windows_guard;
    let slot_idx = match windows
        .iter()
        .position(|e| e.live && e.surface == surface && e.pitch == pitch)
    {
        Some(i) => i,
        None => {
            // First free entry, else the least recently used one.
            let mut oldest = now;
            let mut pick = None;
            for (i, e) in windows.iter().enumerate() {
                if !e.live {
                    pick = Some(i);
                    break;
                }
                if e.last_us <= oldest {
                    oldest = e.last_us;
                    pick = Some(i);
                }
            }
            let i = pick.unwrap_or(0);
            let e = &mut windows[i];
            *e = PE_WINDOW_EMPTY;
            e.live = true;
            e.surface = surface;
            e.pitch = pitch;
            e.last_us = now - 200 * 1000 - 1;
            i
        }
    };
    let slot = &mut windows[slot_idx];
    if now - slot.last_us > 200 * 1000 {
        if slot.settled {
            slot.fx0 = slot.x0;
            slot.fy0 = slot.y0;
            slot.fx1 = slot.x1;
            slot.fy1 = slot.y1;
        }
        slot.x0 = x;
        slot.y0 = y;
        slot.x1 = x.wrapping_add(w);
        slot.y1 = y.wrapping_add(h);
        slot.settled = true;
    } else {
        slot.x0 = slot.x0.min(x);
        slot.y0 = slot.y0.min(y);
        slot.x1 = slot.x1.max(x.wrapping_add(w));
        slot.y1 = slot.y1.max(y.wrapping_add(h));
    }
    slot.pieces = slot.pieces.wrapping_add(1);
    slot.last_us = now;
    slot.scr_w = scr_w;
    slot.scr_h = scr_h;
    if !on || now - pe_windows_printed_us.get() < 2 * 1000000 {
        return;
    }
    pe_windows_printed_us.set(now);
    crate::src::logs::err(format_args!("PEWINDOWS ----\n"));
    for e in windows.iter() {
        if !e.live || now - e.last_us > 5 * 1000000 {
            continue;
        }
        let kind = pe_window_kind(e);
        crate::src::logs::err(format_args!("PEWINDOW surface={:06x} pitch={} frame=({},{})-({},{}) {}x{} kind={} pieces={}\n", ((e.surface) as u32), ((e.pitch) as u32), ((e.x0) as u32), ((e.y0) as u32), ((e.x1) as u32), ((e.y1) as u32), ((e.x1.wrapping_sub(e.x0)) as u32), ((e.y1.wrapping_sub(e.y0)) as u32), kind, ((e.pieces) as u64)));
    }
}
