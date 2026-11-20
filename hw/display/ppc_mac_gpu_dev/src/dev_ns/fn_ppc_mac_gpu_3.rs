use super::*;
pub fn ppc_mac_gpu_ring_run(s: &mut PPCMacGPUFields) {
    let mut old_rptr: uint32_t = 0;
    let mut val: uint32_t = 0;
    if !s.ring_pending {
        return;
    }
    s.ring_pending = 0 as ::core::ffi::c_int != 0;
    old_rptr = s.regs.cp_rb_rptr;
    val = s.regs.cp_rb_wptr;
    if s.regs.cp_me_cntl & ((1 as ::core::ffi::c_int) << 28 as ::core::ffi::c_int) as uint32_t
        == 0
        && val != old_rptr
    {
        s.regs.stall_ring_dwords = s.regs.stall_ring_dwords.wrapping_add(
            (if val >= old_rptr {
                val.wrapping_sub(old_rptr)
            } else {
                val
            }) as uint64_t,
        );
        ppc_mac_gpu_process_ring_buffer(s, old_rptr, val);
    }
    s.regs.cp_rb_rptr = val;
    ppc_mac_gpu_rptr_writeback(s);
    s.regs.cp_csq_stat_counter = s.regs.cp_csq_stat_counter.wrapping_add(1);
}
pub unsafe extern "C" fn ppc_mac_gpu_ring_bh_cb(opaque: *mut ::core::ffi::c_void) {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    if let Some(mut s_guard) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) {
        let s = &mut *s_guard;
        ppc_mac_gpu_ring_run(s);
    }
}
pub fn ppc_mac_gpu_pm4_fifo_push(s: &mut PPCMacGPUFields, val: uint32_t) {
    s.regs.stall_pio_dwords = s.regs.stall_pio_dwords.wrapping_add(1);
    s.regs.csq_just_submitted = 1 as ::core::ffi::c_int != 0;
    if s.pm4_pkt_count == 0 as uint32_t {
        let fresh49 = pm4_recent_n.get();
        pm4_recent_n.set(pm4_recent_n.get().wrapping_add(1));
        let k: ::core::ffi::c_uint = fresh49 & 31 as ::core::ffi::c_uint;
        pm4_recent_mut()[k as usize].hdr = val;
        pm4_recent_mut()[k as usize].type_0 = val >> 30 as ::core::ffi::c_int & 3 as uint32_t;
        pm4_recent_mut()[k as usize].opcode = val >> 8 as ::core::ffi::c_int & 0xff as uint32_t;
        pm4_recent_mut()[k as usize].count =
            (val >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t).wrapping_add(1 as uint32_t);
    }
    if s.pm4_pkt_count == 0 as uint32_t {
        let type_0: uint32_t = val >> 30 as ::core::ffi::c_int & 3 as uint32_t;
        if type_0 == 0 as uint32_t {
            let count: uint32_t =
                (val >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t).wrapping_add(1 as uint32_t);
            let reg_base: uint32_t = val & 0x7fff as uint32_t;
            let one_reg_wr: bool = val >> 15 as ::core::ffi::c_int & 1 as uint32_t != 0;
            s.pm4_pkt_reg = reg_base;
            s.pm4_pkt_one_reg = one_reg_wr;
            s.pm4_pkt_count = count;
            s.pm4_fifo_idx = 0 as uint32_t;
            crate::src::logs::debug(format_args!("PM4_HDR type0 reg=0x{:04x} count={}{}", ((reg_base.wrapping_mul(4 as uint32_t)) as u32), ((count) as u32), crate::src::qemu::cstr(if one_reg_wr as ::core::ffi::c_int != 0 {
                    c" ONE_REG_WR".as_ptr()
                } else {
                    c"".as_ptr()
                })));
        } else if type_0 == 2 as uint32_t {
            crate::src::logs::debug(format_args!("PM4_NOP (batch complete)"));
        } else if type_0 == 3 as uint32_t {
            let count_0: uint32_t =
                (val >> 16 as ::core::ffi::c_int & 0x3fff as uint32_t).wrapping_add(1 as uint32_t);
            s.pm4_pkt_count = count_0;
            s.pm4_pkt_reg = 0xffffffff as ::core::ffi::c_uint as uint32_t;
            s.pm4_pkt_opcode = val >> 8 as ::core::ffi::c_int & 0xff as uint32_t;
            s.pm4_fifo_idx = 0 as uint32_t;
            crate::src::logs::debug(format_args!("PM4_PIO type3 opcode=0x{:02x} count={}", ((s.pm4_pkt_opcode) as u32), ((count_0) as u32)));
        } else {
            crate::src::logs::debug(format_args!("PM4_PIO type{} (ignored)", ((type_0) as u32)));
        }
    } else {
        if (s.pm4_fifo_idx as usize)
            < (::core::mem::size_of::<[uint32_t; 16384]>() as usize)
                .wrapping_div(::core::mem::size_of::<uint32_t>() as usize)
        {
            let fresh50 = s.pm4_fifo_idx;
            s.pm4_fifo_idx = s.pm4_fifo_idx.wrapping_add(1);
            s.pm4_fifo[fresh50 as usize] = val;
        }
        s.pm4_pkt_count = s.pm4_pkt_count.wrapping_sub(1);
        if s.pm4_pkt_count == 0 as uint32_t {
            if s.pm4_pkt_reg != 0xffffffff as ::core::ffi::c_uint {
                let fifo_copy = s.pm4_fifo;
                ppc_mac_gpu_pm4_process_type0(
                    s,
                    s.pm4_pkt_reg,
                    s.pm4_pkt_one_reg,
                    s.pm4_fifo_idx,
                    &fifo_copy[..],
                );
            } else {
                let opcode: uint32_t = s.pm4_pkt_opcode;
                let ndw: uint32_t = s.pm4_fifo_idx;
                if ndw > 0 as uint32_t && ndw <= 60 as uint32_t {
                    let mut pm4_buf: [uint32_t; 64] = [0; 64];
                    pm4_buf[0 as ::core::ffi::c_int as usize] = (3 as uint32_t)
                        << 30 as ::core::ffi::c_int
                        | ndw.wrapping_sub(1 as uint32_t) << 16 as ::core::ffi::c_int
                        | opcode << 8 as ::core::ffi::c_int;
                    // ndw is 1..=60 here, so both slices are in bounds and of equal length.
                    pm4_buf[1..=ndw as usize].copy_from_slice(&s.pm4_fifo[..ndw as usize]);
                    ppc_mac_gpu_process_pm4(s, &pm4_buf[..ndw.wrapping_add(1) as usize]);
                }
            }
            s.pm4_fifo_idx = 0 as uint32_t;
        }
    };
}
pub fn ppc_mac_gpu_post_reg(addr: uint32_t) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (i as usize)
        < (::core::mem::size_of::<[C2RustUnnamed_97; 72]>() as usize)
            .wrapping_div(::core::mem::size_of::<C2RustUnnamed_97>() as usize)
    {
        if ppc_mac_gpu_post_regs_mut()[i as usize].reg as ::core::ffi::c_uint
            == addr as ::core::ffi::c_uint & !(3 as ::core::ffi::c_uint)
        {
            return i;
        }
        i += 1;
    }
    return -(1 as ::core::ffi::c_int);
}
pub fn ppc_mac_gpu_post_reg_name(addr: uint32_t) -> Option<&'static ::core::ffi::CStr> {
    if addr & !3 == 0x1a4 {
        return Some(c"GPIOPAD_Y");
    }
    let i = ppc_mac_gpu_post_reg(addr);
    (i >= 0).then(|| ppc_mac_gpu_post_regs_mut()[i as usize].name)
}
pub fn ppc_mac_gpu_post_regs_reset(s: &mut PPCMacGPUFields) {
    s.mc_ind.fill(0);
    s.mc_ind_index = 0 as uint32_t;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (i as usize)
        < (::core::mem::size_of::<[C2RustUnnamed_103; 16]>() as usize)
            .wrapping_div(::core::mem::size_of::<C2RustUnnamed_103>() as usize)
    {
        s.mc_ind[ppc_mac_gpu_mc_ind_post[i as usize].idx as usize] =
            ppc_mac_gpu_mc_ind_post[i as usize].val;
        i += 1;
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (i_0 as usize)
        < (::core::mem::size_of::<[C2RustUnnamed_97; 72]>() as usize)
            .wrapping_div(::core::mem::size_of::<C2RustUnnamed_97>() as usize)
    {
        s.r300_shadow[(ppc_mac_gpu_post_regs_mut()[i_0 as usize].reg as ::core::ffi::c_int
            / 4 as ::core::ffi::c_int) as usize] = ppc_mac_gpu_post_regs_mut()[i_0 as usize].post;
        i_0 += 1;
    }
}
pub fn ppc_mac_gpu_cursor_sync(s: &mut PPCMacGPUFields) {
    let fb: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let mut off: uint32_t = s.regs.cur_offset & 0xfffffff as ::core::ffi::c_int as uint32_t;
    let on: bool = s.regs.crtc_gen_cntl as ::core::ffi::c_uint
        & (1 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
        != 0;
    let x: ::core::ffi::c_int = (s.regs.cur_horz_vert_posn >> 16 as ::core::ffi::c_int
        & 0x7fff as uint32_t) as ::core::ffi::c_int
        - (s.regs.cur_horz_vert_off >> 16 as ::core::ffi::c_int & 0x3f as uint32_t)
            as ::core::ffi::c_int;
    let y: ::core::ffi::c_int = (s.regs.cur_horz_vert_posn & 0x7fff as uint32_t)
        as ::core::ffi::c_int
        - ({
            let mut _a127: ::core::ffi::c_int = (s.regs.cur_horz_vert_off & 0x3f as uint32_t)
                as ::core::ffi::c_int
                - 25 as ::core::ffi::c_int;
            let mut _b128: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            if _a127 > _b128 {
                _a127 as ::core::ffi::c_int
            } else {
                _b128 as ::core::ffi::c_int
            }
        });
    if s.con.is_null() {
        return;
    }
    if off >= fb {
        off = off.wrapping_sub(fb);
    }
    if on as ::core::ffi::c_int != 0
        && (off as uint64_t).wrapping_add(
            (64 as ::core::ffi::c_int * 64 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                as uint64_t,
        ) <= s.vram_size
    {
        let vram = crate::src::qemu::Vram::of(s);
        let mut i: usize = 0;
        while i < 64 * 64 {
            // The 16 KiB span was checked against vram_size above, so each read is in bounds.
            s.hwc_pix[i] = vram.get32((off as u64) + 4 * (i as u64)).unwrap_or(0);
            i += 1;
        }
        if s.hwc_w == 0 {
            crate::src::qemu::console_set_blank_cursor(s);
            s.hwc_h = 64 as uint32_t;
            s.hwc_w = s.hwc_h;
        }
    }
    s.hwc_x = x as int32_t;
    s.hwc_y = y as int32_t;
    s.hwc_visible = on as ::core::ffi::c_int != 0 && s.hwc_w != 0;
    if !crate::src::qemu::getenv(c"PPCGPU_CURSOR_LOG").is_null() {
        crate::src::logs::err(format_args!("CURSOR posn {:08x} off {:08x} gen {:08x} -> top-left ({},{}) on {}\n", ((s.regs.cur_horz_vert_posn) as u32), ((s.regs.cur_horz_vert_off) as u32), ((s.regs.crtc_gen_cntl) as u32), ((x) as i32), ((y) as i32), ((on as ::core::ffi::c_int) as i32)));
    }
    crate::src::qemu::console_set_mouse_at(s, 0, 0, false);
}
pub fn ppc_mac_gpu_hwc_write(
    s: &mut PPCMacGPUFields,
    reg: uint32_t,
    val: uint32_t,
) {
    match reg {
        0 => {
            s.hwc_w = {
                let mut _a129: uint32_t = val as uint32_t & 0xffff as uint32_t;
                let mut _b130: uint32_t = 64 as uint32_t;
                if _a129 < _b130 {
                    _a129 as uint32_t
                } else {
                    _b130 as uint32_t
                }
            };
            s.hwc_h = {
                let mut _a131: uint32_t = val as uint32_t >> 16 as ::core::ffi::c_int;
                let mut _b132: uint32_t = 64 as uint32_t;
                if _a131 < _b132 {
                    _a131 as uint32_t
                } else {
                    _b132 as uint32_t
                }
            };
            s.hwc_idx = 0 as uint32_t;
        }
        4 => {
            if s.hwc_idx < s.hwc_w.wrapping_mul(s.hwc_h) {
                let fresh51 = s.hwc_idx;
                s.hwc_idx = s.hwc_idx.wrapping_add(1);
                s.hwc_pix[fresh51 as usize] = val;
            }
        }
        8 => {
            if s.hwc_w != 0 && s.hwc_h != 0 && !s.con.is_null() {
                crate::src::qemu::cursor_publish_hwc(s);
                static defined: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
                let fresh52 = defined.get();
                defined.set(defined.get() + 1);
                if fresh52 < 6 as ::core::ffi::c_int {
                    crate::src::logs::qemu(format_args!("ppc-mac-gpu: hardware cursor defined {}x{}\n", ((s.hwc_w) as u32), ((s.hwc_h) as u32)));
                }
            }
        }
        12 => {
            s.hwc_x = val as int32_t;
        }
        16 => {
            s.hwc_y = val as int32_t;
        }
        20 => {
            s.hwc_visible = val & 1 as uint32_t != 0;
            static shown: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
            let fresh53 = shown.get();
            shown.set(shown.get() + 1);
            if fresh53 < 6 as ::core::ffi::c_int {
                crate::src::logs::qemu(format_args!("ppc-mac-gpu: hardware cursor {}\n", crate::src::qemu::cstr(if s.hwc_visible as ::core::ffi::c_int != 0 {
                        c"shown".as_ptr()
                    } else {
                        c"hidden".as_ptr()
                    })));
            }
            crate::src::qemu::console_set_mouse(s);
        }
        _ => {}
    };
}
pub fn ppc_mac_gpu_hwc_read(
    s: &mut PPCMacGPUFields,
    reg: uint32_t,
) -> uint32_t {
    match reg {
        0 => return s.hwc_w | s.hwc_h << 16 as ::core::ffi::c_int,
        12 => return s.hwc_x as uint32_t,
        16 => return s.hwc_y as uint32_t,
        20 => return s.hwc_visible as uint32_t,
        24 => return 0x48574331 as ::core::ffi::c_int as uint32_t,
        _ => {}
    }
    return 0 as uint32_t;
}
pub fn ppc_mac_gpu_mm_aper_read(
    s: &mut PPCMacGPUFields,
    off: uint64_t,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    let vram = crate::src::qemu::Vram::of(s);
    let mut v: uint64_t = 0 as uint64_t;
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < size {
        if off.wrapping_add(i as uint64_t) < s.vram_size {
            v |= ((vram.get8(((off.wrapping_add(i as uint64_t)) as u64)).unwrap_or(0)) as uint64_t)
                << (8 as ::core::ffi::c_uint).wrapping_mul(i);
        }
        i = i.wrapping_add(1);
    }
    return v;
}
pub fn ppc_mac_gpu_mm_aper_write(
    s: &mut PPCMacGPUFields,
    off: uint64_t,
    val: uint64_t,
    size: ::core::ffi::c_uint,
) {
    let vram = crate::src::qemu::Vram::of(s);
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < size {
        if off.wrapping_add(i as uint64_t) < s.vram_size {
            vram.put8(((off.wrapping_add(i as uint64_t)) as u64), ((val >> (8 as ::core::ffi::c_uint).wrapping_mul(i)) as uint8_t) as u8);
            gpu_vram_dirty(s, off.wrapping_add(i as uint64_t), 1 as uint64_t);
        }
        i = i.wrapping_add(1);
    }
}
/// CRTC_STATUS: bit 0 is set in the last 5% of each 60 Hz frame (vertical blank), bit 1 once
/// the frame's blank has passed `crtc_vblank_clear_ns`.  Arithmetic wraps as the C original.
pub fn ppc_mac_gpu_crtc_status(s: &mut PPCMacGPUFields) -> uint32_t {
    if crate::src::qemu::timer_is_pending(&s.vblank_timer) {
        return s.regs.crtc_status;
    }
    let now: int64_t = crate::src::qemu::clock_virtual_ns();
    let frame: int64_t = 1_000_000_000 / 60;
    let start: int64_t = frame.wrapping_sub(frame / 20);
    let mut v: uint32_t = s.regs.crtc_status & !0b11;
    if now % frame >= start {
        v |= 1;
    }
    if now >= start
        && now
            .wrapping_sub(start)
            .wrapping_div(frame)
            .wrapping_mul(frame)
            .wrapping_add(start)
            > s.regs.crtc_vblank_clear_ns
    {
        v |= 1 << 1;
    }
    v
}
pub fn ppc_mac_gpu_mm_reg_index(s: & PPCMacGPUFields) -> uint32_t {
    let idx: uint32_t = s.regs.mm_index & 0xffff as uint32_t;
    return if idx >= 0x4 as uint32_t
        && idx < (0x4 as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as uint32_t
    {
        0 as uint32_t
    } else {
        idx
    };
}
pub fn ppc_mac_gpu_gpio_ddc(mut reg: uint32_t) -> uint32_t {
    if reg & 0x30000 as ::core::ffi::c_int as uint32_t == 0 && reg & 0x300 as uint32_t == 0 {
        reg |= 0x300 as uint32_t;
    }
    return reg;
}
pub fn ppc_mac_gpu_mmio_read_cfg(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    _size: ::core::ffi::c_uint,
    valp: &mut uint64_t,
) -> bool {
    let mut val: uint64_t = 0 as uint64_t;
    match addr {
        0 => {
            val = s.regs.mm_index as uint64_t;
        }
        16..=44 => {
            val = (s.regs.bios_scratch
                [addr.wrapping_sub(0x10 as hwaddr).wrapping_div(4 as hwaddr) as usize]
                >> (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr)) as uint64_t;
        }
        64 => {
            val = s.regs.gen_int_cntl as uint64_t;
        }
        68 => {
            val = s.regs.gen_int_status as uint64_t;
        }
        248 => {
            val = s.regs.config_memsize as uint64_t;
        }
        256 => {
            val = (crate::src::qemu::pci_config_read(
                s.pci_dev(),
                0x10 as uint32_t,
                4 as ::core::ffi::c_int,
            ) as ::core::ffi::c_uint
                & !(0xf as ::core::ffi::c_uint)) as uint64_t;
        }
        260 => {
            val = (crate::src::qemu::pci_config_read(
                s.pci_dev(),
                0x10 as uint32_t,
                4 as ::core::ffi::c_int,
            ) & !(0xf as uint32_t))
                .wrapping_add(s.vram_size as uint32_t) as uint64_t;
        }
        264 => {
            val = s.vram_size as uint32_t as uint64_t;
        }
        268 => {
            val = (crate::src::qemu::pci_config_read(
                s.pci_dev(),
                0x18 as uint32_t,
                4 as ::core::ffi::c_int,
            ) as ::core::ffi::c_uint
                & !(0xf as ::core::ffi::c_uint)) as uint64_t;
        }
        272 => {
            val = 0x10000 as uint64_t;
        }
        224 => {
            val = s.regs.config_cntl as uint64_t;
        }
        328 => {
            val = s.regs.mc_fb_location as uint64_t;
        }
        332 => {
            val = s.regs.mc_agp_location as uint64_t;
        }
        336 => {
            val = (((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << 4 as ::core::ffi::c_int
                | 0xf as ::core::ffi::c_uint) as uint64_t;
        }
        48 => {
            val = s.regs.bus_cntl as uint64_t;
        }
        304 => {
            val = (s.regs.host_path_cntl
                | ((1 as ::core::ffi::c_int) << 23 as ::core::ffi::c_int) as uint32_t)
                as uint64_t;
        }
        3648 => {
            val = 64 as uint64_t;
            crate::src::logs::debug(format_args!("STATUS_RD RBBM_STATUS -> 0x{:08x} (idle, {} free)", ((val as uint32_t) as u32), ((val as uint32_t & 0x7f as uint32_t) as u32)));
            static rbbm_read_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
            rbbm_read_count.set(rbbm_read_count.get() + 1);
            if gpu_diag_on() as ::core::ffi::c_int != 0
                && (rbbm_read_count.get() <= 20 as ::core::ffi::c_int
                    || rbbm_read_count.get() % 1000 as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
            {
                crate::src::logs::qemu(format_args!("[QE_GATE_READ] reg=0x{:04x} value=0x{:08x} context=idle_poll sequence_id={} note=RBBM_STATUS_always_idle\n", ((addr as uint32_t) as u32), ((val as uint32_t) as u32), ((rbbm_read_count.get()) as i32)));
            }
        }
        5952 => {
            val = 64 as uint64_t;
            crate::src::logs::debug(format_args!("STATUS_RD GUI_STAT -> 0x{:08x}", ((val as uint32_t) as u32)));
        }
        16408 => {
            val = 0x1 as uint64_t;
        }
        16420 => {
            val = 0x20 as uint64_t;
        }
        16392 => {
            val = 0 as uint64_t;
        }
        16400 | 16404 => {
            val = 0x66666666 as uint64_t;
        }
        16412 | 16416 => {
            val = 0 as uint64_t;
        }
        8512 => {
            val = 0 as uint64_t;
        }
        8528 => {
            val = 0 as uint64_t;
        }
        5900 => {
            val = 0 as uint64_t;
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    *valp = val;
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_read_display(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    _size: ::core::ffi::c_uint,
    valp: &mut uint64_t,
) -> bool {
    let mut val: uint64_t = 0 as uint64_t;
    match addr {
        80 => {
            val = s.regs.crtc_gen_cntl as uint64_t;
        }
        84 => {
            val = s.regs.crtc_ext_cntl as uint64_t;
        }
        92 => {
            val = ppc_mac_gpu_crtc_status(s) as uint64_t;
        }
        512 => {
            val = s.regs.crtc_h_total_disp as uint64_t;
        }
        516 => {
            val = s.regs.crtc_h_sync_strt_wid as uint64_t;
        }
        520 => {
            val = s.regs.crtc_v_total_disp as uint64_t;
        }
        524 => {
            val = s.regs.crtc_v_sync_strt_wid as uint64_t;
        }
        548 => {
            let now: int64_t = crate::src::qemu::clock_virtual_ns();
            let frame: int64_t =
                (1000000000 as ::core::ffi::c_longlong / 60 as ::core::ffi::c_longlong) as int64_t;
            let pending: bool = s.regs.crtc_offset_write_ns > 0 as int64_t
                && (now / frame == s.regs.crtc_offset_write_ns / frame
                    || s.regs.crtc_offset_locked as ::core::ffi::c_int != 0
                        && now - s.regs.crtc_offset_write_ns < 2 as int64_t * frame);
            val = (s.regs.crtc_offset as ::core::ffi::c_uint
                | (if pending as ::core::ffi::c_int != 0 {
                    (1 as ::core::ffi::c_uint) << 30 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_uint
                })) as uint64_t;
        }
        552 => {
            val = s.regs.crtc_offset_cntl as uint64_t;
            if 1 as ::core::ffi::c_int != 0
                && crate::src::qemu::getenv(c"PPCGPU_KEEP_TRIG")
                    .is_null()
            {
                val &= !((1 as ::core::ffi::c_uint) << 30 as ::core::ffi::c_int) as uint64_t;
            }
        }
        556 => {
            val = s.regs.crtc_pitch as uint64_t;
        }
        536 => {
            val = 0 as uint64_t;
        }
        528 => {
            let mut vtotal: uint32_t =
                (s.regs.crtc_v_total_disp & 0x7ff as uint32_t).wrapping_add(1 as uint32_t);
            if vtotal == 0 as uint32_t {
                vtotal = 628 as uint32_t;
            }
            let now_ns: int64_t = crate::src::qemu::clock_virtual_ns();
            let frame_ns: uint64_t =
                (1000000000 as ::core::ffi::c_longlong / 60 as ::core::ffi::c_longlong) as uint64_t;
            let line: uint32_t = (now_ns as uint64_t)
                .wrapping_rem(frame_ns)
                .wrapping_mul(vtotal as uint64_t)
                .wrapping_div(frame_ns) as uint32_t;
            val = (s.regs.crtc_vline & 0x7ff as uint32_t
                | (line & 0x7ff as uint32_t) << 16 as ::core::ffi::c_int)
                as uint64_t;
        }
        532 => {
            val = (crate::src::qemu::clock_virtual_ns() as ::core::ffi::c_longlong
                / (1000000000 as ::core::ffi::c_longlong / 60 as ::core::ffi::c_longlong))
                as uint32_t as uint64_t;
        }
        88 => {
            val = s.regs.dac_cntl as uint64_t;
            if val & ((1 as ::core::ffi::c_uint) << 3 as ::core::ffi::c_int) as uint64_t != 0 {
                val |= ((1 as ::core::ffi::c_uint) << 7 as ::core::ffi::c_int) as uint64_t;
            }
        }
        124 => {
            val = s.regs.dac_cntl2 as uint64_t;
        }
        176 => {
            val = s.regs.palette_index as uint64_t;
        }
        180 => {
            if s.regs.palette_index < 256 as uint32_t {
                val = gpu_dac_get(s, s.regs.palette_index as ::core::ffi::c_uint) as uint64_t;
            }
        }
        184 => {
            if s.regs.palette_index < 256 as uint32_t {
                let c: uint32_t =
                    gpu_dac_get(s, s.regs.palette_index as ::core::ffi::c_uint);
                let r: uint32_t = c >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
                let g: uint32_t = c >> 8 as ::core::ffi::c_int & 0xff as uint32_t;
                let b: uint32_t = c & 0xff as uint32_t;
                val = ((r << 2 as ::core::ffi::c_int | r >> 6 as ::core::ffi::c_int)
                    << 20 as ::core::ffi::c_int
                    | (g << 2 as ::core::ffi::c_int | g >> 6 as ::core::ffi::c_int)
                        << 10 as ::core::ffi::c_int
                    | (b << 2 as ::core::ffi::c_int | b >> 6 as ::core::ffi::c_int))
                    as uint64_t;
                s.regs.palette_index =
                    s.regs.palette_index.wrapping_add(1 as uint32_t) & 0xff as uint32_t;
            }
        }
        608 => {
            val = s.regs.cur_offset as uint64_t;
        }
        612 => {
            val = s.regs.cur_horz_vert_posn as uint64_t;
        }
        616 => {
            val = s.regs.cur_horz_vert_off as uint64_t;
        }
        620 => {
            val = s.regs.cur_clr0 as uint64_t;
        }
        624 => {
            val = s.regs.cur_clr1 as uint64_t;
        }
        2816 => {
            val = s.regs.surface_cntl as uint64_t;
        }
        3328 => {
            val = s.regs.disp_misc_cntl as uint64_t;
        }
        3428 => {
            val = s.regs.disp_output_cntl as uint64_t;
        }
        3424 => {
            val = s.regs.disp_merge_cntl as uint64_t;
        }
        644 => {
            val = s.regs.fp_gen_cntl as uint64_t;
        }
        3932 => {
            val = 0x1f000203 as uint64_t;
        }
        3936 => {
            val = s.regs.agp_command as uint64_t;
        }
        368 => {
            val = s.regs.agp_base as uint64_t;
        }
        372 => {
            val = s.regs.agp_cntl as uint64_t;
        }
        8 => {
            val = s.regs.clock_cntl_index as uint64_t;
        }
        12 => {
            val =
                s.regs.pll[(s.regs.clock_cntl_index & 0x3f as uint32_t) as usize] as uint64_t;
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    *valp = val;
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_read_2d(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    _size: ::core::ffi::c_uint,
    valp: &mut uint64_t,
) -> bool {
    let mut val: uint64_t = 0 as uint64_t;
    match addr {
        5124 => {
            val = s.regs.dst_offset as uint64_t;
        }
        5128 => {
            val = s.regs.dst_pitch as uint64_t;
        }
        5136 => {
            val = s.regs.dst_height as uint64_t;
        }
        5236 => {
            val = s.regs.brush_y_x as uint64_t;
        }
        5548 => {
            val = s.regs.src_offset as uint64_t;
        }
        5552 => {
            val = s.regs.src_pitch as uint64_t;
        }
        5228 => {
            val = s.regs.dp_gui_master_cntl as uint64_t;
        }
        5824 => {
            val = s.regs.dp_cntl as uint64_t;
        }
        5828 => {
            val = s.regs.dp_datatype as uint64_t;
        }
        5832 => {
            val = s.regs.dp_mix as uint64_t;
        }
        5836 => {
            val = s.regs.dp_write_msk as uint64_t;
        }
        5868 => {
            val = s.regs.sc_top_left as uint64_t;
        }
        5872 => {
            val = s.regs.sc_bottom_right as uint64_t;
        }
        5864 => {
            val = s.regs.default_sc_bottom_right as uint64_t;
        }
        1792 => {
            val = s.regs.cp_rb_base as uint64_t;
        }
        1796 => {
            val = s.regs.cp_rb_cntl as uint64_t;
        }
        1808 => {
            val = s.regs.cp_rb_rptr as uint64_t;
        }
        1812 => {
            val = s.regs.cp_rb_wptr as uint64_t;
        }
        1804 => {
            val = s.regs.cp_rb_rptr_addr as uint64_t;
        }
        134 => {
            val = s.regs.cp_me_cntl as uint64_t;
        }
        1612 => {
            val = 0 as uint64_t;
            crate::src::logs::debug(format_args!("STATUS_RD CP_STAT -> 0x{:08x} (idle)", ((val as uint32_t) as u32)));
        }
        1848 => {
            val = s.regs.cp_ib_base as uint64_t;
        }
        1852 => {
            val = s.regs.cp_ib_bufsz as uint64_t;
        }
        1856 => {
            static mode: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
            if mode.get() < 0 as ::core::ffi::c_int {
                mode.set(match crate::src::qemu::env_string(c"PPCGPU_CSQ").as_deref() {
                    Some("busy") => 1 as ::core::ffi::c_int,
                    Some("idle") => 2 as ::core::ffi::c_int,
                    _ => 0 as ::core::ffi::c_int,
                });
            }
            let top: uint32_t = s.regs.cp_csq_cntl & 0xff000000 as uint32_t;
            val = (if mode.get() == 2 as ::core::ffi::c_int {
                top
            } else {
                top | 0x40 as uint32_t
            }) as uint64_t;
        }
        1904 => {
            val = s.regs.scratch_umsk as uint64_t;
        }
        1908 => {
            val = s.regs.scratch_addr as uint64_t;
        }
        2004 => {
            val = s.regs.cp_me_ram_addr as uint64_t;
        }
        2040 => {
            val = 0 as uint64_t;
            if crate::src::qemu::getenv(c"PPCGPU_NO_CSQ_MOVE").is_null()
            {
                let fresh47 = s.regs.cp_csq_stat_counter;
                s.regs.cp_csq_stat_counter = s.regs.cp_csq_stat_counter.wrapping_add(1);
                let p: uint32_t = fresh47 & 0xff as uint32_t;
                val = (p
                    | p << 8 as ::core::ffi::c_int
                    | p << 16 as ::core::ffi::c_int
                    | p << 24 as ::core::ffi::c_int) as uint64_t;
            }
        }
        2044 => {
            val = 0 as uint64_t;
            if crate::src::qemu::getenv(c"PPCGPU_NO_CSQ_MOVE").is_null()
            {
                let fresh48 = s.regs.cp_csq_stat_counter;
                s.regs.cp_csq_stat_counter = s.regs.cp_csq_stat_counter.wrapping_add(1);
                let p_0: uint32_t = fresh48 & 0xff as uint32_t;
                val = (p_0 | p_0 << 8 as ::core::ffi::c_int) as uint64_t;
            }
        }
        2032 => {
            val = 0 as uint64_t;
        }
        2036 => {
            val = 0 as uint64_t;
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    *valp = val;
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_read_cp(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
    valp: &mut uint64_t,
) -> bool {
    let mut val: uint64_t = 0 as uint64_t;
    match addr {
        96..=99 => {
            val = ppc_mac_gpu_reg_read_offs(
                ppc_mac_gpu_gpio_ddc(s.regs.gpio_vga_ddc),
                addr.wrapping_sub(0x60 as hwaddr) as ::core::ffi::c_int,
                size,
            );
        }
        100..=103 => {
            val = ppc_mac_gpu_reg_read_offs(
                ppc_mac_gpu_gpio_ddc(s.regs.gpio_dvi_ddc),
                addr.wrapping_sub(0x64 as hwaddr) as ::core::ffi::c_int,
                size,
            );
        }
        104..=107 => {
            val = ppc_mac_gpu_reg_read_offs(
                ppc_mac_gpu_gpio_ddc(s.regs.gpio_monid),
                addr.wrapping_sub(0x68 as hwaddr) as ::core::ffi::c_int,
                size,
            );
        }
        5920 => {
            val = 0 as uint64_t;
        }
        5924 => {
            val = s.regs.isync_cntl as uint64_t;
        }
        240 => {
            val = 0 as uint64_t;
        }
        3336 => {
            val = 0 as uint64_t;
        }
        5600..=5620 => {
            let scratch_idx: ::core::ffi::c_int =
                addr.wrapping_sub(0x15e0 as hwaddr)
                    .wrapping_div(4 as hwaddr) as ::core::ffi::c_int;
            val = s.regs.scratch_reg[scratch_idx as usize] as uint64_t;
            if !crate::src::qemu::getenv(c"POWEREMU_FENCE_TRACE")
                .is_null()
            {
                static last: [crate::src::global::Global<uint32_t>; 6] = [
                    crate::src::global::Global::<uint32_t>::new(0xffffffff as ::core::ffi::c_uint),
                    crate::src::global::Global::<uint32_t>::new(0),
                    crate::src::global::Global::<uint32_t>::new(0),
                    crate::src::global::Global::<uint32_t>::new(0),
                    crate::src::global::Global::<uint32_t>::new(0),
                    crate::src::global::Global::<uint32_t>::new(0),
                ];
                static n: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
                if let Some(slot) = last.get(scratch_idx as usize) {
                    if val != slot.get() as uint64_t && {
                        let fresh46 = n.get();
                        n.set(n.get() + 1);
                        fresh46 < 40 as ::core::ffi::c_int
                    } {
                        slot.set(val as uint32_t);
                        crate::src::logs::err(format_args!("ppc-mac-gpu fence: guest reads reg{} = {}\n", ((scratch_idx) as i32), ((val as uint32_t) as u32)));
                    }
                }
            }
            crate::src::logs::debug(format_args!("STATUS_RD SCRATCH_REG{} -> 0x{:08x}", ((scratch_idx) as i32), ((val as uint32_t) as u32)));
            static scratch_read_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
            scratch_read_count.set(scratch_read_count.get() + 1);
            if gpu_diag_on() as ::core::ffi::c_int != 0
                && (scratch_read_count.get() <= 50 as ::core::ffi::c_int
                    || scratch_read_count.get() % 500 as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
            {
                crate::src::logs::qemu(format_args!("[QE_GATE_READ] reg=0x{:04x} value=0x{:08x} context=scratch_poll sequence_id={} note=SCRATCH_REG{}_fence_check\n", ((addr as uint32_t) as u32), ((val as uint32_t) as u32), ((scratch_read_count.get()) as i32), ((scratch_idx) as i32)));
            }
        }
        808 | 572 | 828 | 5904 => {
            val = 0 as uint64_t;
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    *valp = val;
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_read_stub_3d(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
    valp: &mut uint64_t,
) -> bool {
    let mut val: uint64_t = 0 as uint64_t;
    match addr {
        7228 | 7244 | 12888 => {
            let idx: uint32_t = (addr as uint32_t)
                .wrapping_sub(0x1c00 as uint32_t)
                .wrapping_div(4 as uint32_t);
            if idx < 2304 as uint32_t {
                val = s.regs.regs_3d[idx as usize] as uint64_t;
            }
        }
        12892 => {
            let idx_0: uint32_t = (addr as uint32_t)
                .wrapping_sub(0x1c00 as uint32_t)
                .wrapping_div(4 as uint32_t);
            if idx_0 < 2304 as uint32_t {
                let shadow: uint32_t = s.regs.regs_3d[idx_0 as usize];
                if shadow & 0x3 as uint32_t != 0 {
                    crate::src::logs::debug(format_args!("STATUS_RD RB3D_DSTCACHE_CTLSTAT -> 0 (was 0x{:08x}, flush complete)", ((shadow) as u32)));
                }
                s.regs.regs_3d[idx_0 as usize] = 0 as uint32_t;
            }
            val = 0 as uint64_t;
        }
        464 => {
            val = s.regs.aic_ctrl as uint64_t;
        }
        472 => {
            val = s.regs.aic_pt_base as uint64_t;
        }
        476 => {
            val = s.regs.aic_lo_addr as uint64_t;
        }
        480 => {
            val = s.regs.aic_hi_addr as uint64_t;
        }
        _ => {
            if addr == 0x1fb4 as hwaddr && !crate::src::qemu::env_flag(c"PPCGPU_NO_7B0") {
                val = 0x80000000 as uint64_t;
            } else if addr >= 0x1c00 as hwaddr && addr < 0x4000 as hwaddr {
                let idx_1: uint32_t = (addr as uint32_t)
                    .wrapping_sub(0x1c00 as uint32_t)
                    .wrapping_div(4 as uint32_t);
                if idx_1 < 2304 as uint32_t {
                    val = s.regs.regs_3d[idx_1 as usize] as uint64_t;
                }
            } else if addr >= 0xf00 as hwaddr && addr < 0xf40 as hwaddr {
                let pci_off: uint32_t = addr.wrapping_sub(0xf00 as hwaddr) as uint32_t;
                val = crate::src::qemu::pci_config_read(
                    s.pci_dev(),
                    pci_off,
                    size as ::core::ffi::c_int,
                ) as uint64_t;
            } else if addr >= 0x500 as hwaddr && addr < 0x516 as hwaddr {
                let vbe_idx: uint32_t =
                    (addr.wrapping_sub(0x500 as hwaddr) >> 1 as ::core::ffi::c_int) as uint32_t;
                if vbe_idx < 0xa as uint32_t || vbe_idx == 0xa as uint32_t {
                    val = (if vbe_idx == 0xa as uint32_t {
                        (s.vram_size >> 16 as ::core::ffi::c_int) as uint16_t
                            as ::core::ffi::c_int
                    } else {
                        s.regs.vbe_regs[vbe_idx as usize] as ::core::ffi::c_int
                    }) as uint64_t;
                    const VBE_NAMES: [&::core::ffi::CStr; 11] = [
                        c"ID",
                        c"XRES",
                        c"YRES",
                        c"BPP",
                        c"ENABLE",
                        c"BANK",
                        c"VIRT_WIDTH",
                        c"VIRT_HEIGHT",
                        c"X_OFF",
                        c"Y_OFF",
                        c"VMEM_64K",
                    ];
                    let name: &::core::ffi::CStr = VBE_NAMES.get(vbe_idx as usize).copied().unwrap_or(c"?");
                    crate::src::logs::debug(format_args!("VBE_RD [{}] {} -> 0x{:04x}", ((vbe_idx) as u32), crate::src::qemu::cs(name), ((val as uint32_t) as u32)));
                }
            } else if addr >= 0x400 as hwaddr && addr < 0x420 as hwaddr
                || addr >= 0x3c0 as hwaddr && addr < 0x3e0 as hwaddr
            {
                let port: uint16_t = (if addr >= 0x3c0 as hwaddr && addr < 0x3e0 as hwaddr {
                    addr
                } else {
                    addr.wrapping_sub(0x400 as hwaddr)
                        .wrapping_add(0x3c0 as hwaddr)
                }) as uint16_t;
                match port as ::core::ffi::c_int {
                    960 => {
                        val = s.regs.vga_ar_index as uint64_t;
                    }
                    967 => {
                        val = 3 as uint64_t;
                    }
                    968 => {
                        val = s.regs.vga_dac_write_index as uint64_t;
                    }
                    969 => {
                        let idx_2: uint8_t = s.regs.vga_dac_read_index;
                        val = s.regs.vga_dac_palette[idx_2 as usize]
                            [s.regs.vga_dac_sub_index as usize]
                            as uint64_t;
                        s.regs.vga_dac_sub_index = s.regs.vga_dac_sub_index.wrapping_add(1);
                        if s.regs.vga_dac_sub_index as ::core::ffi::c_int
                            >= 3 as ::core::ffi::c_int
                        {
                            s.regs.vga_dac_sub_index = 0 as uint8_t;
                            s.regs.vga_dac_read_index =
                                s.regs.vga_dac_read_index.wrapping_add(1);
                        }
                    }
                    _ => {
                        val = 0 as uint64_t;
                    }
                }
            } else if addr >= 0x600 as hwaddr && addr < 0x620 as hwaddr {
                let ext_reg: uint32_t =
                    (addr.wrapping_sub(0x600 as hwaddr) >> 2 as ::core::ffi::c_int) as uint32_t;
                if ext_reg == 2 as uint32_t {
                    val = (s.regs.gen_int_status
                        & ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint32_t)
                        as uint64_t;
                }
            } else if addr == 0xb08 as hwaddr {
                val = s.r300_surf[2 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb18 as hwaddr {
                val = s.r300_surf[6 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb28 as hwaddr {
                val = s.r300_surf[10 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb38 as hwaddr {
                val = s.r300_surf[14 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb48 as hwaddr {
                val = s.r300_surf[18 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb58 as hwaddr {
                val = s.r300_surf[22 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb68 as hwaddr {
                val = s.r300_surf[26 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr == 0xb78 as hwaddr {
                val = s.r300_surf[30 as ::core::ffi::c_int as usize] as uint64_t;
            } else if addr < 0x5000 as hwaddr
                && is_r300_register(addr as uint32_t) as ::core::ffi::c_int != 0
            {
                val = s.r300_shadow[(addr as uint32_t).wrapping_div(4 as uint32_t) as usize]
                    as uint64_t;
            } else if addr as ::core::ffi::c_ulonglong & !(3 as ::core::ffi::c_ulonglong)
                == 0xe4 as ::core::ffi::c_ulonglong
                || addr as ::core::ffi::c_ulonglong & !(3 as ::core::ffi::c_ulonglong)
                    == 0xe8 as ::core::ffi::c_ulonglong
            {
                let xstrap: bool = addr as ::core::ffi::c_ulonglong
                    & !(3 as ::core::ffi::c_ulonglong)
                    == 0xe4 as ::core::ffi::c_ulonglong;
                let e: Option<String> = crate::src::qemu::env_string(if xstrap {
                    c"PPCGPU_XSTRAP"
                } else {
                    c"PPCGPU_BONDS"
                });
                let dflt: uint32_t = if xstrap { 0 } else { 0x1 };
                let bits: u64 = match e {
                    Some(text) => crate::src::qemu::parse_c_uint(&text),
                    None => dflt as u64,
                };
                val = (bits >> (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr)) as uint64_t;
            } else if addr == 0x1a4 as hwaddr {
                val = 0 as uint64_t;
            } else if ppc_mac_gpu_post_reg(addr as uint32_t) >= 0 as ::core::ffi::c_int {
                val = (s.r300_shadow[((addr as ::core::ffi::c_ulonglong
                    & !(3 as ::core::ffi::c_ulonglong))
                    as uint32_t)
                    .wrapping_div(4 as uint32_t) as usize]
                    >> (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr))
                    as uint64_t;
                if size < 4 as ::core::ffi::c_uint {
                    val &= ((1 as ::core::ffi::c_uint)
                        << size.wrapping_mul(8 as ::core::ffi::c_uint))
                    .wrapping_sub(1 as ::core::ffi::c_uint) as uint64_t;
                }
            } else if addr == 0x1f8 as hwaddr {
                val = s.mc_ind_index as uint64_t;
            } else if addr == 0x1fc as hwaddr {
                val = s.mc_ind[(s.mc_ind_index & 0x7f as uint32_t) as usize] as uint64_t;
            } else if size < 4 as ::core::ffi::c_uint || addr & 3 as hwaddr != 0 {
                val = ppc_mac_gpu_mmio_read(
                    s,
                    (addr as ::core::ffi::c_ulonglong & !(3 as ::core::ffi::c_ulonglong)) as hwaddr,
                    4 as ::core::ffi::c_uint,
                ) >> (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr);
                if size < 4 as ::core::ffi::c_uint {
                    val &= ((1 as ::core::ffi::c_uint)
                        << size.wrapping_mul(8 as ::core::ffi::c_uint))
                    .wrapping_sub(1 as ::core::ffi::c_uint) as uint64_t;
                }
            } else if addr == 0x7b0 as hwaddr && !crate::src::qemu::env_flag(c"PPCGPU_NO_7B0") {
                val = 8 as uint64_t;
            } else {
                // Scripted answers for two polled registers, unless PPCGPU_NO_7B0 is set.
                let scripted = !crate::src::qemu::env_flag(c"PPCGPU_NO_7B0")
                    && match addr {
                        0x790 => {
                            static w790: crate::src::global::Global<::core::ffi::c_uint> = crate::src::global::Global::<::core::ffi::c_uint>::new(0);
                            let fresh45 = w790.get();
                            w790.set(w790.get().wrapping_add(1));
                            val = (if fresh45 & 1 as ::core::ffi::c_uint != 0 {
                                8 as ::core::ffi::c_int
                            } else {
                                2 as ::core::ffi::c_int
                            }) as uint64_t;
                            true
                        }
                        0xa10 => {
                            val = 0x3333 as uint64_t;
                            true
                        }
                        _ => false,
                    };
                if !scripted {
                    crate::src::qemu::trace_mmio_read_unimpl(
                        size,
                        addr as uint64_t,
                        ppc_mac_gpu_reg_name(addr),
                    );
                    static reads: std::sync::Mutex<[uint32_t; 16384]> =
                        std::sync::Mutex::new([0; 16384]);
                    let slot: usize = (addr as uint32_t).wrapping_div(4 as uint32_t) as usize;
                    if slot < 16384 {
                        let n: uint32_t = {
                            let mut g = reads.lock().unwrap_or_else(|e| e.into_inner());
                            g[slot] = g[slot].wrapping_add(1);
                            g[slot]
                        };
                        if n == 1 as uint32_t
                            || n == 10 as uint32_t
                            || n == 100 as uint32_t
                            || n == 1000 as uint32_t
                            || n == 10000 as uint32_t
                        {
                            crate::src::logs::qemu(format_args!("[QE_GATE_READ] reg=0x{:04x} size={} read #{} context=unhandled_read note=UNIMPL_{}_returns_zero\n", ((addr as uint32_t) as u32), ((size) as u32), ((n) as u32), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr))));
                        }
                    }
                    val = 0 as uint64_t;
                }
            }
        }
    }
    *valp = val;
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_read_impl(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    let mut val: uint64_t = 0 as uint64_t;
    if !crate::src::qemu::getenv(c"POWEREMU_STALL_TRACE").is_null() {
        static RING: std::sync::Mutex<[C2RustUnnamed_101; 256]> = std::sync::Mutex::new([C2RustUnnamed_101 {
            addr: 0,
            val: 0,
            wr: false,
        }; 256]);
        static n: crate::src::global::Global<::core::ffi::c_uint> = crate::src::global::Global::<::core::ffi::c_uint>::new(0);
        static last_draw_seen: crate::src::global::Global<int64_t> = crate::src::global::Global::<int64_t>::new(0);
        static dumped: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
        let now: int64_t = crate::src::qemu::clock_realtime_ms();
        RING.lock().unwrap_or_else(|e| e.into_inner())[(n.get() & 255) as usize] = C2RustUnnamed_101 {
            addr: addr as uint32_t,
            val: 0,
            wr: false,
        };
        n.set(n.get().wrapping_add(1));
        if s.regs.stall_draws != s.regs.stall_draws_seen || last_draw_seen.get() == 0 {
            s.regs.stall_draws_seen = s.regs.stall_draws;
            last_draw_seen.set(now);
            dumped.set(0 as ::core::ffi::c_int != 0);
        }
        if !dumped.get() && last_draw_seen.get() != 0 && now - last_draw_seen.get() > 4000 as int64_t {
            dumped.set(1 as ::core::ffi::c_int != 0);
            crate::src::logs::err(format_args!("ppc-mac-gpu stalled: no drawing for 4 s.\n  fences: reg0={} reg1={} reg2={} reg3={} (copy at 0x{:08x}, enabled 0x{:x})\n  ring: rptr={} wptr={}, copy kept at 0x{:08x}; commands by hand: {} dwords, through the ring: {}\n  buffers fetched from memory: {} carried out ({} dwords), {} not found\n  interrupts: asked for 0x{:08x}, pending 0x{:08x}, {} raised\n  the fence copy lands in {}\n  the last {} registers the guest touched:\n", ((s.regs.scratch_reg[0 as ::core::ffi::c_int as usize]) as u32), ((s.regs.scratch_reg[1 as ::core::ffi::c_int as usize]) as u32), ((s.regs.scratch_reg[2 as ::core::ffi::c_int as usize]) as u32), ((s.regs.scratch_reg[3 as ::core::ffi::c_int as usize]) as u32), ((s.regs.scratch_addr) as u32), ((s.regs.scratch_umsk) as u32), ((s.regs.cp_rb_rptr) as u32), ((s.regs.cp_rb_wptr) as u32), ((s.regs.cp_rb_rptr_addr) as u32), ((s.regs.stall_pio_dwords as ::core::ffi::c_ulonglong) as u64), ((s.regs.stall_ring_dwords as ::core::ffi::c_ulonglong) as u64), ((s.regs.stall_ib_done as ::core::ffi::c_ulonglong) as u64), ((s.regs.stall_ib_dwords as ::core::ffi::c_ulonglong) as u64), ((s.regs.stall_ib_lost as ::core::ffi::c_ulonglong) as u64), ((s.regs.gen_int_cntl) as u32), ((s.regs.gen_int_status) as u32), ((s.regs.stall_irqs as ::core::ffi::c_ulonglong) as u64), crate::src::qemu::cstr({
                    let mut ph: hwaddr = 0;
                    if { let hz0 = s.regs.scratch_addr; ppc_mac_gpu_gart_translate(&mut *s, hz0, &mut ph) }
                        as ::core::ffi::c_int != 0
                    {
                        c"memory the guest shares (through GART)".as_ptr()
                    } else if { let hz0 = s.regs.scratch_addr; ppc_mac_gpu_agp_translate(
                        &mut *s,
                        hz0,
                        &mut ph,
                    ) } as ::core::ffi::c_int != 0
                    {
                        c"memory the guest shares (through AGP)".as_ptr()
                    } else if s.regs.scratch_addr.wrapping_add(4 as uint32_t)
                        as uint64_t <= s.vram_size
                    {
                        c"the card's own memory -- the guest cannot see it there".as_ptr()
                    } else {
                        c"nowhere at all".as_ptr()
                    }
                }), ((256 as ::core::ffi::c_uint) as u32)));
            let mut ph: hwaddr = 0;
            if { let hz0 = s.regs.scratch_addr; ppc_mac_gpu_gart_translate(&mut *s, hz0, &mut ph) }
                as ::core::ffi::c_int
                != 0
                || { let hz0 = s.regs.scratch_addr; ppc_mac_gpu_agp_translate(&mut *s, hz0, &mut ph) }
                    as ::core::ffi::c_int
                    != 0
            {
                let mut page: [uint32_t; 16] = [0; 16];
                crate::src::qemu::guest_read_dwords(ph as u64, &mut page);
                crate::src::logs::err(format_args!("  that page (guest address 0x{:x}), as words, each shown both ways round:\n", ((ph) as u64)));
                let mut w: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while w < 16 as ::core::ffi::c_int {
                    crate::src::logs::err(format_args!("    +{:02x}:", ((w * 4 as ::core::ffi::c_int) as u32)));
                    let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while k < 4 as ::core::ffi::c_int {
                        let raw: uint32_t = page[(w + k) as usize];
                        crate::src::logs::err(format_args!("  {:10} / {:10}", ((u32::from_le(raw)) as u32), ((u32::from_be(raw)) as u32)));
                        k += 1;
                    }
                    crate::src::logs::err(format_args!("\n"));
                    w += 4 as ::core::ffi::c_int;
                }
            }
            crate::src::logs::err(format_args!("  the last commands it pushed:\n"));
            let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
            while i < 32 as ::core::ffi::c_uint {
                let k_0: ::core::ffi::c_uint =
                    pm4_recent_n.get().wrapping_add(i) & 31 as ::core::ffi::c_uint;
                if !(pm4_recent_mut()[k_0 as usize].hdr == 0) {
                    if pm4_recent_mut()[k_0 as usize].type_0 == 0 as uint32_t {
                        crate::src::logs::err(format_args!("    write {} register(s) from {}(0x{:03x})\n", ((pm4_recent_mut()[k_0 as usize].count) as u32), crate::src::qemu::cs(ppc_mac_gpu_reg_name(
                                (pm4_recent_mut()[k_0 as usize].hdr & 0x7fff as uint32_t)
                                    .wrapping_mul(4 as uint32_t)
                                    as hwaddr,
                            )), (((pm4_recent_mut()[k_0 as usize].hdr & 0x7fff as uint32_t)
                                .wrapping_mul(4 as uint32_t)) as u32)));
                    } else {
                        crate::src::logs::err(format_args!("    type {} opcode 0x{:02x}, {} word(s)\n", ((pm4_recent_mut()[k_0 as usize].type_0) as u32), ((pm4_recent_mut()[k_0 as usize].opcode) as u32), ((pm4_recent_mut()[k_0 as usize].count) as u32)));
                    }
                }
                i = i.wrapping_add(1);
            }
            crate::src::logs::err(format_args!("  the last {} registers the guest touched:\n", ((32 as ::core::ffi::c_uint) as u32)));
            let ring = RING.lock().unwrap_or_else(|e| e.into_inner());
            let mut i_0: ::core::ffi::c_uint = 224 as ::core::ffi::c_uint;
            while i_0 < 256 as ::core::ffi::c_uint {
                let k_1: ::core::ffi::c_uint = n.get().wrapping_add(i_0) & 255 as ::core::ffi::c_uint;
                let e = ring[k_1 as usize];
                crate::src::logs::err(format_args!("    {} {}(0x{:03x})\n", crate::src::qemu::cstr(if e.wr {
                        c"wrote".as_ptr()
                    } else {
                        c"read".as_ptr()
                    }), crate::src::qemu::cs(ppc_mac_gpu_reg_name(e.addr as hwaddr)), e.addr as u32));
                i_0 = i_0.wrapping_add(1);
            }
        }
    }
    if !crate::src::qemu::getenv(c"POWEREMU_POLL_TRACE").is_null() {
        static COUNT: std::sync::Mutex<[uint32_t; 4096]> = std::sync::Mutex::new([0; 4096]);
        static next_report: crate::src::global::Global<int64_t> = crate::src::global::Global::<int64_t>::new(0);
        let now_0: int64_t = crate::src::qemu::clock_realtime_ms();
        let mut count = COUNT.lock().unwrap_or_else(|e| e.into_inner());
        if addr < 0x4000 as hwaddr {
            let idx = addr.wrapping_div(4 as hwaddr) as usize;
            count[idx] = count[idx].wrapping_add(1);
        }
        if now_0 > next_report.get() {
            if next_report.get() != 0 {
                let mut top: [uint32_t; 5] = [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0];
                let mut topi: [uint32_t; 5] = [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0];
                let mut i_1: uint32_t = 0 as uint32_t;
                while i_1 < (0x4000 as ::core::ffi::c_int / 4 as ::core::ffi::c_int) as uint32_t {
                    let mut k_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while k_2 < 5 as ::core::ffi::c_int {
                        if count[i_1 as usize] > top[k_2 as usize] {
                            let mut j: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
                            while j > k_2 {
                                top[j as usize] = top[(j - 1 as ::core::ffi::c_int) as usize];
                                topi[j as usize] = topi[(j - 1 as ::core::ffi::c_int) as usize];
                                j -= 1;
                            }
                            top[k_2 as usize] = count[i_1 as usize];
                            topi[k_2 as usize] = i_1;
                            break;
                        } else {
                            k_2 += 1;
                        }
                    }
                    i_1 = i_1.wrapping_add(1);
                }
                crate::src::logs::err(format_args!("ppc-mac-gpu polled most:"));
                let mut k_3: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while k_3 < 5 as ::core::ffi::c_int && top[k_3 as usize] != 0 {
                    crate::src::logs::err(format_args!(" {}(0x{:03x})x{}", crate::src::qemu::cs(ppc_mac_gpu_reg_name(
                            topi[k_3 as usize].wrapping_mul(4 as uint32_t) as hwaddr
                        )), ((topi[k_3 as usize].wrapping_mul(4 as uint32_t)) as u32), ((top[k_3 as usize]) as u32)));
                    k_3 += 1;
                }
                crate::src::logs::err(format_args!("\n"));
                count.fill(0);
            }
            next_report.set(now_0 + 5000 as int64_t);
        }
    }
    if addr >= 0xff00 as hwaddr && addr < 0xff20 as hwaddr {
        return (if size == 4 as ::core::ffi::c_uint {
            ppc_mac_gpu_hwc_read(&mut *s, addr.wrapping_sub(0xff00 as hwaddr) as uint32_t)
        } else {
            0 as uint32_t
        }) as uint64_t;
    }
    if addr >= 0x780 as hwaddr && addr < 0x7c0 as hwaddr {
        crate::src::logs::seq(format_args!("DMA  rd reg={:03x}", ((addr as ::core::ffi::c_uint) as u32)));
    }
    if !(addr >= 0x15e0 as hwaddr
        && addr <= 0x15f4 as hwaddr
        && addr & 3 as hwaddr == 0
        && r200_scratch_read_wait(
            &mut *s,
            addr.wrapping_sub(0x15e0 as hwaddr)
                .wrapping_div(4 as hwaddr) as ::core::ffi::c_int,
        ) as ::core::ffi::c_int
            != 0)
    {
        if addr != 0x740 as hwaddr
            && addr != 0x7f8 as hwaddr
            && addr != 0x210 as hwaddr
            && addr != 0x44 as hwaddr
        {
            gpu_sync_at(&mut *s, (addr as uint32_t).wrapping_div(4 as uint32_t));
        }
    }
    if 1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        && size == 1 as ::core::ffi::c_uint
        && addr < 128 as hwaddr
        && !s.edid_read_done
    {
        val = s.edid_blob[addr as usize] as uint64_t;
        crate::src::logs::debug(format_args!("EDID_RD offset=0x{:02x} -> 0x{:02x}", ((addr as ::core::ffi::c_uint) as u32), ((val as ::core::ffi::c_uint) as u32)));
        return val;
    }
    if addr >= 0x4 as hwaddr
        && addr < (0x4 as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as hwaddr
    {
        if s.regs.mm_index as ::core::ffi::c_uint & 0x80000000 as ::core::ffi::c_uint != 0 {
            return { let hz0 = s.regs.mm_index; ppc_mac_gpu_mm_aper_read(
                &mut *s,
                ((hz0 as ::core::ffi::c_uint & 0x7fffffff as ::core::ffi::c_uint)
                    as uint64_t)
                    .wrapping_add((addr as uint64_t).wrapping_sub(0x4 as uint64_t)),
                size,
            ) };
        }
        if ppc_mac_gpu_mm_reg_index(s) != 0 {
            return ppc_mac_gpu_mmio_read(
                s,
                (ppc_mac_gpu_mm_reg_index(s) as hwaddr)
                    .wrapping_add(addr.wrapping_sub(0x4 as hwaddr)),
                size,
            );
        }
    }
    if !ppc_mac_gpu_mmio_read_cfg(&mut *s, addr, size, &mut val)
        && !ppc_mac_gpu_mmio_read_display(&mut *s, addr, size, &mut val)
        && !ppc_mac_gpu_mmio_read_2d(&mut *s, addr, size, &mut val)
        && !ppc_mac_gpu_mmio_read_cp(&mut *s, addr, size, &mut val)
    {
        ppc_mac_gpu_mmio_read_stub_3d(&mut *s, addr, size, &mut val);
    }
    let dc: ::core::ffi::c_int = g_post_probe_draw_count.get();
    if dc >= 20 as ::core::ffi::c_int && g_post_probe_audit_mut().summary_logged == 0 {
        if g_post_probe_audit_mut().audit_active == 0 {
            g_post_probe_audit_mut().audit_active = 1 as ::core::ffi::c_int;
            g_post_probe_audit_mut().first_draw_seen = dc;
            g_post_probe_audit_mut().window_start_draw = dc;
        }
        g_post_probe_audit_mut().total_reads += 1;
        let idx: ::core::ffi::c_int = post_probe_find_or_add_reg(addr as uint32_t);
        if idx >= 0 as ::core::ffi::c_int {
            g_post_probe_audit_mut().entries[idx as usize].count = g_post_probe_audit_mut().entries
                [idx as usize]
                .count
                .wrapping_add(1);
            g_post_probe_audit_mut().entries[idx as usize].last_value = val as uint32_t;
            g_post_probe_audit_mut().entries[idx as usize].last_at_draw = dc as uint32_t;
        }
        let tr: ::core::ffi::c_int = g_post_probe_audit_mut().total_reads;
        if tr <= 200 as ::core::ffi::c_int
            || tr % 500 as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        {
            crate::src::logs::qemu(format_args!("[POST_PROBE_READ] sequence_id={} reg=0x{:04x} value=0x{:08x} count={} context={} delta_from_fence={} note={}\n", ((tr) as i32), ((addr as uint32_t) as u32), ((val as uint32_t) as u32), ((if idx >= 0 as ::core::ffi::c_int {
                    g_post_probe_audit_mut().entries[idx as usize].count
                } else {
                    0 as uint32_t
                }) as i32), crate::src::qemu::cstr(if addr == 0xe40 as hwaddr {
                    c"status_poll".as_ptr()
                } else if addr >= 0x15e0 as hwaddr && addr <= 0x15f4 as hwaddr {
                    c"fence".as_ptr()
                } else if addr == 0x64c as hwaddr {
                    c"cp_status".as_ptr()
                } else if addr >= 0x1c00 as hwaddr && addr < 0x4000 as hwaddr {
                    c"3d_reg".as_ptr()
                } else {
                    c"other".as_ptr()
                }), ((dc) as i32), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr))));
        }
        if g_post_probe_audit_mut().window_logged == 0 && dc >= 100 as ::core::ffi::c_int {
            g_post_probe_audit_mut().window_logged = 1 as ::core::ffi::c_int;
            g_post_probe_audit_mut().window_end_draw = dc;
            crate::src::logs::qemu(format_args!("[PROBE_WINDOW] first_probe_draw={} window_begin={} window_end={} total_reads={} unique_regs={} note=early_window_complete\n", ((g_post_probe_audit_mut().first_draw_seen) as i32), ((g_post_probe_audit_mut().window_start_draw) as i32), ((dc) as i32), ((g_post_probe_audit_mut().total_reads) as i32), ((g_post_probe_audit_mut().num_entries) as i32)));
        }
        if (dc >= 400 as ::core::ffi::c_int || tr >= 5000 as ::core::ffi::c_int)
            && g_post_probe_audit_mut().summary_logged == 0
        {
            g_post_probe_audit_mut().summary_logged = 1 as ::core::ffi::c_int;
            let mut i_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_2 < g_post_probe_audit_mut().num_entries - 1 as ::core::ffi::c_int {
                let mut j_0: ::core::ffi::c_int = i_2 + 1 as ::core::ffi::c_int;
                while j_0 < g_post_probe_audit_mut().num_entries {
                    if g_post_probe_audit_mut().entries[j_0 as usize].count
                        > g_post_probe_audit_mut().entries[i_2 as usize].count
                    {
                        let tmp: PostProbeReadEntry = g_post_probe_audit_mut().entries[i_2 as usize];
                        g_post_probe_audit_mut().entries[i_2 as usize] =
                            g_post_probe_audit_mut().entries[j_0 as usize];
                        g_post_probe_audit_mut().entries[j_0 as usize] = tmp;
                    }
                    j_0 += 1;
                }
                i_2 += 1;
            }
            crate::src::logs::qemu(format_args!("[POST_PROBE_READ_SUMMARY] total_reads={} unique_regs={} draw_range={}-{}\n", ((g_post_probe_audit_mut().total_reads) as i32), ((g_post_probe_audit_mut().num_entries) as i32), ((g_post_probe_audit_mut().window_start_draw) as i32), ((dc) as i32)));
            let mut i_3: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_3 < g_post_probe_audit_mut().num_entries && i_3 < 30 as ::core::ffi::c_int {
                let e: &mut PostProbeReadEntry = &mut g_post_probe_audit_mut().entries[(i_3) as usize];
                let ctx: *const ::core::ffi::c_char = if e.reg == 0xe40 as uint32_t {
                    c"STATUS_POLL".as_ptr()
                } else if e.reg >= 0x15e0 as uint32_t && e.reg <= 0x15f4 as uint32_t {
                    c"FENCE".as_ptr()
                } else if e.reg == 0x64c as uint32_t {
                    c"CP_STATUS".as_ptr()
                } else if e.reg >= 0x1c00 as uint32_t && e.reg < 0x4000 as uint32_t {
                    c"3D_REG".as_ptr()
                } else if e.reg == 0x210 as uint32_t
                    || e.reg == 0x5c as uint32_t
                    || e.reg == 0x214 as uint32_t
                {
                    c"CRTC".as_ptr()
                } else if e.reg >= 0xf00 as uint32_t && e.reg < 0xf40 as uint32_t {
                    c"PCI_MIRROR".as_ptr()
                } else {
                    c"OTHER".as_ptr()
                };
                let mut suspicious: bool = 0 as ::core::ffi::c_int != 0;
                let mut suspicion: *const ::core::ffi::c_char =
                    c"".as_ptr();
                if e.last_value == 0 as uint32_t && e.count >= 5 as uint32_t {
                    suspicious = 1 as ::core::ffi::c_int != 0;
                    suspicion =
                        c"static_zero_high_freq".as_ptr();
                }
                if e.reg >= 0x1c00 as uint32_t
                    && e.reg < 0x4000 as uint32_t
                    && e.count >= 3 as uint32_t
                {
                    suspicious = 1 as ::core::ffi::c_int != 0;
                    suspicion = c"3d_reg_readback_during_probe".as_ptr();
                }
                crate::src::logs::qemu(format_args!("[POST_PROBE_REG_RANK] rank={} reg=0x{:04x} name={} count={} last_value=0x{:08x} first_draw={} last_draw={} context={} suspicious={} suspicion={}\n", ((i_3 + 1 as ::core::ffi::c_int) as i32), ((e.reg) as u32), crate::src::qemu::cs(ppc_mac_gpu_reg_name(e.reg as hwaddr)), ((e.count) as u32), ((e.last_value) as u32), ((e.first_at_draw) as u32), ((e.last_at_draw) as u32), crate::src::qemu::cstr(ctx), ((suspicious as ::core::ffi::c_int) as i32), crate::src::qemu::cstr(if suspicious as ::core::ffi::c_int != 0 {
                        suspicion
                    } else {
                        c"none".as_ptr()
                    })));
                i_3 += 1;
            }
            crate::src::logs::qemu(format_args!("[QE_GATE_CANDIDATE] note=see_POST_PROBE_REG_RANK_above look_for=static_zero_or_3d_readback_regs\n"));
        }
    }
    if crate::src::qemu::trace_mmio_read_enabled() && crate::src::qemu::loglevel_enabled(1 << 15) {
        crate::src::qemu::trace_mmio_read(size, addr as uint64_t, ppc_mac_gpu_reg_name(addr), val);
    }
    if (!s.r3_dump.is_null()
        && !s.r3.is_null()
        && s.r3_ref().draws != 0
        && s.r3_trace_lines < 40000 as uint32_t) as ::core::ffi::c_int
        as ::core::ffi::c_long
        != 0
    {
        static last_addr: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
        static repeats: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
        if addr == last_addr.get() as hwaddr && {
            repeats.set(repeats.get().wrapping_add(1));
            repeats.get() > 3 as uint32_t
        } {
            return val;
        }
        if addr != last_addr.get() as hwaddr {
            repeats.set(0 as uint32_t);
        }
        last_addr.set(addr as uint32_t);
        s.r3_trace_lines = s.r3_trace_lines.wrapping_add(1);
        crate::src::logs::err(format_args!("R {:04x} {:08x} {}\n", ((addr as ::core::ffi::c_uint) as u32), ((val as uint32_t) as u32), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr))));
    }
    return val;
}
pub fn ppc_mac_gpu_mmio_read(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    if s.ring_pending as ::core::ffi::c_int as ::core::ffi::c_long != 0
        && addr >= 0x200 as hwaddr
    {
        ppc_mac_gpu_ring_run(s);
    }
    let val: uint64_t = ppc_mac_gpu_mmio_read_impl(s, addr, size);
    gpu_poll_watch(addr, val);
    if gpu_debug_enabled() as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        gpu_track_pass.set(gpu_tracked_reg(addr));
        crate::src::logs::debug(format_args!("MMIO_RD size={} addr=0x{:04x} ({}) = 0x{:x}", ((size) as u32), ((addr) as u64), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr)), ((val) as u64)));
        if gpu_track_pass.get() {
            gpu_caller_probe(addr);
        }
        gpu_track_pass.set(0 as ::core::ffi::c_int != 0);
    }
    return val;
}
pub fn ppc_mac_gpu_tcl_port_write(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    val: uint32_t,
) {
    match addr {
        8704 => {
            s.regs.tcl_vec_addr = val & 0x7ff as uint32_t;
            s.regs.tcl_vec_stride = val >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
            s.regs.tcl_vec_comp = 0 as uint32_t;
        }
        8708 => {
            s.regs.tcl_vec[(s.regs.tcl_vec_addr & 0x7ff as uint32_t) as usize]
                [s.regs.tcl_vec_comp as usize] = val;
            s.regs.tcl_vec_comp = s.regs.tcl_vec_comp.wrapping_add(1);
            if s.regs.tcl_vec_comp == 4 as uint32_t {
                s.regs.tcl_vec_comp = 0 as uint32_t;
                s.regs.tcl_vec_addr =
                    s.regs
                        .tcl_vec_addr
                        .wrapping_add(if s.regs.tcl_vec_stride != 0 {
                            s.regs.tcl_vec_stride
                        } else {
                            1 as uint32_t
                        });
            }
        }
        8712 => {
            s.regs.tcl_scalar_addr = val & 0x1ff as uint32_t;
            s.regs.tcl_scalar_stride = val >> 16 as ::core::ffi::c_int & 0xff as uint32_t;
        }
        8716 => {
            if !crate::src::qemu::getenv(c"POWEREMU_VP_TRACE").is_null()
            {
                static SEEN_SCALAR: [core::sync::atomic::AtomicU8; 512] =
                    [const { core::sync::atomic::AtomicU8::new(0) }; 512];
                let sa: uint32_t = s.regs.tcl_scalar_addr & 0x1ff as uint32_t;
                // sa is masked to 0x1ff, so the index is always below 512.
                if SEEN_SCALAR[sa as usize].swap(1, core::sync::atomic::Ordering::Relaxed) == 0 {
                    crate::src::logs::err(format_args!("ppc-mac-gpu scalar[{:03x}] {:08x}\n", ((sa) as u32), ((val) as u32)));
                }
            }
            s.regs.tcl_scalar[(s.regs.tcl_scalar_addr & 0x1ff as uint32_t) as usize] = val;
            s.regs.tcl_scalar_addr =
                s.regs
                    .tcl_scalar_addr
                    .wrapping_add(if s.regs.tcl_scalar_stride != 0 {
                        s.regs.tcl_scalar_stride
                    } else {
                        1 as uint32_t
                    });
        }
        _ => {}
    };
}
pub fn ppc_mac_gpu_mmio_write_misc(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    val: uint64_t,
    size: ::core::ffi::c_uint,
) -> bool {
    match addr {
        92 => {
            if val & ((1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as uint64_t != 0 {
                s.regs.crtc_vblank_clear_ns = crate::src::qemu::clock_virtual_ns();
                s.regs.crtc_status &=
                    !((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t;
            }
        }
        0 => {
            s.regs.mm_index = (val
                & (0x80000000 as ::core::ffi::c_uint | 0xffff as ::core::ffi::c_uint) as uint64_t)
                as uint32_t;
        }
        16..=44 => {
            let sh: uint32_t = (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr) as uint32_t;
            let m: uint32_t = if size >= 4 as ::core::ffi::c_uint {
                0xffffffff as uint32_t
            } else {
                ((1 as uint32_t) << size.wrapping_mul(8 as ::core::ffi::c_uint))
                    .wrapping_sub(1 as uint32_t)
                    << sh
            };
            let r: &mut uint32_t = &mut s.regs.bios_scratch[(addr.wrapping_sub(0x10 as hwaddr).wrapping_div(4 as hwaddr)) as usize];
            *r = if size >= 4 as ::core::ffi::c_uint {
                val as uint32_t
            } else {
                *r & !m | (val as uint32_t) << sh & m
            };
        }
        64 => {
            s.regs.gen_int_cntl = val as uint32_t;
            if val & ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint64_t != 0 {
                crate::src::qemu::timer_arm(
                    &mut s.vblank_timer,
                    (crate::src::qemu::clock_virtual_ns() as ::core::ffi::c_longlong
                        + 1000000000 as ::core::ffi::c_longlong / 60 as ::core::ffi::c_longlong)
                        as int64_t,
                );
            } else {
                crate::src::qemu::timer_disarm(&mut s.vblank_timer);
            }
            ppc_mac_gpu_update_irq(s);
        }
        68 => {
            s.regs.gen_int_status = (s.regs.gen_int_status as uint64_t & !val) as uint32_t;
            ppc_mac_gpu_update_irq(s);
        }
        224 => {
            s.regs.config_cntl = val as uint32_t;
        }
        328 => {
            s.regs.mc_fb_location = val as uint32_t;
        }
        332 => {
            s.regs.mc_agp_location = val as uint32_t;
            r200_agp_tc_flush();
        }
        48 => {
            s.regs.bus_cntl = val as uint32_t;
        }
        304 => {
            s.regs.host_path_cntl = val as uint32_t;
        }
        240 => {
            crate::src::qemu::trace_soft_reset(val);
        }
        2816 => {
            s.regs.surface_cntl = val as uint32_t;
        }
        16408 | 16420 | 16392 | 16400 | 16404 | 16412 | 16416 | 8512 | 8528 | 5900 | 16640
        | 16896 | 16900 | 16912 | 16944 | 16976 | 17024 | 17088 | 18084 | 17920 | 17924 | 17928
        | 17936 | 17940 | 17952 | 17956 | 17960 | 17964 | 17968 | 17972 | 17976 | 17980 | 17984
        | 17988 | 17992 | 17996 | 18000 | 18004 | 18008 | 18012 | 18016 | 18020 | 18024 | 18028
        | 18032 | 18036 | 18040 | 18044 | 18048 | 18052 | 18056 | 18060 | 18064 | 18068 | 18072
        | 18076 | 18080 | 20224 | 20228 | 20232 | 20244 | 20248 | 20252 | 20264 | 20268 => {
            if addr < 0x5000 as hwaddr {
                s.r300_shadow[(addr as uint32_t).wrapping_div(4 as uint32_t) as usize] =
                    val as uint32_t;
            }
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_write_display(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    val: uint64_t,
    _size: ::core::ffi::c_uint,
) -> bool {
    match addr {
        80 => {
            s.regs.crtc_gen_cntl = val as uint32_t;
            ppc_mac_gpu_vga_bank_update(s);
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
            ppc_mac_gpu_cursor_sync(s);
        }
        84 => {
            s.regs.crtc_ext_cntl = val as uint32_t;
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
        }
        512 => {
            s.regs.crtc_h_total_disp = val as uint32_t;
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
        }
        516 => {
            s.regs.crtc_h_sync_strt_wid = val as uint32_t;
        }
        520 => {
            s.regs.crtc_v_total_disp = val as uint32_t;
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
        }
        524 => {
            s.regs.crtc_v_sync_strt_wid = val as uint32_t;
        }
        548 => {
            if val != s.regs.crtc_offset as uint64_t {
                crate::src::logs::blit_path("CRTC", format_args!("OFFSET changed 0x{:x} -> 0x{:x}", ((s.regs.crtc_offset) as u32), ((val as uint32_t) as u32)));
            }
            s.regs.crtc_offset = (val
                & !((3 as ::core::ffi::c_uint) << 30 as ::core::ffi::c_int) as uint64_t)
                as uint32_t;
            s.regs.crtc_offset_locked =
                val & ((1 as ::core::ffi::c_uint) << 31 as ::core::ffi::c_int) as uint64_t != 0;
            s.regs.crtc_offset_write_ns = crate::src::qemu::clock_virtual_ns();
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
        }
        552 => {
            if val != s.regs.crtc_offset_cntl as uint64_t {
                crate::src::logs::blit_path("CRTC", format_args!("OFFSET_CNTL changed 0x{:x} -> 0x{:x}", ((s.regs.crtc_offset_cntl) as u32), ((val as uint32_t) as u32)));
            }
            s.regs.crtc_offset_cntl = val as uint32_t;
        }
        556 => {
            if val != s.regs.crtc_pitch as uint64_t {
                crate::src::logs::blit_path("CRTC", format_args!("PITCH changed 0x{:x} -> 0x{:x}", ((s.regs.crtc_pitch) as u32), ((val as uint32_t) as u32)));
            }
            s.regs.crtc_pitch = val as uint32_t;
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
        }
        88 => {
            s.regs.dac_cntl = val as uint32_t;
            if s.legacy_vga {
                s.vga_mut().dac_8bit =
                    (val & 0x100 as uint64_t != 0 as uint64_t) as ::core::ffi::c_int;
            }
        }
        124 => {
            s.regs.dac_cntl2 = val as uint32_t;
        }
        176 => {
            s.regs.palette_index = (val & 0xff as uint64_t) as uint32_t;
        }
        180 => {
            if s.regs.palette_index < 256 as uint32_t {
                gpu_dac_set(
                    s,
                    s.regs.palette_index as ::core::ffi::c_uint,
                    val as uint32_t,
                );
                s.regs.palette_index =
                    s.regs.palette_index.wrapping_add(1 as uint32_t) & 0xff as uint32_t;
            }
        }
        184 => {
            if s.regs.palette_index < 256 as uint32_t {
                gpu_dac_set(
                    s,
                    s.regs.palette_index as ::core::ffi::c_uint,
                    ((val >> 22 as ::core::ffi::c_int & 0xff as uint64_t)
                        << 16 as ::core::ffi::c_int
                        | (val >> 12 as ::core::ffi::c_int & 0xff as uint64_t)
                            << 8 as ::core::ffi::c_int
                        | val >> 2 as ::core::ffi::c_int & 0xff as uint64_t)
                        as uint32_t,
                );
                s.regs.palette_index =
                    s.regs.palette_index.wrapping_add(1 as uint32_t) & 0xff as uint32_t;
            }
        }
        608 => {
            s.regs.cur_offset = val as uint32_t;
            ppc_mac_gpu_cursor_sync(s);
        }
        612 => {
            s.regs.cur_horz_vert_posn = val as uint32_t;
            ppc_mac_gpu_cursor_sync(s);
        }
        616 => {
            s.regs.cur_horz_vert_off = val as uint32_t;
            ppc_mac_gpu_cursor_sync(s);
        }
        620 => {
            s.regs.cur_clr0 = val as uint32_t;
        }
        624 => {
            s.regs.cur_clr1 = val as uint32_t;
        }
        3328 => {
            s.regs.disp_misc_cntl = val as uint32_t;
        }
        3428 => {
            s.regs.disp_output_cntl = val as uint32_t;
        }
        3424 => {
            s.regs.disp_merge_cntl = val as uint32_t;
        }
        536 | 3336 => {}
        644 => {
            s.regs.fp_gen_cntl = val as uint32_t;
        }
        368 => {
            s.regs.agp_base = val as uint32_t;
        }
        372 => {
            s.regs.agp_cntl = val as uint32_t;
        }
        3936 => {
            s.regs.agp_command = val as uint32_t;
        }
        8 => {
            s.regs.clock_cntl_index = val as uint32_t;
        }
        12 => {
            s.regs.clock_cntl_data = val as uint32_t;
            if s.regs.clock_cntl_index & 0x80 as uint32_t != 0 {
                let pi: uint32_t = s.regs.clock_cntl_index & 0x3f as uint32_t;
                s.regs.pll[pi as usize] = (if pi == 0x3 as uint32_t {
                    val & !((1 as ::core::ffi::c_uint) << 15 as ::core::ffi::c_int
                        | (1 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int)
                        as uint64_t
                } else {
                    val
                }) as uint32_t;
            }
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_write_2d(
    s: &mut PPCMacGPUFields,
    addr: hwaddr,
    val: uint64_t,
    _size: ::core::ffi::c_uint,
) -> bool {
    match addr {
        5160 => {
            s.regs.src_pitch_offset = val as uint32_t;
            let spo_base: uint32_t = gpu_po_offset(s, val as uint32_t);
            let spo_pitch: uint32_t =
                ((val & 0x3fc00000 as uint64_t) >> 16 as ::core::ffi::c_int) as uint32_t;
            let spo_macro: bool = val >> 30 as ::core::ffi::c_int & 1 as uint64_t != 0;
            if spo_macro as ::core::ffi::c_int != 0
                && spo_base > 0 as uint32_t
                && spo_pitch > 0 as uint32_t
            {
                mc_register_tiled_surface(
                    s,
                    spo_base,
                    spo_pitch,
                    c"SRC_PO".as_ptr(),
                );
            }
        }
        5164 => {
            s.regs.dst_pitch_offset = val as uint32_t;
            let dpo_base: uint32_t = gpu_po_offset(s, val as uint32_t);
            let dpo_pitch: uint32_t =
                ((val & 0x3fc00000 as uint64_t) >> 16 as ::core::ffi::c_int) as uint32_t;
            let dpo_macro: bool = val >> 30 as ::core::ffi::c_int & 1 as uint64_t != 0;
            if dpo_macro as ::core::ffi::c_int != 0
                && dpo_base > 0 as uint32_t
                && dpo_pitch > 0 as uint32_t
            {
                mc_register_tiled_surface(
                    s,
                    dpo_base,
                    dpo_pitch,
                    c"DST_PO".as_ptr(),
                );
            }
        }
        5124 => {
            s.regs.dst_offset = val as uint32_t;
        }
        5128 => {
            s.regs.dst_pitch = val as uint32_t;
        }
        5132 => {
            s.regs.dst_width = val as uint32_t;
        }
        5136 => {
            s.regs.dst_height = val as uint32_t;
        }
        5180 => {
            if gpu_unified_2d() {
                ppc_mac_gpu_2d(
                    s,
                    (val & 0x3fff as uint64_t) as uint32_t,
                    (val >> 16 as ::core::ffi::c_int & 0x3fff as uint64_t) as uint32_t,
                    1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int,
                );
            }
        }
        5528 => {
            s.regs.dst_width_height = val as uint32_t;
            if gpu_unified_2d() {
                ppc_mac_gpu_2d(
                    s,
                    (val >> 16 as ::core::ffi::c_int & 0x3fff as uint64_t) as uint32_t,
                    (val & 0x3fff as uint64_t) as uint32_t,
                    0 as ::core::ffi::c_int != 0,
                );
            }
        }
        5176 => {
            s.regs.dst_y_x = val as uint32_t;
        }
        5236 => {
            s.regs.brush_y_x = val as uint32_t;
        }
        5248..=5308 => {
            s.regs.brush_data[addr
                .wrapping_sub(0x1480 as hwaddr)
                .wrapping_div(4 as hwaddr) as usize] = val as uint32_t;
        }
        5632 => {
            s.regs.dst_line_start = val as uint32_t;
        }
        5636 => {
            ppc_mac_gpu_2d_line(s, val as uint32_t);
        }
        5524 => {
            let x: uint32_t =
                (val >> 16 as ::core::ffi::c_int & 0xffff as uint64_t) as uint32_t;
            let y: uint32_t = (val & 0xffff as uint64_t) as uint32_t;
            s.regs.dst_y_x = y << 16 as ::core::ffi::c_int | x;
        }
        5548 => {
            s.regs.src_offset = val as uint32_t;
        }
        5552 => {
            s.regs.src_pitch = val as uint32_t;
        }
        5172 => {
            s.regs.src_y_x = val as uint32_t;
        }
        5520 => {
            let x_0: uint32_t =
                (val >> 16 as ::core::ffi::c_int & 0xffff as uint64_t) as uint32_t;
            let y_0: uint32_t = (val & 0xffff as uint64_t) as uint32_t;
            s.regs.src_y_x = y_0 << 16 as ::core::ffi::c_int | x_0;
        }
        5228 => {
            s.regs.dp_gui_master_cntl = val as uint32_t;
        }
        5244 => {
            s.regs.dp_brush_frgd_clr = val as uint32_t;
        }
        5240 => {
            s.regs.dp_brush_bkgd_clr = val as uint32_t;
        }
        5592 => {
            s.regs.dp_src_frgd_clr = val as uint32_t;
        }
        5596 => {
            s.regs.dp_src_bkgd_clr = val as uint32_t;
        }
        5836 => {
            s.regs.dp_write_msk = val as uint32_t;
        }
        5824 => {
            s.regs.dp_cntl = val as uint32_t;
        }
        5828 => {
            s.regs.dp_datatype = val as uint32_t;
            s.regs.dp_gui_master_cntl = ((s.regs.dp_gui_master_cntl as ::core::ffi::c_uint
                & !(0xf00 as ::core::ffi::c_uint))
                as uint64_t
                | (val & 0xf as uint64_t) << 8 as ::core::ffi::c_int)
                as uint32_t;
        }
        5832 => {
            s.regs.dp_mix = val as uint32_t;
        }
        5868 => {
            s.regs.sc_top_left = val as uint32_t;
        }
        5872 => {
            s.regs.sc_bottom_right = val as uint32_t;
        }
        5864 => {
            s.regs.default_sc_bottom_right = val as uint32_t;
        }
        5856 => {
            s.regs.default_pitch_offset = val as uint32_t;
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    return 1 as ::core::ffi::c_int != 0;
}
