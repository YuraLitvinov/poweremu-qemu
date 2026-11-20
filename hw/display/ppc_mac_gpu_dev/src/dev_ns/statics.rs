use super::*;
pub static r200_last_pte_addr: crate::src::global::Global<hwaddr> = crate::src::global::Global::<hwaddr>::new(0);
pub static r200_last_pte_raw: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
#[no_mangle]
pub static mut g_frame_tracker: *mut FrameTracker =
    ::core::ptr::null::<FrameTracker>() as *mut FrameTracker;
#[no_mangle]
pub static mut g_blit_stats: BlitPathStats = BlitPathStats {
    sep_count: 0,
    mmio_count: 0,
    bbm_count: 0,
    paint_count: 0,
    hostdata_count: 0,
    last_dump: 0,
};
pub static g_fb_watch_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
#[no_mangle]
pub static g_blit_region_table: std::sync::Mutex<BlitRegionTable> = std::sync::Mutex::new(BlitRegionTable {
    rt_count: 0,
    sequence: 0,
    rts: [BlitRegionRT {
        rt_offset: 0,
        rt_pitch: 0,
        region_count: 0,
        regions: [BlitRegion {
            src_x: 0,
            src_y: 0,
            dst_x: 0,
            dst_y: 0,
            width: 0,
            height: 0,
            sequence: 0,
        }; 64],
    }; 16],
});
#[no_mangle]
pub static mut g_seq_log: *mut FILE = ::core::ptr::null::<FILE>() as *mut FILE;
pub static g_seq: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0);
pub static g_in_pm4: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
pub static g_seq_log_enabled: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
#[no_mangle]
pub static pe_windows: std::sync::Mutex<[PEWindow; 32]> = std::sync::Mutex::new([PEWindow {
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
}; 32]);
pub static pe_windows_printed_us: crate::src::global::Global<int64_t> = crate::src::global::Global::<int64_t>::new(0);
pub static pe_desktop_surface: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
pub static pe_desktop_surface_us: crate::src::global::Global<int64_t> = crate::src::global::Global::<int64_t>::new(0);
pub static pe_harm_on: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
#[no_mangle]
pub static mut pe_harm_tiles: *mut uint8_t = ::core::ptr::null::<uint8_t>() as *mut uint8_t;
pub static pe_harm_rows: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
pub static pe_harm_cols: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
pub static pe_harm_gen: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
pub static pe_harm_resized_us: crate::src::global::Global<int64_t> = crate::src::global::Global::<int64_t>::new(0);
pub static g_post_probe_draw_count: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0 as ::core::ffi::c_int);
#[no_mangle]
pub static mut g_post_probe_audit: C2RustUnnamed_96 = C2RustUnnamed_96 {
    entries: [PostProbeReadEntry {
        reg: 0,
        last_value: 0,
        count: 0,
        first_at_draw: 0,
        last_at_draw: 0,
    }; 256],
    num_entries: 0,
    total_reads: 0,
    audit_active: 0,
    window_logged: 0,
    summary_logged: 0,
    first_draw_seen: 0,
    window_start_draw: 0,
    window_end_draw: 0,
};
#[no_mangle]
pub static mut gpu_debug_fp: *mut FILE = ::core::ptr::null::<FILE>() as *mut FILE;
pub static gpu_debug_seq: crate::src::global::Global<uint64_t> = crate::src::global::Global::<uint64_t>::new(0 as uint64_t);
pub static gpu_track_pass: crate::src::global::Global<bool> = crate::src::global::Global::<bool>::new(false);
#[no_mangle]
pub static ppc_mac_gpu_vga_bank_ops: MemoryRegionOps = unsafe {
    MemoryRegionOps {
        read: Some(
            ppc_mac_gpu_vga_bank_read
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    hwaddr,
                    ::core::ffi::c_uint,
                ) -> uint64_t,
        ),
        write: Some(
            ppc_mac_gpu_vga_bank_write
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    hwaddr,
                    uint64_t,
                    ::core::ffi::c_uint,
                ) -> (),
        ),
        read_with_attrs: None,
        write_with_attrs: None,
        endianness: DEVICE_LITTLE_ENDIAN,
        valid: C2RustUnnamed_14 {
            min_access_size: 1 as ::core::ffi::c_uint,
            max_access_size: 4 as ::core::ffi::c_uint,
            unaligned: false,
            accepts: None,
        },
        impl_0: C2RustUnnamed_13 {
            min_access_size: 1 as ::core::ffi::c_uint,
            max_access_size: 4 as ::core::ffi::c_uint,
            unaligned: false,
        },
    }
};
#[no_mangle]
pub static ppc_mac_gpu_gfx_ops: GraphicHwOps = unsafe {
    GraphicHwOps {
        get_flags: None,
        invalidate: None,
        gfx_update: Some(
            ppc_mac_gpu_display_update as unsafe extern "C" fn(*mut ::core::ffi::c_void) -> bool,
        ),
        text_update: None,
        ui_info: None,
        gl_block: None,
    }
};
#[no_mangle]
pub static mut r200_fence_bh: *mut QEMUBH = ::core::ptr::null::<QEMUBH>() as *mut QEMUBH;
pub static g_r300_arm_rt: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
#[no_mangle]
pub static mut mc_watch_fn: *const ::core::ffi::c_char =
    c"?".as_ptr();
pub static mc_watch_line: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
#[no_mangle]
pub static mut r200_agp_tc: [C2RustUnnamed_99; 1024] = [C2RustUnnamed_99 {
    tag: 0,
    host: ::core::ptr::null::<uint8_t>() as *mut uint8_t,
    pte: 0,
    pte_raw: 0,
}; 1024];
#[no_mangle]
pub static mut pm4_recent: [C2RustUnnamed_102; 32] = [C2RustUnnamed_102 {
    hdr: 0,
    type_0: 0,
    opcode: 0,
    count: 0,
}; 32];
pub static pm4_recent_n: crate::src::global::Global<::core::ffi::c_uint> = crate::src::global::Global::<::core::ffi::c_uint>::new(0);
#[no_mangle]
pub static mut ppc_mac_gpu_post_regs: [C2RustUnnamed_97; 72] = [
    C2RustUnnamed_97 {
        reg: 0x38 as uint16_t,
        post: 0 as uint32_t,
        name: c"MEM_VGA_WP_SEL",
    },
    C2RustUnnamed_97 {
        reg: 0xc40 as uint16_t,
        post: 0 as uint32_t,
        name: c"VIPH_CONTROL",
    },
    C2RustUnnamed_97 {
        reg: 0x15c0 as uint16_t,
        post: 0 as uint32_t,
        name: c"CLR_CMP_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x15cc as uint16_t,
        post: 0 as uint32_t,
        name: c"CLR_CMP_MASK",
    },
    C2RustUnnamed_97 {
        reg: 0x15c4 as uint16_t,
        post: 0 as uint32_t,
        name: c"CLR_CMP_CLR_SRC",
    },
    C2RustUnnamed_97 {
        reg: 0x15c8 as uint16_t,
        post: 0 as uint32_t,
        name: c"CLR_CMP_CLR_DST",
    },
    C2RustUnnamed_97 {
        reg: 0x3c as uint16_t,
        post: 0 as uint32_t,
        name: c"MEM_VGA_RP_SEL",
    },
    C2RustUnnamed_97 {
        reg: 0x94 as uint16_t,
        post: 0 as uint32_t,
        name: c"I2C_CNTL_1",
    },
    C2RustUnnamed_97 {
        reg: 0x118 as uint16_t,
        post: 0x8bbbb as ::core::ffi::c_int as uint32_t,
        name: c"FW_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x120 as uint16_t,
        post: 0 as uint32_t,
        name: c"TEST_DEBUG_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x140 as uint16_t,
        post: 0x22 as uint32_t,
        name: c"MEM_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x144 as uint16_t,
        post: 0x1a3a5523 as ::core::ffi::c_int as uint32_t,
        name: c"MEM_TIMING_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x154 as uint16_t,
        post: 0x20000000 as ::core::ffi::c_int as uint32_t,
        name: c"MC_INIT_GFX_LAT_TIMER",
    },
    C2RustUnnamed_97 {
        reg: 0x158 as uint16_t,
        post: 0x31420042 as ::core::ffi::c_int as uint32_t,
        name: c"MEM_SDRAM_MODE_REG",
    },
    C2RustUnnamed_97 {
        reg: 0x168 as uint16_t,
        post: 0 as uint32_t,
        name: c"PAD_CTLR_STRENGTH",
    },
    C2RustUnnamed_97 {
        reg: 0x178 as uint16_t,
        post: 0x9025 as uint32_t,
        name: c"MEM_IO_CNTL_A0",
    },
    C2RustUnnamed_97 {
        reg: 0x17c as uint16_t,
        post: 0xba30ba3 as ::core::ffi::c_int as uint32_t,
        name: c"MC_READ_CNTL_AB",
    },
    C2RustUnnamed_97 {
        reg: 0x180 as uint16_t,
        post: 0xf000000 as ::core::ffi::c_int as uint32_t,
        name: c"MC_INIT_MISC_LAT_TIMER",
    },
    C2RustUnnamed_97 {
        reg: 0x184 as uint16_t,
        post: 0x1e3c78f as ::core::ffi::c_int as uint32_t,
        name: c"MC_IOPAD_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x188 as uint16_t,
        post: 0x923def as ::core::ffi::c_int as uint32_t,
        name: c"MC_DEBUG",
    },
    C2RustUnnamed_97 {
        reg: 0x18c as uint16_t,
        post: 0x2f302f30 as ::core::ffi::c_int as uint32_t,
        name: c"MC_CHIP_IO_OE_CNTL_AB",
    },
    C2RustUnnamed_97 {
        reg: 0x198 as uint16_t,
        post: 0x1 as uint32_t,
        name: c"GPIOPAD_MASK",
    },
    C2RustUnnamed_97 {
        reg: 0x19c as uint16_t,
        post: 0x1 as uint32_t,
        name: c"GPIOPAD_A",
    },
    C2RustUnnamed_97 {
        reg: 0x1a0 as uint16_t,
        post: 0x1 as uint32_t,
        name: c"GPIOPAD_EN",
    },
    C2RustUnnamed_97 {
        reg: 0x1a8 as uint16_t,
        post: 0 as uint32_t,
        name: c"ZV_LCDPAD_MASK",
    },
    C2RustUnnamed_97 {
        reg: 0x1ac as uint16_t,
        post: 0 as uint32_t,
        name: c"ZV_LCDPAD_A",
    },
    C2RustUnnamed_97 {
        reg: 0x1b0 as uint16_t,
        post: 0 as uint32_t,
        name: c"ZV_LCDPAD_EN",
    },
    C2RustUnnamed_97 {
        reg: 0x1c0 as uint16_t,
        post: 0 as uint32_t,
        name: c"SEPROM_CNTL1",
    },
    C2RustUnnamed_97 {
        reg: 0x1ec as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_01EC",
    },
    C2RustUnnamed_97 {
        reg: 0x234 as uint16_t,
        post: 0 as uint32_t,
        name: c"OVR_WID_LEFT_RIGHT",
    },
    C2RustUnnamed_97 {
        reg: 0x238 as uint16_t,
        post: 0 as uint32_t,
        name: c"OVR_WID_TOP_BOTTOM",
    },
    C2RustUnnamed_97 {
        reg: 0x27c as uint16_t,
        post: 0 as uint32_t,
        name: c"CRTC_MORE_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x280 as uint16_t,
        post: 0 as uint32_t,
        name: c"DAC_EXT_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x288 as uint16_t,
        post: 0 as uint32_t,
        name: c"FP2_GEN_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x28c as uint16_t,
        post: 0 as uint32_t,
        name: c"FP_HORZ_STRETCH",
    },
    C2RustUnnamed_97 {
        reg: 0x290 as uint16_t,
        post: 0 as uint32_t,
        name: c"FP_VERT_STRETCH",
    },
    C2RustUnnamed_97 {
        reg: 0x2a4 as uint16_t,
        post: 0x10000040 as ::core::ffi::c_int as uint32_t,
        name: c"TMDS_TRANSMITTER_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x2a8 as uint16_t,
        post: 0xb01d7 as ::core::ffi::c_int as uint32_t,
        name: c"TMDS_?_02A8",
    },
    C2RustUnnamed_97 {
        reg: 0x2f0 as uint16_t,
        post: 0x20204f4f as ::core::ffi::c_int as uint32_t,
        name: c"GRPH_BUFFER_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x334 as uint16_t,
        post: 0 as uint32_t,
        name: c"OVR2_WID_LEFT_RIGHT",
    },
    C2RustUnnamed_97 {
        reg: 0x360 as uint16_t,
        post: 0 as uint32_t,
        name: c"CUR2_OFFSET",
    },
    C2RustUnnamed_97 {
        reg: 0x3e8 as uint16_t,
        post: 0xf5474 as ::core::ffi::c_int as uint32_t,
        name: c"?_03E8",
    },
    C2RustUnnamed_97 {
        reg: 0x3f0 as uint16_t,
        post: 0x20204f4f as ::core::ffi::c_int as uint32_t,
        name: c"GRPH2_BUFFER_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x3f8 as uint16_t,
        post: 0 as uint32_t,
        name: c"CRTC2_GEN_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x4dc as uint16_t,
        post: 0x100 as uint32_t,
        name: c"OV0_FLAG_CNTRL",
    },
    C2RustUnnamed_97 {
        reg: 0x800 as uint16_t,
        post: 0x40000000 as ::core::ffi::c_int as uint32_t,
        name: c"TV_MASTER_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x88c as uint16_t,
        post: 0x680000 as ::core::ffi::c_int as uint32_t,
        name: c"TV_DAC_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0x910 as uint16_t,
        post: 0x4 as uint32_t,
        name: c"FCP_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0xaa0 as uint16_t,
        post: 0x6000037d as ::core::ffi::c_int as uint32_t,
        name: c"PAD_CTLR_MISC",
    },
    C2RustUnnamed_97 {
        reg: 0x1608 as uint16_t,
        post: 0 as uint32_t,
        name: c"DST_LINE_PATCOUNT",
    },
    C2RustUnnamed_97 {
        reg: 0xd94 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0D94",
    },
    C2RustUnnamed_97 {
        reg: 0xdd0 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DD0",
    },
    C2RustUnnamed_97 {
        reg: 0xda0 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DA0",
    },
    C2RustUnnamed_97 {
        reg: 0xda8 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DA8",
    },
    C2RustUnnamed_97 {
        reg: 0xd9c as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0D9C",
    },
    C2RustUnnamed_97 {
        reg: 0xdc0 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DC0",
    },
    C2RustUnnamed_97 {
        reg: 0xdc4 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DC4",
    },
    C2RustUnnamed_97 {
        reg: 0xdcc as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DCC",
    },
    C2RustUnnamed_97 {
        reg: 0xda4 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DA4",
    },
    C2RustUnnamed_97 {
        reg: 0xdd4 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DD4",
    },
    C2RustUnnamed_97 {
        reg: 0xdac as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DAC",
    },
    C2RustUnnamed_97 {
        reg: 0xddc as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DDC",
    },
    C2RustUnnamed_97 {
        reg: 0xdd8 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0DD8",
    },
    C2RustUnnamed_97 {
        reg: 0x350 as uint16_t,
        post: 0 as uint32_t,
        name: c"R300_CRTC_TILE_X0_Y0",
    },
    C2RustUnnamed_97 {
        reg: 0x358 as uint16_t,
        post: 0 as uint32_t,
        name: c"R300_CRTC2_TILE_X0_Y0",
    },
    C2RustUnnamed_97 {
        reg: 0x324 as uint16_t,
        post: 0 as uint32_t,
        name: c"CRTC2_OFFSET",
    },
    C2RustUnnamed_97 {
        reg: 0xac as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_00AC",
    },
    C2RustUnnamed_97 {
        reg: 0x908 as uint16_t,
        post: 0 as uint32_t,
        name: c"UNNAMED_0908",
    },
    C2RustUnnamed_97 {
        reg: 0xd00 as uint16_t,
        post: 0x5b000000 as ::core::ffi::c_int as uint32_t,
        name: c"DISP_MISC_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0xd04 as uint16_t,
        post: 0x808 as uint32_t,
        name: c"DAC_MACRO_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0xd10 as uint16_t,
        post: 0x10000000 as ::core::ffi::c_int as uint32_t,
        name: c"DISP_TEST_DEBUG_CNTL",
    },
    C2RustUnnamed_97 {
        reg: 0xd64 as uint16_t,
        post: 0x1000000a as ::core::ffi::c_int as uint32_t,
        name: c"DISP_OUTPUT_CNTL",
    },
];
#[no_mangle]
pub static ppc_mac_gpu_mc_ind_post: [C2RustUnnamed_103; 16] = [
    C2RustUnnamed_103 {
        idx: 0x18 as uint8_t,
        val: 0x4f460d6 as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x19 as uint8_t,
        val: 0x99995555 as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x1a as uint8_t,
        val: 0x141557ff as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x1b as uint8_t,
        val: 0x99995555 as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x1c as uint8_t,
        val: 0x141557ff as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x1d as uint8_t,
        val: 0x99995555 as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x1e as uint8_t,
        val: 0x141557ff as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x1f as uint8_t,
        val: 0x99995555 as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x20 as uint8_t,
        val: 0x141557ff as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x21 as uint8_t,
        val: 0x9249 as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x22 as uint8_t,
        val: 0x554f as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x23 as uint8_t,
        val: 0x2f302f30 as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x24 as uint8_t,
        val: 0xba30ba3 as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x25 as uint8_t,
        val: 0x2222ffff as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x26 as uint8_t,
        val: 0x40001303 as ::core::ffi::c_int as uint32_t,
    },
    C2RustUnnamed_103 {
        idx: 0x2f as uint8_t,
        val: 0xb6db6d as ::core::ffi::c_int as uint32_t,
    },
];
#[no_mangle]
pub static ppc_mac_gpu_mmio_ops: MemoryRegionOps = unsafe {
    MemoryRegionOps {
        read: Some(
            ppc_mac_gpu_mmio_read_timed
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    hwaddr,
                    ::core::ffi::c_uint,
                ) -> uint64_t,
        ),
        write: Some(
            ppc_mac_gpu_mmio_write_timed
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    hwaddr,
                    uint64_t,
                    ::core::ffi::c_uint,
                ) -> (),
        ),
        read_with_attrs: None,
        write_with_attrs: None,
        endianness: DEVICE_LITTLE_ENDIAN,
        valid: C2RustUnnamed_14 {
            min_access_size: 1 as ::core::ffi::c_uint,
            max_access_size: 4 as ::core::ffi::c_uint,
            unaligned: false,
            accepts: None,
        },
        impl_0: C2RustUnnamed_13 {
            min_access_size: 1 as ::core::ffi::c_uint,
            max_access_size: 4 as ::core::ffi::c_uint,
            unaligned: false,
        },
    }
};
#[no_mangle]
pub static r300_watch_ops: MemoryRegionOps = unsafe {
    MemoryRegionOps {
        read: Some(
            r300_watch_read
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    hwaddr,
                    ::core::ffi::c_uint,
                ) -> uint64_t,
        ),
        write: Some(
            r300_watch_write
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    hwaddr,
                    uint64_t,
                    ::core::ffi::c_uint,
                ) -> (),
        ),
        read_with_attrs: None,
        write_with_attrs: None,
        endianness: DEVICE_BIG_ENDIAN,
        valid: C2RustUnnamed_14 {
            min_access_size: 1 as ::core::ffi::c_uint,
            max_access_size: 8 as ::core::ffi::c_uint,
            unaligned: false,
            accepts: None,
        },
        impl_0: C2RustUnnamed_13 {
            min_access_size: 1 as ::core::ffi::c_uint,
            max_access_size: 8 as ::core::ffi::c_uint,
            unaligned: false,
        },
    }
};
#[no_mangle]
pub static mut ppc_mac_gpu_properties: [Property; 5] = [Property {
    name: ::core::ptr::null::<::core::ffi::c_char>(),
    info: ::core::ptr::null::<PropertyInfo>(),
    offset: 0,
    link_type: ::core::ptr::null::<::core::ffi::c_char>(),
    bitmask: 0,
    defval: C2RustUnnamed_81 { i: 0 },
    arrayinfo: ::core::ptr::null::<PropertyInfo>(),
    arrayoffset: 0,
    arrayfieldsize: 0,
    bitnr: 0,
    set_default: false,
}; 5];
#[no_mangle]
pub static mut vmstate_ppc_mac_gpu: VMStateDescription = VMStateDescription {
    name: ::core::ptr::null::<::core::ffi::c_char>(),
    unmigratable: false,
    early_setup: false,
    version_id: 0,
    minimum_version_id: 0,
    priority: migration_sys::MigrationPriority::MIG_PRI_UNINITIALIZED,
    pre_load: None,
    pre_load_errp: None,
    post_load: None,
    post_load_errp: None,
    pre_save: None,
    pre_save_errp: None,
    post_save: None,
    needed: None,
    dev_unplug_pending: None,
    fields: ::core::ptr::null::<VMStateField>(),
    subsections: ::core::ptr::null::<*const VMStateDescription>(),
};
#[no_mangle]
pub static mut ati_radeon_9700_type_info: TypeInfo = unsafe {
    TypeInfo {
        name: c"ati-radeon-9700".as_ptr(),
        parent: c"ppc-mac-gpu".as_ptr(),
        instance_size: 0,
        instance_align: 0,
        instance_init: None,
        instance_post_init: None,
        instance_finalize: None,
        abstract_: false,
        secure: false,
        class_size: 0,
        class_init: Some(
            ati_radeon_9700_class_init
                as unsafe extern "C" fn(*mut ObjectClass, *const ::core::ffi::c_void) -> (),
        ),
        class_base_init: None,
        class_data: ::core::ptr::null::<::core::ffi::c_void>(),
        is_available: None,
        interfaces: ::core::ptr::null::<InterfaceInfo>(),
    }
};
#[used]
#[cfg_attr(target_os = "linux", link_section = ".init_array")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XIB")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
pub static INIT_ARRAY: [unsafe extern "C" fn(); 1] = [run_static_initializers];

#[used]
#[cfg_attr(target_os = "linux", link_section = ".init_array")]
pub static TYPE_INIT_CTOR: unsafe extern "C" fn() = do_qemu_init_ppc_mac_gpu_register_types;

// Accessors for mutable device globals. Everything here runs on the device thread under
// QEMU's big lock, so there is never concurrent access; each accessor is the single place
// that touches the `static mut`.
pub fn g_post_probe_audit_mut() -> &'static mut C2RustUnnamed_96 {
    // SAFETY: only used from the device thread under QEMU's big lock, and callers do not
    // keep the reference across calls that re-enter the same accessor.
    unsafe { &mut *(&raw mut g_post_probe_audit) }
}
pub fn pm4_recent_mut() -> &'static mut [C2RustUnnamed_102; 32] {
    // SAFETY: only used from the device thread under QEMU's big lock, and callers do not
    // keep the reference across calls that re-enter the same accessor.
    unsafe { &mut *(&raw mut pm4_recent) }
}
pub fn r200_agp_tc_mut() -> &'static mut [C2RustUnnamed_99; 1024] {
    // SAFETY: only used from the device thread under QEMU's big lock, and callers do not
    // keep the reference across calls that re-enter the same accessor.
    unsafe { &mut *(&raw mut r200_agp_tc) }
}
pub fn g_blit_stats_mut() -> &'static mut BlitPathStats {
    // SAFETY: only used from the device thread under QEMU's big lock, and callers do not
    // keep the reference across calls that re-enter the same accessor.
    unsafe { &mut *(&raw mut g_blit_stats) }
}
pub fn ppc_mac_gpu_post_regs_mut() -> &'static mut [C2RustUnnamed_97; 72] {
    // SAFETY: only used from the device thread under QEMU's big lock, and callers do not
    // keep the reference across calls that re-enter the same accessor.
    unsafe { &mut *(&raw mut ppc_mac_gpu_post_regs) }
}
