use super::*;
pub fn mc_register_tiled_surface(
    _s: &mut PPCMacGPUState,
    _base: uint32_t,
    _pitch: uint32_t,
    _label: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    static no_tiling: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    if no_tiling.get() < 0 as ::core::ffi::c_int {
        no_tiling.set(if crate::src::qemu::env_first_byte(c"PPCGPU_NO_TILING") == Some(b'1') {
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        });
        if no_tiling.get() != 0 {
            crate::src::logs::err(format_args!("[MC_TILING] disabled by PPCGPU_NO_TILING=1 — all VRAM treated as linear\n"));
        }
    }
    return -(1 as ::core::ffi::c_int);
}
pub fn mc_find_tiled_surface(
    s: &mut PPCMacGPUState,
    linear_addr: uint64_t,
    surf_base: &mut uint32_t,
    surf_pitch: &mut uint32_t,
) -> ::core::ffi::c_int {
    if 0 as ::core::ffi::c_int == 0 {
        return -(1 as ::core::ffi::c_int);
    }
    // Closest surface (smallest offset from its base) that contains linear_addr; on a tie the
    // lowest slot wins. Fields: (slot, offset, base, pitch).
    let mut best: Option<(::core::ffi::c_int, uint64_t, uint32_t, uint32_t)> = None;
    for (i, surf) in s.tiled_surfaces.iter().enumerate() {
        if !surf.active {
            continue;
        }
        let end: uint32_t = surf.base.wrapping_add(surf.size);
        if linear_addr >= surf.base as uint64_t && linear_addr < end as uint64_t {
            let off: uint64_t = linear_addr.wrapping_sub(surf.base as uint64_t);
            if best.map_or(true, |(_, best_off, _, _)| off < best_off) {
                best = Some((i as ::core::ffi::c_int, off, surf.base, surf.pitch));
            }
        }
    }
    match best {
        Some((slot, _, base, pitch)) => {
            *surf_base = base;
            *surf_pitch = pitch;
            slot
        }
        None => -(1 as ::core::ffi::c_int),
    }
}
pub fn mc_rect_is_tiled(
    s: &mut PPCMacGPUState,
    offset: uint32_t,
    pitch: uint32_t,
    x: uint32_t,
    y: uint32_t,
    bpp: uint32_t,
) -> bool {
    let mut surf_base: uint32_t = 0;
    let mut surf_pitch: uint32_t = 0;
    let addr: uint64_t = (offset as uint64_t)
        .wrapping_add((y as uint64_t).wrapping_mul(pitch as uint64_t))
        .wrapping_add((x as uint64_t).wrapping_mul(bpp as uint64_t));
    return mc_find_tiled_surface(s, addr, &mut surf_base, &mut surf_pitch)
        >= 0 as ::core::ffi::c_int;
}
pub fn mc_vram_write32(
    s: &mut PPCMacGPUState,
    vram: crate::src::qemu::Vram,
    linear_addr: uint64_t,
    value: uint32_t,
) {
    let mut surf_base: uint32_t = 0;
    let mut surf_pitch: uint32_t = 0;
    let slot: ::core::ffi::c_int =
        mc_find_tiled_surface(s, linear_addr, &mut surf_base, &mut surf_pitch);
    // A zero pitch cannot describe a tiled layout; such a surface is addressed linearly.
    let phys_addr: uint64_t = if slot >= 0 as ::core::ffi::c_int && surf_pitch != 0 as uint32_t {
        let off: uint64_t = linear_addr.wrapping_sub(surf_base as uint64_t);
        let y: uint32_t = off.wrapping_div(surf_pitch as uint64_t) as uint32_t;
        let x: uint32_t = off
            .wrapping_rem(surf_pitch as uint64_t)
            .wrapping_div(4 as uint64_t) as uint32_t;
        r200_macro_tile_addr(surf_base, surf_pitch, x, y, 4 as uint32_t)
    } else {
        linear_addr
    };
    if phys_addr.wrapping_add(4 as uint64_t) <= s.vram_size {
        static watch_init: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
        static watch_max: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(50 as uint64_t);
        static watch_lo: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0x353000 as uint64_t);
        static watch_hi: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0x413000 as uint64_t);
        if !watch_init.get() {
            watch_init.set(true);
            // PPCGPU_VRAM_WATCH="max,lo,hi": comma-separated fields, each read as C strtoull(.., 0).
            if let Some(e) = crate::src::qemu::env_string(c"PPCGPU_VRAM_WATCH").filter(|e| !e.is_empty()) {
                let mut fields = e.split(',').map(crate::src::qemu::parse_c_uint);
                if let Some(max) = fields.next() {
                    watch_max.set(max as uint64_t);
                }
                if let Some(lo) = fields.next() {
                    watch_lo.set(lo as uint64_t);
                    if let Some(hi) = fields.next() {
                        watch_hi.set(hi as uint64_t);
                    }
                }
            }
        }
        if phys_addr >= watch_lo.get() && phys_addr < watch_hi.get() {
            static vram_watch_log: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0);
            static zero_write_log: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0);
            if watch_max.get() == 0 as uint64_t
                || vram_watch_log.get() < watch_max.get()
                || value == 0 as uint32_t && zero_write_log.get() < 20 as uint64_t
            {
                crate::src::logs::err(format_args!("[VRAM_WATCH] mc_vram_write32 phys=0x{:06x} linear=0x{:06x} val=0x{:08x} tiled={} src={}:{} gmc=0x{:08x} dst_po=0x{:08x} default_po=0x{:08x}\n", ((phys_addr as ::core::ffi::c_ulonglong) as u64), ((linear_addr as ::core::ffi::c_ulonglong) as u64), ((value) as u32), (((slot >= 0 as ::core::ffi::c_int) as ::core::ffi::c_int) as i32), crate::src::qemu::mc_watch_name(), ((mc_watch_line.get()) as i32), ((s.regs.dp_gui_master_cntl) as u32), ((s.regs.dst_pitch_offset) as u32), ((s.regs.default_pitch_offset) as u32)));
                vram_watch_log.set(vram_watch_log.get().wrapping_add(1));
                if value == 0 as uint32_t {
                    zero_write_log.set(zero_write_log.get().wrapping_add(1));
                }
            }
        }
        vram.put32(phys_addr, value);
    }
}
pub fn mc_vram_read32(
    s: &mut PPCMacGPUState,
    vram: crate::src::qemu::Vram,
    linear_addr: uint64_t,
) -> uint32_t {
    let mut surf_base: uint32_t = 0;
    let mut surf_pitch: uint32_t = 0;
    let slot: ::core::ffi::c_int =
        mc_find_tiled_surface(s, linear_addr, &mut surf_base, &mut surf_pitch);
    // A zero pitch cannot describe a tiled layout; such a surface is addressed linearly.
    let phys_addr: uint64_t = if slot >= 0 as ::core::ffi::c_int && surf_pitch != 0 as uint32_t {
        let off: uint64_t = linear_addr.wrapping_sub(surf_base as uint64_t);
        let y: uint32_t = off.wrapping_div(surf_pitch as uint64_t) as uint32_t;
        let x: uint32_t = off
            .wrapping_rem(surf_pitch as uint64_t)
            .wrapping_div(4 as uint64_t) as uint32_t;
        let phys_addr = r200_macro_tile_addr(surf_base, surf_pitch, x, y, 4 as uint32_t);
        static audit_base: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0 as uint32_t);
        static audit_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
        if audit_count.get() < 6 as ::core::ffi::c_int
            && surf_base >= 0x100000 as ::core::ffi::c_int as uint32_t
        {
            if audit_base.get() == 0 as uint32_t {
                audit_base.set(surf_base);
            }
            if surf_base == audit_base.get()
                && (x == 0 as uint32_t && y == 0 as uint32_t
                    || x == 63 as uint32_t && y == 0 as uint32_t
                    || x == 64 as uint32_t && y == 0 as uint32_t
                    || x == 0 as uint32_t && y == 15 as uint32_t
                    || x == 0 as uint32_t && y == 16 as uint32_t
                    || x == 100 as uint32_t && y == 20 as uint32_t)
            {
                crate::src::logs::err(format_args!("[TILE_PATH] rt_off=0x{:x} path=mc_read tiling_applied=yes formula=mc_macro x={} y={} pitch={}(bytes) linear_addr=0x{:x} resolved_addr=0x{:x} note=surf_slot={}\n", ((surf_base) as u32), ((x) as u32), ((y) as u32), ((surf_pitch) as u32), ((linear_addr as ::core::ffi::c_ulonglong) as u64), ((phys_addr as ::core::ffi::c_ulonglong) as u64), ((slot) as i32)));
                audit_count.set(audit_count.get() + 1);
            }
        }
        phys_addr
    } else {
        linear_addr
    };
    if phys_addr.wrapping_add(4 as uint64_t) <= s.vram_size {
        return vram.get32(phys_addr).unwrap_or(0);
    }
    return 0 as uint32_t;
}
