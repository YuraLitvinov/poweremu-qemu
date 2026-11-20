use super::*;
pub fn ppc_mac_gpu_mmio_write_cp(
    s: &mut PPCMacGPUState,
    addr: hwaddr,
    val: uint64_t,
    size: ::core::ffi::c_uint,
) -> bool {
    match addr {
        1792 => {
            s.regs.cp_rb_base = val as uint32_t;
            crate::src::logs::debug(format_args!("CP_SETUP RB_BASE=0x{:08x}", ((val as uint32_t) as u32)));
            crate::src::qemu::trace_cp_ring_setup(val, s.regs.cp_rb_cntl);
        }
        1804 => {
            s.regs.cp_rb_rptr_addr = val as uint32_t;
            ppc_mac_gpu_rptr_writeback(s);
        }
        1796 => {
            s.regs.cp_rb_cntl = val as uint32_t;
            crate::src::logs::debug(format_args!("CP_SETUP RB_CNTL=0x{:08x} (log2size={})", ((val as uint32_t) as u32), ((val as uint32_t & 0x3f as uint32_t) as u32)));
        }
        1808 => {
            s.regs.cp_rb_rptr = val as uint32_t;
            ppc_mac_gpu_rptr_writeback(s);
            crate::src::logs::debug(format_args!("CP_RING RPTR <- {}", ((val as uint32_t) as u32)));
        }
        1812 => {
            crate::src::logs::debug(format_args!("CP_RING WPTR <- {} (prev={}, delta={})", ((val as uint32_t) as u32), ((s.regs.cp_rb_wptr) as u32), ((val as uint32_t as ::core::ffi::c_int - s.regs.cp_rb_wptr as ::core::ffi::c_int) as i32)));
            s.regs.cp_rb_wptr = val as uint32_t;
            s.ring_pending = 1 as ::core::ffi::c_int != 0;
            if s.ring_async && !s.ring_bh.is_null() {
                crate::src::qemu::ring_bh_schedule(s);
            } else {
                ppc_mac_gpu_ring_run(s);
            }
        }
        134 => {
            s.regs.cp_me_cntl = val as uint32_t;
            crate::src::logs::debug(format_args!("CP_SETUP ME_CNTL=0x{:08x} (ME_start={})", ((val as uint32_t) as u32), (((val as uint32_t & 0x10000000 as ::core::ffi::c_int as uint32_t == 0)
                    as ::core::ffi::c_int) as i32)));
        }
        1848 => {
            s.regs.cp_ib_base = val as uint32_t;
            crate::src::logs::debug(format_args!("CP_IB BASE <- 0x{:08x}", ((val as uint32_t) as u32)));
        }
        1852 => {
            s.regs.cp_ib_bufsz = val as uint32_t;
            crate::src::logs::debug(format_args!("CP_IB BUFSZ <- {} dwords (KICK! base=0x{:08x})", ((val as uint32_t) as u32), ((s.regs.cp_ib_base) as u32)));
            ppc_mac_gpu_execute_ib(s, s.regs.cp_ib_base, val as uint32_t);
        }
        1856 => {
            s.regs.cp_csq_cntl = val as uint32_t;
            crate::src::logs::debug(format_args!("CP_SETUP CSQ_CNTL=0x{:08x} (mode={})", ((val as uint32_t) as u32), ((val as uint32_t >> 28 as ::core::ffi::c_int & 0xf as uint32_t) as u32)));
        }
        1904 => {
            s.regs.scratch_umsk = val as uint32_t;
        }
        1908 => {
            s.regs.scratch_addr = val as uint32_t;
        }
        2004 => {
            s.regs.cp_me_ram_addr = val as uint32_t;
        }
        2016 => {
            s.regs.cp_me_ram_addr = s.regs.cp_me_ram_addr.wrapping_add(1);
        }
        2012 | 2032 | 2036 => {}
        96..=99 => {
            ppc_mac_gpu_reg_write_offs(
                &mut s.regs.gpio_vga_ddc,
                addr.wrapping_sub(0x60 as hwaddr) as ::core::ffi::c_int,
                val,
                size,
            );
            if addr <= (0x60 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as hwaddr
                && addr.wrapping_add(size as hwaddr)
                    > (0x60 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as hwaddr
                || addr == 0x60 as hwaddr
                    && s.regs.gpio_vga_ddc & 0x30000 as ::core::ffi::c_int as uint32_t != 0
            {
                s.regs.gpio_vga_ddc = ppc_mac_gpu_i2c(
                    &mut s.bbi2c,
                    s.regs.gpio_vga_ddc as uint64_t,
                    0 as ::core::ffi::c_int,
                ) as uint32_t;
            }
        }
        100..=103 => {
            ppc_mac_gpu_reg_write_offs(
                &mut s.regs.gpio_dvi_ddc,
                addr.wrapping_sub(0x64 as hwaddr) as ::core::ffi::c_int,
                val,
                size,
            );
            if addr <= (0x64 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as hwaddr
                && addr.wrapping_add(size as hwaddr)
                    > (0x64 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as hwaddr
                || addr == 0x64 as hwaddr
                    && s.regs.gpio_dvi_ddc & 0x30000 as ::core::ffi::c_int as uint32_t != 0
            {
                s.regs.gpio_dvi_ddc |= 0x300 as uint32_t;
            }
        }
        104..=107 => {
            ppc_mac_gpu_reg_write_offs(
                &mut s.regs.gpio_monid,
                addr.wrapping_sub(0x68 as hwaddr) as ::core::ffi::c_int,
                val,
                size,
            );
            if addr <= (0x68 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as hwaddr
                && addr.wrapping_add(size as hwaddr)
                    > (0x68 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as hwaddr
                || addr == 0x68 as hwaddr
                    && s.regs.gpio_monid & 0x30000 as ::core::ffi::c_int as uint32_t != 0
            {
                s.regs.gpio_monid |= 0x300 as uint32_t;
            }
        }
        5920 => {
            s.regs.wait_until = 0 as uint32_t;
        }
        5924 => {
            s.regs.isync_cntl = val as uint32_t;
        }
        5600..=5620 => {
            let idx: ::core::ffi::c_int =
                addr.wrapping_sub(0x15e0 as hwaddr)
                    .wrapping_div(4 as hwaddr) as ::core::ffi::c_int;
            s.regs.scratch_reg[idx as usize] = val as uint32_t;
            r200_scratch_write(s, idx, val as uint32_t);
        }
        4096..=5119 => {
            ppc_mac_gpu_pm4_fifo_push(s, val as uint32_t);
        }
        6080..=6112 => {
            ppc_mac_gpu_host_data_write(s, val as uint32_t);
        }
        _ => return 0 as ::core::ffi::c_int != 0,
    }
    return 1 as ::core::ffi::c_int != 0;
}
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
pub fn ppc_mac_gpu_mmio_write_gart_stub_3d(
    s: &mut PPCMacGPUState,
    addr: hwaddr,
    val: uint64_t,
    size: ::core::ffi::c_uint,
) -> bool {
    match addr {
        464 => {
            s.regs.aic_ctrl = val as uint32_t;
            r200_agp_tc_flush();
            crate::src::logs::debug(format_args!("GART AIC_CTRL=0x{:x} (enabled={})", ((val as uint32_t) as u32), ((val as uint32_t & 1 as uint32_t) as i32)));
        }
        472 => {
            s.regs.aic_pt_base = val as uint32_t;
            r200_agp_tc_flush();
            crate::src::logs::debug(format_args!("GART PT_BASE=0x{:x}", ((val as uint32_t) as u32)));
        }
        476 => {
            s.regs.aic_lo_addr = val as uint32_t;
            r200_agp_tc_flush();
            crate::src::logs::debug(format_args!("GART LO_ADDR=0x{:x}", ((val as uint32_t) as u32)));
        }
        480 => {
            s.regs.aic_hi_addr = val as uint32_t;
            r200_agp_tc_flush();
            crate::src::logs::debug(format_args!("GART HI_ADDR=0x{:x}", ((val as uint32_t) as u32)));
        }
        808 | 572 | 828 | 5904 | 1084 | 1056 | 1344 | 2384 | 3432 | 408 | 412 | 1156 | 962
        | 864 => {}
        528 => {
            s.regs.crtc_vline = (val & 0x7ff as uint64_t) as uint32_t;
        }
        7228 | 7244 => {
            let idx: uint32_t = addr
                .wrapping_sub(0x1c00 as hwaddr)
                .wrapping_div(4 as hwaddr) as uint32_t;
            if idx < 2304 as uint32_t {
                s.regs.regs_3d[idx as usize] = val as uint32_t;
                crate::src::qemu::trace_3d_reg_write(addr as uint64_t, val);
            }
            crate::src::logs::debug(format_args!("3D_MMIO 0x{:04x} <- 0x{:08x}{}", ((addr as uint32_t) as u32), ((val as uint32_t) as u32), crate::src::qemu::cstr(if addr == 0x1c3c as hwaddr {
                    c" (RB3D_CNTL)".as_ptr()
                } else if addr == 0x1c4c as hwaddr {
                    c" (SE_CNTL)".as_ptr()
                } else {
                    c"".as_ptr()
                })));
        }
        _ => {
            if addr >= 0x1c00 as hwaddr && addr < 0x4000 as hwaddr {
                let idx_0: uint32_t = (addr as uint32_t)
                    .wrapping_sub(0x1c00 as uint32_t)
                    .wrapping_div(4 as uint32_t);
                ppc_mac_gpu_tcl_port_write(s, addr, val as uint32_t);
                if idx_0 < 2304 as uint32_t {
                    s.regs.regs_3d[idx_0 as usize] = val as uint32_t;
                    crate::src::qemu::trace_3d_reg_write(addr as uint64_t, val);
                    if addr == 0x1c40 as hwaddr
                        || addr == 0x1c48 as hwaddr
                        || addr == 0x325c as hwaddr
                        || addr == 0x1c3c as hwaddr
                        || addr == 0x1c4c as hwaddr
                        || addr == 0x3258 as hwaddr
                        || addr == 0x20b0 as hwaddr
                        || addr == 0x20b4 as hwaddr
                        || addr == 0x20bc as hwaddr
                    {
                        let name: &str = match addr as uint32_t {
                            7232 => "RB3D_COLOROFFSET",
                            7240 => "RB3D_COLORPITCH",
                            12892 => "RB3D_DSTCACHE_CTLSTAT",
                            7228 => "RB3D_CNTL",
                            7244 => "SE_CNTL",
                            12888 => "RB3D_DSTCACHE_MODE",
                            8368 => "RE_WIDTH_HEIGHT",
                            8372 => "RE_MISC",
                            8380 => "RE_STIPPLE_ADDR",
                            _ => "?",
                        };
                        crate::src::logs::debug(format_args!("3D_MMIO 0x{:04x} <- 0x{:08x} ({})", ((addr as uint32_t) as u32), ((val as uint32_t) as u32), name));
                    }
                }
            } else if addr >= 0x500 as hwaddr && addr < 0x516 as hwaddr {
                let vbe_idx: uint32_t =
                    (addr.wrapping_sub(0x500 as hwaddr) >> 1 as ::core::ffi::c_int) as uint32_t;
                if vbe_idx < 0xa as uint32_t {
                    s.regs.vbe_regs[vbe_idx as usize] = val as uint16_t;
                    let name_0: &::core::ffi::CStr = VBE_NAMES.get(vbe_idx as usize).copied().unwrap_or(c"?");
                    crate::src::logs::debug(format_args!("VBE_WR [{}] {} <- 0x{:04x}", ((vbe_idx) as u32), crate::src::qemu::cs(name_0), ((val as uint16_t as ::core::ffi::c_int) as u32)));
                    if vbe_idx == 0x9 as uint32_t {
                        let mut w: uint32_t =
                            s.regs.vbe_regs[0x1 as ::core::ffi::c_int as usize] as uint32_t;
                        let mut bpp: uint32_t =
                            s.regs.vbe_regs[0x3 as ::core::ffi::c_int as usize] as uint32_t;
                        if w == 0 as uint32_t {
                            w = 800 as uint32_t;
                        }
                        if bpp == 0 as uint32_t {
                            bpp = 32 as uint32_t;
                        }
                        s.regs.crtc_offset = val.wrapping_mul(w as uint64_t).wrapping_mul(
                            bpp.wrapping_add(7 as uint32_t).wrapping_div(8 as uint32_t) as uint64_t,
                        ) as uint32_t;
                        s.display_invalid = 1 as ::core::ffi::c_int != 0;
                        crate::src::logs::debug(format_args!("VBE Y_OFFSET={} -> crtc_offset=0x{:x}", ((val as uint32_t) as u32), ((s.regs.crtc_offset) as u32)));
                    }
                    if vbe_idx == 0x4 as uint32_t && val & 0x1 as uint64_t != 0 {
                        let mut w_0: uint32_t =
                            s.regs.vbe_regs[0x1 as ::core::ffi::c_int as usize] as uint32_t;
                        let mut h: uint32_t =
                            s.regs.vbe_regs[0x2 as ::core::ffi::c_int as usize] as uint32_t;
                        let mut bpp_0: uint32_t =
                            s.regs.vbe_regs[0x3 as ::core::ffi::c_int as usize] as uint32_t;
                        let pix_fmt: uint32_t;
                        if w_0 == 0 as uint32_t {
                            w_0 = 800 as uint32_t;
                        }
                        if h == 0 as uint32_t {
                            h = 600 as uint32_t;
                        }
                        if bpp_0 == 0 as uint32_t {
                            bpp_0 = 32 as uint32_t;
                        }
                        match bpp_0 {
                            8 => {
                                pix_fmt = ((2 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                                    as uint32_t;
                            }
                            15 => {
                                pix_fmt = ((3 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                                    as uint32_t;
                            }
                            16 => {
                                pix_fmt = ((4 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                                    as uint32_t;
                            }
                            24 => {
                                pix_fmt = ((5 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                                    as uint32_t;
                            }
                            _ => {
                                pix_fmt = ((6 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                                    as uint32_t;
                            }
                        }
                        s.regs.crtc_gen_cntl = ((1 as ::core::ffi::c_int)
                            << 25 as ::core::ffi::c_int
                            | (1 as ::core::ffi::c_int) << 24 as ::core::ffi::c_int)
                            as uint32_t
                            | pix_fmt;
                        s.regs.crtc_h_total_disp =
                            w_0.wrapping_div(8 as uint32_t).wrapping_sub(1 as uint32_t)
                                << 16 as ::core::ffi::c_int
                                | w_0.wrapping_div(8 as uint32_t).wrapping_add(31 as uint32_t);
                        s.regs.crtc_v_total_disp = h.wrapping_sub(1 as uint32_t)
                            << 16 as ::core::ffi::c_int
                            | h.wrapping_add(27 as uint32_t);
                        s.regs.crtc_pitch = w_0.wrapping_div(8 as uint32_t);
                        let y_off: uint32_t =
                            s.regs.vbe_regs[0x9 as ::core::ffi::c_int as usize] as uint32_t;
                        s.regs.crtc_offset = y_off.wrapping_mul(w_0).wrapping_mul(
                            bpp_0
                                .wrapping_add(7 as uint32_t)
                                .wrapping_div(8 as uint32_t),
                        );
                        s.display_invalid = 1 as ::core::ffi::c_int != 0;
                        crate::src::logs::debug(format_args!("VBE mode set: {}x{}x{}", ((w_0) as u32), ((h) as u32), ((bpp_0) as u32)));
                        if !s.edid_read_done {
                            s.edid_read_done = 1 as ::core::ffi::c_int != 0;
                            crate::src::logs::debug(format_args!("EDID read phase complete - switching to ATI register mode"));
                        }
                    }
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
                        if s.regs.vga_ar_flip_flop as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int
                        {
                            s.regs.vga_ar_index = (val & 0x3f as uint64_t) as uint8_t;
                        }
                        s.regs.vga_ar_flip_flop =
                            (s.regs.vga_ar_flip_flop as ::core::ffi::c_int
                                ^ 1 as ::core::ffi::c_int) as uint8_t;
                    }
                    967 => {
                        s.regs.vga_dac_read_index = val as uint8_t;
                        s.regs.vga_dac_sub_index = 0 as uint8_t;
                    }
                    968 => {
                        s.regs.vga_dac_write_index = val as uint8_t;
                        s.regs.vga_dac_sub_index = 0 as uint8_t;
                    }
                    969 => {
                        let idx_1: uint8_t = s.regs.vga_dac_write_index;
                        s.regs.vga_dac_palette[idx_1 as usize]
                            [s.regs.vga_dac_sub_index as usize] =
                            (val & 0x3f as uint64_t) as uint8_t;
                        s.regs.vga_dac_sub_index = s.regs.vga_dac_sub_index.wrapping_add(1);
                        if s.regs.vga_dac_sub_index as ::core::ffi::c_int
                            >= 3 as ::core::ffi::c_int
                        {
                            s.regs.vga_dac_sub_index = 0 as uint8_t;
                            s.regs.vga_dac_write_index =
                                s.regs.vga_dac_write_index.wrapping_add(1);
                        }
                    }
                    _ => {}
                }
            } else if addr >= 0x600 as hwaddr && addr < 0x620 as hwaddr {
                let ext_reg: uint32_t =
                    (addr.wrapping_sub(0x600 as hwaddr) >> 2 as ::core::ffi::c_int) as uint32_t;
                if ext_reg == 2 as uint32_t {
                    if val & 1 as uint64_t != 0 {
                        s.regs.gen_int_status &=
                            !((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint32_t;
                    }
                }
            } else {
                if addr == 0x15d4 as hwaddr {
                    s.r300_src_swap = (val & 3 as uint64_t) as uint32_t;
                }
                if addr < 0x5000 as hwaddr
                    && is_r300_register(addr as uint32_t) as ::core::ffi::c_int != 0
                {
                    s.r300_shadow[(addr as uint32_t).wrapping_div(4 as uint32_t) as usize] =
                        val as uint32_t;
                } else if addr == 0x15d4 as hwaddr {
                    s.r300_src_swap = (val & 3 as uint64_t) as uint32_t;
                } else if !(addr == 0x15d0 as hwaddr || addr == 0x19e4 as hwaddr) {
                    if !(addr == 0x1700 as hwaddr || addr == 0x1704 as hwaddr) {
                        if ppc_mac_gpu_post_reg(addr as uint32_t) >= 0 as ::core::ffi::c_int {
                            let sh: uint32_t =
                                (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr) as uint32_t;
                            let m: uint32_t = if size >= 4 as ::core::ffi::c_uint {
                                0xffffffff as uint32_t
                            } else {
                                ((1 as uint32_t) << size.wrapping_mul(8 as ::core::ffi::c_uint))
                                    .wrapping_sub(1 as uint32_t)
                                    << sh
                            };
                            let idx: usize = ((addr & !(3 as hwaddr)) as uint32_t).wrapping_div(4 as uint32_t) as usize;
                            if let Some(r) = s.r300_shadow.get_mut(idx) {
                                *r = if size >= 4 as ::core::ffi::c_uint {
                                    val as uint32_t
                                } else {
                                    *r & !m | (val as uint32_t) << sh & m
                                };
                            }
                        } else if addr == 0x1f8 as hwaddr {
                            s.mc_ind_index = val as uint32_t;
                        } else if addr == 0x1fc as hwaddr {
                            if s.mc_ind_index & 0x100 as uint32_t != 0 {
                                s.mc_ind[(s.mc_ind_index & 0x7f as uint32_t) as usize] =
                                    val as uint32_t;
                            }
                        } else if size < 4 as ::core::ffi::c_uint || addr & 3 as hwaddr != 0 {
                            let sh_0: uint32_t =
                                (addr & 3 as hwaddr).wrapping_mul(8 as hwaddr) as uint32_t;
                            let m_0: uint32_t = (if size >= 4 as ::core::ffi::c_uint {
                                0xffffffff as uint32_t
                            } else {
                                ((1 as uint32_t) << size.wrapping_mul(8 as ::core::ffi::c_uint))
                                    .wrapping_sub(1 as uint32_t)
                            }) << sh_0;
                            let cur: uint32_t = ppc_mac_gpu_mmio_read(
                                s,
                                (addr as ::core::ffi::c_ulonglong
                                    & !(3 as ::core::ffi::c_ulonglong))
                                    as hwaddr,
                                4 as ::core::ffi::c_uint,
                            ) as uint32_t;
                            crate::src::qemu::mmio_write_dev(
                                s,
                                (addr as ::core::ffi::c_ulonglong
                                    & !(3 as ::core::ffi::c_ulonglong))
                                    as hwaddr,
                                (cur & !m_0 | (val as uint32_t) << sh_0 & m_0) as uint64_t,
                                4 as ::core::ffi::c_uint,
                            );
                        } else {
                            crate::src::qemu::trace_mmio_write_unimpl(
                                size,
                                addr as uint64_t,
                                ppc_mac_gpu_reg_name(addr),
                                val,
                            );
                        }
                    }
                }
            }
        }
    }
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_mmio_write(
    s: &mut PPCMacGPUState,
    addr: hwaddr,
    mut val: uint64_t,
    mut size: ::core::ffi::c_uint,
) {
    if addr >= 0xff00 as hwaddr && addr < 0xff20 as hwaddr {
        if size == 4 as ::core::ffi::c_uint {
            ppc_mac_gpu_hwc_write(
                &mut *s,
                addr.wrapping_sub(0xff00 as hwaddr) as uint32_t,
                val as uint32_t,
            );
        }
        return;
    }
    if s.ring_pending as ::core::ffi::c_int as ::core::ffi::c_long != 0
        && addr != 0x714 as hwaddr
    {
        ppc_mac_gpu_ring_run(&mut *s);
    }
    if addr >= 0x780 as hwaddr && addr < 0x7c0 as hwaddr {
        crate::src::logs::seq(format_args!("DMA  wr reg={:03x} val={:08x}", ((addr as ::core::ffi::c_uint) as u32), ((val as uint32_t) as u32)));
    }
    if addr >= 0x400 as hwaddr && addr < 0x560 as hwaddr {
        static ov0_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
        if ov0_log.get() < 0 as ::core::ffi::c_int {
            ov0_log.set(crate::src::qemu::env_flag(c"PPCGPU_OV0_LOG") as ::core::ffi::c_int);
        }
        if ov0_log.get() != 0 {
            crate::src::logs::err(format_args!("[OV0] wr reg=0x{:04x} ({}) val=0x{:08x}\n", ((addr as ::core::ffi::c_uint) as u32), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr)), ((val as uint32_t) as u32)));
        }
    }
    if gpu_debug_enabled() as ::core::ffi::c_int as ::core::ffi::c_long != 0 {
        let nm: &::core::ffi::CStr = ppc_mac_gpu_reg_name(addr);
        gpu_track_pass.set(gpu_tracked_reg(addr));
        crate::src::logs::debug(format_args!("MMIO_WR size={} addr=0x{:04x} val=0x{:x} ({})", ((size) as u32), ((addr) as u64), ((val) as u64), crate::src::qemu::cs(nm)));
        gpu_track_pass.set(0 as ::core::ffi::c_int != 0);
        crate::src::qemu::trace_mmio_write(size, addr as uint64_t, nm, val);
    }
    if !s.r3.is_null() && r300_state_owns(addr as uint32_t) as ::core::ffi::c_int != 0 {
        crate::src::qemu::r3_state_write(s, addr as uint32_t, val as uint32_t);
        if addr == 0x4f58 as hwaddr || addr == 0x4f5c as hwaddr {
            r300_zpass_write(&mut *s, addr as uint32_t, val as uint32_t);
        }
    }
    if addr >= 0xb00 as hwaddr && addr < 0xb80 as hwaddr {
        s.r300_surf[addr.wrapping_sub(0xb00 as hwaddr).wrapping_div(4 as hwaddr) as usize] =
            val as uint32_t;
        if addr == 0xb0c as hwaddr {
            r300_surface_changed(&mut *s);
            r300_surfwatch_update(&mut *s);
        }
    }
    if addr == 0xab0 as hwaddr {
        s.r300_aic_pt_base = (val & !(0xfff as ::core::ffi::c_uint) as uint64_t) as uint32_t;
        r200_agp_tc_flush();
    }
    if (!s.r3_dump.is_null() && s.r3_ref().draws != 0 && s.r3_trace_lines < 40000 as uint32_t)
        as ::core::ffi::c_int as ::core::ffi::c_long
        != 0
    {
        let nm_0: Option<&::core::ffi::CStr> = if r300_state_owns(addr as uint32_t) {
            crate::src::qemu::r300_reg_cstr(addr as uint32_t)
        } else {
            None
        };
        s.r3_trace_lines = s.r3_trace_lines.wrapping_add(1);
        crate::src::logs::err(format_args!("W {:04x} {:08x} {} (mmio)\n", ((addr as ::core::ffi::c_uint) as u32), ((val as uint32_t) as u32), crate::src::qemu::cs(nm_0.unwrap_or_else(|| ppc_mac_gpu_reg_name(addr)))));
    }
    if size < 4 as ::core::ffi::c_uint
        && addr & 3 as hwaddr == 0 as hwaddr
        && (addr == 0x50 as hwaddr
            || addr == 0x54 as hwaddr
            || addr == 0x58 as hwaddr
            || addr == 0x7c as hwaddr)
    {
        let m: uint32_t = ((1 as uint32_t) << size.wrapping_mul(8 as ::core::ffi::c_uint))
            .wrapping_sub(1 as uint32_t);
        let cur: uint32_t = if addr == 0x50 as hwaddr {
            s.regs.crtc_gen_cntl
        } else if addr == 0x54 as hwaddr {
            s.regs.crtc_ext_cntl
        } else if addr == 0x58 as hwaddr {
            s.regs.dac_cntl
        } else {
            s.regs.dac_cntl2
        };
        val = (cur & !m) as uint64_t | val & m as uint64_t;
        size = 4 as ::core::ffi::c_uint;
    }
    if addr >= 0x4 as hwaddr
        && addr < (0x4 as ::core::ffi::c_int + 4 as ::core::ffi::c_int) as hwaddr
    {
        if s.regs.mm_index as ::core::ffi::c_uint & 0x80000000 as ::core::ffi::c_uint != 0 {
            { let hz0 = s.regs.mm_index; ppc_mac_gpu_mm_aper_write(
                &mut *s,
                ((hz0 as ::core::ffi::c_uint & 0x7fffffff as ::core::ffi::c_uint)
                    as uint64_t)
                    .wrapping_add((addr as uint64_t).wrapping_sub(0x4 as uint64_t)),
                val,
                size,
            ) };
            return;
        }
        let idx_r: uint32_t = ppc_mac_gpu_mm_reg_index(s);
        if idx_r != 0 {
            crate::src::qemu::mmio_write_dev(
                s,
                (idx_r as hwaddr).wrapping_add(addr.wrapping_sub(0x4 as hwaddr)),
                val,
                size,
            );
            return;
        }
    }
    if ppc_mac_gpu_mmio_write_misc(&mut *s, addr, val, size) {
        return;
    }
    if ppc_mac_gpu_mmio_write_display(&mut *s, addr, val, size) {
        return;
    }
    if ppc_mac_gpu_mmio_write_2d(&mut *s, addr, val, size) {
        return;
    }
    if ppc_mac_gpu_mmio_write_cp(&mut *s, addr, val, size) {
        return;
    }
    if ppc_mac_gpu_mmio_write_gart_stub_3d(&mut *s, addr, val, size) {
        return;
    }
}
pub unsafe extern "C" fn ppc_mac_gpu_mmio_read_timed(
    opaque: *mut ::core::ffi::c_void,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    let t0: int64_t = crate::src::qemu::clock_realtime_ns();
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let v: uint64_t = match unsafe { crate::src::qemu::state_of_opaque(opaque) } {
        Some(s) => ppc_mac_gpu_mmio_read(s, addr, size),
        None => 0,
    };
    let dt: int64_t = crate::src::qemu::clock_realtime_ns() - t0;
    if dt >= gpu_slow_limit_ns() {
        crate::src::logs::err(format_args!("[SLOW_MMIO] read  reg=0x{:04x} ({}) took {:.1} ms\n", ((addr) as u64), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr)), ((dt as ::core::ffi::c_double / 1e6f64) as f64)));
    }
    return v;
}
pub unsafe extern "C" fn ppc_mac_gpu_mmio_write_timed(
    opaque: *mut ::core::ffi::c_void,
    addr: hwaddr,
    val: uint64_t,
    size: ::core::ffi::c_uint,
) {
    let t0: int64_t = crate::src::qemu::clock_realtime_ns();
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    if let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) {
        crate::src::qemu::mmio_write_dev(s, addr, val, size);
    }
    let dt: int64_t = crate::src::qemu::clock_realtime_ns() - t0;
    if dt >= gpu_slow_limit_ns() {
        crate::src::logs::err(format_args!("[SLOW_MMIO] write reg=0x{:04x} ({}) val=0x{:08x} took {:.1} ms\n", ((addr) as u64), crate::src::qemu::cs(ppc_mac_gpu_reg_name(addr)), ((val) as u64), ((dt as ::core::ffi::c_double / 1e6f64) as f64)));
    }
}
pub unsafe extern "C" fn ppc_mac_gpu_reset(dev: *mut DeviceState) {
    // SAFETY: `dev` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(dev as *mut ::core::ffi::c_void) }) else {
        return;
    };
    let (gw, gh, gd) = crate::src::qemu::host_graphic_size();
    s.ring_pending = 0 as ::core::ffi::c_int != 0;
    crate::src::qemu::zero_regs(&mut s.regs);
    s.hwc_idx = 0 as uint32_t;
    s.hwc_h = s.hwc_idx;
    s.hwc_w = s.hwc_h;
    s.regs.regs_3d[((0x3230 as ::core::ffi::c_int - 0x1c00 as ::core::ffi::c_int)
        / 4 as ::core::ffi::c_int) as usize] = 0xffffffff as ::core::ffi::c_uint as uint32_t;
    s.hwc_visible = 0 as ::core::ffi::c_int != 0;
    crate::src::qemu::console_set_mouse_to(s, 0, 0, false);
    let cp_headroom: uint64_t = (4 as ::core::ffi::c_long
        * ((1 as ::core::ffi::c_long) << 20 as ::core::ffi::c_int))
        as uint64_t;
    let reported_memsize: uint64_t = s.vram_size.wrapping_sub(cp_headroom);
    s.regs.config_memsize = reported_memsize as uint32_t;
    s.regs.mc_fb_location = (0 as uint64_t
        | (reported_memsize.wrapping_sub(1 as uint64_t) >> 16 as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int) as uint32_t;
    s.regs.rbbm_status = 64 as uint32_t;
    s.regs.gui_stat = 64 as uint32_t;
    s.regs.dac_cntl = 0 as uint32_t;
    s.regs.dp_write_msk = 0xffffffff as ::core::ffi::c_uint as uint32_t;
    s.regs.default_sc_bottom_right = 0x1fff1fff as ::core::ffi::c_int as uint32_t;
    s.regs.host_path_cntl = ((1 as ::core::ffi::c_int) << 23 as ::core::ffi::c_int) as uint32_t;
    let boot_w: uint32_t = (if gw > 0 as ::core::ffi::c_int {
        gw
    } else {
        800 as ::core::ffi::c_int
    }) as uint32_t;
    let boot_h: uint32_t = (if gh > 0 as ::core::ffi::c_int {
        gh
    } else {
        600 as ::core::ffi::c_int
    }) as uint32_t;
    s.regs.crtc_gen_cntl = ((1 as ::core::ffi::c_int) << 25 as ::core::ffi::c_int
        | (1 as ::core::ffi::c_int) << 24 as ::core::ffi::c_int
        | (6 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
        as uint32_t;
    s.regs.crtc_h_total_disp = boot_w
        .wrapping_div(8 as uint32_t)
        .wrapping_sub(1 as uint32_t)
        << 16 as ::core::ffi::c_int
        | boot_w
            .wrapping_div(8 as uint32_t)
            .wrapping_add(31 as uint32_t);
    s.regs.crtc_v_total_disp = boot_h.wrapping_sub(1 as uint32_t) << 16 as ::core::ffi::c_int
        | boot_h.wrapping_add(27 as uint32_t);
    s.regs.crtc_pitch = boot_w.wrapping_div(8 as uint32_t);
    s.regs.crtc_offset = 0 as uint32_t;
    s.regs.agp_status = 0x1f000203 as ::core::ffi::c_int as uint32_t;
    let boot_w_0: uint32_t = (if gw > 0 as ::core::ffi::c_int {
        gw
    } else {
        800 as ::core::ffi::c_int
    }) as uint32_t;
    let boot_h_0: uint32_t = (if gh > 0 as ::core::ffi::c_int {
        gh
    } else {
        600 as ::core::ffi::c_int
    }) as uint32_t;
    let boot_d: uint32_t = (if gd > 0 as ::core::ffi::c_int {
        gd
    } else {
        32 as ::core::ffi::c_int
    }) as uint32_t;
    s.regs.vbe_regs[0 as ::core::ffi::c_int as usize] = 0xb0c5 as uint16_t;
    s.regs.vbe_regs[0x1 as ::core::ffi::c_int as usize] = boot_w_0 as uint16_t;
    s.regs.vbe_regs[0x2 as ::core::ffi::c_int as usize] = boot_h_0 as uint16_t;
    s.regs.vbe_regs[0x3 as ::core::ffi::c_int as usize] = boot_d as uint16_t;
    s.regs.vbe_regs[0x4 as ::core::ffi::c_int as usize] =
        (0x1 as ::core::ffi::c_int | 0x40 as ::core::ffi::c_int) as uint16_t;
    s.regs.vbe_regs[0x5 as ::core::ffi::c_int as usize] = 0 as uint16_t;
    s.regs.vbe_regs[0x6 as ::core::ffi::c_int as usize] = boot_w_0 as uint16_t;
    s.regs.vbe_regs[0x7 as ::core::ffi::c_int as usize] = boot_h_0 as uint16_t;
    s.regs.vbe_regs[0x8 as ::core::ffi::c_int as usize] = 0 as uint16_t;
    s.regs.vbe_regs[0x9 as ::core::ffi::c_int as usize] = 0 as uint16_t;
    ppc_mac_gpu_post_regs_reset(&mut *s);
    s.regs.pll[0xa as ::core::ffi::c_int as usize] = (12 as ::core::ffi::c_int
        | (69 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
        | (72 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int)
        as uint32_t;
    s.regs.pll[0x3 as ::core::ffi::c_int as usize] = 12 as uint32_t;
    if s.legacy_vga {
        s.regs.crtc_gen_cntl = 0 as uint32_t;
        ppc_mac_gpu_vga_bank_update(&mut *s);
        s.vga_shown = 0 as ::core::ffi::c_int != 0;
        crate::src::qemu::vga_reset(s);
    }
    s.mode = 1 as uint8_t;
    s.display_invalid = 1 as ::core::ffi::c_int != 0;
    s.edid_read_done = 0 as ::core::ffi::c_int != 0;
    s.disp_stride_override_active = 0 as ::core::ffi::c_int != 0;
    s.disp_stride_override_value = 0 as uint32_t;
    s.host_data_active = 0 as ::core::ffi::c_int != 0;
    s.pm4_fifo_idx = 0 as uint32_t;
    s.pm4_pkt_count = 0 as uint32_t;
    s.compositor_valid = 0 as ::core::ffi::c_int != 0;
    crate::src::qemu::zero_tiled_surfaces(&mut s.tiled_surfaces);
    s.metal_rt_count = 0 as ::core::ffi::c_int;
    s.vb_count = 0 as uint32_t;
    crate::src::qemu::timer_delete(&mut s.vblank_timer);
}

pub unsafe extern "C" fn ppc_mac_gpu_vram_dirty(
    arg: *mut ::core::ffi::c_void,
    bitmap: *mut ::core::ffi::c_ulong,
    npages: uint64_t,
) {
    // SAFETY: `arg` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(arg) }) else {
        return;
    };
    let pg: uint64_t = 4096;
    let chunk: uint64_t = 256 * pg;
    let snap = crate::src::qemu::VramDirtySnapshot::take(s);
    let words = crate::src::qemu::dirty_log_words(bitmap, npages);
    let mut off: uint64_t = 0;
    while off < s.vram_size {
        let len: uint64_t = chunk.min(s.vram_size - off);
        if snap.get_dirty(s, off, len) {
            let mut p: uint64_t = off;
            while p < off.wrapping_add(len) {
                if p / pg < npages && snap.get_dirty(s, p, pg) {
                    crate::src::qemu::set_bit_in(words, (p / pg) as usize);
                }
                p = p.wrapping_add(pg);
            }
        }
        off = off.wrapping_add(chunk);
    }
}
/// Why realize stopped: `Report` is a new QEMU error (the C source line and its message) for
/// errp; `Reported` means a QEMU call already stored its error there.
enum RealizeError {
    Report(::core::ffi::c_int, String),
    Reported,
}

const REALIZE_FILE: &::core::ffi::CStr = c"../../hw/display/ppc_mac_gpu.c";

/// `strtoul(v, NULL, 16)` truncated to 16 bits: leading blanks, an optional sign, an optional
/// `0x`, then hex digits up to the first other byte; 0 when there are none.
fn strtoul_hex16(v: &[u8]) -> u16 {
    let mut i = 0;
    while i < v.len() && matches!(v[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let neg = match v.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    if v.get(i) == Some(&b'0')
        && matches!(v.get(i + 1), Some(b'x' | b'X'))
        && v.get(i + 2).is_some_and(|c| c.is_ascii_hexdigit())
    {
        i += 2;
    }
    let mut acc: u64 = 0;
    while let Some(d) = v.get(i).and_then(|c| (*c as char).to_digit(16)) {
        acc = acc.saturating_mul(16).saturating_add(u64::from(d));
        i += 1;
    }
    (if neg { acc.wrapping_neg() } else { acc }) as u16
}

pub unsafe extern "C" fn ppc_mac_gpu_realize(dev: *mut PCIDevice, errp: *mut *mut Error) {
    // SAFETY: `dev` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(dev as *mut ::core::ffi::c_void) }) else {
        return;
    };
    let mut scratch: *mut Error = ::core::ptr::null_mut();
    let slot = crate::src::qemu::errp_slot(errp).unwrap_or(&mut scratch);
    if let Err(RealizeError::Report(line, msg)) = realize_inner(s, slot) {
        crate::src::qemu::error_setg_at(errp, REALIZE_FILE, line, c"ppc_mac_gpu_realize", &msg);
    }
}

fn realize_inner(s: &mut PPCMacGPUState, errp: &mut *mut Error) -> Result<(), RealizeError> {
    use crate::src::qemu as q;
    use std::os::unix::ffi::OsStrExt;
    let tn = q::object_typename(crate::src::qemu::pci_raw_ref(&s.pci));
    s.regs_size = ::core::mem::size_of::<PPCMacGPURegs>() as uint32_t;
    s.ring_async = std::env::var_os("PPCGPU_SYNC_RING").is_none();
    let opaque = s as *mut PPCMacGPUState as *mut ::core::ffi::c_void;
    if s.ring_async {
        s.ring_bh = q::bh_new(ppc_mac_gpu_ring_bh_cb, opaque, c"ppc_mac_gpu_ring_bh_cb");
    }
    s.r300_src_swap = 2;
    s.r3 = q::glib_zeroed::<R300State>();
    let dump = std::env::var_os("R300_DUMP").and_then(|v| std::ffi::CString::new(v.as_bytes()).ok());
    s.r3_dump = match dump {
        Some(path) => q::open_r3_dump(path.as_c_str()),
        None => ::core::ptr::null_mut(),
    };
    s.vram_size = u64::from(s.vram_size_mb).wrapping_mul(1 << 20);
    if s.vram_size < 8 << 20 {
        return Err(RealizeError::Report(11018, "ppc-mac-gpu: VRAM must be at least 8 MB".into()));
    }
    if s.vram_size > 256 << 20 {
        return Err(RealizeError::Report(11022, "ppc-mac-gpu: VRAM must be at most 256 MB".into()));
    }
    s.con = q::graphic_console_create(crate::src::qemu::pci_raw(&mut s.pci), &ppc_mac_gpu_gfx_ops, opaque);
    if s.legacy_vga {
        let vga = q::glib_zeroed_ref::<VGACommonState>();
        s.vga = vga as *mut VGACommonState;
        vga.vram_size_mb = 8;
        if !q::vga_setup(vga, crate::src::qemu::pci_raw(&mut s.pci), errp) {
            return Err(RealizeError::Reported);
        }
        vga.con = s.con;
        q::mr_init_io(&mut s.vga_bank, crate::src::qemu::pci_raw(&mut s.pci), &ppc_mac_gpu_vga_bank_ops, opaque, c"ppc-mac-gpu-vga-bank", 0x10000);
        q::pci_mem_add_subregion_overlap(crate::src::qemu::pci_raw(&mut s.pci), 0xa0000, &mut s.vga_bank, 3);
        q::mr_set_enabled(&mut s.vga_bank, false);
    }
    let rname: Option<&[u8]> = q::c_arg(s.renderer_name)
        .map(::core::ffi::CStr::to_bytes)
        .filter(|b| !b.is_empty() && *b != b"auto");
    let want_vulkan = rname == Some(&b"vulkan"[..]);
    if let Some(name) = rname {
        if name != b"metal" && !want_vulkan {
            return Err(RealizeError::Report(
                11058,
                format!("{}: unknown renderer '{}' (metal or vulkan)", tn, String::from_utf8_lossy(name)),
            ));
        }
        if name == b"metal" {
            return Err(RealizeError::Report(11064, format!("{}: renderer=metal is only available on macOS", tn)));
        }
    }
    if want_vulkan {
        s.metal_vram_ptr = q::vulkan_alloc_vram(s.vram_size, &mut s.metal_vram_opaque);
        if s.metal_vram_ptr.is_null() {
            return Err(RealizeError::Report(
                11092,
                format!("{}: renderer=vulkan unavailable: {}", tn, q::vulkan_error_text()),
            ));
        }
    }
    if !s.metal_vram_ptr.is_null() {
        q::mr_init_ram_ptr(&mut s.vram, crate::src::qemu::pci_raw(&mut s.pci), c"ppc-mac-gpu-vram", s.vram_size, s.metal_vram_ptr);
        q::vmstate_register_ram(&mut s.vram, crate::src::qemu::pci_raw(&mut s.pci));
    } else {
        q::mr_init_ram_fatal(&mut s.vram, crate::src::qemu::pci_raw(&mut s.pci), c"ppc-mac-gpu-vram", s.vram_size);
    }
    q::mr_init(&mut s.vram_bar, crate::src::qemu::pci_raw(&mut s.pci), c"ppc-mac-gpu-vram-bar", s.vram_size);
    q::mr_add_subregion(&mut s.vram_bar, 0, &mut s.vram);
    q::pci_register_bar(crate::src::qemu::pci_raw(&mut s.pci), 0, 0x8, &mut s.vram_bar);
    q::mr_init_io(&mut s.mmio, crate::src::qemu::pci_raw(&mut s.pci), &ppc_mac_gpu_mmio_ops, opaque, c"ppc-mac-gpu-mmio", 0x10000);
    q::pci_register_bar(crate::src::qemu::pci_raw(&mut s.pci), 2, 0, &mut s.mmio);
    q::mr_init_alias(&mut s.io, crate::src::qemu::pci_raw(&mut s.pci), c"ppc-mac-gpu-io", &mut s.mmio, 0, 0x100);
    q::pci_register_bar(crate::src::qemu::pci_raw(&mut s.pci), 1, 0x1, &mut s.io);
    q::mr_set_log(&mut s.vram, true, 0);
    q::timer_init_virtual(&mut s.vblank_timer, ppc_mac_gpu_vblank, opaque);
    if let Some(bios) = q::c_arg(s.biosrom).filter(|c| !c.to_bytes().is_empty()) {
        let (path, data) = q::read_bios_file(bios);
        match data {
            Some(rom) => {
                let n = rom.len().min(s.vram_size as usize);
                q::Vram::of(s).write_bytes(0, &rom[..n]);
                crate::src::logs::debug(format_args!("BIOS ROM copied to VRAM: {} bytes from {}", rom.len() as u64, path));
            }
            None => crate::src::logs::debug(format_args!("BIOS ROM load failed: {}", path)),
        }
    }
    q::i2c_ddc_init(crate::src::qemu::pci_raw(&mut s.pci), &mut s.bbi2c);
    let e: &mut [uint8_t; 128] = &mut s.edid_blob;
    let mut i: ::core::ffi::c_int = 0;
    let mut sum: uint8_t = 0;
    e.fill(0);
    e[0] = 0 as uint8_t;
    e[1] = 0xff as uint8_t;
    e[2] = 0xff as uint8_t;
    e[3] = 0xff as uint8_t;
    e[4] = 0xff as uint8_t;
    e[5] = 0xff as uint8_t;
    e[6] = 0xff as uint8_t;
    e[7] = 0 as uint8_t;
    e[8] = 0x45 as uint8_t;
    e[9] = 0x4d as uint8_t;
    e[10] = 0 as uint8_t;
    e[11] = 0x92 as uint8_t;
    e[12] = 0x1 as uint8_t;
    e[13] = 0 as uint8_t;
    e[14] = 0 as uint8_t;
    e[15] = 0 as uint8_t;
    e[16] = 1 as uint8_t;
    e[17] = 16 as uint8_t;
    e[18] = 1 as uint8_t;
    e[19] = 3 as uint8_t;
    e[20] = 0x80 as uint8_t;
    e[21] = 33 as uint8_t;
    e[22] = 25 as uint8_t;
    e[23] = 120 as uint8_t;
    e[24] = 0xa as uint8_t;
    e[25] = 0xa5 as uint8_t;
    e[26] = 0x59 as uint8_t;
    e[27] = 0xa6 as uint8_t;
    e[28] = 0x54 as uint8_t;
    e[29] = 0x13 as uint8_t;
    e[30] = 0xab as uint8_t;
    e[31] = 0x26 as uint8_t;
    e[32] = 0xf as uint8_t;
    e[33] = 0x50 as uint8_t;
    e[34] = 0x54 as uint8_t;
    e[35] = 0x20 as uint8_t;
    e[36] = 0x9 as uint8_t;
    e[37] = 0x80 as uint8_t;
    e[38] = 0xa9 as uint8_t;
    e[39] = 0x40 as uint8_t;
    e[40] = 0x81 as uint8_t;
    e[41] = 0x40 as uint8_t;
    e[42] = 0xd1 as uint8_t;
    e[43] = 0 as uint8_t;
    e[44] = 0x1 as uint8_t;
    e[45] = 0x1 as uint8_t;
    e[46] = 0x1 as uint8_t;
    e[47] = 0x1 as uint8_t;
    e[48] = 0x1 as uint8_t;
    e[49] = 0x1 as uint8_t;
    e[50] = 0x1 as uint8_t;
    e[51] = 0x1 as uint8_t;
    e[52] = 0x1 as uint8_t;
    e[53] = 0x1 as uint8_t;
    e[54] = 0x64 as uint8_t;
    e[55] = 0x19 as uint8_t;
    e[56] = 0 as uint8_t;
    e[57] = 0x18 as uint8_t;
    e[58] = 0x41 as uint8_t;
    e[59] = 0 as uint8_t;
    e[60] = 0x23 as uint8_t;
    e[61] = 0x30 as uint8_t;
    e[62] = 0x18 as uint8_t;
    e[63] = 0x88 as uint8_t;
    e[64] = 0x36 as uint8_t;
    e[65] = 0 as uint8_t;
    e[66] = 0x30 as uint8_t;
    e[67] = 0xe4 as uint8_t;
    e[68] = 0x10 as uint8_t;
    e[69] = 0 as uint8_t;
    e[70] = 0 as uint8_t;
    e[71] = 0x18 as uint8_t;
    e[72] = 0 as uint8_t;
    e[73] = 0 as uint8_t;
    e[74] = 0 as uint8_t;
    e[75] = 0xfc as uint8_t;
    e[76] = 0 as uint8_t;
    e[77] = 'Q' as i32 as uint8_t;
    e[78] = 'E' as i32 as uint8_t;
    e[79] = 'M' as i32 as uint8_t;
    e[80] = 'U' as i32 as uint8_t;
    e[81] = ' ' as i32 as uint8_t;
    e[82] = 'R' as i32 as uint8_t;
    e[83] = 'V' as i32 as uint8_t;
    e[84] = '2' as i32 as uint8_t;
    e[85] = '8' as i32 as uint8_t;
    e[86] = '0' as i32 as uint8_t;
    e[87] = '\n' as i32 as uint8_t;
    e[88] = ' ' as i32 as uint8_t;
    e[89] = ' ' as i32 as uint8_t;
    e[90] = 0 as uint8_t;
    e[91] = 0 as uint8_t;
    e[92] = 0 as uint8_t;
    e[93] = 0xfd as uint8_t;
    e[94] = 0 as uint8_t;
    e[95] = 50 as uint8_t;
    e[96] = 75 as uint8_t;
    e[97] = 30 as uint8_t;
    e[98] = 81 as uint8_t;
    e[99] = 140 as uint8_t;
    e[100] = 0 as uint8_t;
    e[101] = 0xa as uint8_t;
    e[102] = 0x20 as uint8_t;
    e[103] = 0x20 as uint8_t;
    e[104] = 0x20 as uint8_t;
    e[105] = 0x20 as uint8_t;
    e[106] = 0x20 as uint8_t;
    e[107] = 0x20 as uint8_t;
    e[108] = 0 as uint8_t;
    e[109] = 0 as uint8_t;
    e[110] = 0 as uint8_t;
    e[111] = 0xf7 as uint8_t;
    e[112] = 0 as uint8_t;
    e[113] = 0xa as uint8_t;
    i = 114 as ::core::ffi::c_int;
    while i < 126 as ::core::ffi::c_int {
        e[i as usize] = 0 as uint8_t;
        i += 1;
    }
    e[116] = 0x8 as uint8_t;
    e[117] = 0x8 as uint8_t;
    if s.host_aspect_modes {
        e[118] = 0xfc as uint8_t;
        e[119] = 0x30 as uint8_t;
    }
    e[126] = 0 as uint8_t;
    sum = 0 as uint8_t;
    i = 0 as ::core::ffi::c_int;
    while i < 127 as ::core::ffi::c_int {
        sum = (sum as ::core::ffi::c_int + e[i as usize] as ::core::ffi::c_int) as uint8_t;
        i += 1;
    }
    e[127] =
        (256 as ::core::ffi::c_int - sum as ::core::ffi::c_int) as uint8_t;
    crate::src::logs::qemu(format_args!("ppc-mac-gpu: EDID blob initialized (checksum 0x{:02x})\n", ((e[127] as ::core::ffi::c_int) as u32)));
    if want_vulkan {
        s.renderer = q::vulkan_renderer();
        s.renderer_opaque = q::renderer_init(s.renderer_ref(), q::Vram::of(s), s.vram_size);
        if s.renderer_opaque.is_null() {
            return Err(RealizeError::Report(
                11400,
                format!("{}: renderer=vulkan unavailable: {}", tn, q::vulkan_error_text()),
            ));
        }
        crate::src::logs::qemu(format_args!("ppc-mac-gpu: using Vulkan renderer for 3D\n"));
    }
    if s.renderer.is_null() {
        return Err(RealizeError::Report(
            11426,
            format!("{}: no 3D renderer available: set renderer=vulkan (or renderer=metal on macOS)", tn),
        ));
    }
    q::renderer_set_dirty_source(s.renderer_ref(), s.renderer_opaque, opaque);
    q::pci_set_u8(crate::src::qemu::pci_raw(&mut s.pci), 0x8, 0);
    q::pci_set_u8(crate::src::qemu::pci_raw(&mut s.pci), 0x3d, 1);
    q::pci_set_u16(crate::src::qemu::pci_raw(&mut s.pci), 0x2c, 0x1002);
    q::pci_set_u16(crate::src::qemu::pci_raw(&mut s.pci), 0x2e, 0x4e44);
    if let Some(id) = std::env::var_os("PPCGPU_DEVICE_ID").filter(|v| !v.is_empty()) {
        q::pci_set_u16(crate::src::qemu::pci_raw(&mut s.pci), 0x2, strtoul_hex16(id.as_bytes()));
    }
    // AGP capability (id 0x2), through QEMU's safe `pci` crate.
    pci::PCIDeviceMethods::add_capability(&&*s, 0x2, 0x58, 12)
        .map_err(|e| RealizeError::Report(line!() as ::core::ffi::c_int, format!("{}: AGP capability: {}", tn, e)))?;
    q::pci_set_u8(crate::src::qemu::pci_raw(&mut s.pci), 0x58 + 2, 0x20);
    q::pci_set_u32(crate::src::qemu::pci_raw(&mut s.pci), 0x58 + 4, 0x200 | 0x4 | 0x2 | 0x1 | (0x1f << 24));
    q::pci_set_u32(crate::src::qemu::pci_raw(&mut s.pci), 0x58 + 8, 0x100 | 0x200 | 0x4 | (0x1f << 24));
    q::pci_set_wmask_u32(crate::src::qemu::pci_raw(&mut s.pci), 0x58 + 8, 0xffff_ffff);
    // Power management capability (id 0x1).
    pci::PCIDeviceMethods::add_capability(&&*s, 0x1, 0x50, 8)
        .map_err(|e| RealizeError::Report(line!() as ::core::ffi::c_int, format!("{}: PM capability: {}", tn, e)))?;
    q::pci_set_u16(crate::src::qemu::pci_raw(&mut s.pci), 0x50 + 2, 0x1 | (1 << 9) | (1 << 10));
    q::trace_realize(s.vram_size_mb, s.vram_size);
    Ok(())
}

pub extern "C" fn ppc_mac_gpu_config_read(
    dev: *mut PCIDevice,
    addr: uint32_t,
    len: ::core::ffi::c_int,
) -> uint32_t {
    let val: uint32_t = match crate::src::qemu::pci_device_mut(dev) {
        Some(d) => crate::src::qemu::pci_config_read(d, addr, len),
        None => 0,
    };
    crate::src::logs::debug(format_args!("CFG_RD off=0x{:02x} len={} val=0x{:x}", ((addr) as u32), ((len) as i32), ((val) as u32)));
    val
}

pub extern "C" fn ppc_mac_gpu_config_write(
    dev: *mut PCIDevice,
    addr: uint32_t,
    val: uint32_t,
    len: ::core::ffi::c_int,
) {
    crate::src::logs::debug(format_args!("CFG_WR off=0x{:02x} len={} val=0x{:x}", ((addr) as u32), ((len) as i32), ((val) as u32)));
    if let Some(d) = crate::src::qemu::pci_device_mut(dev) {
        crate::src::qemu::pci_config_write(d, addr, val, len);
    }
}

pub unsafe extern "C" fn ppc_mac_gpu_exit(dev: *mut PCIDevice) {
    // SAFETY: `dev` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(dev as *mut ::core::ffi::c_void) }) else {
        return;
    };
    crate::src::qemu::timer_delete(&mut s.vblank_timer);
    crate::src::qemu::shadow_free(s);
    crate::src::qemu::graphic_console_close(s);
}
/// Parses a trace property value: an optional leading '-' (turn the trace off), then a name
/// that must begin with POWEREMU_ or PPCGPU_. Returns (off, name).
fn parse_trace_name(value: Option<&[u8]>) -> Result<(bool, &[u8]), &'static str> {
    let value = value.ok_or("trace names must begin with POWEREMU_ or PPCGPU_")?;
    let (off, name) = match value.strip_prefix(b"-") {
        Some(rest) => (true, rest),
        None => (false, value),
    };
    if name.starts_with(b"POWEREMU_") || name.starts_with(b"PPCGPU_") {
        Ok((off, name))
    } else {
        Err("trace names must begin with POWEREMU_ or PPCGPU_")
    }
}

pub extern "C" fn ppc_mac_gpu_set_trace(
    _obj: *mut Object,
    value: *const ::core::ffi::c_char,
    errp: *mut *mut Error,
) {
    let value = crate::src::qemu::c_arg(value).map(|v| v.to_bytes());
    match parse_trace_name(value) {
        Err(msg) => crate::src::qemu::error_setg_at(
            errp,
            c"../../hw/display/ppc_mac_gpu.c",
            11574,
            c"ppc_mac_gpu_set_trace",
            msg,
        ),
        Ok((off, name)) => {
            crate::src::qemu::set_trace_env(name, !off);
            g_seq_log_enabled.set(-1);
            crate::src::logs::err(format_args!(
                "ppc-mac-gpu: {} {}\n",
                String::from_utf8_lossy(name),
                if off { "off" } else { "on" }
            ));
        }
    }
}

/// The trace switches read from the environment, in report order.
const TRACE_ENV_NAMES: [&::core::ffi::CStr; 8] = [
    c"POWEREMU_STALL_TRACE",
    c"POWEREMU_FENCE_TRACE",
    c"POWEREMU_TEX_TRACE",
    c"POWEREMU_VP_TRACE",
    c"POWEREMU_POLL_TRACE",
    c"PPCGPU_SEQ_LOG",
    c"PPCGPU_DIAG",
    c"PPCGPU_WINDOWS",
];

pub extern "C" fn ppc_mac_gpu_get_trace(
    _obj: *mut Object,
    _errp: *mut *mut Error,
) -> *mut ::core::ffi::c_char {
    let mut set: Vec<&str> = Vec::new();
    for name in TRACE_ENV_NAMES {
        if crate::src::qemu::env_flag(name) {
            set.push(crate::src::qemu::cs(name));
        }
    }
    let text = if set.is_empty() { "none".to_string() } else { set.join(" ") };
    crate::src::qemu::glib_strdup(&text)
}

pub unsafe extern "C" fn ppc_mac_gpu_post_load(
    opaque: *mut ::core::ffi::c_void,
    _version_id: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return 0;
    };
    s.display_invalid = true;
    s.disp.width = 0;
    s.disp.height = 0;
    s.surface_width = 0;
    s.surface_height = 0;
    s.surface_stride = 0;
    let vram_size = s.vram_size;
    gpu_vram_dirty(s, 0, vram_size);
    s.hwc_announce = true;
    crate::src::qemu::console_update_full(s);
    0
}

pub extern "C" fn ppc_mac_gpu_class_init(
    klass: *mut ObjectClass,
    _data: *const ::core::ffi::c_void,
) {
    let Some(k) = crate::src::qemu::pci_device_class(klass) else {
        return;
    };
    {
        use pci::PCIDeviceClassIds;
        k.set_vendor_id(0x1002);
        k.set_device_id(0x4e44);
        k.set_class_id(0x300);
    }
    k.realize = Some(ppc_mac_gpu_realize);
    k.exit = Some(ppc_mac_gpu_exit);
    k.config_read = Some(ppc_mac_gpu_config_read);
    k.config_write = Some(ppc_mac_gpu_config_write);
    let dc = &mut k.parent_class;
    crate::src::qemu::class_set_reset(dc, Some(ppc_mac_gpu_reset));
    dc.vmsd = &raw const vmstate_ppc_mac_gpu;
    crate::src::qemu::class_set_ppc_mac_gpu_props(dc);
    dc.hotpluggable = false;
    crate::src::qemu::set_bit_in(&mut dc.categories, DEVICE_CATEGORY_DISPLAY as usize);
    crate::src::qemu::class_add_trace_property(
        &mut dc.parent_class,
        Some(ppc_mac_gpu_get_trace),
        Some(ppc_mac_gpu_set_trace),
    );
}

pub extern "C" fn ppc_mac_gpu_register_types() {
    crate::src::qemu::register_device_types();
}
