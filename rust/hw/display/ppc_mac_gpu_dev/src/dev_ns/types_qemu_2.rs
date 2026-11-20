use super::*;
#[repr(C)]
pub struct VGACommonState {
    pub legacy_address_space: *mut MemoryRegion,
    pub vram_ptr: *mut uint8_t,
    pub vram: MemoryRegion,
    pub vram_size: uint32_t,
    pub vram_size_mb: uint32_t,
    pub vbe_size: uint32_t,
    pub vbe_size_mask: uint32_t,
    pub latch: uint32_t,
    pub has_chain4_alias: bool,
    pub chain4_alias: MemoryRegion,
    pub sr_index: uint8_t,
    pub sr: [uint8_t; 256],
    pub sr_vbe: [uint8_t; 256],
    pub gr_index: uint8_t,
    pub gr: [uint8_t; 256],
    pub ar_index: uint8_t,
    pub ar: [uint8_t; 21],
    pub ar_flip_flop: ::core::ffi::c_int,
    pub cr_index: uint8_t,
    pub cr: [uint8_t; 256],
    pub msr: uint8_t,
    pub fcr: uint8_t,
    pub st00: uint8_t,
    pub st01: uint8_t,
    pub dac_state: uint8_t,
    pub dac_sub_index: uint8_t,
    pub dac_read_index: uint8_t,
    pub dac_write_index: uint8_t,
    pub dac_cache: [uint8_t; 3],
    pub dac_8bit: ::core::ffi::c_int,
    pub palette: [uint8_t; 768],
    pub bank_offset: int32_t,
    pub get_bpp: Option<unsafe extern "C" fn(*mut VGACommonState) -> ::core::ffi::c_int>,
    pub get_params: Option<unsafe extern "C" fn(*mut VGACommonState, *mut VGADisplayParams) -> ()>,
    pub get_resolution: Option<
        unsafe extern "C" fn(
            *mut VGACommonState,
            *mut ::core::ffi::c_int,
            *mut ::core::ffi::c_int,
        ) -> (),
    >,
    pub vga_port_list: PortioList,
    pub vbe_port_list: PortioList,
    pub vbe_index: uint16_t,
    pub vbe_regs: [uint16_t; 10],
    pub vbe_start_addr: uint32_t,
    pub vbe_line_offset: uint32_t,
    pub vbe_bank_mask: uint32_t,
    pub con: *mut QemuConsole,
    pub font_offsets: [uint32_t; 2],
    pub panning_buf: *mut uint8_t,
    pub graphic_mode: ::core::ffi::c_int,
    pub shift_control: uint8_t,
    pub double_scan: uint8_t,
    pub params: VGADisplayParams,
    pub plane_updated: uint32_t,
    pub last_line_offset: uint32_t,
    pub last_cw: uint8_t,
    pub last_ch: uint8_t,
    pub last_width: uint32_t,
    pub last_height: uint32_t,
    pub last_text_width: uint32_t,
    pub last_text_height: uint32_t,
    pub last_scr_width: uint32_t,
    pub last_scr_height: uint32_t,
    pub last_depth: uint32_t,
    pub last_byteswap: bool,
    pub force_shadow: bool,
    pub cursor_start: uint8_t,
    pub cursor_end: uint8_t,
    pub cursor_visible_phase: bool,
    pub cursor_blink_time: int64_t,
    pub blink_visible_phase: bool,
    pub blink_time: int64_t,
    pub cursor_offset: uint32_t,
    pub hw_ops: *const GraphicHwOps,
    pub full_update_text: bool,
    pub full_update_gfx: bool,
    pub big_endian_fb: bool,
    pub default_endian_fb: bool,
    pub global_vmstate: bool,
    pub invalidated_y_table: [uint32_t; 64],
    pub hw_cursor_x: uint32_t,
    pub hw_cursor_y: uint32_t,
    pub cursor_invalidate: Option<unsafe extern "C" fn(*mut VGACommonState) -> ()>,
    pub cursor_draw_line:
        Option<unsafe extern "C" fn(*mut VGACommonState, *mut uint8_t, ::core::ffi::c_int) -> ()>,
    pub last_palette: [uint32_t; 256],
    pub last_ch_attr: [uint32_t; 16000],
    pub retrace: vga_retrace_fn,
    pub update_retrace_info: vga_update_retrace_info_fn,
    pub retrace_info: vga_retrace,
    pub is_vbe_vmstate: uint8_t,
}
pub type vga_update_retrace_info_fn = Option<unsafe extern "C" fn(*mut VGACommonState) -> ()>;
pub type vga_retrace_fn = Option<unsafe extern "C" fn(*mut VGACommonState) -> uint8_t>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VGADisplayParams {
    pub line_offset: uint32_t,
    pub start_addr: uint32_t,
    pub line_compare: uint32_t,
    pub hpel: uint8_t,
    pub hpel_split: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct bitbang_i2c_interface {
    pub bus: *mut I2CBus,
    pub state: bitbang_i2c_state,
    pub last_data: ::core::ffi::c_int,
    pub last_clock: ::core::ffi::c_int,
    pub device_out: ::core::ffi::c_int,
    pub buffer: uint8_t,
    pub current_addr: ::core::ffi::c_int,
}
pub type bitbang_i2c_state = ::core::ffi::c_uint;
pub const SENT_NACK: bitbang_i2c_state = 19;
pub const SENDING_ACK: bitbang_i2c_state = 18;
pub const RECEIVING_BIT0: bitbang_i2c_state = 17;
pub const RECEIVING_BIT1: bitbang_i2c_state = 16;
pub const RECEIVING_BIT2: bitbang_i2c_state = 15;
pub const RECEIVING_BIT3: bitbang_i2c_state = 14;
pub const RECEIVING_BIT4: bitbang_i2c_state = 13;
pub const RECEIVING_BIT5: bitbang_i2c_state = 12;
pub const RECEIVING_BIT6: bitbang_i2c_state = 11;
pub const RECEIVING_BIT7: bitbang_i2c_state = 10;
pub const WAITING_FOR_ACK: bitbang_i2c_state = 9;
pub const SENDING_BIT0: bitbang_i2c_state = 8;
pub const SENDING_BIT1: bitbang_i2c_state = 7;
pub const SENDING_BIT2: bitbang_i2c_state = 6;
pub const SENDING_BIT3: bitbang_i2c_state = 5;
pub const SENDING_BIT4: bitbang_i2c_state = 4;
pub const SENDING_BIT5: bitbang_i2c_state = 3;
pub const SENDING_BIT6: bitbang_i2c_state = 2;
pub const SENDING_BIT7: bitbang_i2c_state = 1;
pub const STOPPED: bitbang_i2c_state = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct qemu_edid_info {
    pub vendor: *const ::core::ffi::c_char,
    pub name: *const ::core::ffi::c_char,
    pub serial: *const ::core::ffi::c_char,
    pub width_mm: uint16_t,
    pub height_mm: uint16_t,
    pub prefx: uint32_t,
    pub prefy: uint32_t,
    pub maxx: uint32_t,
    pub maxy: uint32_t,
    pub refresh_rate: uint32_t,
}
#[repr(C)]
pub struct I2CDDCState {
    pub i2c: I2CSlave,
    pub firstbyte: bool,
    pub reg: uint8_t,
    pub edid_info: qemu_edid_info,
    pub edid_blob: [uint8_t; 128],
}
pub type PassEventType = ::core::ffi::c_uint;
pub const PASS_EVENT_DRAG_COPY: PassEventType = 8;
pub const PASS_EVENT_BLIT_UPLOAD: PassEventType = 7;
pub const PASS_EVENT_HOST_UPLOAD: PassEventType = 6;
pub const PASS_EVENT_FALLBACK_2D: PassEventType = 5;
pub const PASS_EVENT_PRESENT: PassEventType = 4;
pub const PASS_EVENT_WRITEBACK: PassEventType = 3;
pub const PASS_EVENT_RENDER: PassEventType = 2;
pub const PASS_EVENT_SAMPLE_TEX: PassEventType = 1;
pub const PASS_EVENT_BIND_RT: PassEventType = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PassEvent {
    pub type_0: PassEventType,
    pub pass_id: uint32_t,
    pub surface_offset: uint32_t,
    pub pitch: uint32_t,
    pub src_offset: uint32_t,
    pub dst_offset: uint32_t,
    pub width: uint16_t,
    pub height: uint16_t,
    pub src_x: uint16_t,
    pub src_y: uint16_t,
    pub dst_x: uint16_t,
    pub dst_y: uint16_t,
    pub move_x: int16_t,
    pub move_y: int16_t,
    pub surface_class: SurfaceClass,
    pub from_srt: bool,
}
pub type C2RustUnnamed_92 = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PostProbeReadEntry {
    pub reg: uint32_t,
    pub last_value: uint32_t,
    pub count: uint32_t,
    pub first_at_draw: uint32_t,
    pub last_at_draw: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_96 {
    pub entries: [PostProbeReadEntry; 256],
    pub num_entries: ::core::ffi::c_int,
    pub total_reads: ::core::ffi::c_int,
    pub audit_active: ::core::ffi::c_int,
    pub window_logged: ::core::ffi::c_int,
    pub summary_logged: ::core::ffi::c_int,
    pub first_draw_seen: ::core::ffi::c_int,
    pub window_start_draw: ::core::ffi::c_int,
    pub window_end_draw: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_97 {
    pub reg: uint16_t,
    pub post: uint32_t,
    pub name: &'static core::ffi::CStr,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_99 {
    pub tag: uint32_t,
    pub host: *mut uint8_t,
    pub pte: hwaddr,
    pub pte_raw: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_100 {
    pub addr: uint32_t,
    pub h: uint64_t,
}

opaque!(_IO_wide_data);
opaque!(_IO_codecvt);
opaque!(_IO_marker);
opaque!(_GHashTable);
opaque!(AccelCPUState);
opaque!(QEMUBH);
opaque!(RamDiscardSource);
opaque!(TypeImpl);
opaque!(MemoryRegionIoeventfd);
opaque!(CoalescedMemoryRange);
opaque!(HotplugHandler);
opaque!(Clock);
opaque!(IRQState);
opaque!(RAMBlock);
opaque!(AddressSpaceDispatch);
opaque!(FlatRange);
opaque!(QEMUTimerList);
opaque!(KVMState);
opaque!(qemu_work_item);
opaque!(SemihostingCPUOps);
opaque!(TCGCPUOps);
opaque!(SysemuCPUOps);
opaque!(AccelCPUClass);
opaque!(JSONWriter);
opaque!(QEMUFile);
opaque!(Visitor);
opaque!(QObject);
opaque!(DirtyBitmapSnapshot);
opaque!(QemuConsole);
opaque!(pixman_image);
opaque!(MSIMessage);
opaque!(SHPCDevice);
