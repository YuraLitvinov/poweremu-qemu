use super::*;
use crate::src::qemu::Vram;
pub fn ppc_mac_gpu_harmony_enable(on: bool) {
    pe_harm_on.set(on);
    pe_harm_gen.set(pe_harm_gen.get().wrapping_add(1));
}
pub fn ppc_mac_gpu_harmony_tiles(
    tiles: &mut *const uint8_t,
    cols: &mut ::core::ffi::c_int,
    rows: &mut ::core::ffi::c_int,
    generation: &mut uint32_t,
) -> bool {
    let harm = crate::src::qemu::harm_tiles_ptr();
    if !pe_harm_on.get() || harm.is_null() {
        return false;
    }
    *tiles = harm as *const uint8_t;
    *cols = pe_harm_cols.get();
    *rows = pe_harm_rows.get();
    *generation = pe_harm_gen.get();
    true
}
/// Register name for the access log, from the R300 crate's `REG_NAMES`.
pub fn ppc_mac_gpu_r3xx_reg_name(addr: hwaddr) -> Option<&'static ::core::ffi::CStr> {
    r300::state::reg_name((addr & !3) as u32)
}
pub fn ppc_mac_gpu_reg_name(addr: hwaddr) -> &'static ::core::ffi::CStr {
    match addr as ::core::ffi::c_ulonglong & !(3 as ::core::ffi::c_ulonglong) {
        0 => return c"MM_INDEX",
        4 => return c"MM_DATA",
        48 => return c"BUS_CNTL",
        64 => return c"GEN_INT_CNTL",
        68 => return c"GEN_INT_STATUS",
        8 => return c"CLOCK_CNTL_INDEX",
        12 => return c"CLOCK_CNTL_DATA",
        328 => return c"MC_FB_LOCATION",
        332 => return c"MC_AGP_LOCATION",
        336 => return c"MC_STATUS",
        256 => return c"CONFIG_APER_0_BASE",
        264 => return c"CONFIG_APER_SIZE",
        248 => return c"CONFIG_MEMSIZE",
        224 => return c"CONFIG_CNTL",
        3648 => return c"RBBM_STATUS",
        240 => return c"RBBM_SOFT_RESET",
        5952 => return c"GUI_STAT",
        304 => return c"HOST_PATH_CNTL",
        2816 => return c"SURFACE_CNTL",
        80 => return c"CRTC_GEN_CNTL",
        84 => return c"CRTC_EXT_CNTL",
        92 => return c"CRTC_STATUS",
        528 => {
            return c"CRTC_VLINE_CRNT_VLINE";
        }
        532 => return c"CRTC_CRNT_FRAME",
        512 => return c"CRTC_H_TOTAL_DISP",
        520 => return c"CRTC_V_TOTAL_DISP",
        548 => return c"CRTC_OFFSET",
        552 => return c"CRTC_OFFSET_CNTL",
        556 => return c"CRTC_PITCH",
        88 => return c"DAC_CNTL",
        124 => return c"DAC_CNTL2",
        176 => return c"PALETTE_INDEX",
        180 => return c"PALETTE_DATA",
        608 => return c"CUR_OFFSET",
        612 => return c"CUR_HORZ_VERT_POSN",
        616 => return c"CUR_HORZ_VERT_OFF",
        620 => return c"CUR_CLR0",
        624 => return c"CUR_CLR1",
        3328 => return c"DISP_MISC_CNTL",
        3428 => return c"DISP_OUTPUT_CNTL",
        3424 => return c"DISP_MERGE_CNTL",
        644 => return c"FP_GEN_CNTL",
        5124 => return c"DST_OFFSET",
        5128 => return c"DST_PITCH",
        5132 => return c"DST_WIDTH",
        5136 => return c"DST_HEIGHT",
        5180 => return c"DST_HEIGHT_WIDTH",
        5548 => return c"SRC_OFFSET",
        5552 => return c"SRC_PITCH",
        5228 => return c"DP_GUI_MASTER_CNTL",
        5824 => return c"DP_CNTL",
        5828 => return c"DP_DATATYPE",
        5832 => return c"DP_MIX",
        1792 => return c"CP_RB_BASE",
        1796 => return c"CP_RB_CNTL",
        1808 => return c"CP_RB_RPTR",
        1812 => return c"CP_RB_WPTR",
        134 => return c"CP_ME_CNTL",
        1848 => return c"CP_IB_BASE",
        1852 => return c"CP_IB_BUFSZ",
        1856 => return c"CP_CSQ_CNTL",
        1612 => return c"CP_STAT",
        1904 => return c"SCRATCH_UMSK",
        1908 => return c"SCRATCH_ADDR",
        5600 => return c"SCRATCH_REG0",
        5604 => return c"SCRATCH_REG1",
        5608 => return c"SCRATCH_REG2",
        5612 => return c"SCRATCH_REG3",
        5616 => return c"SCRATCH_REG4",
        5620 => return c"SCRATCH_REG5",
        2004 => return c"CP_ME_RAM_ADDR",
        2012 => return c"CP_ME_RAM_DATAH",
        2016 => return c"CP_ME_RAM_DATAL",
        2032 => return c"CP_CSQ_IND_ADDR",
        2036 => return c"CP_CSQ_IND_DATA",
        2040 => return c"CP_CSQ_STAT",
        2044 => return c"CP_CSQ2_STAT",
        5924 => return c"ISYNC_CNTL",
        5920 => return c"WAIT_UNTIL",
        368 => return c"AGP_BASE",
        372 => return c"AGP_CNTL",
        3936 => return c"AGP_COMMAND",
        3932 => return c"AGP_STATUS",
        96 => return c"GPIO_VGA_DDC",
        100 => return c"GPIO_DVI_DDC",
        104 => return c"GPIO_MONID",
        164 => return c"UNNAMED_00A4",
        852 => return c"UNNAMED_0354",
        _ => {
            if let Some(n) = ppc_mac_gpu_post_reg_name(addr as uint32_t) {
                return n;
            }
            if addr & !(3 as ::core::ffi::c_uint) as hwaddr == 0xe4 as hwaddr {
                return c"CONFIG_XSTRAP";
            }
            if addr & !(3 as ::core::ffi::c_uint) as hwaddr == 0xe8 as hwaddr {
                return c"CONFIG_BONDS";
            }
            if addr >= 0x10 as hwaddr && addr <= 0x2c as hwaddr {
                return c"BIOS_SCRATCH";
            }
            if addr >= 0x17c0 as hwaddr && addr <= 0x17e0 as hwaddr {
                return c"HOST_DATA";
            }
            if addr >= 0xf00 as hwaddr && addr < 0xf40 as hwaddr {
                return c"PCI_MIRROR";
            }
            if let Some(n) = ppc_mac_gpu_r3xx_reg_name(addr) {
                return n;
            }
            return c"UNKNOWN";
        }
    };
}
pub fn ppc_mac_gpu_i2c(
    i2c: &mut bitbang_i2c_interface,
    mut data: uint64_t,
    base: ::core::ffi::c_int,
) -> uint64_t {
    let c: bool = if data as ::core::ffi::c_ulong
        & (1 as ::core::ffi::c_ulong) << base + 17 as ::core::ffi::c_int
        != 0
    {
        (data as ::core::ffi::c_ulong
            & (1 as ::core::ffi::c_ulong) << base + 1 as ::core::ffi::c_int
            != 0) as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    } != 0;
    let mut d: bool = if data as ::core::ffi::c_ulong
        & (1 as ::core::ffi::c_ulong) << base + 16 as ::core::ffi::c_int
        != 0
    {
        (data as ::core::ffi::c_ulong & (1 as ::core::ffi::c_ulong) << base != 0)
            as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    } != 0;
    crate::src::qemu::bitbang_line_set(i2c, 1 as ::core::ffi::c_int, c as ::core::ffi::c_int);
    d = crate::src::qemu::bitbang_line_set(i2c, 0 as ::core::ffi::c_int, d as ::core::ffi::c_int) != 0;
    data = (data as ::core::ffi::c_ulonglong & !(0xf00 as ::core::ffi::c_ulonglong)) as uint64_t;
    if c {
        data |= (1 as ::core::ffi::c_ulong) << base + 9 as ::core::ffi::c_int;
    }
    if d {
        data |= (1 as ::core::ffi::c_ulong) << base + 8 as ::core::ffi::c_int;
    }
    return data;
}
pub fn ppc_mac_gpu_reg_read_offs(
    reg: uint32_t,
    offs: ::core::ffi::c_int,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    if offs == 0 && size == 4 {
        return uint64_t::from(reg);
    }
    // extract32(reg, offs * 8, size * 8): bit field at `start` of width `len`.  An out-of-range
    // start yields 0; a width of 32 or more keeps every bit above `start`.
    let start: u32 = (offs as u32).wrapping_mul(8);
    let len: u32 = size.wrapping_mul(8);
    let shifted: u32 = reg.checked_shr(start).unwrap_or(0);
    let mask: u32 = 1u32.checked_shl(len).map_or(u32::MAX, |b| b - 1);
    uint64_t::from(shifted & mask)
}
pub fn ppc_mac_gpu_reg_write_offs(
    reg: &mut uint32_t,
    offs: ::core::ffi::c_int,
    data: uint64_t,
    size: ::core::ffi::c_uint,
) {
    if offs == 0 as ::core::ffi::c_int && size == 4 as ::core::ffi::c_uint {
        *reg = data as uint32_t;
    } else {
        *reg = crate::src::qemu::deposit_field(
            *reg,
            offs * 8 as ::core::ffi::c_int,
            size.wrapping_mul(8 as ::core::ffi::c_uint) as ::core::ffi::c_int,
            data as uint32_t,
        );
    };
}
pub fn ppc_mac_gpu_update_irq(s: &mut PPCMacGPUState) {
    let level: bool = s.regs.gen_int_status & s.regs.gen_int_cntl != 0;
    crate::src::qemu::pci_irq_set(&mut s.pci, level);
    crate::src::qemu::trace_irq_event(s.regs.gen_int_status, s.regs.gen_int_cntl, level);
}
pub unsafe extern "C" fn ppc_mac_gpu_vblank(opaque: *mut ::core::ffi::c_void) {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return;
    };
    let now: int64_t = crate::src::qemu::clock_virtual_ns();
    let frame: int64_t =
        (1000000000 as ::core::ffi::c_longlong / 60 as ::core::ffi::c_longlong) as int64_t;
    crate::src::qemu::timer_arm(&mut s.vblank_timer, (now / frame + 1 as int64_t) * frame);
    s.regs.gen_int_status |= ((1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int) as uint32_t;
    s.regs.stall_irqs = s.regs.stall_irqs.wrapping_add(1);
    s.regs.crtc_status ^= ((1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int) as uint32_t;
    ppc_mac_gpu_update_irq(s);
}
/// Bytes per pixel for a bit depth, rounded up (the C `(bpp + 7) / 8`).
fn bytes_of_bpp(bpp: uint32_t) -> uint32_t {
    bpp.wrapping_add(7).wrapping_div(8)
}

/// Whether two display modes agree field by field.  The C code compared the structs with memcmp;
/// both sides are copies of one struct, so this is the same test without comparing padding.
fn mode_eq(a: &PPCMacGPUDisplayMode, b: &PPCMacGPUDisplayMode) -> bool {
    a.width == b.width
        && a.height == b.height
        && a.bpp == b.bpp
        && a.stride == b.stride
        && a.offset == b.offset
        && a.format == b.format
        && a.crtc_ext == b.crtc_ext
}

/// Derives the display mode from the CRTC registers and the stride override.  Returns true when
/// the mode changed.
pub fn ppc_mac_gpu_update_display_mode(s: &mut PPCMacGPUState) -> bool {
    let old: PPCMacGPUDisplayMode = s.disp;
    let crtc_gen: uint32_t = s.regs.crtc_gen_cntl;
    let h_total_disp: uint32_t = s.regs.crtc_h_total_disp;
    let v_total_disp: uint32_t = s.regs.crtc_v_total_disp;
    let pitch: uint32_t = s.regs.crtc_pitch;
    let fmt32 = crate::src::qemu::guest_fmt32();
    let fmt24 = crate::src::qemu::guest_fmt24();
    let m: &mut PPCMacGPUDisplayMode = &mut s.disp;
    if crtc_gen & (1 << 25) == 0 && crtc_gen & (1 << 24) == 0 {
        if m.width == 0 {
            m.width = 640;
            m.height = 480;
            m.bpp = 32;
            m.stride = 640 * 4;
            m.offset = 0;
            m.format = fmt32;
        }
        m.crtc_ext = false;
        return false;
    }
    m.crtc_ext = true;
    m.width = ((h_total_disp >> 16) & 0xff).wrapping_add(1).wrapping_mul(8);
    m.height = ((v_total_disp >> 16) & 0x7ff).wrapping_add(1);
    if m.width < 64 || m.width > 4096 || m.height < 64 || m.height > 4096 {
        m.width = 640;
        m.height = 480;
    }
    match crtc_gen & (0xf << 8) {
        512 => {
            m.bpp = 8;
            m.format = fmt32;
        }
        768 => {
            m.bpp = 15;
            m.format = PIXMAN_x1r5g5b5;
        }
        1024 => {
            m.bpp = 16;
            m.format = PIXMAN_r5g6b5;
        }
        1280 => {
            m.bpp = 24;
            m.format = fmt24;
        }
        1536 => {
            m.bpp = 32;
            m.format = fmt32;
        }
        _ => {
            m.bpp = 32;
            m.format = fmt32;
        }
    }
    m.stride = if pitch & 0x7ff != 0 {
        (pitch & 0x7ff)
            .wrapping_mul(8)
            .wrapping_mul(bytes_of_bpp(m.bpp))
    } else {
        m.width.wrapping_mul(bytes_of_bpp(m.bpp))
    };
    if s.disp_stride_override_active
        && old.width != 0
        && old.height != 0
        && (old.width != m.width || old.height != m.height || old.bpp != m.bpp)
    {
        crate::src::logs::err(format_args!(
            "[STRIDE_CHANGE] mode change {}x{}->{}x{}: clearing stride override (was {})\n",
            old.width, old.height, m.width, m.height, s.disp_stride_override_value
        ));
        s.disp_stride_override_active = false;
        s.disp_stride_override_value = 0;
    }
    if s.disp_stride_override_active && s.disp_stride_override_value > 0 {
        if m.stride != s.disp_stride_override_value {
            static stride_override_log: crate::src::global::Global<core::ffi::c_int> =
                crate::src::global::Global::<core::ffi::c_int>::new(0);
            if stride_override_log.get() < 20 {
                crate::src::logs::err(format_args!(
                    "[STRIDE_CHANGE] update_display_mode: CRTC gives {}, override forces {}\n",
                    m.stride, s.disp_stride_override_value
                ));
                stride_override_log.set(stride_override_log.get() + 1);
            }
        }
        m.stride = s.disp_stride_override_value;
    }
    let min_stride = m.width.wrapping_mul(bytes_of_bpp(m.bpp));
    if m.stride < min_stride {
        static small_stride_logs: crate::src::global::Global<core::ffi::c_int> =
            crate::src::global::Global::<core::ffi::c_int>::new(0);
        let seen = small_stride_logs.get();
        small_stride_logs.set(seen + 1);
        if seen < 5 {
            crate::src::logs::err(format_args!(
                "[STRIDE_CHANGE] stride {} too small for {}x{}@{}, using {}\n",
                m.stride, m.width, m.height, m.bpp, min_stride
            ));
        }
        m.stride = min_stride;
    }
    m.offset = s.regs.crtc_offset;
    if u64::from(m.offset).wrapping_add(u64::from(m.stride).wrapping_mul(u64::from(m.height)))
        > s.vram_size
    {
        m.offset = 0;
    }
    if !mode_eq(&old, m) {
        if old.width != m.width
            || old.height != m.height
            || old.bpp != m.bpp
            || old.stride != m.stride
            || old.crtc_ext != m.crtc_ext
        {
            crate::src::logs::err(format_args!(
                "GPU MODE: {}x{} bpp={} stride={}{} (crtc_gen=0x{:x})\n",
                m.width,
                m.height,
                m.bpp,
                m.stride,
                crate::src::qemu::cstr(if m.crtc_ext {
                    c"".as_ptr()
                } else {
                    c" [CRTC not in extended mode]".as_ptr()
                }),
                crtc_gen
            ));
        }
        if old.stride != m.stride {
            crate::src::logs::err(format_args!(
                "[STRIDE_CHANGE] update_display_mode: {} -> {} (override={} val={})\n",
                old.stride,
                m.stride,
                if s.disp_stride_override_active { "yes" } else { "no" },
                s.disp_stride_override_value
            ));
        }
        return true;
    }
    false
}
pub fn ppc_mac_gpu_vga_bank_off(
    s: &mut PPCMacGPUState,
    a: hwaddr,
    write: bool,
) -> uint64_t {
    let sel: uint32_t = s.r300_shadow[((if write as ::core::ffi::c_int != 0 {
        0x38 as ::core::ffi::c_int
    } else {
        0x3c as ::core::ffi::c_int
    }) / 4 as ::core::ffi::c_int) as usize];
    let blk: uint32_t = if a & 0x8000 as hwaddr != 0 {
        sel >> 16 as ::core::ffi::c_int
    } else {
        sel & 0xffff as uint32_t
    };
    let unit: uint32_t = (if s.regs.crtc_ext_cntl & 8 as uint32_t != 0 {
        0x8000 as ::core::ffi::c_int
    } else {
        0x2000 as ::core::ffi::c_int
    }) as uint32_t;
    return (blk as uint64_t)
        .wrapping_mul(unit as uint64_t)
        .wrapping_add(a as uint64_t & 0x7fff as uint64_t);
}
pub unsafe extern "C" fn ppc_mac_gpu_vga_bank_read(
    opaque: *mut ::core::ffi::c_void,
    addr: hwaddr,
    size: ::core::ffi::c_uint,
) -> uint64_t {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return 0;
    };
    let vram = crate::src::qemu::Vram::of(s);
    let mut v: uint64_t = 0 as uint64_t;
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < size {
        let off: uint64_t = ppc_mac_gpu_vga_bank_off(
            s,
            addr.wrapping_add(i as hwaddr),
            0 as ::core::ffi::c_int != 0,
        );
        if off < s.vram_size {
            v |= ((vram.get8(off as u64).unwrap_or(0)) as uint64_t)
                << (8 as ::core::ffi::c_uint).wrapping_mul(i);
        }
        i = i.wrapping_add(1);
    }
    return v;
}
pub unsafe extern "C" fn ppc_mac_gpu_vga_bank_write(
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
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < size {
        let off: uint64_t = ppc_mac_gpu_vga_bank_off(
            s,
            addr.wrapping_add(i as hwaddr),
            1 as ::core::ffi::c_int != 0,
        );
        if off < s.vram_size {
            vram.put8(off as u64, ((val >> (8 as ::core::ffi::c_uint).wrapping_mul(i)) as uint8_t) as u8);
            gpu_vram_dirty(s, off, 1 as uint64_t);
        }
        i = i.wrapping_add(1);
    }
}
pub fn ppc_mac_gpu_vga_bank_update(s: &mut PPCMacGPUState) {
    if s.legacy_vga {
        let on: bool = s.regs.crtc_gen_cntl
            & ((1 as ::core::ffi::c_int) << 24 as ::core::ffi::c_int) as uint32_t
            != 0 as uint32_t;
        if !crate::src::qemu::getenv(c"PPCGPU_VGA_LOG").is_null() {
            crate::src::logs::err(format_args!("VGA bank window {} (crtc_gen={:08x})\n", crate::src::qemu::cstr(if on as ::core::ffi::c_int != 0 {
                    c"on".as_ptr()
                } else {
                    c"off".as_ptr()
                }), ((s.regs.crtc_gen_cntl) as u32)));
        }
        crate::src::qemu::mr_set_enabled(&mut s.vga_bank, on);
    }
}
/// Little-endian scanout pixel to 0xFFRRGGBB (the C `ppc_mac_gpu_px16_argb`): 565 or 555.
fn px16_argb(v: u32, is565: bool) -> u32 {
    let (r, g, b) = if is565 {
        let (r, g, b) = ((v >> 11) & 31, (v >> 5) & 63, v & 31);
        ((r << 3) | (r >> 2), (g << 2) | (g >> 4), b)
    } else {
        let (r, g, b) = ((v >> 10) & 31, (v >> 5) & 31, v & 31);
        ((r << 3) | (r >> 2), (g << 3) | (g >> 2), b)
    };
    let b = (b << 3) | (b >> 2);
    0xff00_0000 | (r << 16) | (g << 8) | b
}

/// Converts one row of little-endian 16-bit scanout pixels into 32-bit shadow pixels.
fn scan16_le(src: &[u8], dst: &mut [u8], is565: bool) {
    for (d, px) in dst.chunks_exact_mut(4).zip(src.chunks_exact(2)) {
        let v = u32::from(px[0]) | (u32::from(px[1]) << 8);
        d.copy_from_slice(&px16_argb(v, is565).to_ne_bytes());
    }
}

/// Shadow bytes for a `width` x `height` frame of 32-bit pixels, if they fit the 32-bit size field.
fn shadow_need(width: uint32_t, height: uint32_t) -> Option<u64> {
    u64::from(width)
        .checked_mul(4)?
        .checked_mul(u64::from(height))
        .filter(|&n| n <= u64::from(u32::MAX))
}

/// Sums of non-zero, bright and zero pixels in `n` pixels of VRAM starting at byte `base`, from
/// column `x_start` (the drag diagnostic).
fn drag_sample(vram: Vram, base: u64, x_start: uint32_t, n: uint32_t) -> (uint32_t, uint32_t, uint32_t) {
    let (mut nz, mut bright, mut zero) = (0u32, 0u32, 0u32);
    for x in x_start..x_start.wrapping_add(n) {
        let px = vram
            .get32(base + 4 * u64::from(x))
            .unwrap_or(0)
            .swap_bytes();
        if px == 0 {
            zero += 1;
        } else {
            nz += 1;
            let r = (px >> 16) & 0xff;
            let g = (px >> 8) & 0xff;
            let b = px & 0xff;
            if r > 200 && g > 200 && b > 200 {
                bright += 1;
            }
        }
    }
    (nz, bright, zero)
}

/// C-ABI display-change callback: recovers the device state and runs one display update.
pub unsafe extern "C" fn ppc_mac_gpu_display_update(opaque: *mut ::core::ffi::c_void) -> bool {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return false;
    };
    display_update(s)
}

/// One display update: legacy VGA hand-off, cursor, mode change, and the copy of the scanout into
/// the shadow surface.  Guest-controlled sizes and offsets are checked before any allocation or
/// VRAM access.
fn display_update(s: &mut PPCMacGPUState) -> bool {
    use crate::src::qemu as q;
    if s.legacy_vga {
        if s.regs.crtc_gen_cntl & (1 << 24) == 0 || q::env_flag(c"PPCGPU_FORCE_VGA") {
            if !s.vga_shown {
                s.vga_shown = true;
                let v = s.vga_mut();
                v.graphic_mode = -1;
                v.last_height = 0;
                v.last_width = v.last_height;
                v.last_scr_height = 0;
                v.last_scr_width = v.last_scr_height;
                v.last_depth = 0;
            }
            if q::env_flag(c"PPCGPU_VGA_LOG") {
                static last: crate::src::global::Global<int64_t> =
                    crate::src::global::Global::<int64_t>::new(0);
                let now = q::clock_realtime_ms();
                if now - last.get() > 2000 {
                    last.set(now);
                    let v = s.vga_ref();
                    crate::src::logs::err(format_args!(
                        "VGA core: crtc_gen={:08x} ext_cntl={:08x} ar_index={:02x} sr1={:02x} gr6={:02x} cr17={:02x} gmode={} size={}x{} depth={}\n",
                        s.regs.crtc_gen_cntl,
                        s.regs.crtc_ext_cntl,
                        v.ar_index,
                        v.sr[1],
                        v.gr[6],
                        v.cr[0x17],
                        v.graphic_mode,
                        v.last_width,
                        v.last_height,
                        v.last_depth
                    ));
                }
            }
            q::vga_gfx_update(s);
            return true;
        }
        if s.vga_shown {
            s.vga_shown = false;
            s.disp.stride = 0;
            s.disp.height = 0;
            s.disp.width = 0;
            s.display_invalid = true;
        }
    }
    if s.hwc_announce && !s.con.is_null() {
        s.hwc_announce = false;
        if s.hwc_w != 0 && s.hwc_h != 0 {
            q::cursor_publish_hwc(s);
        }
        q::console_set_mouse(s);
    }
    let why = (0x4000 as uint32_t).wrapping_add(GPU_WHY_DISPLAY as ::core::ffi::c_int as uint32_t);
    if s.disp.stride != 0 && s.disp.height != 0 {
        let lo = u64::from(s.regs.crtc_offset);
        let hi = lo.wrapping_add(u64::from(s.disp.stride).wrapping_mul(u64::from(s.disp.height)));
        if q::renderer_range_busy_sync(s, lo, hi, false) {
            gpu_sync_at(s, why);
        }
    } else {
        gpu_sync_at(s, why);
    }
    let mut mode_changed = ppc_mac_gpu_update_display_mode(s);
    s.metal_rt_count = 0;
    s.metal_rt_frame_id = s.metal_rt_frame_id.wrapping_add(1);
    if s.disp.width == 0 || s.disp.height == 0 {
        static zero_count: crate::src::global::Global<core::ffi::c_int> =
            crate::src::global::Global::<core::ffi::c_int>::new(0);
        if zero_count.get() < 3 {
            crate::src::logs::err(format_args!(
                "GPU DIAG: display_update called but w={} h={}, skipping\n",
                s.disp.width, s.disp.height
            ));
            zero_count.set(zero_count.get() + 1);
        }
        return true;
    }
    let vram = Vram::of(s);
    let vram_size = s.vram_size;
    let width = s.disp.width;
    let height = s.disp.height;
    let stride = s.disp.stride;
    let off = u64::from(s.disp.offset);
    // Whole frame inside VRAM: every row read below is then in bounds.
    let frame_bytes = u64::from(stride) * u64::from(height);
    let fits = off
        .checked_add(frame_bytes)
        .is_some_and(|end| end <= vram_size);
    static phase_d_frame: crate::src::global::Global<core::ffi::c_int> =
        crate::src::global::Global::<core::ffi::c_int>::new(0);
    phase_d_frame.set(phase_d_frame.get() + 1);
    let pd = phase_d_frame.get();
    if (pd == 500 || pd == 700 || pd == 1000 || pd == 1500)
        && q::env_flag(c"PPCGPU_TILE_CHECK")
    {
        let mut nonzero_count: core::ffi::c_int = 0;
        let mut first_nz_off: uint32_t = 0;
        let mut first_nz_val: uint32_t = 0;
        let mut a: u64 = 0x353000;
        while a < 0x413000 {
            if a.wrapping_add(4) <= vram_size {
                let v = vram.get32(a).unwrap_or(0);
                if v != 0 {
                    if nonzero_count == 0 {
                        first_nz_off = a as uint32_t;
                        first_nz_val = v;
                    }
                    nonzero_count += 1;
                }
            }
            a += 64;
        }
        crate::src::logs::err(format_args!(
            "[TILE_CHECK_FRAME] frame={} range=0x353000-0x413000 nonzero_samples={}/{} first_nz=0x{:x}(0x{:08x})\n",
            pd,
            nonzero_count,
            (0x413000 - 0x353000) / 64,
            first_nz_off,
            first_nz_val
        ));
    }
    let bpp_bytes = bytes_of_bpp(s.disp.bpp);
    if bpp_bytes == 2 {
        let Some(need) = shadow_need(width, height) else {
            return true;
        };
        if q::shadow_ensure(s, need) {
            mode_changed = true;
        }
        if mode_changed || s.display_invalid {
            s.display_invalid = false;
            q::console_set_shadow_surface(s, width, height, PIXMAN_x8r8g8b8, width.wrapping_mul(4));
        }
        let is565 = s.disp.bpp == 16;
        if fits {
            let mut row = vec![0u8; width as usize * 2];
            let shadow = q::shadow_bytes(s);
            for y in 0..u64::from(height) {
                if !vram.read_bytes(off + y * u64::from(stride), &mut row) {
                    break;
                }
                let dst_off = (y * u64::from(width) * 4) as usize;
                let Some(dst) = shadow.get_mut(dst_off..dst_off + width as usize * 4) else {
                    break;
                };
                scan16_le(&row, dst, is565);
            }
        }
        q::console_update_full(s);
        return true;
    }
    if s.disp.bpp == 8 {
        let Some(need) = shadow_need(width, height) else {
            return true;
        };
        if q::shadow_ensure(s, need) {
            mode_changed = true;
        }
        if mode_changed || s.display_invalid {
            s.display_invalid = false;
            q::console_set_shadow_surface(s, width, height, PIXMAN_x8r8g8b8, width.wrapping_mul(4));
        }
        let mut pal = [0u32; 256];
        for (i, p) in pal.iter_mut().enumerate() {
            *p = gpu_dac_get(s, i as u32);
        }
        if fits {
            let mut src_row = vec![0u8; width as usize];
            let shadow = q::shadow_bytes(s);
            for y in 0..u64::from(height) {
                if !vram.read_bytes(off + y * u64::from(stride), &mut src_row) {
                    break;
                }
                let dst_off = (y * u64::from(width) * 4) as usize;
                let Some(dst) = shadow.get_mut(dst_off..dst_off + width as usize * 4) else {
                    break;
                };
                for (d, &idx) in dst.chunks_exact_mut(4).zip(src_row.iter()) {
                    d.copy_from_slice(&pal[usize::from(idx)].to_ne_bytes());
                }
            }
        }
        q::console_update_full(s);
        return true;
    }
    if bpp_bytes != 4 {
        static non32_count: crate::src::global::Global<core::ffi::c_int> =
            crate::src::global::Global::<core::ffi::c_int>::new(0);
        if non32_count.get() < 3 {
            crate::src::logs::err(format_args!(
                "GPU DIAG: non-32bpp path, bpp={} w={} h={}\n",
                s.disp.bpp, width, height
            ));
            non32_count.set(non32_count.get() + 1);
        }
        if mode_changed || s.display_invalid {
            s.display_invalid = false;
            if fits {
                q::console_set_vram_surface(s, width, height, s.disp.format, stride, off);
            }
        }
        q::console_update_full(s);
        return true;
    }
    static trap_frame: crate::src::global::Global<uint32_t> =
        crate::src::global::Global::<uint32_t>::new(0);
    trap_frame.set(trap_frame.get().wrapping_add(1));
    if trap_frame.get() == 300 {
        crate::src::logs::err(format_args!("[VRAM_WRITE_TRAP] watching 4 RT offsets + framebuffer\n"));
    }
    // 32-bit path.  A frame that does not lie wholly inside VRAM is not drawn.
    if !fits || frame_bytes > u64::from(u32::MAX) {
        return true;
    }
    let mut surface_replaced = false;
    if q::shadow_ensure(s, frame_bytes) {
        mode_changed = true;
    }
    if mode_changed
        || width != s.surface_width
        || height != s.surface_height
        || stride != s.surface_stride
        || s.shadow_buf.cast::<::core::ffi::c_void>() != s.surface_data
    {
        surface_replaced = true;
        s.display_invalid = false;
        s.surface_width = width;
        s.surface_height = height;
        s.surface_stride = stride;
        s.surface_data = s.shadow_buf.cast::<::core::ffi::c_void>();
        q::trace_mode_change(width, height, s.disp.bpp, stride);
        q::console_set_shadow_surface(s, width, height, PIXMAN_x8r8g8b8, stride);
    } else {
        s.display_invalid = false;
    }
    if let Some(d) = q::renderer_drag_state(s) {
        static prev_diag_ox: crate::src::global::Global<uint32_t> =
            crate::src::global::Global::<uint32_t>::new(0);
        static prev_diag_oy: crate::src::global::Global<uint32_t> =
            crate::src::global::Global::<uint32_t>::new(0);
        static prev_diag_gen: crate::src::global::Global<uint32_t> =
            crate::src::global::Global::<uint32_t>::new(0);
        static prev_diag_valid: crate::src::global::Global<bool> =
            crate::src::global::Global::<bool>::new(false);
        static diag_log: crate::src::global::Global<core::ffi::c_int> =
            crate::src::global::Global::<core::ffi::c_int>::new(0);
        if prev_diag_valid.get()
            && d.generation != prev_diag_gen.get()
            && (d.ox != prev_diag_ox.get() || d.oy != prev_diag_oy.get())
        {
            let margin: uint32_t = 50;
            let sample_y_rel = d.h / 2;
            let sample_w: uint32_t = 100;
            let new_y = d.oy.wrapping_add(sample_y_rel);
            let new_x_start = d.ox.wrapping_add(margin);
            let (new_nz, new_bright, new_zero) =
                if new_y < height && new_x_start.wrapping_add(sample_w) <= width {
                    drag_sample(
                        vram,
                        off + u64::from(new_y.wrapping_mul(stride)),
                        new_x_start,
                        sample_w,
                    )
                } else {
                    (0, 0, 0)
                };
            let old_y = prev_diag_oy.get().wrapping_add(sample_y_rel);
            let old_x_start = prev_diag_ox.get().wrapping_add(margin);
            let (old_nz, old_bright, old_zero) =
                if old_y < height && old_x_start.wrapping_add(sample_w) <= width {
                    drag_sample(
                        vram,
                        off + u64::from(old_y.wrapping_mul(stride)),
                        old_x_start,
                        sample_w,
                    )
                } else {
                    (0, 0, 0)
                };
            if diag_log.get() < 40 {
                crate::src::logs::err(format_args!(
                    "[DRAG_BODY_CHECK] gen={} old=({},{}) new=({},{}) OLD_body: nz={} bright={} zero={} | NEW_body: nz={} bright={} zero={} | IOFBBlit={}\n",
                    d.generation,
                    prev_diag_ox.get(),
                    prev_diag_oy.get(),
                    d.ox,
                    d.oy,
                    old_nz,
                    old_bright,
                    old_zero,
                    new_nz,
                    new_bright,
                    new_zero,
                    q::cstr(if new_bright > 50 {
                        c"LIKELY_YES".as_ptr()
                    } else {
                        c"NO".as_ptr()
                    })
                ));
                diag_log.set(diag_log.get() + 1);
            }
        }
        prev_diag_ox.set(d.ox);
        prev_diag_oy.set(d.oy);
        prev_diag_gen.set(d.generation);
        prev_diag_valid.set(true);
    }
    q::renderer_flush_drag_paste(s, vram);
    // Copy each scanline of VRAM into the shadow buffer, recording the rows that changed.
    let stride_u32 = stride.wrapping_div(4);
    let row_bytes = u64::from(stride_u32) * 4;
    let row_len = width as usize * 4;
    let mut tmp = vec![0u8; row_len];
    let mut lo: core::ffi::c_int = -1;
    let mut hi: core::ffi::c_int = -1;
    {
        let shadow = q::shadow_bytes(s);
        for y in 0..height {
            let src_off = off + u64::from(y) * row_bytes;
            if !vram.read_bytes(src_off, &mut tmp) {
                break;
            }
            let dst_off = (u64::from(y) * row_bytes) as usize;
            let Some(row) = shadow.get_mut(dst_off..dst_off + row_len) else {
                break;
            };
            if surface_replaced || row[..] != tmp[..] {
                row.copy_from_slice(&tmp);
                if lo < 0 {
                    lo = y as core::ffi::c_int;
                }
                hi = y as core::ffi::c_int + 1;
            }
        }
    }
    // Hardware cursor overlay: a 64x64 ARGB sprite blended over the shadow rows.
    if s.hwc_visible && s.hwc_w == 64 {
        let cx = s.hwc_x;
        let cy = s.hwc_y;
        let pix = s.hwc_pix;
        let y0 = cy.max(0);
        let y1 = cy.saturating_add(64).min(height as core::ffi::c_int);
        let x0 = cx.max(0);
        let x1 = cx.saturating_add(64).min(width as core::ffi::c_int);
        if y0 < y1 && x0 < x1 {
            let shadow = q::shadow_bytes(s);
            for y in y0..y1 {
                let row_base = (y as usize) * (row_bytes as usize);
                for x in x0..x1 {
                    let Some(&c) = pix.get(((y - cy) * 64 + (x - cx)) as usize) else {
                        continue;
                    };
                    let a = c >> 24;
                    if a == 0 {
                        continue;
                    }
                    let at = row_base + x as usize * 4;
                    let Some(px) = shadow.get_mut(at..at + 4) else {
                        continue;
                    };
                    let d = u32::from_ne_bytes([px[0], px[1], px[2], px[3]]);
                    let inv = 255 - a;
                    let mix = |shift: u32| -> u32 {
                        let cc = (c >> shift) & 255;
                        let dd = (d >> shift) & 255;
                        (cc + (dd * inv + 127) / 255).min(255) << shift
                    };
                    let v = mix(16) | mix(8) | mix(0);
                    px.copy_from_slice(&v.to_ne_bytes());
                }
            }
            if lo < 0 || y0 < lo {
                lo = y0;
            }
            if y1 > hi {
                hi = y1;
            }
        }
    }
    if surface_replaced {
        q::console_update_full(s);
        q::fps_count(s, height);
    } else if lo >= 0 {
        q::console_update_rect(s, 0, lo, width as core::ffi::c_int, hi - lo);
        q::fps_count(s, (hi - lo) as uint32_t);
    }
    true
}
pub fn ppc_mac_gpu_scratch_writeback_val(
    s: &mut PPCMacGPUState,
    reg_idx: ::core::ffi::c_int,
    wb_val: uint32_t,
) {
    if reg_idx < 0 as ::core::ffi::c_int || reg_idx > 5 as ::core::ffi::c_int {
        return;
    }
    if s.regs.scratch_umsk & ((1 as ::core::ffi::c_int) << reg_idx) as uint32_t == 0 {
        return;
    }
    let wb_addr: uint32_t = s
        .regs
        .scratch_addr
        .wrapping_add((reg_idx * 4 as ::core::ffi::c_int) as uint32_t);
    let mut phys: hwaddr = 0;
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let wire_val: uint32_t = if !crate::src::qemu::getenv(c"POWEREMU_WB_BE").is_null() {
        wb_val.to_be()
    } else {
        wb_val.to_le()
    };
    if ppc_mac_gpu_gart_translate(s, wb_addr, &mut phys) as ::core::ffi::c_int != 0
        || ppc_mac_gpu_agp_translate(s, wb_addr, &mut phys) as ::core::ffi::c_int != 0
    {
        crate::src::qemu::guest_write32(phys as u64, wire_val);
        crate::src::logs::debug(format_args!("SCRATCH_WB reg{}=0x{:x} -> GART[0x{:x}] phys=0x{:x}", ((reg_idx) as i32), ((wb_val) as u32), ((wb_addr) as u32), ((phys) as u64)));
    } else if wb_addr >= fb_base
        && wb_addr.wrapping_add(4 as uint32_t) as uint64_t
            <= (fb_base as uint64_t).wrapping_add(s.vram_size)
    {
        let off: uint32_t = wb_addr.wrapping_sub(fb_base);
        gpu_vram_access(
            s,
            off as uint64_t,
            off.wrapping_add(4 as uint32_t) as uint64_t,
            1 as ::core::ffi::c_int != 0,
        );
        // Stores the dword's native-endian bytes at `off`, as the former memcpy did.
        crate::src::qemu::Vram::of(s).write_bytes(off as u64, &wire_val.to_ne_bytes());
        gpu_vram_dirty(s, off as uint64_t, 4 as uint64_t);
        crate::src::logs::debug(format_args!("SCRATCH_WB reg{}=0x{:x} -> VRAM[0x{:x}]", ((reg_idx) as i32), ((wb_val) as u32), ((off) as u32)));
    } else {
        crate::src::qemu::guest_write32(wb_addr as hwaddr as u64, wire_val);
        crate::src::logs::debug(format_args!("SCRATCH_WB reg{}=0x{:x} -> memory[0x{:x}]", ((reg_idx) as i32), ((wb_val) as u32), ((wb_addr) as u32)));
    };
}
pub fn ppc_mac_gpu_rptr_writeback(s: &mut PPCMacGPUState) {
    let addr: uint32_t = s.regs.cp_rb_rptr_addr & !(3 as uint32_t);
    let mut phys: hwaddr = 0;
    if addr == 0
        || s.regs.cp_rb_cntl as ::core::ffi::c_uint
            & (1 as ::core::ffi::c_uint) << 27 as ::core::ffi::c_int
            != 0
    {
        return;
    }
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let wire_val: uint32_t = if !crate::src::qemu::getenv(c"POWEREMU_WB_BE").is_null() {
        s.regs.cp_rb_rptr.to_be()
    } else {
        s.regs.cp_rb_rptr.to_le()
    };
    if !ppc_mac_gpu_gart_translate(s, addr, &mut phys)
        && !ppc_mac_gpu_agp_translate(s, addr, &mut phys)
    {
        if addr >= fb_base
            && addr.wrapping_add(4 as uint32_t) as uint64_t
                <= (fb_base as uint64_t).wrapping_add(s.vram_size)
        {
            let off: uint32_t = addr.wrapping_sub(fb_base);
            // Stores the dword's native-endian bytes at `off`, as the former memcpy did.
            crate::src::qemu::Vram::of(s).write_bytes(off as u64, &wire_val.to_ne_bytes());
            gpu_vram_dirty(s, off as uint64_t, 4 as uint64_t);
        } else {
            crate::src::qemu::guest_write32(addr as hwaddr as u64, wire_val);
        }
        return;
    }
    crate::src::qemu::guest_write32(phys as u64, wire_val);
}
/// Runs one R300 draw through `r300_render`. `idx` is None for a direct draw; for an indexed
/// draw it holds the index dwords.
fn r300_render_checked(
    s: &mut PPCMacGPUState,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
    idx: Option<&[uint32_t]>,
) {
    r300_render(s, opcode, d, body_dw, idx);
}
pub fn ppc_mac_gpu_r300_packet3(
    s: &mut PPCMacGPUState,
    mut opcode: uint32_t,
    mut d: &[uint32_t],
    mut body_dw: uint32_t,
) -> bool {
    static op_seen: ::std::sync::Mutex<[uint32_t; 256]> = ::std::sync::Mutex::new([0; 256]);
    if opcode == 0x29 as uint32_t && body_dw >= 2 as uint32_t {
        opcode = 0x35 as uint32_t;
        d = d.get(1..).unwrap_or(&[]);
        body_dw = body_dw.wrapping_sub(1);
        s.r3_mut().win_immd = 1 as ::core::ffi::c_int != 0;
    }
    let draw: bool =
        opcode == 0x34 as uint32_t || opcode == 0x35 as uint32_t || opcode == 0x36 as uint32_t;
    if !s.r3_dump.is_null() && {
        let mut seen = op_seen.lock().unwrap_or_else(|e| e.into_inner());
        let slot = (opcode & 0xff as uint32_t) as usize;
        let fresh25 = seen[slot];
        seen[slot] = fresh25.wrapping_add(1);
        drop(seen);
        fresh25 == 0 as uint32_t
    } {
        crate::src::logs::err(format_args!("--- first type-3 opcode 0x{:02X} ({} dwords):", ((opcode) as u32), ((body_dw) as u32)));
        let mut k: uint32_t = 0 as uint32_t;
        while k < body_dw && k < 16 as uint32_t {
            crate::src::logs::err(format_args!(" {:08X}", ((dw(d, (k) as usize)) as u32)));
            k = k.wrapping_add(1);
        }
        crate::src::logs::err(format_args!("\n"));
        crate::src::qemu::r3_dump_flush(s);
    }
    if !s.r3_dump.is_null() && s.r3_ref().draws != 0 && s.r3_trace_lines < 40000 as uint32_t {
        s.r3_trace_lines = s.r3_trace_lines.wrapping_add(1);
        crate::src::logs::err(format_args!("T3 {:02x} n={} {:08x} {:08x}\n", ((opcode) as u32), ((body_dw) as u32), ((if body_dw != 0 {
                dw(d, (0 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32), ((if body_dw > 1 as uint32_t {
                dw(d, (1 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32)));
    }
    if !draw && r300_drawlog() {
        crate::src::logs::err(format_args!("P op{:02x} n={} {:08x} {:08x} {:08x} {:08x}\n", ((opcode) as u32), ((body_dw) as u32), ((if body_dw != 0 {
                dw(d, (0 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32), ((if body_dw > 1 as uint32_t {
                dw(d, (1 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32), ((if body_dw > 2 as uint32_t {
                dw(d, (2 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32), ((if body_dw > 3 as uint32_t {
                dw(d, (3 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32)));
    }
    if opcode == 0x2f as uint32_t {
        let mut k_0: uint32_t = 0 as uint32_t;
        while k_0 < body_dw
            && k_0
                < (1 as ::core::ffi::c_int + 3 as ::core::ffi::c_int * 8 as ::core::ffi::c_int)
                    as uint32_t
        {
            crate::src::qemu::r3_state_write(
                s,
                (0x20c0 as uint32_t).wrapping_add((4 as uint32_t).wrapping_mul(k_0)),
                dw(d, (k_0) as usize),
            );
            k_0 = k_0.wrapping_add(1);
        }
        return 1 as ::core::ffi::c_int != 0;
    }
    if opcode == 0x32 as uint32_t {
        r300_zmask_clear(s);
        return 1 as ::core::ffi::c_int != 0;
    }
    if opcode == 0x38 as uint32_t {
        r300_cmask_clear(s);
        return 1 as ::core::ffi::c_int != 0;
    }
    if opcode == 0x37 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    let indx_mem: bool = opcode == 0x36 as uint32_t
        && body_dw == 1 as uint32_t
        && dw(d, (0 as ::core::ffi::c_int) as usize) >> 4 as ::core::ffi::c_int & 3 as uint32_t
            == 1 as uint32_t
        && dw(d, (0 as ::core::ffi::c_int) as usize) >> 16 as ::core::ffi::c_int != 0 as uint32_t;
    if opcode == 0x33 as uint32_t
        || indx_mem as ::core::ffi::c_int != 0
            && s.r3_indx_buf_pending as ::core::ffi::c_int != 0
    {
        let vf: uint32_t;
        let mut ib: [uint32_t; 3] = [0; 3];
        if opcode == 0x33 as uint32_t {
            if body_dw < 3 as uint32_t {
                return 1 as ::core::ffi::c_int != 0;
            }
            match d.get(..3) {
                Some(src) => ib.copy_from_slice(src),
                None => return true,
            }
            if s.r3_indx_vf == 0 {
                s.r3_indx_buf = ib;
                s.r3_indx_buf_pending = 1 as ::core::ffi::c_int != 0;
                return 1 as ::core::ffi::c_int != 0;
            }
            vf = s.r3_indx_vf;
        } else {
            ib = s.r3_indx_buf;
            vf = dw(d, (0 as ::core::ffi::c_int) as usize);
        }
        s.r3_indx_vf = 0 as uint32_t;
        s.r3_indx_buf_pending = 0 as ::core::ffi::c_int != 0;
        let ndw: uint32_t = {
            let mut _a85: ::core::ffi::c_uint = ib[2 as ::core::ffi::c_int as usize]
                & 0xffffff as ::core::ffi::c_int as ::core::ffi::c_uint;
            let mut _b86: ::core::ffi::c_uint = (4 as ::core::ffi::c_uint)
                .wrapping_mul(1024 as ::core::ffi::c_uint)
                .wrapping_mul(1024 as ::core::ffi::c_uint);
            if _a85 < _b86 {
                _a85 as ::core::ffi::c_uint
            } else {
                _b86 as ::core::ffi::c_uint
            }
        };
        // The index buffer: `ndw` dwords copied from guest memory. `idx_buf` owns them and
        // stays alive until the draw has returned.
        let mut bytes: Vec<u8> = Vec::new();
        if bytes.try_reserve_exact(ndw as usize * 4).is_err() {
            r300_warn_once("INDX_BUFFER outside VRAM and the GART", None);
            return 1 as ::core::ffi::c_int != 0;
        }
        bytes.resize(ndw as usize * 4, 0);
        if !r300_read_gpu_bytes(s, ib[1 as ::core::ffi::c_int as usize] & !(3 as uint32_t), &mut bytes) {
            r300_warn_once("INDX_BUFFER outside VRAM and the GART", None);
            return 1 as ::core::ffi::c_int != 0;
        }
        // Native-endian dwords, the same bytes the former memcpy produced.
        let mut idx_buf: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(bytes);
        idx_buf.truncate(ndw as usize);
        s.r3_mut().draws = s.r3_ref().draws.wrapping_add(1);
        r300_render_checked(
            s,
            0x36 as uint32_t,
            core::slice::from_ref(&vf),
            1 as uint32_t,
            Some(&idx_buf),
        );
        return 1 as ::core::ffi::c_int != 0;
    }
    if indx_mem {
        s.r3_indx_vf = dw(d, (0 as ::core::ffi::c_int) as usize);
        return 1 as ::core::ffi::c_int != 0;
    }
    if !draw {
        return 0 as ::core::ffi::c_int != 0;
    }
    s.r3_mut().draws = s.r3_ref().draws.wrapping_add(1);
    if !s.r3_dump.is_null() && s.r3_ref().draws == 1 as uint64_t {
        let mut ph: hwaddr = 0 as hwaddr;
        let (g, a) = {
            let g: bool = ppc_mac_gpu_gart_translate(
                s,
                s.regs.cp_rb_rptr_addr & !(3 as uint32_t),
                &mut ph,
            );
            let a: bool = !g
                && ppc_mac_gpu_agp_translate(
                    s,
                    s.regs.cp_rb_rptr_addr & !(3 as uint32_t),
                    &mut ph,
                );
            (g, a)
        };
        crate::src::logs::err(format_args!("MC: fb_location {:08x} agp_location {:08x} agp_base {:08x} config_memsize {:08x}\nCP: rb_base {:08x} rb_cntl {:08x} rptr_addr {:08x} (gart {} agp {} -> {:08x}) scratch_addr {:08x} umsk {:x}\n", ((s.regs.mc_fb_location) as u32), ((s.regs.mc_agp_location) as u32), ((s.regs.agp_base) as u32), ((s.regs.config_memsize) as u32), ((s.regs.cp_rb_base) as u32), ((s.regs.cp_rb_cntl) as u32), ((s.regs.cp_rb_rptr_addr) as u32), ((g as ::core::ffi::c_int) as i32), ((a as ::core::ffi::c_int) as i32), ((ph) as u64), ((s.regs.scratch_addr) as u32), ((s.regs.scratch_umsk) as u32)));
    }
    static screen_dumps: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
    static scratch_w: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
    static scratch_r: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
    static scratch_rt: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
    let rt_off: uint32_t = r300_reg(s.r3_ref(), 0x4e28 as uint32_t) & !(0x1f as uint32_t);
    let rt_pitch: uint32_t = r300_reg(s.r3_ref(), 0x4e38 as uint32_t) & 0x3ffe as uint32_t;
    let scratch_write: bool = s.r3_ref().draws > 3000 as uint64_t
        && rt_off != 0
        && rt_pitch != 0x340 as uint32_t
        && rt_pitch >= 0x100 as uint32_t;
    if scratch_write {
        scratch_rt.set(rt_off);
    }
    let scratch_read: bool = scratch_rt.get() != 0
        && r300_reg(s.r3_ref(), 0x4104 as uint32_t) & 1 as uint32_t != 0
        && r300_reg(s.r3_ref(), 0x4540 as uint32_t) as ::core::ffi::c_uint
            & !(0x1f as ::core::ffi::c_uint)
            == scratch_rt.get();
    if !s.r3_dump.is_null()
        && (scratch_write as ::core::ffi::c_int != 0 || scratch_read as ::core::ffi::c_int != 0)
    {
        static seq_lines: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
        let fresh26 = seq_lines.get();
        seq_lines.set(seq_lines.get() + 1);
        if fresh26 < 3000 as ::core::ffi::c_int {
            let f0: uint32_t = r300_reg(s.r3_ref(), 0x4480 as uint32_t);
            let f2: uint32_t = r300_reg(s.r3_ref(), 0x4500 as uint32_t);
            crate::src::logs::err(format_args!("SEQ {} rt {:06x}/{} tex {:06x} {}x{}/{} draw {}\n", crate::src::qemu::cstr(if scratch_write as ::core::ffi::c_int != 0 {
                    c"W".as_ptr()
                } else {
                    c"R".as_ptr()
                }), ((rt_off) as u32), ((rt_pitch) as u32), ((r300_reg(s.r3_ref(), 0x4540 as uint32_t) as ::core::ffi::c_uint
                    & !(0x1f as ::core::ffi::c_uint)) as u32), (((f0 & 0x7ff as uint32_t).wrapping_add(1 as uint32_t)) as u32), (((f0 >> 11 as ::core::ffi::c_int & 0x7ff as uint32_t).wrapping_add(1 as uint32_t)) as u32), (((f2 & 0x3fff as uint32_t).wrapping_add(1 as uint32_t)) as u32), ((s.r3_ref().draws as ::core::ffi::c_ulonglong) as u64)));
        }
    }
    if !s.r3_dump.is_null()
        && (s.r3_ref().draws <= 4 as uint64_t
            || r300_reg(s.r3_ref(), 0x4e28 as uint32_t) == 0 as uint32_t && {
                let fresh27 = screen_dumps.get();
                screen_dumps.set(screen_dumps.get() + 1);
                fresh27 < 3 as ::core::ffi::c_int
            }
            || scratch_write as ::core::ffi::c_int != 0 && {
                let fresh28 = scratch_w.get();
                scratch_w.set(scratch_w.get() + 1);
                fresh28 < 0 as ::core::ffi::c_int
            }
            || scratch_read as ::core::ffi::c_int != 0 && {
                let fresh29 = scratch_r.get();
                scratch_r.set(scratch_r.get() + 1);
                fresh29 < 0 as ::core::ffi::c_int
            })
    {
        crate::src::logs::err(format_args!("{}", crate::src::qemu::cstr(if scratch_write as ::core::ffi::c_int != 0 {
                c"### SCRATCH WRITE\n".as_ptr()
            } else if scratch_read as ::core::ffi::c_int != 0 {
                c"### SCRATCH READ\n".as_ptr()
            } else {
                c"".as_ptr()
            })));
        crate::src::logs::err(format_args!("--- draw op 0x{:02X} vf_cntl {:08X}, {} dwords:", ((opcode) as u32), ((if body_dw != 0 {
                dw(d, (0 as ::core::ffi::c_int) as usize)
            } else {
                0 as uint32_t
            }) as u32), ((body_dw) as u32)));
        let mut k_1: uint32_t = 1 as uint32_t;
        while k_1 < body_dw && k_1 < 100 as uint32_t {
            crate::src::logs::err(format_args!(" {:08X}", ((dw(d, (k_1) as usize)) as u32)));
            k_1 = k_1.wrapping_add(1);
        }
        crate::src::logs::err(format_args!("\n"));
        crate::src::qemu::r3_state_dump_to_file(s);
    }
    r300_render_checked(s, opcode, d, body_dw, None);
    return 1 as ::core::ffi::c_int != 0;
}
pub fn ppc_mac_gpu_gart_translate(
    s: &mut PPCMacGPUState,
    gpu_addr: uint32_t,
    phys_addr: &mut hwaddr,
) -> bool {
    if s.regs.aic_ctrl & 1 as uint32_t == 0 {
        return 0 as ::core::ffi::c_int != 0;
    }
    if gpu_addr < s.regs.aic_lo_addr || gpu_addr > s.regs.aic_hi_addr {
        return 0 as ::core::ffi::c_int != 0;
    }
    let offset: uint32_t = gpu_addr.wrapping_sub(s.regs.aic_lo_addr);
    let page_idx: uint32_t = offset >> 12 as ::core::ffi::c_int;
    let page_off: uint32_t = offset & 0xfff as uint32_t;
    let pt_base: uint32_t = if s.r300_aic_pt_base != 0 {
        s.r300_aic_pt_base
    } else {
        s.regs.aic_pt_base
    };
    let pte_addr: hwaddr =
        (pt_base as hwaddr).wrapping_add(page_idx.wrapping_mul(4 as uint32_t) as hwaddr);
    let mut pte_raw: uint32_t = 0 as uint32_t;
    let r: MemTxResult = crate::src::qemu::guest_read32(pte_addr as u64, &mut pte_raw);
    if r != 0 as MemTxResult {
        return 0 as ::core::ffi::c_int != 0;
    }
    r200_last_pte_addr.set(pte_addr);
    r200_last_pte_raw.set(pte_raw);
    let pte_val: uint32_t = u32::from_le(pte_raw);
    let phys_page: hwaddr = hwaddr::from(pte_val & !0xfff_u32);
    if phys_page == 0 {
        return false;
    }
    static gart_log_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
    if gart_log_count.get() < 20 as ::core::ffi::c_int {
        crate::src::logs::debug(format_args!("GART_XLAT gpu=0x{:x} page={} pte_addr=0x{:x} pte_raw=0x{:x} pte_val=0x{:x} phys=0x{:x}", ((gpu_addr) as u32), ((page_idx) as u32), ((pte_addr) as u64), ((pte_raw) as u32), ((pte_val) as u32), ((phys_page | page_off as hwaddr) as u64)));
        gart_log_count.set(gart_log_count.get() + 1);
    }
    *phys_addr = phys_page | hwaddr::from(page_off);
    true
}
pub fn ppc_mac_gpu_agp_translate(
    s: &mut PPCMacGPUState,
    gpu_addr: uint32_t,
    phys_addr: &mut hwaddr,
) -> bool {
    let agp_loc: uint32_t = s.regs.mc_agp_location;
    if agp_loc == 0 as uint32_t {
        return 0 as ::core::ffi::c_int != 0;
    }
    let agp_start: uint32_t = (agp_loc & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let agp_end: uint32_t = (agp_loc >> 16 as ::core::ffi::c_int & 0xffff as uint32_t)
        << 16 as ::core::ffi::c_int
        | 0xffff as uint32_t;
    if gpu_addr < agp_start || gpu_addr > agp_end {
        return 0 as ::core::ffi::c_int != 0;
    }
    let mut gart_table: hwaddr = crate::src::qemu::agp_gart_base();
    if gart_table == 0 as hwaddr {
        gart_table = s.regs.aic_pt_base as hwaddr;
        if gart_table == 0 as hwaddr {
            return 0 as ::core::ffi::c_int != 0;
        }
    }
    let offset: uint32_t = gpu_addr.wrapping_sub(agp_start);
    let page_idx: uint32_t = offset >> 12 as ::core::ffi::c_int;
    let page_off: uint32_t = offset & 0xfff as uint32_t;
    let pte_addr: hwaddr =
        gart_table.wrapping_add(page_idx.wrapping_mul(4 as uint32_t) as hwaddr);
    let mut pte_raw: uint32_t = 0 as uint32_t;
    let r: MemTxResult = crate::src::qemu::guest_read32(pte_addr as u64, &mut pte_raw);
    if r != 0 as MemTxResult {
        return 0 as ::core::ffi::c_int != 0;
    }
    r200_last_pte_addr.set(pte_addr);
    r200_last_pte_raw.set(pte_raw);
    let pte_val: uint32_t = u32::from_le(pte_raw);
    let phys_page: hwaddr = hwaddr::from(pte_val & 0xffff_f000_u32);
    if phys_page == 0 {
        return false;
    }
    static agp_log_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
    if agp_log_count.get() < 50 as ::core::ffi::c_int {
        crate::src::logs::debug(format_args!("AGP_XLAT gpu=0x{:x} offset=0x{:x} page={} pte_addr=0x{:x} pte_raw=0x{:x} pte_val=0x{:x} gart_base=0x{:x} phys=0x{:x}", ((gpu_addr) as u32), ((offset) as u32), ((page_idx) as u32), ((pte_addr) as u64), ((pte_raw) as u32), ((pte_val) as u32), ((gart_table) as u64), ((phys_page | page_off as hwaddr) as u64)));
        agp_log_count.set(agp_log_count.get() + 1);
    }
    *phys_addr = phys_page | hwaddr::from(page_off);
    true
}
pub fn ppc_mac_gpu_read_ib_via_gart(
    s: &mut PPCMacGPUState,
    ib_base: uint32_t,
    ib_size_dw: uint32_t,
) -> Option<Vec<uint32_t>> {
    // `None` for a failed translation or read.  A guest-chosen size that cannot be reserved is
    // treated like a failed read.
    let mut dwords: Vec<uint32_t> = Vec::new();
    if dwords.try_reserve_exact(ib_size_dw as usize).is_err() {
        return None;
    }
    let mut i: uint32_t = 0 as uint32_t;
    while i < ib_size_dw {
        let gpu_addr: uint32_t = ib_base.wrapping_add(i.wrapping_mul(4 as uint32_t));
        let mut phys: hwaddr = 0;
        if !ppc_mac_gpu_gart_translate(s, gpu_addr, &mut phys)
            && !ppc_mac_gpu_agp_translate(s, gpu_addr, &mut phys)
        {
            return None;
        }
        let mut raw: uint32_t = 0 as uint32_t;
        let r: MemTxResult = crate::src::qemu::guest_read32(phys as u64, &mut raw);
        if r != 0 as MemTxResult {
            return None;
        }
        dwords.push(gpu_guest_dw(s, raw));
        i = i.wrapping_add(1);
    }
    Some(dwords)
}
pub fn ppc_mac_gpu_pm4_process_type0(
    s: &mut PPCMacGPUState,
    reg_base: uint32_t,
    one_reg_wr: bool,
    count: uint32_t,
    data: &[uint32_t],
) {
    // `count` is guest-controlled: only the dwords the slice holds are written.
    for (i, &val) in data.iter().take(count as usize).enumerate() {
        let i = i as uint32_t;
        let reg_addr: uint32_t = if one_reg_wr as ::core::ffi::c_int != 0 {
            reg_base.wrapping_mul(4 as uint32_t)
        } else {
            reg_base.wrapping_add(i).wrapping_mul(4 as uint32_t)
        };
        crate::src::logs::debug(format_args!("PM4_EXEC type0 reg=0x{:04x} val=0x{:x}{}", ((reg_addr) as u32), ((val) as u32), crate::src::qemu::cstr(if one_reg_wr as ::core::ffi::c_int != 0 {
                c" [ONE_REG]".as_ptr()
            } else {
                c"".as_ptr()
            })));
        crate::src::qemu::mmio_write_dev(s, reg_addr as hwaddr, val as uint64_t, 4);
    }
}
pub fn ppc_mac_gpu_host_data_write(
    s: &mut PPCMacGPUState,
    val: uint32_t,
) {
    if !s.host_data_active {
        return;
    }
    let vram = crate::src::qemu::Vram::of(s);
    let bpp: uint32_t = if s.host_data_bpp != 0 {
        s.host_data_bpp
    } else {
        4 as uint32_t
    };
    if bpp < 4 as uint32_t {
        let mut cur_x = s.host_data_cur_x;
        let mut cur_y = s.host_data_cur_y;
        let more = host_data_put_narrow(
            s,
            vram,
            val,
            bpp,
            s.host_data_offset as uint64_t,
            s.host_data_pitch,
            s.host_data_dst_x,
            s.host_data_dst_y,
            s.host_data_w,
            s.host_data_h,
            &mut cur_x,
            &mut cur_y,
        );
        s.host_data_cur_x = cur_x;
        s.host_data_cur_y = cur_y;
        if !more {
            let start: uint64_t = (s.host_data_offset as uint64_t).wrapping_add(
                (s.host_data_dst_y as uint64_t).wrapping_mul(s.host_data_pitch as uint64_t),
            );
            let len: uint64_t =
                (s.host_data_h as uint64_t).wrapping_mul(s.host_data_pitch as uint64_t);
            if start.wrapping_add(len) <= s.vram_size {
                gpu_vram_dirty(s, start, len);
            }
            s.host_data_active = 0 as ::core::ffi::c_int != 0;
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
        }
        return;
    }
    let x: uint32_t = s.host_data_dst_x.wrapping_add(s.host_data_cur_x);
    let y: uint32_t = s.host_data_dst_y.wrapping_add(s.host_data_cur_y);
    let linear_addr: uint64_t = (s.host_data_offset as uint64_t)
        .wrapping_add((y as uint64_t).wrapping_mul(s.host_data_pitch as uint64_t))
        .wrapping_add((x as uint64_t).wrapping_mul(bpp as uint64_t));
    if s.host_data_offset <= 0x10000 as ::core::ffi::c_int as uint32_t
        && s.host_data_cur_x == 0 as uint32_t
        && s.host_data_cur_y == 0 as uint32_t
    {
        fb_write_watch(
            c"host_data".as_ptr(),
            c"cpu".as_ptr(),
            0 as uint32_t,
            s.host_data_offset,
            s.host_data_dst_x,
            s.host_data_dst_y,
            s.host_data_w,
            s.host_data_h,
            0 as ::core::ffi::c_int != 0,
            val != 0 as uint32_t,
        );
    }
    if linear_addr >= 0x353000 as uint64_t && linear_addr < 0x413000 as uint64_t {
        static hd_watch_log: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
        if hd_watch_log.get() < 50 as ::core::ffi::c_int {
            crate::src::logs::err(format_args!("[VRAM_WATCH] host_data_write linear=0x{:06x} val=0x{:08x} x={} y={} off=0x{:x} pitch={}\n", ((linear_addr as ::core::ffi::c_ulonglong) as u64), ((val) as u32), ((x) as u32), ((y) as u32), ((s.host_data_offset) as u32), ((s.host_data_pitch) as u32)));
            hd_watch_log.set(hd_watch_log.get() + 1);
        }
    }
    mc_vram_write32(s, vram, linear_addr, val);
    s.host_data_cur_x = s.host_data_cur_x.wrapping_add(1);
    if s.host_data_cur_x >= s.host_data_w {
        s.host_data_cur_x = 0 as uint32_t;
        s.host_data_cur_y = s.host_data_cur_y.wrapping_add(1);
        if s.host_data_cur_y >= s.host_data_h {
            let dirty_start: uint64_t = (s.host_data_offset as uint64_t)
                .wrapping_add(
                    (s.host_data_dst_y as uint64_t)
                        .wrapping_mul(s.host_data_pitch as uint64_t),
                )
                .wrapping_add((s.host_data_dst_x as uint64_t).wrapping_mul(bpp as uint64_t));
            let dirty_len: uint64_t = if s.host_data_h > 1 as uint32_t {
                (s.host_data_h.wrapping_sub(1 as uint32_t) as uint64_t)
                    .wrapping_mul(s.host_data_pitch as uint64_t)
                    .wrapping_add((s.host_data_w as uint64_t).wrapping_mul(bpp as uint64_t))
            } else {
                (s.host_data_w as uint64_t).wrapping_mul(bpp as uint64_t)
            };
            if dirty_start.wrapping_add(dirty_len) <= s.vram_size {
                gpu_vram_dirty(s, dirty_start, dirty_len);
            }
            let (off, pitch4, w, h) = (
                s.host_data_offset,
                s.host_data_pitch.wrapping_div(4 as uint32_t),
                s.host_data_w as uint16_t,
                s.host_data_h as uint16_t,
            );
            let (dst_x, dst_y) = (s.host_data_dst_x as uint16_t, s.host_data_dst_y as uint16_t);
            crate::src::qemu::with_frame_tracker(|ft| {
                frame_tracker_record_detail(
                    ft,
                    PASS_EVENT_HOST_UPLOAD,
                    off,
                    pitch4,
                    0 as uint32_t,
                    off,
                    w,
                    h,
                    0 as uint16_t,
                    0 as uint16_t,
                    dst_x,
                    dst_y,
                    0 as int16_t,
                    0 as int16_t,
                    0 as ::core::ffi::c_int != 0,
                );
            });
            s.host_data_active = 0 as ::core::ffi::c_int != 0;
            s.display_invalid = 1 as ::core::ffi::c_int != 0;
            static hd_log_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
            if hd_log_count.get() < 50 as ::core::ffi::c_int {
                crate::src::logs::debug(format_args!("HOST_DATA complete dst=0x{:x}+{} xy={},{} {}x{}", ((s.host_data_offset) as u32), ((s.host_data_pitch) as u32), ((s.host_data_dst_x) as u32), ((s.host_data_dst_y) as u32), ((s.host_data_w) as u32), ((s.host_data_h) as u32)));
                hd_log_count.set(hd_log_count.get() + 1);
            }
        }
    }
}
pub fn ppc_mac_gpu_hostdata_mono(
    s: &mut PPCMacGPUState,
    d: &[uint32_t],
    n: uint32_t,
) -> bool {
    let gmc: uint32_t = dw(d, (0 as ::core::ffi::c_int) as usize);
    let po: uint32_t = dw(d, (1 as ::core::ffi::c_int) as usize);
    let src_dt: uint32_t = gmc >> 12 as ::core::ffi::c_int & 3 as uint32_t;
    let off: uint32_t = gpu_po_offset(s, po);
    let pitch: uint32_t =
        (po & 0x3fc00000 as ::core::ffi::c_int as uint32_t) >> 16 as ::core::ffi::c_int;
    let bpp: uint32_t = gpu_gmc_bpp(gmc);
    let w: uint32_t = dw(d, (7 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t;
    let mut h: uint32_t = dw(d, (7 as ::core::ffi::c_int) as usize) >> 16 as ::core::ffi::c_int;
    let dx: uint32_t = dw(d, (6 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t;
    let dy: uint32_t = dw(d, (6 as ::core::ffi::c_int) as usize) >> 16 as ::core::ffi::c_int;
    let mut wps: uint32_t = w.wrapping_add(31 as uint32_t).wrapping_div(32 as uint32_t);
    let lsb: bool =
        gmc as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << 14 as ::core::ffi::c_int != 0;
    let clip: bool =
        gmc as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << 3 as ::core::ffi::c_int != 0;
    let vram = crate::src::qemu::Vram::of(s);
    if src_dt == 3 as uint32_t {
        wps = w
            .wrapping_mul(bpp)
            .wrapping_add(3 as uint32_t)
            .wrapping_div(4 as uint32_t);
    }
    if src_dt == 2 as uint32_t || n < 9 as uint32_t || w == 0 || h == 0 || pitch == 0 {
        return 0 as ::core::ffi::c_int != 0;
    }
    if (n as uint64_t).wrapping_sub(9 as uint64_t) < (wps as uint64_t).wrapping_mul(h as uint64_t) {
        h = n.wrapping_sub(9 as uint32_t).wrapping_div(wps);
    }
    gpu_vram_access(
        s,
        (off as uint64_t).wrapping_add((dy as uint64_t).wrapping_mul(pitch as uint64_t)),
        (off as uint64_t).wrapping_add(
            (dy as uint64_t)
                .wrapping_add(h as uint64_t)
                .wrapping_mul(pitch as uint64_t),
        ),
        1 as ::core::ffi::c_int != 0,
    );
    let mut y: uint32_t = 0 as uint32_t;
    while y < h {
        let mut x: uint32_t = 0 as uint32_t;
        while x < w {
            let px: uint32_t = dx.wrapping_add(x);
            let py: uint32_t = dy.wrapping_add(y);
            if src_dt == 3 as uint32_t {
                let v: uint32_t = if bpp == 4 as uint32_t {
                    dw(d, ((9 as uint32_t)
                            .wrapping_add(y.wrapping_mul(wps))
                            .wrapping_add(x)) as usize)
                } else if bpp == 2 as uint32_t {
                    dw(d, ((9 as uint32_t)
                            .wrapping_add(y.wrapping_mul(wps))
                            .wrapping_add(x.wrapping_div(2 as uint32_t))) as usize) >> (x & 1 as uint32_t).wrapping_mul(16 as uint32_t)
                        & 0xffff as uint32_t
                } else {
                    dw(d, ((9 as uint32_t)
                            .wrapping_add(y.wrapping_mul(wps))
                            .wrapping_add(x.wrapping_div(4 as uint32_t))) as usize) >> (x & 3 as uint32_t).wrapping_mul(8 as uint32_t)
                        & 0xff as uint32_t
                };
                if !clip
                    || !(px < dw(d, (2 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t
                        || py
                            < dw(d, (2 as ::core::ffi::c_int) as usize)
                                >> 16 as ::core::ffi::c_int
                        || px >= dw(d, (3 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t
                        || py
                            >= dw(d, (3 as ::core::ffi::c_int) as usize)
                                >> 16 as ::core::ffi::c_int)
                {
                    gpu_px_put(
                        s,
                        vram,
                        (off as uint64_t)
                            .wrapping_add((py as uint64_t).wrapping_mul(pitch as uint64_t))
                            .wrapping_add((px as uint64_t).wrapping_mul(bpp as uint64_t)),
                        v,
                        bpp,
                    );
                }
            } else {
                let word: uint32_t = dw(d, ((9 as uint32_t)
                        .wrapping_add(y.wrapping_mul(wps))
                        .wrapping_add(x.wrapping_div(32 as uint32_t))) as usize);
                let bit: bool = if lsb as ::core::ffi::c_int != 0 {
                    word >> (x & 31 as uint32_t) & 1 as uint32_t
                } else {
                    word >> (31 as uint32_t).wrapping_sub(x & 31 as uint32_t) & 1 as uint32_t
                } != 0;
                if !(clip as ::core::ffi::c_int != 0
                    && (px < dw(d, (2 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t
                        || py
                            < dw(d, (2 as ::core::ffi::c_int) as usize)
                                >> 16 as ::core::ffi::c_int
                        || px >= dw(d, (3 as ::core::ffi::c_int) as usize) & 0xffff as uint32_t
                        || py
                            >= dw(d, (3 as ::core::ffi::c_int) as usize)
                                >> 16 as ::core::ffi::c_int))
                {
                    if bit as ::core::ffi::c_int != 0 || src_dt == 0 as uint32_t {
                        crate::src::qemu::mc_watch_tag(c"ppc_mac_gpu_hostdata_mono", 4790 as ::core::ffi::c_int);
                        fill_vram_px(
                            s,
                            vram,
                            (off as uint64_t)
                                .wrapping_add((py as uint64_t).wrapping_mul(pitch as uint64_t))
                                .wrapping_add((px as uint64_t).wrapping_mul(bpp as uint64_t)),
                            if bit as ::core::ffi::c_int != 0 {
                                dw(d, (4 as ::core::ffi::c_int) as usize)
                            } else {
                                dw(d, (5 as ::core::ffi::c_int) as usize)
                            },
                            bpp,
                        );
                    }
                }
            }
            x = x.wrapping_add(1);
        }
        y = y.wrapping_add(1);
    }
    gpu_vram_dirty(
        s,
        (off as uint64_t).wrapping_add((dy as uint64_t).wrapping_mul(pitch as uint64_t)),
        (h as uint64_t).wrapping_mul(pitch as uint64_t),
    );
    return 1 as ::core::ffi::c_int != 0;
}
