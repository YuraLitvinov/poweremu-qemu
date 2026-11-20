use super::*;
pub fn c6_to_8(mut v: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut b: ::core::ffi::c_int = 0;
    v &= 0x3f as ::core::ffi::c_int;
    b = v & 1 as ::core::ffi::c_int;
    return v << 2 as ::core::ffi::c_int | b << 1 as ::core::ffi::c_int | b;
}
pub fn I2CDDC(obj: *const ::core::ffi::c_void) -> *mut I2CDDCState {
    crate::src::qemu::qom_object_cast(
        obj,
        c"i2c-ddc",
        c"/home/yurii-sama/Downloads/qemu-master/include/hw/display/i2c-ddc.h",
        39 as ::core::ffi::c_int,
        c"I2CDDC",
    )
}
pub fn PPC_MAC_GPU(obj: *const ::core::ffi::c_void) -> *mut PPCMacGPUFields {
    crate::src::qemu::qom_object_cast(
        obj,
        c"ppc-mac-gpu",
        c"/home/yurii-sama/Downloads/qemu-master/include/hw/display/ppc_mac_gpu.h",
        614 as ::core::ffi::c_int,
        c"PPC_MAC_GPU",
    )
}
pub fn surface_class_name(c: SurfaceClass) -> &'static str {
    match c as ::core::ffi::c_uint {
        0 => return "tex_tile",
        1 => return "comp_rt",
        2 => return "win_rt",
        3 => return "intermediate",
        4 => return "visible_fb",
        5 => return "2d_only",
        _ => return "unknown",
    };
}
pub fn pass_event_name(e: PassEventType) -> &'static str {
    match e as ::core::ffi::c_uint {
        0 => return "bind_rt",
        1 => return "sample_tex",
        2 => return "render",
        3 => return "writeback",
        4 => return "present",
        5 => return "fallback_2d",
        6 => return "host_upload",
        7 => return "blit_upload",
        8 => return "drag_copy",
        _ => return "?",
    };
}
pub fn classify_surface(
    offset: uint32_t,
    _pitch: uint32_t,
    dst_offset: uint32_t,
) -> SurfaceClass {
    if offset == 0 as uint32_t || dst_offset == 0 as uint32_t {
        return SURF_VISIBLE_FB;
    }
    if offset == 0x300000 as ::core::ffi::c_int as uint32_t {
        return SURF_COMPOSITOR_RT;
    }
    if offset == 0x54a000 as ::core::ffi::c_int as uint32_t
        || offset == 0x60a000 as ::core::ffi::c_int as uint32_t
        || offset == 0x992000 as ::core::ffi::c_int as uint32_t
    {
        return SURF_WINDOW_RT;
    }
    if offset == 0x1ee000 as ::core::ffi::c_int as uint32_t
        || offset == 0x462000 as ::core::ffi::c_int as uint32_t
    {
        return SURF_INTERMEDIATE;
    }
    if offset >= 0x30e000 as ::core::ffi::c_int as uint32_t
        && offset < 0x370000 as ::core::ffi::c_int as uint32_t
    {
        return SURF_INTERMEDIATE;
    }
    return SURF_2D_ONLY;
}
pub fn fb_write_watch(
    path: *const ::core::ffi::c_char,
    src_kind: *const ::core::ffi::c_char,
    src_off: uint32_t,
    dst_off: uint32_t,
    dst_x: uint32_t,
    dst_y: uint32_t,
    w: uint32_t,
    h: uint32_t,
    from_srt: bool,
    nonzero_sample: bool,
) {
    if !gpu_diag_on() {
        return;
    }
    if g_fb_watch_count.get() >= 500 as ::core::ffi::c_int {
        return;
    }
    if dst_off > 0x10000 as ::core::ffi::c_int as uint32_t {
        return;
    }
    if w < 4 as uint32_t || h < 4 as uint32_t {
        return;
    }
    g_fb_watch_count.set(g_fb_watch_count.get() + 1);
    crate::src::logs::err(format_args!("[FB_WRITE_WATCH] #{} path={} src_off=0x{:x} src_kind={} dst_xy=({},{}) size=({}x{}) from_srt={} nz={}\n", ((g_fb_watch_count.get()) as i32), crate::src::qemu::cstr(path), ((src_off) as u32), crate::src::qemu::cstr(src_kind), ((dst_x) as u32), ((dst_y) as u32), ((w) as u32), ((h) as u32), ((from_srt as ::core::ffi::c_int) as i32), ((nonzero_sample as ::core::ffi::c_int) as i32)));
}
pub fn post_probe_find_or_add_reg(reg: uint32_t) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < g_post_probe_audit_mut().num_entries {
        if g_post_probe_audit_mut().entries[i as usize].reg == reg {
            return i;
        }
        i += 1;
    }
    if g_post_probe_audit_mut().num_entries >= 256 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    let fresh8 = g_post_probe_audit_mut().num_entries;
    g_post_probe_audit_mut().num_entries = g_post_probe_audit_mut().num_entries + 1;
    let idx: ::core::ffi::c_int = fresh8;
    g_post_probe_audit_mut().entries[idx as usize].reg = reg;
    g_post_probe_audit_mut().entries[idx as usize].count = 0 as uint32_t;
    g_post_probe_audit_mut().entries[idx as usize].first_at_draw = g_post_probe_draw_count.get() as uint32_t;
    return idx;
}
pub fn gmc_dst_bpp(gmc: uint32_t) -> uint32_t {
    if !crate::src::qemu::getenv(c"POWEREMU_TEX_TRACE").is_null() {
        static seen_2d: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
        let dt: uint32_t = gmc >> 8 as ::core::ffi::c_int & 0xf as uint32_t;
        if seen_2d.get() as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << dt == 0 {
            seen_2d.set(seen_2d.get() | (1 as ::core::ffi::c_uint) << dt);
            crate::src::logs::err(format_args!("ppc-mac-gpu 2d: datatype {} (gmc=0x{:08x})\n", ((dt) as u32), ((gmc) as u32)));
        }
    }
    return r200_datatype_bpp(gmc >> 8 as ::core::ffi::c_int);
}
pub fn host_data_put_narrow(
    s: &mut PPCMacGPUFields,
    vram: crate::src::qemu::Vram,
    val: uint32_t,
    bpp: uint32_t,
    base: uint64_t,
    pitch: uint32_t,
    x0: uint32_t,
    y0: uint32_t,
    w: uint32_t,
    h: uint32_t,
    cx: &mut uint32_t,
    cy: &mut uint32_t,
) -> bool {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        if *cy >= h {
            return 0 as ::core::ffi::c_int != 0;
        }
        let a: uint64_t = base
            .wrapping_add((y0.wrapping_add(*cy) as uint64_t).wrapping_mul(pitch as uint64_t))
            .wrapping_add((x0.wrapping_add(*cx) as uint64_t).wrapping_mul(bpp as uint64_t));
        if a.wrapping_add(bpp as uint64_t) <= s.vram_size {
            let mut b: uint32_t = 0 as uint32_t;
            while b < bpp {
                vram.put8(((a.wrapping_add(b as uint64_t)) as u64), ((val
                    .wrapping_shr((8 as uint32_t).wrapping_mul((i as uint32_t).wrapping_add(b))))
                    as uint8_t) as u8);
                b = b.wrapping_add(1);
            }
        }
        *cx = (*cx).wrapping_add(1);
        if *cx >= w {
            *cx = 0 as uint32_t;
            *cy = (*cy).wrapping_add(1);
            break;
        } else {
            i = (i as uint32_t).wrapping_add(bpp) as ::core::ffi::c_int as ::core::ffi::c_int;
        }
    }
    return *cy < h;
}
pub fn fill_vram_px(
    s: &mut PPCMacGPUFields,
    vram: crate::src::qemu::Vram,
    addr: uint64_t,
    color: uint32_t,
    bpp: uint32_t,
) {
    if bpp == 4 {
        mc_vram_write32(s, vram, addr, color);
    } else if addr.wrapping_add(bpp as uint64_t) <= s.vram_size {
        if bpp == 2 {
            vram.put16(addr, color as u16);
        } else {
            vram.put8(addr, color as u8);
        }
    }
}
pub fn is_r300_register(addr: uint32_t) -> bool {
    if addr >= 0x2080 as uint32_t && addr <= 0x22d8 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x3428 as uint32_t && addr <= 0x342c as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x4000 as uint32_t && addr <= 0x4f78 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x400 as uint32_t && addr <= 0x514 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr == 0x15c as uint32_t
        || addr == 0x1d0 as uint32_t
        || addr == 0x1dc as uint32_t
        || addr == 0x1e0 as uint32_t
    {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr == 0x218 as uint32_t || addr == 0x23c as uint32_t || addr == 0x328 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x70c as uint32_t && addr <= 0x744 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr == 0xab0 as uint32_t || addr == 0xab4 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x142c as uint32_t && addr <= 0x1438 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x15d0 as uint32_t && addr <= 0x15fc as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x16cc as uint32_t && addr <= 0x1710 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr >= 0x1d98 as uint32_t && addr <= 0x1dac as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    if addr == 0x1fa8 as uint32_t {
        return 1 as ::core::ffi::c_int != 0;
    }
    return 0 as ::core::ffi::c_int != 0;
}
pub extern "C" fn do_qemu_init_ppc_mac_gpu_register_types() {
    crate::src::qemu::register_qom_module_init(ppc_mac_gpu_register_types);
}
/// One `VMStateField` for the device's vmstate table; `num`, `vmsd` and the
/// indirect-size members are left at their zero defaults (set them with
/// struct-update syntax where a field needs them).
fn ppc_vmfield(
    name: *const ::core::ffi::c_char,
    offset: usize,
    size: usize,
    info: *const VMStateInfo,
    flags: VMStateFlags,
    version_id: ::core::ffi::c_int,
) -> VMStateField {
    VMStateField {
        name,
        offset: offset as usize,
        size: size as usize,
        size_indirect: VMStateStructMember { offset: 0, size: 0 },
        num_indirect: VMStateStructMember { offset: 0, size: 0 },
        start: 0,
        num: 0,
        info,
        flags,
        vmsd: ::core::ptr::null(),
        version_id,
        struct_version_id: 0,
        field_exists: None,
    }
}

/// Builds the device's property table and vmstate description once, at load
/// time. The field array is leaked so the description keeps a valid pointer.
pub extern "C" fn run_static_initializers() {
    let props: [Property; 5] = [
        Property {
            name: c"vgamem_mb".as_ptr(),
            info: &raw const qdev_prop_uint32,
            offset: foff(::core::mem::offset_of!(PPCMacGPUFields, vram_size_mb)) as ptrdiff_t,
            link_type: ::core::ptr::null::<::core::ffi::c_char>(),
            bitmask: 0,
            defval: C2RustUnnamed_81 { u: 128 },
            arrayinfo: ::core::ptr::null::<PropertyInfo>(),
            arrayoffset: 0,
            arrayfieldsize: 0,
            bitnr: 0,
            set_default: true,
        },
        Property {
            name: c"host-aspect-modes".as_ptr(),
            info: &raw const qdev_prop_bool,
            offset: foff(::core::mem::offset_of!(PPCMacGPUFields, host_aspect_modes)) as ptrdiff_t,
            link_type: ::core::ptr::null::<::core::ffi::c_char>(),
            bitmask: 0,
            defval: C2RustUnnamed_81 { u: 0 },
            arrayinfo: ::core::ptr::null::<PropertyInfo>(),
            arrayoffset: 0,
            arrayfieldsize: 0,
            bitnr: 0,
            set_default: true,
        },
        Property {
            name: c"legacy-vga".as_ptr(),
            info: &raw const qdev_prop_bool,
            offset: foff(::core::mem::offset_of!(PPCMacGPUFields, legacy_vga)) as ptrdiff_t,
            link_type: ::core::ptr::null::<::core::ffi::c_char>(),
            bitmask: 0,
            defval: C2RustUnnamed_81 { u: 0 },
            arrayinfo: ::core::ptr::null::<PropertyInfo>(),
            arrayoffset: 0,
            arrayfieldsize: 0,
            bitnr: 0,
            set_default: true,
        },
        Property {
            name: c"biosrom".as_ptr(),
            info: &raw const qdev_prop_string,
            offset: foff(::core::mem::offset_of!(PPCMacGPUFields, biosrom)) as ptrdiff_t,
            link_type: ::core::ptr::null::<::core::ffi::c_char>(),
            bitmask: 0,
            defval: C2RustUnnamed_81 { i: 0 },
            arrayinfo: ::core::ptr::null::<PropertyInfo>(),
            arrayoffset: 0,
            arrayfieldsize: 0,
            bitnr: 0,
            set_default: false,
        },
        Property {
            name: c"renderer".as_ptr(),
            info: &raw const qdev_prop_string,
            offset: foff(::core::mem::offset_of!(PPCMacGPUFields, renderer_name)) as ptrdiff_t,
            link_type: ::core::ptr::null::<::core::ffi::c_char>(),
            bitmask: 0,
            defval: C2RustUnnamed_81 { i: 0 },
            arrayinfo: ::core::ptr::null::<PropertyInfo>(),
            arrayoffset: 0,
            arrayfieldsize: 0,
            bitnr: 0,
            set_default: false,
        },
    ];

    let u32_info = &raw const vmstate_info_uint32;
    let bool_info = &raw const vmstate_info_bool;
    let buffer_info = &raw const vmstate_info_buffer;
    let fields: &'static [VMStateField] = Box::leak(Box::new([
        // VMSTATE_PCI_DEVICE: a VMS_STRUCT field must carry the sub-description, or QEMU dereferences a
        // null `vmsd` while saving (savevm crashed in vmsd_can_compress).
        VMStateField {
            vmsd: &raw const pci_sys::vmstate_pci_device,
            ..ppc_vmfield(
                c"pci".as_ptr(),
                ::core::mem::offset_of!(PPCMacGPUState, pci),
                ::core::mem::size_of::<PCIDevice>(),
                ::core::ptr::null(),
                VMS_STRUCT,
                0,
            )
        },
        ppc_vmfield(
            c"regs_size".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, regs_size)),
            ::core::mem::size_of::<uint32_t>(),
            &raw const vmstate_info_uint32_equal,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"regs".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, regs)),
            ::core::mem::size_of::<PPCMacGPURegs>(),
            buffer_info,
            VMS_BUFFER,
            1,
        ),
        ppc_vmfield(
            c"display_invalid".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, display_invalid)),
            ::core::mem::size_of::<bool>(),
            bool_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"disp_stride_override_active".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, disp_stride_override_active)),
            ::core::mem::size_of::<bool>(),
            bool_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"disp_stride_override_value".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, disp_stride_override_value)),
            ::core::mem::size_of::<uint32_t>(),
            &raw const vmstate_info_uint32,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"compositor_base".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, compositor_base)),
            ::core::mem::size_of::<uint32_t>(),
            u32_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"compositor_pitch".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, compositor_pitch)),
            ::core::mem::size_of::<uint32_t>(),
            u32_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"compositor_valid".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, compositor_valid)),
            ::core::mem::size_of::<bool>(),
            bool_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"tiled_surfaces".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, tiled_surfaces)),
            ::core::mem::size_of::<[C2RustUnnamed_91; 8]>(),
            buffer_info,
            VMS_BUFFER,
            1,
        ),
        ppc_vmfield(
            c"hwc_pix".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_pix)),
            ::core::mem::size_of::<[uint32_t; 4096]>(),
            buffer_info,
            VMS_BUFFER,
            1,
        ),
        ppc_vmfield(
            c"hwc_w".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_w)),
            ::core::mem::size_of::<uint32_t>(),
            u32_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"hwc_h".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_h)),
            ::core::mem::size_of::<uint32_t>(),
            u32_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"hwc_idx".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_idx)),
            ::core::mem::size_of::<uint32_t>(),
            u32_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"hwc_x".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_x)),
            ::core::mem::size_of::<int32_t>(),
            &raw const vmstate_info_int32,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"hwc_y".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_y)),
            ::core::mem::size_of::<int32_t>(),
            &raw const vmstate_info_int32,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"hwc_visible".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, hwc_visible)),
            ::core::mem::size_of::<bool>(),
            bool_info,
            VMS_SINGLE,
            0,
        ),
        ppc_vmfield(
            c"r300_shadow".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, r300_shadow)),
            ::core::mem::size_of::<[uint32_t; 5120]>(),
            buffer_info,
            VMS_BUFFER,
            1,
        ),
        ppc_vmfield(
            c"mc_ind_index".as_ptr(),
            foff(::core::mem::offset_of!(PPCMacGPUFields, mc_ind_index)),
            ::core::mem::size_of::<uint32_t>(),
            u32_info,
            VMS_SINGLE,
            2,
        ),
        VMStateField {
            num: 128,
            ..ppc_vmfield(
                c"mc_ind".as_ptr(),
                foff(::core::mem::offset_of!(PPCMacGPUFields, mc_ind)),
                ::core::mem::size_of::<uint32_t>(),
                u32_info,
                VMS_ARRAY,
                2,
            )
        },
        ppc_vmfield(::core::ptr::null(), 0, 0, ::core::ptr::null(), VMS_END, 0),
    ]));

    let vmsd = VMStateDescription {
        name: c"ppc-mac-gpu".as_ptr(),
        unmigratable: false,
        early_setup: false,
        version_id: 2,
        minimum_version_id: 1,
        priority: migration_sys::MigrationPriority::MIG_PRI_UNINITIALIZED,
        pre_load: None,
        pre_load_errp: None,
        post_load: Some(ppc_mac_gpu_post_load),
        post_load_errp: None,
        pre_save: None,
        pre_save_errp: None,
        post_save: None,
        needed: None,
        dev_unplug_pending: None,
        fields: fields.as_ptr(),
        subsections: ::core::ptr::null::<*const VMStateDescription>(),
    };
    crate::src::qemu::install_ppc_mac_gpu_tables(props, vmsd);
}
