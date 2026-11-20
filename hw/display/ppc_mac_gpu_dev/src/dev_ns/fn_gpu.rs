use super::*;
pub fn gpu_guest_dw(_s: &mut PPCMacGPUFields, raw: uint32_t) -> uint32_t {
    return raw;
}
pub fn gpu_diag_on() -> bool {
    static on: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    if on.get() < 0 as ::core::ffi::c_int {
        on.set(crate::src::qemu::env_flag(c"PPCGPU_DIAG") as ::core::ffi::c_int);
    }
    return on.get() != 0;
}
pub fn gpu_sync_at(s: &mut PPCMacGPUFields, _why: uint32_t) {
    if !s.renderer.is_null() && s.renderer_ref().flush_sync.is_some() {
        // SAFETY: flush_sync is the renderer's callback, set together with renderer_opaque
        // when the renderer was attached; both stay valid while renderer is non-null.
        unsafe {
            s.renderer_ref()
                .flush_sync
                .expect("non-null function pointer")(s.renderer_opaque);
            if s.regs.r200_fence_n != 0 {
                r200_fence_drain(s, s.regs.r200_fence_last_seq);
            }
        }
    }
}
pub fn gpu_vram_dirty(
    s: &mut PPCMacGPUFields,
    addr: uint64_t,
    size: uint64_t,
) {
    static tex_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    if (tex_log.get() < 0 as ::core::ffi::c_int) as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        tex_log.set(crate::src::qemu::env_flag(c"PPCGPU_TEX_LOG") as ::core::ffi::c_int);
    }
    if (tex_log.get() != 0) as ::core::ffi::c_int as ::core::ffi::c_long != 0 && size >= 4096 as uint64_t
    {
        crate::src::logs::err(format_args!("VRAMDIRTY {:x} +{:x}\n", ((addr as ::core::ffi::c_ulonglong) as u64), ((size as ::core::ffi::c_ulonglong) as u64)));
    }
    s.vram_dev_dirty = 1 as ::core::ffi::c_int != 0;
    // SAFETY: s.vram is the device's initialised VRAM MemoryRegion; marking a range dirty
    // only updates QEMU's dirty bitmap and clamps the range itself.
    unsafe { memory_region_set_dirty(&raw mut s.vram, addr as hwaddr, size as hwaddr) };
}
pub fn gpu_vram_access(
    s: &mut PPCMacGPUFields,
    lo: uint64_t,
    hi: uint64_t,
    write_access: bool,
) {
    if s.renderer.is_null() || s.renderer_ref().range_busy_sync.is_none() || hi <= lo {
        return;
    }
    // SAFETY: range_busy_sync was checked non-null above; the callback is paired with
    // renderer_opaque, both set when the renderer was attached.
    let busy = unsafe {
        s.renderer_ref()
            .range_busy_sync
            .expect("non-null function pointer")(s.renderer_opaque, lo, hi, write_access)
    };
    if busy {
        gpu_sync_at(
            s,
            (0x4000 as uint32_t).wrapping_add(GPU_WHY_2D as ::core::ffi::c_int as uint32_t),
        );
    }
}
pub fn gpu_debug_enabled() -> bool {
    static enabled: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    if enabled.get() < 0 as ::core::ffi::c_int {
        enabled.set((crate::src::qemu::env_first_byte(c"PPCGPU_DEBUG_LOG") == Some(b'1')) as ::core::ffi::c_int);
    }
    enabled.get() != 0
}
pub fn gpu_track_only() -> bool {
    static on: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    if on.get() < 0 as ::core::ffi::c_int {
        on.set(crate::src::qemu::env_flag(c"PPCGPU_TRACK_ONLY") as ::core::ffi::c_int);
    }
    return on.get() != 0;
}
pub fn gpu_tracked_reg(addr: hwaddr) -> bool {
    match addr as ::core::ffi::c_ulonglong & !(3 as ::core::ffi::c_ulonglong) {
        160 | 164 | 224 | 228 | 232 | 336 | 344 | 372 | 64 | 68 => {
            return 1 as ::core::ffi::c_int != 0;
        }
        492 | 496 | 820 | 2044 | 2052 | 2184 | 2312 | 5640 | 8116 => {
            return 1 as ::core::ffi::c_int != 0;
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    };
}
pub fn gpu_set_present_pitch(
    s: &mut PPCMacGPUFields,
    dst_pitch: uint32_t,
) {
    let min: uint32_t = s.disp.width.wrapping_mul(
        s.disp
            .bpp
            .wrapping_add(7 as uint32_t)
            .wrapping_div(8 as uint32_t),
    );
    if dst_pitch < min {
        static pitch_reject_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
        if pitch_reject_log.get() < 10 as ::core::ffi::c_int {
            crate::src::logs::err(format_args!("[STRIDE_CHANGE] ignoring BLT pitch {} for {}x{}@{} (needs {})\n", ((dst_pitch) as u32), ((s.disp.width) as u32), ((s.disp.height) as u32), ((s.disp.bpp) as u32), ((min) as u32)));
            pitch_reject_log.set(pitch_reject_log.get() + 1);
        }
        return;
    }
    s.disp_stride_override_active = 1 as ::core::ffi::c_int != 0;
    s.disp_stride_override_value = dst_pitch;
    s.disp.stride = dst_pitch;
    s.display_invalid = 1 as ::core::ffi::c_int != 0;
}
/// Expands a 6-bit VGA DAC value to 8 bits by bit replication. Kept in u32
/// arithmetic so out-of-range palette bytes produce the same bits as before.
fn dac6(b: u8) -> u32 {
    let b = u32::from(b);
    (b << 2) | (b >> 4)
}
pub fn gpu_dac_get(s: &PPCMacGPUFields, i: u32) -> u32 {
    if s.legacy_vga {
        let vga = s.vga_ref();
        let idx = (i & 0xff) as usize;
        if let Some(&[r, g, b]) = vga.palette.chunks_exact(3).nth(idx) {
            return if vga.dac_8bit != 0 {
                (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
            } else {
                (dac6(r) << 16) | (dac6(g) << 8) | dac6(b)
            };
        }
    }
    s.regs.palette[(i & 0xff) as usize] & 0x00ff_ffff
}
pub fn gpu_dac_set(
    s: &mut PPCMacGPUFields,
    i: ::core::ffi::c_uint,
    rgb: uint32_t,
) {
    let idx = (i & 0xff) as usize;
    s.regs.palette[idx] = rgb;
    if s.legacy_vga {
        let sh: uint32_t = if s.vga_ref().dac_8bit != 0 { 0 } else { 2 };
        // The palette is 256 entries of 3 bytes, so `idx * 3 .. idx * 3 + 3` is always in range.
        let slot = &mut s.vga_mut().palette[idx * 3..idx * 3 + 3];
        slot[0] = ((rgb >> 16 & 0xff) >> sh) as uint8_t;
        slot[1] = ((rgb >> 8 & 0xff) >> sh) as uint8_t;
        slot[2] = ((rgb & 0xff) >> sh) as uint8_t;
    }
}
pub fn gpu_po_offset(s: &mut PPCMacGPUFields, po: uint32_t) -> uint32_t {
    let mut off: uint32_t =
        (po & 0x3fffff as ::core::ffi::c_int as uint32_t) << 10 as ::core::ffi::c_int;
    let fb: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    if off >= fb {
        off = off.wrapping_sub(fb);
    }
    return off;
}
pub fn gpu_gmc_bpp(gmc: uint32_t) -> uint32_t {
    match gmc >> 8 as ::core::ffi::c_int & 0xf as uint32_t {
        2 => return 1 as uint32_t,
        3 | 4 => return 2 as uint32_t,
        _ => return 4 as uint32_t,
    };
}
pub fn gpu_px_ptr(
    s: &mut PPCMacGPUFields,
    vram: crate::src::qemu::Vram,
    a: uint64_t,
    bpp: uint32_t,
) -> *mut uint8_t {
    if a < s.vram_size {
        return if a.wrapping_add(bpp as uint64_t) <= s.vram_size {
            // a < vram_size, so the offset stays inside the VRAM mapping.
            vram.as_mut_ptr().wrapping_add(a as usize)
        } else {
            ::core::ptr::null_mut::<uint8_t>()
        };
    }
    if a > 0xffff_ffff {
        return ::core::ptr::null_mut::<uint8_t>();
    }
    let mut pg: *mut uint8_t = gpu_agp_page(s, a as uint32_t);
    if pg.is_null() {
        let fb: uint64_t = ((s.regs.mc_fb_location & 0xffff as uint32_t) as uint64_t) << 16;
        if fb != 0 && a.wrapping_add(fb) <= 0xffff_ffff {
            pg = gpu_agp_page(s, a.wrapping_add(fb) as uint32_t);
        }
    }
    if pg.is_null() {
        ::core::ptr::null_mut::<uint8_t>()
    } else {
        // The page-relative offset is below 0x1000, so it stays inside the page.
        pg.wrapping_add((a & 0xfff) as usize)
    }
}
pub fn gpu_px_get(
    s: &mut PPCMacGPUFields,
    vram: crate::src::qemu::Vram,
    a: uint64_t,
    bpp: uint32_t,
) -> uint32_t {
    if a < s.vram_size {
        if a.wrapping_add(bpp as uint64_t) > s.vram_size {
            return 0;
        }
        return match bpp {
            4 => vram.get32(a).unwrap_or(0),
            2 => vram.get16(a).unwrap_or(0) as uint32_t,
            _ => vram.get8(a).unwrap_or(0) as uint32_t,
        };
    }
    let p: *mut uint8_t = gpu_px_ptr(s, vram, a, bpp);
    if p.is_null() {
        return 0 as uint32_t;
    }
    crate::src::qemu::px_read(s, p, bpp as usize)
}
pub fn gpu_px_put(
    s: &mut PPCMacGPUFields,
    vram: crate::src::qemu::Vram,
    a: uint64_t,
    v: uint32_t,
    bpp: uint32_t,
) {
    if a < s.vram_size {
        // VRAM: bounds-checked; a write that would run past the end is dropped.
        if a.wrapping_add(bpp as uint64_t) <= s.vram_size {
            match bpp {
                4 => vram.put32(a, v),
                2 => vram.put16(a, v as u16),
                _ => vram.put8(a, v as u8),
            };
        }
        return;
    }
    let p: *mut uint8_t = gpu_px_ptr(s, vram, a, bpp);
    if !p.is_null() {
        crate::src::qemu::px_write(s, p, bpp as usize, v);
    }
}
pub fn gpu_mc_copy(
    s: &mut PPCMacGPUFields,
    vram: crate::src::qemu::Vram,
    mut da: uint64_t,
    mut sa: uint64_t,
    mut n: uint32_t,
) {
    while n != 0 {
        let mut chunk: uint32_t = n;
        let mut d: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
        let mut r: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
        if da >= s.vram_size {
            chunk = {
                let mut _a89: uint32_t = chunk as uint32_t;
                let mut _b90: uint32_t =
                    (0x1000 as uint32_t).wrapping_sub((da & 0xfff as uint64_t) as uint32_t);
                if _a89 < _b90 {
                    _a89 as uint32_t
                } else {
                    _b90 as uint32_t
                }
            };
        }
        if sa >= s.vram_size {
            chunk = {
                let mut _a91: uint32_t = chunk as uint32_t;
                let mut _b92: uint32_t =
                    (0x1000 as uint32_t).wrapping_sub((sa & 0xfff as uint64_t) as uint32_t);
                if _a91 < _b92 {
                    _a91 as uint32_t
                } else {
                    _b92 as uint32_t
                }
            };
        }
        d = gpu_px_ptr(s, vram, da, 1 as uint32_t);
        r = gpu_px_ptr(s, vram, sa, 1 as uint32_t);
        if d.is_null() || r.is_null() {
            static nwarn: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
            let fresh44 = nwarn.get();
            nwarn.set(nwarn.get() + 1);
            if fresh44 < 20 as ::core::ffi::c_int {
                crate::src::logs::err(format_args!("MC_COPY dropped: da={:x}({}) sa={:x}({}) n={}\n", ((da as ::core::ffi::c_ulonglong) as u64), crate::src::qemu::cstr(if !d.is_null() {
                        c"ok".as_ptr()
                    } else {
                        c"UNMAPPED".as_ptr()
                    }), ((sa as ::core::ffi::c_ulonglong) as u64), crate::src::qemu::cstr(if !r.is_null() {
                        c"ok".as_ptr()
                    } else {
                        c"UNMAPPED".as_ptr()
                    }), ((n) as u32)));
            }
            return;
        }
        if da < s.vram_size {
            chunk = {
                let mut _a93: uint32_t = chunk as uint32_t;
                let mut _b94: uint32_t = s.vram_size.wrapping_sub(da) as uint32_t;
                if _a93 < _b94 {
                    _a93 as uint32_t
                } else {
                    _b94 as uint32_t
                }
            };
        }
        if sa < s.vram_size {
            chunk = {
                let mut _a95: uint32_t = chunk as uint32_t;
                let mut _b96: uint32_t = s.vram_size.wrapping_sub(sa) as uint32_t;
                if _a95 < _b96 {
                    _a95 as uint32_t
                } else {
                    _b96 as uint32_t
                }
            };
        }
        crate::src::qemu::px_move(s, d, r, chunk as usize);
        da = da.wrapping_add(chunk as uint64_t);
        sa = sa.wrapping_add(chunk as uint64_t);
        n = n.wrapping_sub(chunk);
    }
}
pub fn gpu_rop3(
    rop: uint32_t,
    pat: uint32_t,
    src: uint32_t,
    dst: uint32_t,
) -> uint32_t {
    let mut r: uint32_t = 0 as uint32_t;
    let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while k < 8 as ::core::ffi::c_int {
        if rop as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << k != 0 {
            r |= (if k & 4 as ::core::ffi::c_int != 0 {
                pat
            } else {
                !pat
            }) & (if k & 2 as ::core::ffi::c_int != 0 {
                src
            } else {
                !src
            }) & (if k & 1 as ::core::ffi::c_int != 0 {
                dst
            } else {
                !dst
            });
        }
        k += 1;
    }
    return r;
}
pub fn gpu_unified_2d() -> bool {
    return 1 as ::core::ffi::c_int != 0;
}
pub fn gpu_agp_page(s: &mut PPCMacGPUFields, gpu_addr: uint32_t) -> *mut uint8_t {
    let page: uint32_t = gpu_addr >> 12 as ::core::ffi::c_int;
    let i: usize = (page as usize) & (r200_agp_tc_mut().len() - 1);
    let hit = r200_agp_tc_mut()[i];
    if hit.tag == page.wrapping_add(1 as uint32_t) {
        let mut cur: uint32_t = 0 as uint32_t;
        if crate::src::qemu::guest_read32(hit.pte as u64, &mut cur) == 0 as MemTxResult
            && cur == hit.pte_raw
        {
            return hit.host;
        }
    }
    let mut phys: hwaddr = 0;
    if !ppc_mac_gpu_agp_translate(s, page << 12 as ::core::ffi::c_int, &mut phys)
        && !ppc_mac_gpu_gart_translate(s, page << 12 as ::core::ffi::c_int, &mut phys)
    {
        return ::core::ptr::null_mut::<uint8_t>();
    }
    let Some(h) = crate::src::qemu::guest_ram_page_host(phys & !(0xfff as hwaddr)) else {
        return ::core::ptr::null_mut::<uint8_t>();
    };
    let e = &mut r200_agp_tc_mut()[i];
    e.tag = page.wrapping_add(1 as uint32_t);
    e.host = h;
    e.pte = r200_last_pte_addr.get();
    e.pte_raw = r200_last_pte_raw.get();
    h
}
pub fn gpu_caller_probe(addr: hwaddr) {
    static WATCH: [uint32_t; 10] = [0x1ec, 0x1f0, 0x334, 0x7fc, 0x804, 0x888, 0x908, 0x1608, 0x1fb4, 0x44];
    static CNT: [crate::src::global::Global<::core::ffi::c_uint>; 10] =
        [const { crate::src::global::Global::<::core::ffi::c_uint>::new(0) }; 10];
    let key = addr & !3;
    let Some(i) = WATCH.iter().position(|&w| u64::from(w) == key) else {
        return;
    };
    let n = CNT[i].get().wrapping_add(1);
    CNT[i].set(n);
    if !(n == 1 || n == 10 || n == 100 || n == 10000) {
        return;
    }
    let Some(kr) = crate::src::qemu::cur_cpu_kvm_regs() else {
        return;
    };
    let mut raw = [0u8; 4 * 24];
    crate::src::qemu::cur_cpu_read_mem(kr.rsp & 0xffff_ffff, &mut raw);
    let mut stk = [0u32; 24];
    for (s, c) in stk.iter_mut().zip(raw.chunks_exact(4)) {
        *s = u32::from_ne_bytes([c[0], c[1], c[2], c[3]]);
    }
    crate::src::logs::debug(format_args!("CALLER reg=0x{:04x} read #{} rip=0x{:x} rsp=0x{:x} cr: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", WATCH[i], n, kr.rip, kr.rsp, stk[0], stk[1], stk[2], stk[3], stk[4], stk[5], stk[6], stk[7], stk[8], stk[9], stk[10], stk[11], stk[12], stk[13], stk[14], stk[15]));
}
pub fn gpu_poll_watch(addr: hwaddr, val: uint64_t) {
    static last_addr: crate::src::global::Global<hwaddr> = crate::src::global::Global::<hwaddr>::new(!(0 as ::core::ffi::c_ulonglong) as hwaddr);
    static last_val: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0);
    static run: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0);
    static n1: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(100000 as uint64_t);
    static init: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
    if !init.get() {
        init.set(true);
        if let Some(e) = crate::src::qemu::env_string(c"PPCGPU_POLL_N") {
            // strtoull(e, NULL, 0) semantics, including a leading '-' (negated, wrapping).
            let t = e.trim_start();
            let (neg, t) = match t.strip_prefix('-') {
                Some(rest) => (true, rest),
                None => (false, t),
            };
            let v = crate::src::qemu::parse_c_uint(t);
            let v = if neg { v.wrapping_neg() } else { v };
            if v != 0 {
                n1.set(v);
            }
        }
    }
    if addr == last_addr.get() && val == last_val.get() {
        run.set(run.get().wrapping_add(1));
        let r = run.get();
        let n = n1.get();
        if r == n || r == n.wrapping_mul(10) || r == n.wrapping_mul(100) {
            if let Some(kr) = crate::src::qemu::cur_cpu_kvm_regs() {
                crate::src::logs::err(format_args!("[POLL_STUCK] reg=0x{:04x} ({}) read {} times in a row, always 0x{:x}, guest rip=0x{:x}\n", ((addr) as u64), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr)), r, ((val) as u64), kr.rip));
            }
        }
    } else {
        last_addr.set(addr);
        last_val.set(val);
        run.set(1 as uint64_t);
    };
}
/// The slow-MMIO threshold in nanoseconds: `PPCGPU_SLOW_MS` milliseconds (C `strtoll` base 0:
/// decimal, `0x` hex or leading-0 octal; unparsable text reads as 0), default 5 ms.
pub fn gpu_slow_limit_ns() -> int64_t {
    static lim: crate::src::global::Global<int64_t> = crate::src::global::Global::<int64_t>::new(-1);
    if lim.get() < 0 {
        let ms: i64 = match crate::src::qemu::env_string(c"PPCGPU_SLOW_MS") {
            Some(t) => {
                let t = t.trim_start();
                let (neg, t) = match t.as_bytes().first() {
                    Some(b'-') => (true, &t[1..]),
                    Some(b'+') => (false, &t[1..]),
                    _ => (false, t),
                };
                let (radix, digits) = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                    (16, h)
                } else if t.len() > 1 && t.starts_with('0') {
                    (8, &t[1..])
                } else {
                    (10, t)
                };
                let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
                let v = i64::from_str_radix(&digits[..end], radix).unwrap_or(0);
                if neg { -v } else { v }
            }
            None => 5,
        };
        lim.set(ms.wrapping_mul(1_000_000) as int64_t);
    }
    lim.get()
}
impl PPCMacGPUFields {
    /// The renderer vtable. Callers have already checked `renderer` is non-null (the
    /// vtable is a static table that outlives the device).
    pub fn renderer_ref(&self) -> &'static PPCMacGPURenderer {
        debug_assert!(!self.renderer.is_null());
        // SAFETY: `renderer` points at a static renderer table, set at realize and never freed.
        unsafe { &*self.renderer }
    }
}
impl PPCMacGPUFields {
    /// The R300 register/state block (allocated at realize, freed at unrealize).
    pub fn r3_ref(&self) -> &'static R300State {
        debug_assert!(!self.r3.is_null());
        // SAFETY: `r3` points at the heap R300State owned by this device; the device thread
        // is the only user and holds QEMU's big lock.
        unsafe { &*self.r3 }
    }
    pub fn r3_mut(&mut self) -> &mut R300State {
        debug_assert!(!self.r3.is_null());
        // SAFETY: as r3_ref; `&mut self` makes this the only access through the device.
        unsafe { &mut *self.r3 }
    }
}
/// Dword `i` of a command packet, or 0 if the packet is shorter (guest-controlled lengths
/// must never turn into a panic or an out-of-bounds read).
pub fn dw(d: &[uint32_t], i: usize) -> uint32_t {
    d.get(i).copied().unwrap_or(0)
}
impl PPCMacGPUFields {
    /// The embedded VGA core (owned by this device, valid from realize on).
    pub fn vga_ref(&self) -> &'static VGACommonState {
        debug_assert!(!self.vga.is_null());
        // SAFETY: `vga` points at the VGACommonState inside this device's state.
        unsafe { &*self.vga }
    }
    pub fn vga_mut(&mut self) -> &mut VGACommonState {
        debug_assert!(!self.vga.is_null());
        // SAFETY: as vga_ref; `&mut self` makes this the only access through the device.
        unsafe { &mut *self.vga }
    }
}
/// Byte `i` of a buffer, or 0 past the end (for diagnostics that peek at guest data).
pub fn byte_at(b: &[uint8_t], i: usize) -> uint8_t {
    b.get(i).copied().unwrap_or(0)
}
impl R300DrawPacket {
    /// The packet's vertex array (empty if none was built).
    pub fn verts(&self) -> &[R300Vertex] {
        if self.verts.is_null() {
            return &[];
        }
        // SAFETY: when non-null, `verts` points at `num_verts` vertices allocated by the draw
        // setup for this packet and not freed until r300_draw_release.
        unsafe { core::slice::from_raw_parts(self.verts, self.num_verts as usize) }
    }
}
