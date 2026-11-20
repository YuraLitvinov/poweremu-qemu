use super::*;
#[repr(C)]
pub struct CPUState {
    pub parent_obj: DeviceState,
    pub cc: *const CPUClass,
    pub nr_threads: ::core::ffi::c_int,
    pub thread: *mut QemuThread,
    pub thread_id: ::core::ffi::c_int,
    pub running: bool,
    pub has_waiter: bool,
    pub halt_cond: *mut QemuCond,
    pub thread_kicked: bool,
    pub created: bool,
    pub stop: bool,
    pub stopped: bool,
    pub start_powered_off: bool,
    pub unplug: bool,
    pub crash_occurred: bool,
    pub exit_request: bool,
    pub exclusive_context_count: ::core::ffi::c_int,
    pub cflags_next_tb: uint32_t,
    pub interrupt_request: uint32_t,
    pub singlestep_flags: ::core::ffi::c_uint,
    pub icount_budget: int64_t,
    pub icount_extra: int64_t,
    pub random_seed: uint64_t,
    pub jmp_env: sigjmp_buf,
    pub work_mutex: QemuMutex,
    pub work_list: C2RustUnnamed_78,
    pub cpu_ases: *mut CPUAddressSpace,
    pub as_0: *mut AddressSpace,
    pub memory: *mut MemoryRegion,
    pub tb_jmp_cache: *mut CPUJumpCache,
    pub gdb_regs: *mut GArray,
    pub gdb_num_regs: ::core::ffi::c_int,
    pub gdb_num_g_regs: ::core::ffi::c_int,
    pub node: C2RustUnnamed_77,
    pub breakpoints: C2RustUnnamed_75,
    pub watchpoints: C2RustUnnamed_74,
    pub watchpoint_hit: *mut CPUWatchpoint,
    pub opaque: *mut ::core::ffi::c_void,
    pub mem_io_pc: uintptr_t,
    pub kvm_fd: ::core::ffi::c_int,
    pub kvm_state: *mut KVMState,
    pub kvm_run: *mut kvm_run,
    pub kvm_dirty_gfns: *mut kvm_dirty_gfn,
    pub kvm_fetch_index: uint32_t,
    pub dirty_pages: uint64_t,
    pub kvm_vcpu_stats_fd: ::core::ffi::c_int,
    pub in_ioctl_lock: QemuLockCnt,
    pub plugin_state: *mut CPUPluginState,
    pub cpu_index: ::core::ffi::c_int,
    pub cluster_index: ::core::ffi::c_int,
    pub tcg_cflags: uint32_t,
    pub halted: uint32_t,
    pub exception_index: int32_t,
    pub vcpu_dirty: bool,
    pub accel: *mut AccelCPUState,
    pub throttle_thread_scheduled: bool,
    pub throttle_us_per_full: int64_t,
    pub ignore_memory_transaction_failures: bool,
    pub prctl_unalign_sigbus: bool,
    pub iommu_notifiers: *mut GArray,
    pub neg_align: [::core::ffi::c_char; 0],
    pub neg: CPUNegativeOffsetState,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUNegativeOffsetState {
    pub tlb: CPUTLB,
    pub plugin_mem_cbs: *mut GArray,
    pub plugin_mem_value_low: uint64_t,
    pub plugin_mem_value_high: uint64_t,
    pub plugin_cb_flags: int32_t,
    pub icount_decr: IcountDecr,
    pub can_do_io: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUTLB {
    pub c: CPUTLBCommon,
    pub d: [CPUTLBDesc; 22],
    pub f: [CPUTLBDescFast; 22],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUTLBDescFast {
    pub mask: uintptr_t,
    pub table: *mut CPUTLBEntry,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union CPUTLBEntry {
    pub c2rust_unnamed: C2RustUnnamed_19,
    pub addr_idx: [uintptr_t; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_19 {
    pub addr_read: uintptr_t,
    pub addr_write: uintptr_t,
    pub addr_code: uintptr_t,
    pub addend: uintptr_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUTLBDesc {
    pub large_page_addr: vaddr,
    pub large_page_mask: vaddr,
    pub window_begin_ns: int64_t,
    pub window_max_entries: size_t,
    pub n_used_entries: size_t,
    pub vindex: size_t,
    pub vtable: [CPUTLBEntry; 8],
    pub vfulltlb: [CPUTLBEntryFull; 8],
    pub fulltlb: *mut CPUTLBEntryFull,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUTLBEntryFull {
    pub xlat_offset: hwaddr,
    pub section: *mut MemoryRegionSection,
    pub phys_addr: hwaddr,
    pub attrs: MemTxAttrs,
    pub prot: uint8_t,
    pub lg_page_size: uint8_t,
    pub tlb_fill_flags: uint8_t,
    pub slow_flags: [uint8_t; 3],
    pub extra: C2RustUnnamed_20,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_20 {
    pub arm: C2RustUnnamed_21,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_21 {
    pub pte_attrs: uint8_t,
    pub shareability: uint8_t,
    pub guarded: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUTLBCommon {
    pub lock: QemuSpin,
    pub dirty: MMUIdxMap,
    pub full_flush_count: size_t,
    pub part_flush_count: size_t,
    pub elide_flush_count: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUWatchpoint {
    pub vaddr: vaddr,
    pub len: vaddr,
    pub hitaddr: vaddr,
    pub hitattrs: MemTxAttrs,
    pub flags: ::core::ffi::c_int,
    pub entry: C2RustUnnamed_73,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_73 {
    pub tqe_next: *mut CPUWatchpoint,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_74 {
    pub tqh_first: *mut CPUWatchpoint,
    pub tqh_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_75 {
    pub tqh_first: *mut CPUBreakpoint,
    pub tqh_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CPUBreakpoint {
    pub pc: vaddr,
    pub flags: ::core::ffi::c_int,
    pub entry: C2RustUnnamed_76,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_76 {
    pub tqe_next: *mut CPUBreakpoint,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_77 {
    pub tqe_next: *mut CPUState,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_78 {
    pub sqh_first: *mut qemu_work_item,
    pub sqh_last: *mut *mut qemu_work_item,
}
#[repr(C)]
pub struct CPUClass {
    pub parent_class: DeviceClass,
    pub class_by_name: Option<unsafe extern "C" fn(*const ::core::ffi::c_char) -> *mut ObjectClass>,
    pub list_cpus: Option<unsafe extern "C" fn() -> ()>,
    pub parse_features: Option<
        unsafe extern "C" fn(
            *const ::core::ffi::c_char,
            *mut ::core::ffi::c_char,
            *mut *mut Error,
        ) -> (),
    >,
    pub memory_rw_debug: Option<
        unsafe extern "C" fn(
            *mut CPUState,
            vaddr,
            *mut uint8_t,
            size_t,
            bool,
        ) -> ::core::ffi::c_int,
    >,
    pub dump_state:
        Option<unsafe extern "C" fn(*mut CPUState, *mut FILE, ::core::ffi::c_int) -> ()>,
    pub query_cpu_fast: Option<unsafe extern "C" fn(*mut CPUState, *mut CpuInfoFast) -> ()>,
    pub get_arch_id: Option<unsafe extern "C" fn(*mut CPUState) -> int64_t>,
    pub set_pc: Option<unsafe extern "C" fn(*mut CPUState, vaddr) -> ()>,
    pub get_pc: Option<unsafe extern "C" fn(*mut CPUState) -> vaddr>,
    pub gdb_read_register: Option<
        unsafe extern "C" fn(
            *mut CPUState,
            *mut GByteArray,
            ::core::ffi::c_int,
        ) -> ::core::ffi::c_int,
    >,
    pub gdb_write_register: Option<
        unsafe extern "C" fn(*mut CPUState, *mut uint8_t, ::core::ffi::c_int) -> ::core::ffi::c_int,
    >,
    pub gdb_adjust_breakpoint: Option<unsafe extern "C" fn(*mut CPUState, vaddr) -> vaddr>,
    pub gdb_core_xml_file: *const ::core::ffi::c_char,
    pub gdb_arch_name: Option<unsafe extern "C" fn(*mut CPUState) -> *const ::core::ffi::c_char>,
    pub gdb_get_core_xml_file:
        Option<unsafe extern "C" fn(*mut CPUState) -> *const ::core::ffi::c_char>,
    pub exception_name:
        Option<unsafe extern "C" fn(::core::ffi::c_int) -> *const ::core::ffi::c_char>,
    pub disas_set_info: Option<unsafe extern "C" fn(*const CPUState, *mut disassemble_info) -> ()>,
    pub deprecation_note: *const ::core::ffi::c_char,
    pub accel_cpu: *mut AccelCPUClass,
    pub sysemu_ops: *const SysemuCPUOps,
    pub tcg_ops: *const TCGCPUOps,
    pub semi_ops: *const SemihostingCPUOps,
    pub reset_dump_flags: ::core::ffi::c_int,
    pub gdb_num_core_regs: ::core::ffi::c_int,
    pub gdb_stop_before_watchpoint: bool,
    pub max_as: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CpuInfoFast {
    pub cpu_index: int64_t,
    pub qom_type: *mut ::core::ffi::c_char,
    pub qom_path: *mut ::core::ffi::c_char,
    pub thread_id: int64_t,
    pub props: *mut CpuInstanceProperties,
    pub target: SysEmuTarget,
    pub u: C2RustUnnamed_80,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_80 {
    pub s390x: CpuInfoS390,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CpuInfoS390 {
    pub cpu_state: S390CpuState,
    pub has_dedicated: bool,
    pub dedicated: bool,
    pub has_entitlement: bool,
    pub entitlement: S390CpuEntitlement,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CpuInstanceProperties {
    pub has_node_id: bool,
    pub node_id: int64_t,
    pub has_drawer_id: bool,
    pub drawer_id: int64_t,
    pub has_book_id: bool,
    pub book_id: int64_t,
    pub has_socket_id: bool,
    pub socket_id: int64_t,
    pub has_die_id: bool,
    pub die_id: int64_t,
    pub has_cluster_id: bool,
    pub cluster_id: int64_t,
    pub has_module_id: bool,
    pub module_id: int64_t,
    pub has_core_id: bool,
    pub core_id: int64_t,
    pub has_thread_id: bool,
    pub thread_id: int64_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300State {
    pub regs: [uint32_t; 3328],
    pub pvs_mem: [uint32_t; 6144],
    pub pvs_upload_dw: uint32_t,
    pub pvs_gen: uint64_t,
    pub draws: uint64_t,
    pub win_immd: bool,
    pub le_screen_h: uint32_t,
    pub glsl_flags: uint32_t,
}
pub type R300WriteFn =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *const uint8_t, size_t) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300Vertex {
    pub pos: [::core::ffi::c_float; 4],
    pub v: [[::core::ffi::c_float; 4]; 10],
    pub aux: [::core::ffi::c_float; 4],
    pub ucp: [::core::ffi::c_float; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300FSUniforms {
    pub consts: [[::core::ffi::c_float; 4]; 32],
    pub blend_color: [::core::ffi::c_float; 4],
    pub tex_swz: [[uint32_t; 4]; 16],
    pub tex_info: [[uint32_t; 4]; 16],
    pub cblend: uint32_t,
    pub ablend: uint32_t,
    pub chanmask: uint32_t,
    pub alpha_func: uint32_t,
    pub out_sel: [uint32_t; 4],
    pub rt_swap32: uint32_t,
    pub clip_rule: uint32_t,
    pub rt_endian: uint32_t,
    pub rop: uint32_t,
    pub cliprect: [[int32_t; 4]; 4],
    pub zinfo: [uint32_t; 4],
    pub zpass_count: uint32_t,
    pub poly_en: uint32_t,
    pub pad2: [uint32_t; 2],
    pub tex_border: [[::core::ffi::c_float; 4]; 16],
    pub tex_lod: [[::core::ffi::c_float; 4]; 16],
    pub tex_dim: [[uint32_t; 4]; 16],
    pub fog_color: [::core::ffi::c_float; 4],
    pub fog_blend: uint32_t,
    pub depth_src: uint32_t,
    pub pad3: [uint32_t; 2],
    pub poly_offset: [::core::ffi::c_float; 4],
    pub tex_addr: [[uint32_t; 4]; 16],
    pub tex_lvl: [[[uint32_t; 4]; 9]; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300Arrays {
    pub count: ::core::ffi::c_uint,
    pub a: [C2RustUnnamed_87; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_87 {
    pub addr: uint32_t,
    pub size_dw: uint32_t,
    pub stride_dw: uint32_t,
}
pub type C2RustUnnamed_88 = ::core::ffi::c_uint;
pub const R300_TEXK_DXT5: C2RustUnnamed_88 = 7;
pub const R300_TEXK_DXT3: C2RustUnnamed_88 = 6;
pub const R300_TEXK_DXT1: C2RustUnnamed_88 = 5;
pub const R300_TEXK_RAW: C2RustUnnamed_88 = 4;
pub const R300_TEXK_CONVERT16: C2RustUnnamed_88 = 3;
pub const R300_TEXK_RG8: C2RustUnnamed_88 = 2;
pub const R300_TEXK_R8: C2RustUnnamed_88 = 1;
pub const R300_TEXK_RGBA8: C2RustUnnamed_88 = 0;
pub type C2RustUnnamed_89 = ::core::ffi::c_uint;
pub const R300_TEXDIM_CUBE: C2RustUnnamed_89 = 2;
pub const R300_TEXDIM_3D: C2RustUnnamed_89 = 1;
pub const R300_TEXDIM_2D: C2RustUnnamed_89 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300TexDesc {
    pub bound: bool,
    pub gpu_addr: uint32_t,
    pub width: uint32_t,
    pub height: uint32_t,
    pub endian: uint32_t,
    pub le: bool,
    pub tile: uint32_t,
    pub pitch_bytes: uint32_t,
    pub format: uint32_t,
    pub kind: uint32_t,
    pub view_bpp: uint32_t,
    pub filter0: uint32_t,
    pub filter1: uint32_t,
    pub host_data: *mut uint8_t,
    pub dim: uint32_t,
    pub depth: uint32_t,
    pub levels: uint32_t,
    pub lvl_off: [uint32_t; 12],
    pub lvl_pitch: [uint32_t; 12],
    pub lvl_rows: [uint32_t; 12],
    pub size_bytes: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300DepthDesc {
    pub attach: bool,
    pub gpu_addr: uint32_t,
    pub pitch: uint32_t,
    pub bpp: uint32_t,
    pub format: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300ColorDesc {
    pub gpu_addr: uint32_t,
    pub pitch: uint32_t,
    pub format: uint32_t,
    pub bpp: uint32_t,
    pub view: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300DrawPacket {
    pub rt_gpu_addr: uint32_t,
    pub rt_pitch: uint32_t,
    pub rt_width: uint32_t,
    pub rt_height: uint32_t,
    pub rt_format: uint32_t,
    pub rt_bpp: uint32_t,
    pub rt_dummy: bool,
    pub harvest_hint: bool,
    pub depth_min: ::core::ffi::c_float,
    pub depth_max: ::core::ffi::c_float,
    pub depth_clip: bool,
    pub depth_dx: bool,
    pub rt_view: uint32_t,
    pub scissor: [uint32_t; 4],
    pub num_cb: uint32_t,
    pub cb: [R300ColorDesc; 4],
    pub depth: R300DepthDesc,
    pub cull: uint32_t,
    pub aa_samples: uint32_t,
    pub aa_pos: [[::core::ffi::c_float; 2]; 6],
    pub glsl: *mut ::core::ffi::c_char,
    pub uniforms: R300FSUniforms,
    pub tex: [R300TexDesc; 16],
    pub verts: *mut R300Vertex,
    pub gpu_vs: bool,
    pub gpu_data: *mut uint32_t,
    pub gpu_words: uint32_t,
    pub num_verts: uint32_t,
    pub num_line_verts: uint32_t,
    pub prim_class: uint32_t,
    pub warn: uint32_t,
}
pub type R300ReadFn = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        uint32_t,
        *mut ::core::ffi::c_void,
        uint32_t,
    ) -> bool,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct R300Indices {
    pub dw: *const uint32_t,
    pub ndw: uint32_t,
}
pub type PEHarmonyArea = ::core::ffi::c_uint;
pub const PE_AREA_WINDOW: PEHarmonyArea = 3;
pub const PE_AREA_MENUBAR: PEHarmonyArea = 2;
pub const PE_AREA_DESKTOP: PEHarmonyArea = 1;
pub const PE_AREA_UNKNOWN: PEHarmonyArea = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPURenderer {
    pub name: *const ::core::ffi::c_char,
    pub r300_glsl_flags: uint32_t,
    pub init: Option<unsafe extern "C" fn(*mut uint8_t, uint64_t) -> *mut ::core::ffi::c_void>,
    pub fini: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()>,
    pub scanout: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *const PPCMacGPUScanout,
        ) -> *mut ::core::ffi::c_void,
    >,
    pub blit_2d: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint8_t,
            *const PPCMacGPUBlit,
        ) -> ::core::ffi::c_int,
    >,
    pub fill_2d: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint8_t,
            *const PPCMacGPUBlit,
        ) -> ::core::ffi::c_int,
    >,
    pub draw_3d: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint8_t,
            uint64_t,
            *const PPCMacGPU3DState,
            *const PPCMacGPU3DDrawCmd,
        ) -> ::core::ffi::c_int,
    >,
    pub draw_r300: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint8_t,
            uint64_t,
            *const R300DrawPacket,
        ) -> ::core::ffi::c_int,
    >,
    pub zpass_r300:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, bool, uint32_t) -> uint32_t>,
    pub flush_sync: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> bool>,
    pub submit_fence: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, uint32_t) -> ()>,
            *mut ::core::ffi::c_void,
        ) -> uint32_t,
    >,
    pub range_busy_sync:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, uint64_t, uint64_t, bool) -> bool>,
    pub fill_notify: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
        ) -> (),
    >,
    pub set_dirty_source: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            PPCMacGPUDirtyFn,
            *mut ::core::ffi::c_void,
        ) -> (),
    >,
    pub mode_change:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *const PPCMacGPUScanout) -> ()>,
    pub get_caps: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> uint32_t>,
    pub srt_write_through: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint8_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
            uint32_t,
        ) -> (),
    >,
    pub get_drag_state: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint32_t,
            *mut uint32_t,
            *mut uint32_t,
            *mut uint32_t,
            *mut uint32_t,
        ) -> bool,
    >,
    pub get_drag_snap: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut uint32_t,
            *mut uint32_t,
        ) -> *const uint32_t,
    >,
    pub flush_drag_paste:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut uint8_t) -> ()>,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPUScanout {
    pub width: uint32_t,
    pub height: uint32_t,
    pub bpp: uint32_t,
    pub stride: uint32_t,
    pub offset: uint64_t,
    pub big_endian: bool,
}
pub type PPCMacGPUDirtyFn = Option<
    unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_ulong, uint64_t) -> (),
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPU3DDrawCmd {
    pub opcode: uint32_t,
    pub prim_type: uint32_t,
    pub num_vertices: uint32_t,
    pub vertex_data: *const uint32_t,
    pub vertex_data_dwords: uint32_t,
    pub vb_addr: uint32_t,
    pub vb_stride: uint32_t,
    pub index_data: *const uint16_t,
    pub num_indices: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPU3DState {
    pub se_vtx_fmt_0: uint32_t,
    pub se_vtx_fmt_1: uint32_t,
    pub se_vte_cntl: uint32_t,
    pub pp_cntl: uint32_t,
    pub pp_misc: uint32_t,
    pub pp_txfilter_0: uint32_t,
    pub pp_txformat_0: uint32_t,
    pub pp_txformat_x_0: uint32_t,
    pub pp_txsize_0: uint32_t,
    pub pp_txpitch_0: uint32_t,
    pub pp_txoffset_0: uint32_t,
    pub r100_pp_txfilter_0: uint32_t,
    pub r100_pp_txformat_0: uint32_t,
    pub r100_pp_txoffset_0: uint32_t,
    pub r100_pp_tex_size_0: uint32_t,
    pub r100_pp_tex_pitch_0: uint32_t,
    pub pp_txcblend_0: uint32_t,
    pub pp_txcblend2_0: uint32_t,
    pub pp_txablend_0: uint32_t,
    pub pp_txablend2_0: uint32_t,
    pub rb3d_cntl: uint32_t,
    pub rb3d_coloroffset: uint32_t,
    pub rb3d_colorpitch: uint32_t,
    pub rb3d_blendcntl: uint32_t,
    pub rb3d_ablendcntl: uint32_t,
    pub rb3d_cblendcntl: uint32_t,
    pub rb3d_zstencilcntl: uint32_t,
    pub se_cntl: uint32_t,
    pub re_cntl: uint32_t,
    pub se_vport_xscale: uint32_t,
    pub se_vport_xoffset: uint32_t,
    pub se_vport_yscale: uint32_t,
    pub se_vport_yoffset: uint32_t,
    pub se_vport_zscale: uint32_t,
    pub se_vport_zoffset: uint32_t,
    pub re_top_left: uint32_t,
    pub re_width_height: uint32_t,
    pub screen_width: uint32_t,
    pub screen_height: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPUBlit {
    pub src_x: uint32_t,
    pub src_y: uint32_t,
    pub src_offset: uint32_t,
    pub src_pitch: uint32_t,
    pub dst_x: uint32_t,
    pub dst_y: uint32_t,
    pub dst_offset: uint32_t,
    pub dst_pitch: uint32_t,
    pub width: uint32_t,
    pub height: uint32_t,
    pub rop3: uint32_t,
    pub fg_color: uint32_t,
    pub bg_color: uint32_t,
    pub bpp: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPURegs {
    pub mm_index: uint32_t,
    pub bios_scratch: [uint32_t; 8],
    pub gen_int_cntl: uint32_t,
    pub gen_int_status: uint32_t,
    pub crtc_gen_cntl: uint32_t,
    pub crtc_ext_cntl: uint32_t,
    pub crtc_status: uint32_t,
    pub crtc_h_total_disp: uint32_t,
    pub crtc_h_sync_strt_wid: uint32_t,
    pub crtc_v_total_disp: uint32_t,
    pub crtc_vline: uint32_t,
    pub crtc_v_sync_strt_wid: uint32_t,
    pub crtc_offset: uint32_t,
    pub crtc_offset_cntl: uint32_t,
    pub crtc_offset_locked: bool,
    pub crtc_offset_write_ns: int64_t,
    pub crtc_vblank_clear_ns: int64_t,
    pub crtc_pitch: uint32_t,
    pub dac_cntl: uint32_t,
    pub dac_cntl2: uint32_t,
    pub config_cntl: uint32_t,
    pub config_memsize: uint32_t,
    pub mc_fb_location: uint32_t,
    pub mc_agp_location: uint32_t,
    pub mc_status: uint32_t,
    pub bus_cntl: uint32_t,
    pub host_path_cntl: uint32_t,
    pub surface_cntl: uint32_t,
    pub agp_base: uint32_t,
    pub agp_cntl: uint32_t,
    pub agp_command: uint32_t,
    pub agp_status: uint32_t,
    pub rbbm_status: uint32_t,
    pub disp_misc_cntl: uint32_t,
    pub disp_output_cntl: uint32_t,
    pub disp_merge_cntl: uint32_t,
    pub fp_gen_cntl: uint32_t,
    pub cur_offset: uint32_t,
    pub cur_horz_vert_posn: uint32_t,
    pub cur_horz_vert_off: uint32_t,
    pub cur_clr0: uint32_t,
    pub cur_clr1: uint32_t,
    pub dst_offset: uint32_t,
    pub dst_pitch: uint32_t,
    pub dst_width: uint32_t,
    pub dst_height: uint32_t,
    pub dst_y_x: uint32_t,
    pub src_offset: uint32_t,
    pub src_pitch: uint32_t,
    pub src_y_x: uint32_t,
    pub src_pitch_offset: uint32_t,
    pub dst_pitch_offset: uint32_t,
    pub default_pitch_offset: uint32_t,
    pub dst_width_height: uint32_t,
    pub dst_line_start: uint32_t,
    pub brush_y_x: uint32_t,
    pub brush_data: [uint32_t; 16],
    pub dp_gui_master_cntl: uint32_t,
    pub dp_brush_frgd_clr: uint32_t,
    pub dp_brush_bkgd_clr: uint32_t,
    pub dp_src_frgd_clr: uint32_t,
    pub dp_src_bkgd_clr: uint32_t,
    pub dp_write_msk: uint32_t,
    pub dp_cntl: uint32_t,
    pub dp_datatype: uint32_t,
    pub dp_mix: uint32_t,
    pub sc_top_left: uint32_t,
    pub sc_bottom_right: uint32_t,
    pub default_sc_bottom_right: uint32_t,
    pub wait_until: uint32_t,
    pub isync_cntl: uint32_t,
    pub gui_stat: uint32_t,
    pub aic_ctrl: uint32_t,
    pub aic_pt_base: uint32_t,
    pub aic_lo_addr: uint32_t,
    pub aic_hi_addr: uint32_t,
    pub cp_rb_base: uint32_t,
    pub cp_rb_cntl: uint32_t,
    pub cp_rb_rptr: uint32_t,
    pub cp_rb_wptr: uint32_t,
    pub cp_me_cntl: uint32_t,
    pub cp_csq_cntl: uint32_t,
    pub cp_me_ram_addr: uint32_t,
    pub cp_ib_base: uint32_t,
    pub cp_ib_bufsz: uint32_t,
    pub scratch_reg: [uint32_t; 6],
    pub scratch_umsk: uint32_t,
    pub scratch_addr: uint32_t,
    pub stall_draws: uint64_t,
    pub stall_pio_dwords: uint64_t,
    pub stall_ib_done: uint64_t,
    pub stall_ib_dwords: uint64_t,
    pub stall_ib_lost: uint64_t,
    pub csq_just_submitted: bool,
    pub stall_irqs: uint64_t,
    pub stall_ring_dwords: uint64_t,
    pub stall_draws_seen: uint64_t,
    pub cp_rb_rptr_addr: uint32_t,
    pub clock_cntl_index: uint32_t,
    pub clock_cntl_data: uint32_t,
    pub pll: [uint32_t; 64],
    pub gpio_vga_ddc: uint32_t,
    pub gpio_dvi_ddc: uint32_t,
    pub gpio_monid: uint32_t,
    pub palette_index: uint32_t,
    pub palette: [uint32_t; 256],
    pub vbe_regs: [uint16_t; 10],
    pub vga_dac_read_index: uint8_t,
    pub vga_dac_write_index: uint8_t,
    pub vga_dac_sub_index: uint8_t,
    pub vga_dac_palette: [[uint8_t; 3]; 256],
    pub vga_ar_index: uint8_t,
    pub vga_ar_flip_flop: uint8_t,
    pub regs_3d: [uint32_t; 2304],
    pub tcl_vec: [[uint32_t; 4]; 2048],
    pub tcl_vec_addr: uint32_t,
    pub tcl_vec_stride: uint32_t,
    pub tcl_vec_comp: uint32_t,
    pub tcl_scalar: [uint32_t; 512],
    pub tcl_scalar_addr: uint32_t,
    pub tcl_scalar_stride: uint32_t,
    pub r200_fence_q: [C2RustUnnamed_90; 64],
    pub r200_fence_n: uint32_t,
    pub r200_fence_last_seq: uint32_t,
    pub r200_fence_done: uint32_t,
    pub cp_csq_stat_counter: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_90 {
    pub seq: uint32_t,
    pub reg: uint32_t,
    pub val: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PPCMacGPUDisplayMode {
    pub width: uint32_t,
    pub height: uint32_t,
    pub bpp: uint32_t,
    pub stride: uint32_t,
    pub offset: uint32_t,
    pub format: pixman_format_code_t,
    pub crtc_ext: bool,
}
#[repr(C)]
pub struct PPCMacGPUState {
    pub pci: PCIDevice,
    pub vram_dev_dirty: bool,
    pub kick_epoch: uint32_t,
    pub last_draw_epoch: uint32_t,
    pub ring_bh: *mut QEMUBH,
    pub ring_pending: bool,
    pub ring_async: bool,
    pub con: *mut QemuConsole,
    pub vram: MemoryRegion,
    pub vram_bar: MemoryRegion,
    pub vga_bank: MemoryRegion,
    pub vram_swap: MemoryRegion,
    pub r300_surf: [uint32_t; 32],
    pub r300_watch: MemoryRegion,
    pub r300_watch_on: bool,
    pub r300_watch_base: uint32_t,
    pub r300_watch_logged: uint32_t,
    pub io: MemoryRegion,
    pub mmio: MemoryRegion,
    pub rom_mr: MemoryRegion,
    pub rom_data: *mut uint8_t,
    pub rom_size: uint32_t,
    pub biosrom: *mut ::core::ffi::c_char,
    pub renderer_name: *mut ::core::ffi::c_char,
    pub r3: *mut R300State,
    pub r3_arrays: R300Arrays,
    pub r3_dump: *mut FILE,
    pub r3_trace_lines: uint32_t,
    pub r300_src_swap: uint32_t,
    pub r3_zpass_active: bool,
    pub r3_indx_vf: uint32_t,
    pub r3_indx_buf: [uint32_t; 3],
    pub r3_indx_buf_pending: bool,
    pub r3_zb_offset: uint32_t,
    pub r3_zb_height: uint32_t,
    pub r3_zb_ns: uint32_t,
    pub r3_cb_aa_off: uint32_t,
    pub r3_cb_aa_ns: uint32_t,
    pub r3_zconv: bool,
    pub r3_zconv_rows_ns: bool,
    pub r3_zconv_off: uint32_t,
    pub r3_zconv_pitch: uint32_t,
    pub r3_zconv_rows: uint32_t,
    pub r3_zconv_ns: uint32_t,
    pub r300_aic_pt_base: uint32_t,
    pub vram_size_mb: uint32_t,
    pub vram_size: uint64_t,
    pub mode: uint8_t,
    pub regs: PPCMacGPURegs,
    pub regs_size: uint32_t,
    pub disp: PPCMacGPUDisplayMode,
    pub vblank_timer: QEMUTimer,
    pub display_invalid: bool,
    pub pm4_fifo: [uint32_t; 16384],
    pub pm4_fifo_idx: uint32_t,
    pub pm4_pkt_count: uint32_t,
    pub pm4_pkt_reg: uint32_t,
    pub pm4_pkt_one_reg: bool,
    pub pm4_pkt_opcode: uint32_t,
    pub host_data_dst_x: uint32_t,
    pub host_data_dst_y: uint32_t,
    pub host_data_w: uint32_t,
    pub host_data_h: uint32_t,
    pub host_data_cur_x: uint32_t,
    pub host_data_bpp: uint32_t,
    pub host_data_cur_y: uint32_t,
    pub host_data_offset: uint32_t,
    pub host_data_pitch: uint32_t,
    pub host_data_macro_tile: bool,
    pub host_data_active: bool,
    pub shadow_buf: *mut uint8_t,
    pub surface_width: uint32_t,
    pub surface_height: uint32_t,
    pub surface_stride: uint32_t,
    pub surface_data: *mut ::core::ffi::c_void,
    pub shadow_buf_size: uint32_t,
    pub fps_frames_ptr: *mut uint32_t,
    pub fps_rows_ptr: *mut uint32_t,
    pub compositor_base: uint32_t,
    pub compositor_pitch: uint32_t,
    pub compositor_valid: bool,
    pub bbi2c: bitbang_i2c_interface,
    pub edid_blob: [uint8_t; 128],
    pub edid_read_done: bool,
    pub vb_count: uint32_t,
    pub vb_stride: [uint32_t; 4],
    pub vb_addr: [uint32_t; 4],
    pub tiled_surfaces: [C2RustUnnamed_91; 8],
    pub disp_stride_override_active: bool,
    pub disp_stride_override_value: uint32_t,
    pub metal_rt_offsets: [uint32_t; 16],
    pub metal_rt_count: ::core::ffi::c_int,
    pub metal_rt_frame_id: uint32_t,
    pub vram_watch_active: bool,
    pub r300_shadow: [uint32_t; 5120],
    pub mc_ind_index: uint32_t,
    pub mc_ind: [uint32_t; 128],
    pub hwc_pix: [uint32_t; 4096],
    pub hwc_w: uint32_t,
    pub hwc_h: uint32_t,
    pub hwc_idx: uint32_t,
    pub host_aspect_modes: bool,
    pub legacy_vga: bool,
    pub vga_shown: bool,
    pub vga: *mut VGACommonState,
    pub hwc_x: int32_t,
    pub hwc_y: int32_t,
    pub hwc_visible: bool,
    pub hwc_announce: bool,
    pub metal_vram_ptr: *mut ::core::ffi::c_void,
    pub metal_vram_opaque: *mut ::core::ffi::c_void,
    pub renderer: *mut PPCMacGPURenderer,
    pub renderer_opaque: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_91 {
    pub base: uint32_t,
    pub pitch: uint32_t,
    pub size: uint32_t,
    pub active: bool,
    pub label: [::core::ffi::c_char; 16],
}
pub type SurfaceClass = ::core::ffi::c_uint;
pub const SURF_CLASS_COUNT: SurfaceClass = 6;
pub const SURF_2D_ONLY: SurfaceClass = 5;
pub const SURF_VISIBLE_FB: SurfaceClass = 4;
pub const SURF_INTERMEDIATE: SurfaceClass = 3;
pub const SURF_WINDOW_RT: SurfaceClass = 2;
pub const SURF_COMPOSITOR_RT: SurfaceClass = 1;
pub const SURF_TEXTURE_TILE: SurfaceClass = 0;
pub const SURF_OWNER_RENDER: C2RustUnnamed_92 = 32;
pub const SURF_OWNER_BLIT_UPLOAD: C2RustUnnamed_92 = 16;
pub const SURF_OWNER_HOST_UPLOAD: C2RustUnnamed_92 = 8;
pub const SURF_OWNER_DRAG_COPY: C2RustUnnamed_92 = 4;
pub const SURF_OWNER_CPU_PRESENT: C2RustUnnamed_92 = 2;
pub const SURF_OWNER_QE_PRESENT: C2RustUnnamed_92 = 1;
pub const SURF_OWNER_NONE: C2RustUnnamed_92 = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SurfaceFrameState {
    pub valid: bool,
    pub offset: uint32_t,
    pub pitch: uint32_t,
    pub surface_class: SurfaceClass,
    pub owner_mask: uint32_t,
    pub qe_presents: uint16_t,
    pub cpu_presents: uint16_t,
    pub drag_copies: uint16_t,
    pub host_uploads: uint16_t,
    pub blit_uploads: uint16_t,
    pub renders: uint16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FrameTracker {
    pub frame_id: uint32_t,
    pub pass_id: uint32_t,
    pub event_count: uint32_t,
    pub events: [PassEvent; 512],
    pub has_compositor_rt: bool,
    pub has_window_rt: bool,
    pub has_present: bool,
    pub has_fallback_2d: bool,
    pub has_host_upload: bool,
    pub has_blit_upload: bool,
    pub has_drag_copy: bool,
    pub drag_observed: bool,
    pub has_mixed_surface_owner: bool,
    pub surface_state_count: uint32_t,
    pub surface_states: [SurfaceFrameState; 32],
    pub dumped: bool,
    pub dump_limit: uint32_t,
    pub force_dump_remaining: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_93 {
    pub bit: uint32_t,
    pub name: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BlitRegion {
    pub src_x: uint32_t,
    pub src_y: uint32_t,
    pub dst_x: uint32_t,
    pub dst_y: uint32_t,
    pub width: uint32_t,
    pub height: uint32_t,
    pub sequence: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BlitRegionRT {
    pub rt_offset: uint32_t,
    pub rt_pitch: uint32_t,
    pub region_count: ::core::ffi::c_int,
    pub regions: [BlitRegion; 64],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BlitRegionTable {
    pub rt_count: ::core::ffi::c_int,
    pub sequence: uint32_t,
    pub rts: [BlitRegionRT; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BlitPathStats {
    pub sep_count: uint64_t,
    pub mmio_count: uint64_t,
    pub bbm_count: uint64_t,
    pub paint_count: uint64_t,
    pub hostdata_count: uint64_t,
    pub last_dump: uint64_t,
}
pub const GPU_WHY_2D: C2RustUnnamed_95 = 64;
pub type C2RustUnnamed_94 = ::core::ffi::c_uint;
pub const GPU_WHY_OTHER: C2RustUnnamed_94 = 4;
pub const GPU_WHY_FENCE: C2RustUnnamed_94 = 3;
pub const GPU_WHY_DISPLAY: C2RustUnnamed_94 = 2;
pub const GPU_WHY_T3: C2RustUnnamed_94 = 1;
pub type C2RustUnnamed_95 = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PEWindow {
    pub surface: uint32_t,
    pub pitch: uint32_t,
    pub x0: uint32_t,
    pub y0: uint32_t,
    pub x1: uint32_t,
    pub y1: uint32_t,
    pub fx0: uint32_t,
    pub fy0: uint32_t,
    pub fx1: uint32_t,
    pub fy1: uint32_t,
    pub scr_w: uint32_t,
    pub scr_h: uint32_t,
    pub pieces: uint64_t,
    pub last_us: int64_t,
    pub live: bool,
    pub settled: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Gpu2DOp {
    pub gmc: uint32_t,
    pub dst_off: uint32_t,
    pub dst_pitch: uint32_t,
    pub src_off: uint32_t,
    pub src_pitch: uint32_t,
    pub dx: ::core::ffi::c_int,
    pub dy: ::core::ffi::c_int,
    pub sx: ::core::ffi::c_int,
    pub sy: ::core::ffi::c_int,
    pub w: uint32_t,
    pub h: uint32_t,
    pub xl2r: bool,
    pub yt2b: bool,
    pub auto_dir: bool,
    pub clip: bool,
    pub clip_x0: ::core::ffi::c_int,
    pub clip_y0: ::core::ffi::c_int,
    pub clip_x1: ::core::ffi::c_int,
    pub clip_y1: ::core::ffi::c_int,
    pub frgd: uint32_t,
    pub bkgd: uint32_t,
    pub brush_yx: uint32_t,
    pub brush_data: *const uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_101 {
    pub addr: uint32_t,
    pub val: uint32_t,
    pub wr: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_102 {
    pub hdr: uint32_t,
    pub type_0: uint32_t,
    pub opcode: uint32_t,
    pub count: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_103 {
    pub idx: uint8_t,
    pub val: uint32_t,
}
opaque!(CPUPluginState);
opaque!(CPUJumpCache);
opaque!(CPUAddressSpace);
