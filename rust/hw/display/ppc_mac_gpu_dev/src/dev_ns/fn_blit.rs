use super::*;
pub fn blit_region_record(
    src_offset: uint32_t,
    src_pitch: uint32_t,
    src_x: uint32_t,
    src_y: uint32_t,
    dst_x: uint32_t,
    dst_y: uint32_t,
    w: uint32_t,
    h: uint32_t,
) {
    let mut guard = g_blit_region_table.lock().unwrap_or_else(|e| e.into_inner());
    let t = &mut *guard;
    let region = BlitRegion {
        src_x,
        src_y,
        dst_x,
        dst_y,
        width: w,
        height: h,
        sequence: 0,
    };
    let used = t.rt_count as usize;
    let idx = match t.rts[..used]
        .iter()
        .position(|rt| rt.rt_offset == src_offset && rt.rt_pitch == src_pitch)
    {
        Some(i) => i,
        None => {
            // Table full: drop the oldest render target.
            if used >= t.rts.len() {
                t.rts.copy_within(1.., 0);
                t.rt_count = t.rts.len() as ::core::ffi::c_int - 1;
            }
            let i = t.rt_count as usize;
            t.rt_count += 1;
            t.rts[i].rt_offset = src_offset;
            t.rts[i].rt_pitch = src_pitch;
            t.rts[i].region_count = 0;
            i
        }
    };
    let sequence = t.sequence;
    t.sequence = t.sequence.wrapping_add(1);
    let rt = &mut t.rts[idx];
    let count = rt.region_count as usize;
    if let Some(r) = rt.regions[..count].iter_mut().find(|r| {
        r.src_x == src_x
            && r.src_y == src_y
            && r.dst_x == dst_x
            && r.dst_y == dst_y
            && r.width == w
            && r.height == h
    }) {
        r.sequence = sequence;
        return;
    }
    let slot = if count >= rt.regions.len() {
        // Region list full: replace the least recently used entry.
        rt.regions[..count]
            .iter()
            .enumerate()
            .fold((0, u32::MAX), |best, (i, r)| if r.sequence < best.1 { (i, r.sequence) } else { best })
            .0
    } else {
        rt.region_count += 1;
        count
    };
    rt.regions[slot] = BlitRegion { sequence, ..region };
}
pub fn blit_overlaps(
    src_offset: uint32_t,
    src_pitch: uint32_t,
    src_x: uint32_t,
    src_y: uint32_t,
    dst_offset: uint32_t,
    dst_pitch: uint32_t,
    dst_x: uint32_t,
    dst_y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    _bpp: uint32_t,
) -> bool {
    if src_offset != dst_offset || src_pitch != dst_pitch {
        return 0 as ::core::ffi::c_int != 0;
    }
    if src_x == dst_x && src_y == dst_y {
        return 0 as ::core::ffi::c_int != 0;
    }
    return !(src_x.wrapping_add(w) <= dst_x
        || dst_x.wrapping_add(w) <= src_x
        || src_y.wrapping_add(h) <= dst_y
        || dst_y.wrapping_add(h) <= src_y);
}
pub fn blit_rect_untiled(
    s: &mut PPCMacGPUState,
    vram: crate::src::qemu::Vram,
    src_offset: uint32_t,
    src_pitch: uint32_t,
    src_x: uint32_t,
    src_y: uint32_t,
    dst_offset: uint32_t,
    dst_pitch: uint32_t,
    dst_x: uint32_t,
    dst_y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    bpp: uint32_t,
) -> bool {
    if mc_rect_is_tiled(s, src_offset, src_pitch, src_x, src_y, bpp) as ::core::ffi::c_int != 0
        || mc_rect_is_tiled(s, dst_offset, dst_pitch, dst_x, dst_y, bpp) as ::core::ffi::c_int != 0
    {
        return 0 as ::core::ffi::c_int != 0;
    }
    let rev_rows: bool = blit_overlaps(
        src_offset, src_pitch, src_x, src_y, dst_offset, dst_pitch, dst_x, dst_y, w, h, bpp,
    ) as ::core::ffi::c_int
        != 0
        && dst_y > src_y;
    let row_bytes: size_t = (w as size_t).wrapping_mul(bpp as size_t);
    let mut r: uint32_t = 0 as uint32_t;
    while r < h {
        let row: uint32_t = if rev_rows as ::core::ffi::c_int != 0 {
            h.wrapping_sub(1 as uint32_t).wrapping_sub(r)
        } else {
            r
        };
        let s_row: uint64_t = (src_offset as uint64_t)
            .wrapping_add((src_y.wrapping_add(row) as uint64_t).wrapping_mul(src_pitch as uint64_t))
            .wrapping_add((src_x as uint64_t).wrapping_mul(bpp as uint64_t));
        let d_row: uint64_t = (dst_offset as uint64_t)
            .wrapping_add((dst_y.wrapping_add(row) as uint64_t).wrapping_mul(dst_pitch as uint64_t))
            .wrapping_add((dst_x as uint64_t).wrapping_mul(bpp as uint64_t));
        if !((s_row as size_t).wrapping_add(row_bytes) > s.vram_size
            || (d_row as size_t).wrapping_add(row_bytes) > s.vram_size)
        {
            vram.copy_within(((s_row) as u64), ((d_row) as u64), (row_bytes) as usize);
        }
        r = r.wrapping_add(1);
    }
    return 1 as ::core::ffi::c_int != 0;
}
