use super::*;
pub fn r200_fill_notify(
    s: &mut PPCMacGPUState,
    offset: uint32_t,
    pitch: uint32_t,
    x: uint32_t,
    y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    bpp: uint32_t,
    value: uint32_t,
) {
    crate::src::qemu::renderer_fill_notify(s, offset, pitch, x, y, w, h, bpp, value);
}
/// Retires the fence entries whose sequence number has been reached by `done`, in queue order.
/// Each retired entry is written back to its scratch register; the rest are shifted to the
/// front of the queue.  The queue is bounded by its array length, so a corrupt count cannot
/// index past it.
pub fn r200_fence_drain(s: &mut PPCMacGPUState, done: uint32_t) {
    let pending = (s.regs.r200_fence_n as usize).min(s.regs.r200_fence_q.len());
    let mut n: usize = 0;
    while n < pending {
        let entry = s.regs.r200_fence_q[n];
        if (done.wrapping_sub(entry.seq) as i32) < 0 {
            break;
        }
        ppc_mac_gpu_scratch_writeback_val(s, entry.reg as ::core::ffi::c_int, entry.val);
        n += 1;
    }
    if n != 0 {
        s.regs.r200_fence_q.copy_within(n..pending, 0);
        s.regs.r200_fence_n = s.regs.r200_fence_n.wrapping_sub(n as uint32_t);
    }
}
pub unsafe extern "C" fn r200_fence_bh_cb(opaque: *mut ::core::ffi::c_void) {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return;
    };
    let fence_now: uint32_t =
        crate::src::qemu::fence_done_atomic(s).load(::core::sync::atomic::Ordering::Relaxed);
    r200_fence_drain(s, fence_now);
}
pub unsafe extern "C" fn r200_fence_done_cb(arg: *mut ::core::ffi::c_void, seq: uint32_t) {
    // SAFETY: `arg` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(arg) }) else {
        return;
    };
    crate::src::qemu::fence_done_atomic(s).store(seq, ::core::sync::atomic::Ordering::Relaxed);
    crate::src::qemu::fence_bh_schedule();
}
pub fn r200_scratch_read_wait(s: &mut PPCMacGPUState, idx: ::core::ffi::c_int) -> bool {
    let mut want: uint32_t = 0;
    let mut pending: bool = false;
    // Entries past the queue length (a corrupt count) are ignored, never indexed.
    let queued = (s.regs.r200_fence_n as usize).min(s.regs.r200_fence_q.len());
    for entry in &s.regs.r200_fence_q[..queued] {
        if entry.reg == idx as uint32_t {
            want = entry.seq;
            pending = true;
        }
    }
    if pending {
        let deadline: int64_t = crate::src::qemu::monotonic_us() as int64_t + 100000;
        while (crate::src::qemu::fence_done_atomic(s)
            .load(::core::sync::atomic::Ordering::Relaxed)
            .wrapping_sub(want) as i32)
            < 0
        {
            if crate::src::qemu::monotonic_us() >= deadline {
                return false;
            }
            crate::src::qemu::sleep_us(20);
        }
        let fence_now: uint32_t =
            crate::src::qemu::fence_done_atomic(s).load(::core::sync::atomic::Ordering::Relaxed);
        r200_fence_drain(s, fence_now);
    }
    true
}
pub fn r200_scratch_write(s: &mut PPCMacGPUState, idx: ::core::ffi::c_int, val: uint32_t) {
    let mut seq: uint32_t = 0;
    if s.regs.aic_ctrl & 1 as uint32_t != 0 {
        r200_agp_tc_flush();
    }
    static fence_mode: crate::src::global::Global<::core::ffi::c_int> =
        crate::src::global::Global::<::core::ffi::c_int>::new(-1);
    static nowait: crate::src::global::Global<::core::ffi::c_int> =
        crate::src::global::Global::<::core::ffi::c_int>::new(0);
    if fence_mode.get() < 0 {
        fence_mode.set(match crate::src::qemu::env_first_byte(c"PPCGPU_ASYNC_FENCE") {
            Some(b'1') => 1,
            Some(b'0') => 0,
            _ => 2,
        });
    }
    let sync_fences: bool = fence_mode.get() == 0;
    if sync_fences {
        gpu_sync_at(
            s,
            (0x4000 as uint32_t).wrapping_add(GPU_WHY_FENCE as ::core::ffi::c_int as uint32_t),
        );
        ppc_mac_gpu_scratch_writeback_val(s, idx, val);
        return;
    }
    if !s.renderer.is_null() && s.renderer_ref().submit_fence.is_some() {
        crate::src::qemu::fence_bh_ensure(s, r200_fence_bh_cb);
        seq = crate::src::qemu::renderer_submit_fence(s, r200_fence_done_cb).unwrap_or(0);
    }
    if seq == 0 && s.regs.r200_fence_n != 0 {
        seq = s.regs.r200_fence_last_seq;
    }
    let queued = s.regs.r200_fence_n as usize;
    if seq == 0 || queued >= s.regs.r200_fence_q.len() {
        gpu_sync_at(
            s,
            (0x4000 as uint32_t).wrapping_add(GPU_WHY_FENCE as ::core::ffi::c_int as uint32_t),
        );
        ppc_mac_gpu_scratch_writeback_val(s, idx, val);
        return;
    }
    s.regs.r200_fence_q[queued].seq = seq;
    s.regs.r200_fence_q[queued].reg = idx as uint32_t;
    s.regs.r200_fence_q[queued].val = val;
    s.regs.r200_fence_n = s.regs.r200_fence_n.wrapping_add(1);
    s.regs.r200_fence_last_seq = seq;
    if fence_mode.get() == 2 {
        if nowait.get() > 0 {
            nowait.set(nowait.get() - 1);
        } else {
            static wait_us: crate::src::global::Global<::core::ffi::c_int> =
                crate::src::global::Global::<::core::ffi::c_int>::new(-1);
            if wait_us.get() < 0 {
                wait_us.set(match crate::src::qemu::env_string(c"PPCGPU_FENCE_WAIT_US") {
                    Some(w) => crate::src::qemu::parse_atoi(&w),
                    None => 250,
                });
            }
            let deadline: int64_t =
                crate::src::qemu::monotonic_us() as int64_t + wait_us.get() as int64_t;
            while (crate::src::qemu::fence_done_atomic(s)
                .load(::core::sync::atomic::Ordering::Relaxed)
                .wrapping_sub(seq) as i32)
                < 0
            {
                if crate::src::qemu::monotonic_us() >= deadline {
                    nowait.set(16);
                    break;
                } else {
                    crate::src::qemu::yield_now();
                }
            }
        }
    }
    let fence_now: uint32_t =
        crate::src::qemu::fence_done_atomic(s).load(::core::sync::atomic::Ordering::Relaxed);
    r200_fence_drain(s, fence_now);
}
pub fn r200_datatype_bpp(dt: uint32_t) -> uint32_t {
    match dt & 0xf as uint32_t {
        2 | 7 | 8 | 9 => return 1 as uint32_t,
        3 | 4 | 11 | 12 | 15 => return 2 as uint32_t,
        _ => return 4 as uint32_t,
    };
}
pub fn r200_macro_tile_addr(
    base_offset: uint32_t,
    pitch: uint32_t,
    x: uint32_t,
    y: uint32_t,
    bpp: uint32_t,
) -> uint64_t {
    let tile_w_pixels: uint32_t = (256 as uint32_t).wrapping_div(bpp);
    let tile_h: uint32_t = 16 as uint32_t;
    let tiles_per_row: uint32_t = pitch.wrapping_div(256 as uint32_t);
    let tile_x: uint32_t = x.wrapping_div(tile_w_pixels);
    let tile_y: uint32_t = y.wrapping_div(tile_h);
    let in_x: uint32_t = x.wrapping_rem(tile_w_pixels);
    let in_y: uint32_t = y.wrapping_rem(tile_h);
    let tile_idx: uint64_t = (tile_y as uint64_t)
        .wrapping_mul(tiles_per_row as uint64_t)
        .wrapping_add(tile_x as uint64_t);
    return (base_offset as uint64_t)
        .wrapping_add(tile_idx.wrapping_mul((256 as uint32_t).wrapping_mul(tile_h) as uint64_t))
        .wrapping_add((in_y as uint64_t).wrapping_mul(256 as uint64_t))
        .wrapping_add((in_x as uint64_t).wrapping_mul(bpp as uint64_t));
}
pub fn r200_agp_tc_flush() {
    // Same bytes as the former memset to zero: every field of an entry is zero.
    r200_agp_tc_mut().fill_with(|| C2RustUnnamed_99 {
        tag: 0,
        host: ::core::ptr::null_mut(),
        pte: 0,
        pte_raw: 0,
    });
}
