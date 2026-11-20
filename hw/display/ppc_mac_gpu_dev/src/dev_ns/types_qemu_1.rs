use super::*;
pub type __int128_t = i128;
pub type __builtin_va_list = [__va_list_tag; 1];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: ::core::ffi::c_uint,
    pub fp_offset: ::core::ffi::c_uint,
    pub overflow_arg_area: *mut ::core::ffi::c_void,
    pub reg_save_area: *mut ::core::ffi::c_void,
}
pub type __gnuc_va_list = __builtin_va_list;
pub type va_list = __gnuc_va_list;
pub type ptrdiff_t = ::core::ffi::c_long;
pub type size_t = ::core::ffi::c_ulong;
pub type __uint8_t = ::core::ffi::c_uchar;
pub type __int16_t = ::core::ffi::c_short;
pub type __uint16_t = ::core::ffi::c_ushort;
pub type __int32_t = ::core::ffi::c_int;
pub type __uint32_t = ::core::ffi::c_uint;
pub type __int64_t = ::core::ffi::c_long;
pub type __uint64_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type int16_t = __int16_t;
pub type int32_t = __int32_t;
pub type int64_t = __int64_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type uintptr_t = ::core::ffi::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sigset_t {
    pub __val: [::core::ffi::c_ulong; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union __atomic_wide_counter {
    pub __value64: ::core::ffi::c_ulonglong,
    pub __value32: C2RustUnnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub __low: ::core::ffi::c_uint,
    pub __high: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __pthread_internal_list {
    pub __prev: *mut __pthread_internal_list,
    pub __next: *mut __pthread_internal_list,
}
pub type __pthread_list_t = __pthread_internal_list;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __pthread_mutex_s {
    pub __lock: ::core::ffi::c_int,
    pub __count: ::core::ffi::c_uint,
    pub __owner: ::core::ffi::c_int,
    pub __nusers: ::core::ffi::c_uint,
    pub __kind: ::core::ffi::c_int,
    pub __spins: ::core::ffi::c_short,
    pub __glibc_reserved: ::core::ffi::c_short,
    pub __list: __pthread_list_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __pthread_cond_s {
    pub __wseq: __atomic_wide_counter,
    pub __g1_start: __atomic_wide_counter,
    pub __g_size: [::core::ffi::c_uint; 2],
    pub __g1_orig_size: ::core::ffi::c_uint,
    pub __wrefs: ::core::ffi::c_uint,
    pub __g_signals: [::core::ffi::c_uint; 2],
    pub __unused_initialized_1: ::core::ffi::c_uint,
    pub __unused_initialized_2: ::core::ffi::c_uint,
}
pub type pthread_t = ::core::ffi::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub union pthread_mutex_t {
    pub __data: __pthread_mutex_s,
    pub __size: [::core::ffi::c_char; 40],
    pub __align: ::core::ffi::c_long,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union pthread_cond_t {
    pub __data: __pthread_cond_s,
    pub __size: [::core::ffi::c_char; 48],
    pub __align: ::core::ffi::c_longlong,
}
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    #[bitfield(name = "_flags2", ty = "::core::ffi::c_int", bits = "0..=23")]
    pub _flags2: [u8; 3],
    pub _short_backupbuf: [::core::ffi::c_char; 1],
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub _prevchain: *mut *mut _IO_FILE,
    pub _mode: ::core::ffi::c_int,
    pub _unused3: ::core::ffi::c_int,
    pub _total_written: __uint64_t,
    pub _unused2: [::core::ffi::c_char; 8],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type __u8 = ::core::ffi::c_uchar;
pub type __u16 = ::core::ffi::c_ushort;
pub type __u32 = ::core::ffi::c_uint;
pub type __u64 = ::core::ffi::c_ulonglong;
pub type __jmp_buf = [::core::ffi::c_long; 8];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __jmp_buf_tag {
    pub __jmpbuf: __jmp_buf,
    pub __mask_was_saved: ::core::ffi::c_int,
    pub __saved_mask: __sigset_t,
}
pub type sigjmp_buf = [__jmp_buf_tag; 1];
pub type guint8 = ::core::ffi::c_uchar;
pub type guint32 = ::core::ffi::c_uint;
pub type gint64 = ::core::ffi::c_long;
pub type gssize = ::core::ffi::c_long;
pub type gsize = ::core::ffi::c_ulong;
pub type gchar = ::core::ffi::c_char;
pub type gint = ::core::ffi::c_int;
pub type gboolean = gint;
pub type gulong = ::core::ffi::c_ulong;
pub type guint = ::core::ffi::c_uint;
pub type gpointer = *mut ::core::ffi::c_void;
pub type gconstpointer = *const ::core::ffi::c_void;
pub type GEqualFunc = Option<unsafe extern "C" fn(gconstpointer, gconstpointer) -> gboolean>;
pub type GHashFunc = Option<unsafe extern "C" fn(gconstpointer) -> guint>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _GArray {
    pub data: *mut gchar,
    pub len: guint,
}
pub type GArray = _GArray;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _GByteArray {
    pub data: *mut guint8,
    pub len: guint,
}
pub type GByteArray = _GByteArray;
pub type GQuark = guint32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _GError {
    pub domain: GQuark,
    pub code: gint,
    pub message: *mut gchar,
}
pub type GError = _GError;
pub type GHashTable = _GHashTable;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _GSList {
    pub data: gpointer,
    pub next: *mut GSList,
}
pub type GSList = _GSList;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _GString {
    pub str_0: *mut gchar,
    pub len: gsize,
    pub allocated_len: gsize,
}
pub type GString = _GString;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AddressSpace {
    pub rcu: rcu_head,
    pub name: *mut ::core::ffi::c_char,
    pub root: *mut MemoryRegion,
    pub current_map: *mut FlatView,
    pub ioeventfd_nb: ::core::ffi::c_int,
    pub ioeventfd_notifiers: ::core::ffi::c_int,
    pub ioeventfds: *mut MemoryRegionIoeventfd,
    pub listeners: C2RustUnnamed_3,
    pub address_spaces_link: C2RustUnnamed_2,
    pub max_bounce_buffer_size: size_t,
    pub bounce_buffer_size: size_t,
    pub map_client_list_lock: QemuMutex,
    pub map_client_list: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub lh_first: *mut AddressSpaceMapClient,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AddressSpaceMapClient {
    pub bh: *mut QEMUBH,
    pub link: C2RustUnnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub le_next: *mut AddressSpaceMapClient,
    pub le_prev: *mut *mut AddressSpaceMapClient,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QemuMutex {
    pub lock: pthread_mutex_t,
    pub initialized: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_2 {
    pub tqe_next: *mut AddressSpace,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QTailQLink {
    pub tql_next: *mut ::core::ffi::c_void,
    pub tql_prev: *mut QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_3 {
    pub tqh_first: *mut MemoryListener,
    pub tqh_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemoryListener {
    pub begin: Option<unsafe extern "C" fn(*mut MemoryListener) -> ()>,
    pub commit: Option<unsafe extern "C" fn(*mut MemoryListener) -> ()>,
    pub region_add:
        Option<unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection) -> ()>,
    pub region_del:
        Option<unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection) -> ()>,
    pub region_nop:
        Option<unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection) -> ()>,
    pub log_start: Option<
        unsafe extern "C" fn(
            *mut MemoryListener,
            *mut MemoryRegionSection,
            ::core::ffi::c_int,
            ::core::ffi::c_int,
        ) -> (),
    >,
    pub log_stop: Option<
        unsafe extern "C" fn(
            *mut MemoryListener,
            *mut MemoryRegionSection,
            ::core::ffi::c_int,
            ::core::ffi::c_int,
        ) -> (),
    >,
    pub log_sync: Option<unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection) -> ()>,
    pub log_sync_global: Option<unsafe extern "C" fn(*mut MemoryListener, bool) -> ()>,
    pub log_clear:
        Option<unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection) -> ()>,
    pub log_global_start:
        Option<unsafe extern "C" fn(*mut MemoryListener, *mut *mut Error) -> bool>,
    pub log_global_stop: Option<unsafe extern "C" fn(*mut MemoryListener) -> ()>,
    pub log_global_after_sync: Option<unsafe extern "C" fn(*mut MemoryListener) -> ()>,
    pub eventfd_add: Option<
        unsafe extern "C" fn(
            *mut MemoryListener,
            *mut MemoryRegionSection,
            bool,
            uint64_t,
            *mut EventNotifier,
        ) -> (),
    >,
    pub eventfd_del: Option<
        unsafe extern "C" fn(
            *mut MemoryListener,
            *mut MemoryRegionSection,
            bool,
            uint64_t,
            *mut EventNotifier,
        ) -> (),
    >,
    pub coalesced_io_add: Option<
        unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection, hwaddr, hwaddr) -> (),
    >,
    pub coalesced_io_del: Option<
        unsafe extern "C" fn(*mut MemoryListener, *mut MemoryRegionSection, hwaddr, hwaddr) -> (),
    >,
    pub priority: ::core::ffi::c_uint,
    pub name: *const ::core::ffi::c_char,
    pub address_space: *mut AddressSpace,
    pub link: C2RustUnnamed_5,
    pub link_as: C2RustUnnamed_4,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_4 {
    pub tqe_next: *mut MemoryListener,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_5 {
    pub tqe_next: *mut MemoryListener,
    pub tqe_circ: QTailQLink,
}
pub type hwaddr = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemoryRegionSection {
    pub size: Int128,
    pub mr: *mut MemoryRegion,
    pub fv: *mut FlatView,
    pub offset_within_region: hwaddr,
    pub offset_within_address_space: hwaddr,
    pub readonly: bool,
    pub nonvolatile: bool,
    pub unmergeable: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FlatView {
    pub rcu: rcu_head,
    pub ref_0: ::core::ffi::c_uint,
    pub ranges: *mut FlatRange,
    pub nr: ::core::ffi::c_uint,
    pub nr_allocated: ::core::ffi::c_uint,
    pub dispatch: *mut AddressSpaceDispatch,
    pub root: *mut MemoryRegion,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemoryRegion {
    pub parent_obj: Object,
    pub romd_mode: bool,
    pub ram: bool,
    pub subpage: bool,
    pub readonly: bool,
    pub nonvolatile: bool,
    pub rom_device: bool,
    pub flush_coalesced_mmio: bool,
    pub lockless_io: bool,
    pub unmergeable: bool,
    pub dirty_log_mask: uint8_t,
    pub is_iommu: bool,
    pub ram_block: *mut RAMBlock,
    pub owner: *mut Object,
    pub dev: *mut DeviceState,
    pub ops: *const MemoryRegionOps,
    pub opaque: *mut ::core::ffi::c_void,
    pub container: *mut MemoryRegion,
    pub mapped_via_alias: ::core::ffi::c_int,
    pub size: Int128,
    pub addr: hwaddr,
    pub destructor: Option<unsafe extern "C" fn(*mut MemoryRegion) -> ()>,
    pub align: uint64_t,
    pub terminates: bool,
    pub ram_device: bool,
    pub enabled: bool,
    pub vga_logging_count: uint8_t,
    pub alias: *mut MemoryRegion,
    pub alias_offset: hwaddr,
    pub priority: int32_t,
    pub subregions: C2RustUnnamed_12,
    pub subregions_link: C2RustUnnamed_11,
    pub coalesced: C2RustUnnamed_10,
    pub name: *const ::core::ffi::c_char,
    pub ioeventfd_nb: ::core::ffi::c_uint,
    pub ioeventfds: *mut MemoryRegionIoeventfd,
    pub rdm: *mut RamDiscardManager,
    pub disable_reentrancy_guard: bool,
    pub ram_device_skip_iommu_map: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RamDiscardManager {
    pub parent: Object,
    pub mr: *mut MemoryRegion,
    pub source_list: C2RustUnnamed_8,
    pub min_granularity: uint64_t,
    pub rdl_list: C2RustUnnamed_6,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_6 {
    pub lh_first: *mut RamDiscardListener,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RamDiscardListener {
    pub notify_populate: NotifyRamPopulate,
    pub notify_discard: NotifyRamDiscard,
    pub section: *mut MemoryRegionSection,
    pub next: C2RustUnnamed_7,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_7 {
    pub le_next: *mut RamDiscardListener,
    pub le_prev: *mut *mut RamDiscardListener,
}
pub type NotifyRamDiscard =
    Option<unsafe extern "C" fn(*mut RamDiscardListener, *const MemoryRegionSection) -> ()>;
pub type NotifyRamPopulate = Option<
    unsafe extern "C" fn(*mut RamDiscardListener, *const MemoryRegionSection) -> ::core::ffi::c_int,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_8 {
    pub lh_first: *mut RamDiscardSourceEntry,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RamDiscardSourceEntry {
    pub rds: *mut RamDiscardSource,
    pub next: C2RustUnnamed_9,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_9 {
    pub le_next: *mut RamDiscardSourceEntry,
    pub le_prev: *mut *mut RamDiscardSourceEntry,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Object {
    pub class: *mut ObjectClass,
    pub free: Option<ObjectFree>,
    pub properties: *mut GHashTable,
    pub ref_0: uint32_t,
    pub parent: *mut Object,
}
pub type ObjectFree = unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ObjectClass {
    pub type_0: Type,
    pub interfaces: *mut GSList,
    pub object_cast_cache: [*const ::core::ffi::c_char; 4],
    pub class_cast_cache: [*const ::core::ffi::c_char; 4],
    pub unparent: Option<ObjectUnparent>,
    pub properties: *mut GHashTable,
}
pub type ObjectUnparent = unsafe extern "C" fn(*mut Object) -> ();
pub type Type = *mut TypeImpl;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_10 {
    pub tqh_first: *mut CoalescedMemoryRange,
    pub tqh_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_11 {
    pub tqe_next: *mut MemoryRegion,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_12 {
    pub tqh_first: *mut MemoryRegion,
    pub tqh_circ: QTailQLink,
}
pub type Int128 = __int128_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemoryRegionOps {
    pub read: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, hwaddr, ::core::ffi::c_uint) -> uint64_t,
    >,
    pub write: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, hwaddr, uint64_t, ::core::ffi::c_uint) -> (),
    >,
    pub read_with_attrs: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            hwaddr,
            *mut uint64_t,
            ::core::ffi::c_uint,
            MemTxAttrs,
        ) -> MemTxResult,
    >,
    pub write_with_attrs: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            hwaddr,
            uint64_t,
            ::core::ffi::c_uint,
            MemTxAttrs,
        ) -> MemTxResult,
    >,
    pub endianness: device_endian,
    pub valid: C2RustUnnamed_14,
    pub impl_0: C2RustUnnamed_13,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_13 {
    pub min_access_size: ::core::ffi::c_uint,
    pub max_access_size: ::core::ffi::c_uint,
    pub unaligned: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_14 {
    pub min_access_size: ::core::ffi::c_uint,
    pub max_access_size: ::core::ffi::c_uint,
    pub unaligned: bool,
    pub accepts: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            hwaddr,
            ::core::ffi::c_uint,
            bool,
            MemTxAttrs,
        ) -> bool,
    >,
}
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct MemTxAttrs {
    #[bitfield(name = "secure", ty = "::core::ffi::c_uint", bits = "0..=0")]
    #[bitfield(name = "space", ty = "::core::ffi::c_uint", bits = "1..=2")]
    #[bitfield(name = "user", ty = "::core::ffi::c_uint", bits = "3..=3")]
    #[bitfield(name = "memory", ty = "::core::ffi::c_uint", bits = "4..=4")]
    #[bitfield(name = "debug", ty = "::core::ffi::c_uint", bits = "5..=5")]
    #[bitfield(name = "requester_id", ty = "::core::ffi::c_uint", bits = "6..=21")]
    #[bitfield(name = "pid", ty = "::core::ffi::c_uint", bits = "22..=29")]
    #[bitfield(name = "address_type", ty = "::core::ffi::c_uint", bits = "30..=30")]
    pub secure_space_user_memory_debug_requester_id_pid_address_type: [u8; 4],
    pub unspecified: bool,
    pub _reserved1: uint8_t,
    pub _reserved2: uint16_t,
}
pub type device_endian = ::core::ffi::c_uint;
pub const DEVICE_LITTLE_ENDIAN: device_endian = 2;
pub const DEVICE_BIG_ENDIAN: device_endian = 1;
pub const DEVICE_NATIVE_ENDIAN: device_endian = 0;
pub type MemTxResult = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DeviceState {
    pub parent_obj: Object,
    pub id: *mut ::core::ffi::c_char,
    pub canonical_path: *mut ::core::ffi::c_char,
    pub realized: bool,
    pub pending_deleted_event: bool,
    pub pending_deleted_expires_ms: int64_t,
    pub hotplugged: ::core::ffi::c_int,
    pub allow_unplug_during_migration: bool,
    pub parent_bus: *mut BusState,
    pub gpios: NamedGPIOListHead,
    pub clocks: NamedClockListHead,
    pub child_bus: BusStateHead,
    pub num_child_bus: ::core::ffi::c_int,
    pub instance_id_alias: ::core::ffi::c_int,
    pub alias_required_for_version: ::core::ffi::c_int,
    pub reset: ResettableState,
    pub unplug_blockers: *mut GSList,
    pub mem_reentrancy_guard: MemReentrancyGuard,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemReentrancyGuard {
    pub engaged_in_io: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ResettableState {
    pub count: ::core::ffi::c_uint,
    pub hold_phase_pending: bool,
    pub exit_phase_in_progress: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BusStateHead {
    pub lh_first: *mut BusState,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BusState {
    pub obj: Object,
    pub parent: *mut DeviceState,
    pub name: *mut ::core::ffi::c_char,
    pub hotplug_handler: *const HotplugHandler,
    pub max_index: ::core::ffi::c_int,
    pub realized: bool,
    pub full: bool,
    pub num_children: ::core::ffi::c_int,
    pub children: BusChildHead,
    pub sibling: BusStateEntry,
    pub reset: ResettableState,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BusStateEntry {
    pub le_next: *mut BusState,
    pub le_prev: *mut *mut BusState,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union BusChildHead {
    pub tqh_first: *mut BusChild,
    pub tqh_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct BusChild {
    pub rcu: rcu_head,
    pub child: *mut DeviceState,
    pub index: ::core::ffi::c_int,
    pub sibling: C2RustUnnamed_15,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_15 {
    pub tqe_next: *mut BusChild,
    pub tqe_circ: QTailQLink,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rcu_head {
    pub next: *mut rcu_head,
    pub func: Option<RCUCBFunc>,
}
pub type RCUCBFunc = unsafe extern "C" fn(*mut rcu_head) -> ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct NamedClockListHead {
    pub lh_first: *mut NamedClockList,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct NamedClockList {
    pub name: *mut ::core::ffi::c_char,
    pub clock: *mut Clock,
    pub output: bool,
    pub alias: bool,
    pub node: C2RustUnnamed_16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_16 {
    pub le_next: *mut NamedClockList,
    pub le_prev: *mut *mut NamedClockList,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct NamedGPIOListHead {
    pub lh_first: *mut NamedGPIOList,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct NamedGPIOList {
    pub name: *mut ::core::ffi::c_char,
    pub in_0: *mut qemu_irq,
    pub num_in: ::core::ffi::c_int,
    pub num_out: ::core::ffi::c_int,
    pub node: C2RustUnnamed_17,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_17 {
    pub le_next: *mut NamedGPIOList,
    pub le_prev: *mut *mut NamedGPIOList,
}
pub type qemu_irq = *mut IRQState;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct EventNotifier {
    pub rfd: ::core::ffi::c_int,
    pub wfd: ::core::ffi::c_int,
    pub initialized: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QemuLockCnt {
    pub count: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union IcountDecr {
    pub u32_0: uint32_t,
    pub u16_0: C2RustUnnamed_18,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_18 {
    pub low: uint16_t,
    pub high: uint16_t,
}
pub type vaddr = uintptr_t;
pub type MMUIdxMap = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QemuSpin {
    pub value: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_dirty_gfn {
    pub flags: __u32,
    pub slot: __u32,
    pub offset: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_run {
    pub request_interrupt_window: __u8,
    pub immediate_exit: __u8,
    pub padding1: [__u8; 6],
    pub exit_reason: __u32,
    pub ready_for_interrupt_injection: __u8,
    pub if_flag: __u8,
    pub flags: __u16,
    pub cr8: __u64,
    pub apic_base: __u64,
    pub c2rust_unnamed: C2RustUnnamed_28,
    pub kvm_valid_regs: __u64,
    pub kvm_dirty_regs: __u64,
    pub s: C2RustUnnamed_22,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_22 {
    pub regs: kvm_sync_regs,
    pub padding: [::core::ffi::c_char; 2048],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_sync_regs {
    pub regs: kvm_regs,
    pub sregs: kvm_sregs,
    pub events: kvm_vcpu_events,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_vcpu_events {
    pub exception: C2RustUnnamed_27,
    pub interrupt: C2RustUnnamed_26,
    pub nmi: C2RustUnnamed_25,
    pub sipi_vector: __u32,
    pub flags: __u32,
    pub smi: C2RustUnnamed_24,
    pub triple_fault: C2RustUnnamed_23,
    pub reserved: [__u8; 26],
    pub exception_has_payload: __u8,
    pub exception_payload: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_23 {
    pub pending: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_24 {
    pub smm: __u8,
    pub pending: __u8,
    pub smm_inside_nmi: __u8,
    pub latched_init: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_25 {
    pub injected: __u8,
    pub pending: __u8,
    pub masked: __u8,
    pub pad: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_26 {
    pub injected: __u8,
    pub nr: __u8,
    pub soft: __u8,
    pub shadow: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_27 {
    pub injected: __u8,
    pub nr: __u8,
    pub has_error_code: __u8,
    pub pending: __u8,
    pub error_code: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_sregs {
    pub cs: kvm_segment,
    pub ds: kvm_segment,
    pub es: kvm_segment,
    pub fs: kvm_segment,
    pub gs: kvm_segment,
    pub ss: kvm_segment,
    pub tr: kvm_segment,
    pub ldt: kvm_segment,
    pub gdt: kvm_dtable,
    pub idt: kvm_dtable,
    pub cr0: __u64,
    pub cr2: __u64,
    pub cr3: __u64,
    pub cr4: __u64,
    pub cr8: __u64,
    pub efer: __u64,
    pub apic_base: __u64,
    pub interrupt_bitmap: [__u64; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_dtable {
    pub base: __u64,
    pub limit: __u16,
    pub padding: [__u16; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_segment {
    pub base: __u64,
    pub limit: __u32,
    pub selector: __u16,
    pub type_0: __u8,
    pub present: __u8,
    pub dpl: __u8,
    pub db: __u8,
    pub s: __u8,
    pub l: __u8,
    pub g: __u8,
    pub avl: __u8,
    pub unusable: __u8,
    pub padding: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_regs {
    pub rax: __u64,
    pub rbx: __u64,
    pub rcx: __u64,
    pub rdx: __u64,
    pub rsi: __u64,
    pub rdi: __u64,
    pub rsp: __u64,
    pub rbp: __u64,
    pub r8: __u64,
    pub r9: __u64,
    pub r10: __u64,
    pub r11: __u64,
    pub r12: __u64,
    pub r13: __u64,
    pub r14: __u64,
    pub r15: __u64,
    pub rip: __u64,
    pub rflags: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_28 {
    pub hw: C2RustUnnamed_72,
    pub fail_entry: C2RustUnnamed_71,
    pub ex: C2RustUnnamed_70,
    pub io: C2RustUnnamed_69,
    pub debug: C2RustUnnamed_68,
    pub mmio: C2RustUnnamed_67,
    pub iocsr_io: C2RustUnnamed_66,
    pub hypercall: C2RustUnnamed_64,
    pub tpr_access: C2RustUnnamed_63,
    pub s390_sieic: C2RustUnnamed_62,
    pub s390_reset_flags: __u64,
    pub s390_ucontrol: C2RustUnnamed_61,
    pub dcr: C2RustUnnamed_60,
    pub internal: C2RustUnnamed_59,
    pub emulation_failure: C2RustUnnamed_56,
    pub osi: C2RustUnnamed_55,
    pub papr_hcall: C2RustUnnamed_54,
    pub s390_tsch: C2RustUnnamed_53,
    pub epr: C2RustUnnamed_52,
    pub system_event: C2RustUnnamed_50,
    pub s390_stsi: C2RustUnnamed_49,
    pub eoi: C2RustUnnamed_48,
    pub hyperv: kvm_hyperv_exit,
    pub arm_nisv: C2RustUnnamed_43,
    pub msr: C2RustUnnamed_42,
    pub xen: kvm_xen_exit,
    pub riscv_sbi: C2RustUnnamed_39,
    pub riscv_csr: C2RustUnnamed_38,
    pub notify: C2RustUnnamed_37,
    pub memory_fault: C2RustUnnamed_36,
    pub tdx: C2RustUnnamed_30,
    pub arm_sea: C2RustUnnamed_29,
    pub snp_req_certs: kvm_exit_snp_req_certs,
    pub padding: [::core::ffi::c_char; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_exit_snp_req_certs {
    pub gpa: __u64,
    pub npages: __u64,
    pub ret: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_29 {
    pub flags: __u64,
    pub esr: __u64,
    pub gva: __u64,
    pub gpa: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_30 {
    pub flags: __u64,
    pub nr: __u64,
    pub c2rust_unnamed: C2RustUnnamed_31,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_31 {
    pub unknown: C2RustUnnamed_35,
    pub get_quote: C2RustUnnamed_34,
    pub get_tdvmcall_info: C2RustUnnamed_33,
    pub setup_event_notify: C2RustUnnamed_32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_32 {
    pub ret: __u64,
    pub vector: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_33 {
    pub ret: __u64,
    pub leaf: __u64,
    pub r11: __u64,
    pub r12: __u64,
    pub r13: __u64,
    pub r14: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_34 {
    pub ret: __u64,
    pub gpa: __u64,
    pub size: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_35 {
    pub ret: __u64,
    pub data: [__u64; 5],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_36 {
    pub flags: __u64,
    pub gpa: __u64,
    pub size: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_37 {
    pub flags: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_38 {
    pub csr_num: ::core::ffi::c_ulong,
    pub new_value: ::core::ffi::c_ulong,
    pub write_mask: ::core::ffi::c_ulong,
    pub ret_value: ::core::ffi::c_ulong,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_39 {
    pub extension_id: ::core::ffi::c_ulong,
    pub function_id: ::core::ffi::c_ulong,
    pub args: [::core::ffi::c_ulong; 6],
    pub ret: [::core::ffi::c_ulong; 2],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_xen_exit {
    pub type_0: __u32,
    pub u: C2RustUnnamed_40,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_40 {
    pub hcall: C2RustUnnamed_41,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_41 {
    pub longmode: __u32,
    pub cpl: __u32,
    pub input: __u64,
    pub result: __u64,
    pub params: [__u64; 6],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_42 {
    pub error: __u8,
    pub pad: [__u8; 7],
    pub reason: __u32,
    pub index: __u32,
    pub data: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_43 {
    pub esr_iss: __u64,
    pub fault_ipa: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_hyperv_exit {
    pub type_0: __u32,
    pub pad1: __u32,
    pub u: C2RustUnnamed_44,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_44 {
    pub synic: C2RustUnnamed_47,
    pub hcall: C2RustUnnamed_46,
    pub syndbg: C2RustUnnamed_45,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_45 {
    pub msr: __u32,
    pub pad2: __u32,
    pub control: __u64,
    pub status: __u64,
    pub send_page: __u64,
    pub recv_page: __u64,
    pub pending_page: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_46 {
    pub input: __u64,
    pub result: __u64,
    pub params: [__u64; 2],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_47 {
    pub msr: __u32,
    pub pad2: __u32,
    pub control: __u64,
    pub evt_page: __u64,
    pub msg_page: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_48 {
    pub vector: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_49 {
    pub addr: __u64,
    pub ar: __u8,
    pub reserved: __u8,
    pub fc: __u8,
    pub sel1: __u8,
    pub sel2: __u16,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_50 {
    pub type_0: __u32,
    pub ndata: __u32,
    pub c2rust_unnamed: C2RustUnnamed_51,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_51 {
    pub flags: __u64,
    pub data: [__u64; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_52 {
    pub epr: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_53 {
    pub subchannel_id: __u16,
    pub subchannel_nr: __u16,
    pub io_int_parm: __u32,
    pub io_int_word: __u32,
    pub ipb: __u32,
    pub dequeued: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_54 {
    pub nr: __u64,
    pub ret: __u64,
    pub args: [__u64; 9],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_55 {
    pub gprs: [__u64; 32],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_56 {
    pub suberror: __u32,
    pub ndata: __u32,
    pub flags: __u64,
    pub c2rust_unnamed: C2RustUnnamed_57,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_57 {
    pub c2rust_unnamed: C2RustUnnamed_58,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_58 {
    pub insn_size: __u8,
    pub insn_bytes: [__u8; 15],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_59 {
    pub suberror: __u32,
    pub ndata: __u32,
    pub data: [__u64; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_60 {
    pub dcrn: __u32,
    pub data: __u32,
    pub is_write: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_61 {
    pub trans_exc_code: __u64,
    pub pgm_code: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_62 {
    pub icptcode: __u8,
    pub ipa: __u16,
    pub ipb: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_63 {
    pub rip: __u64,
    pub is_write: __u32,
    pub pad: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_64 {
    pub nr: __u64,
    pub args: [__u64; 6],
    pub ret: __u64,
    pub c2rust_unnamed: C2RustUnnamed_65,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_65 {
    pub longmode: __u32,
    pub flags: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_66 {
    pub phys_addr: __u64,
    pub data: [__u8; 8],
    pub len: __u32,
    pub is_write: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_67 {
    pub phys_addr: __u64,
    pub data: [__u8; 8],
    pub len: __u32,
    pub is_write: __u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_68 {
    pub arch: kvm_debug_exit_arch,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kvm_debug_exit_arch {
    pub exception: __u32,
    pub pad: __u32,
    pub pc: __u64,
    pub dr6: __u64,
    pub dr7: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_69 {
    pub direction: __u8,
    pub size: __u8,
    pub port: __u16,
    pub count: __u32,
    pub data_offset: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_70 {
    pub exception: __u32,
    pub error_code: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_71 {
    pub hardware_entry_failure_reason: __u64,
    pub cpu: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_72 {
    pub hardware_exit_reason: __u64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QemuCond {
    pub cond: pthread_cond_t,
    pub initialized: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QemuThread {
    pub thread: pthread_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct disassemble_info {
    pub fprintf_func: fprintf_function,
    pub stream: *mut FILE,
    pub application_data: PTR,
    pub flavour: bfd_flavour,
    pub arch: bfd_architecture,
    pub mach: ::core::ffi::c_ulong,
    pub endian: bfd_endian,
    pub symbols: *mut *mut asymbol,
    pub num_symbols: ::core::ffi::c_int,
    pub flags: ::core::ffi::c_ulong,
    pub private_data: PTR,
    pub read_memory_func: Option<
        unsafe extern "C" fn(
            bfd_vma,
            *mut bfd_byte,
            ::core::ffi::c_int,
            *mut disassemble_info,
        ) -> ::core::ffi::c_int,
    >,
    pub memory_error_func:
        Option<unsafe extern "C" fn(::core::ffi::c_int, bfd_vma, *mut disassemble_info) -> ()>,
    pub print_address_func: Option<unsafe extern "C" fn(bfd_vma, *mut disassemble_info) -> ()>,
    pub print_insn:
        Option<unsafe extern "C" fn(bfd_vma, *mut disassemble_info) -> ::core::ffi::c_int>,
    pub symbol_at_address_func:
        Option<unsafe extern "C" fn(bfd_vma, *mut disassemble_info) -> ::core::ffi::c_int>,
    pub buffer: *const bfd_byte,
    pub buffer_vma: bfd_vma,
    pub buffer_length: ::core::ffi::c_int,
    pub bytes_per_line: ::core::ffi::c_int,
    pub bytes_per_chunk: ::core::ffi::c_int,
    pub display_endian: bfd_endian,
    pub insn_info_valid: ::core::ffi::c_char,
    pub branch_delay_insns: ::core::ffi::c_char,
    pub data_size: ::core::ffi::c_char,
    pub insn_type: dis_insn_type,
    pub target: bfd_vma,
    pub target2: bfd_vma,
    pub disassembler_options: *mut ::core::ffi::c_char,
    pub show_opcodes: bool,
    pub target_info: *const ::core::ffi::c_void,
    pub cap_arch: ::core::ffi::c_int,
    pub cap_mode: ::core::ffi::c_int,
    pub cap_insn_unit: ::core::ffi::c_int,
    pub cap_insn_split: ::core::ffi::c_int,
}
pub type bfd_vma = uint64_t;
pub type dis_insn_type = ::core::ffi::c_uint;
pub const dis_dref2: dis_insn_type = 7;
pub const dis_dref: dis_insn_type = 6;
pub const dis_condjsr: dis_insn_type = 5;
pub const dis_jsr: dis_insn_type = 4;
pub const dis_condbranch: dis_insn_type = 3;
pub const dis_branch: dis_insn_type = 2;
pub const dis_nonbranch: dis_insn_type = 1;
pub const dis_noninsn: dis_insn_type = 0;
pub type bfd_endian = ::core::ffi::c_uint;
pub const BFD_ENDIAN_UNKNOWN: bfd_endian = 2;
pub const BFD_ENDIAN_LITTLE: bfd_endian = 1;
pub const BFD_ENDIAN_BIG: bfd_endian = 0;
pub type bfd_byte = uint8_t;
pub type PTR = *mut ::core::ffi::c_void;
pub type asymbol = symbol_cache_entry;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct symbol_cache_entry {
    pub name: *const ::core::ffi::c_char,
    pub udata: C2RustUnnamed_79,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_79 {
    pub p: PTR,
    pub i: bfd_vma,
}
pub type bfd_architecture = ::core::ffi::c_uint;
pub const bfd_arch_last: bfd_architecture = 41;
pub const bfd_arch_loongarch: bfd_architecture = 40;
pub const bfd_arch_rx: bfd_architecture = 39;
pub const bfd_arch_ia64: bfd_architecture = 38;
pub const bfd_arch_moxie: bfd_architecture = 37;
pub const bfd_arch_microblaze: bfd_architecture = 36;
pub const bfd_arch_avr: bfd_architecture = 35;
pub const bfd_arch_mn10300: bfd_architecture = 34;
pub const bfd_arch_mn10200: bfd_architecture = 33;
pub const bfd_arch_m32r: bfd_architecture = 32;
pub const bfd_arch_arc: bfd_architecture = 31;
pub const bfd_arch_v850: bfd_architecture = 30;
pub const bfd_arch_tic30: bfd_architecture = 29;
pub const bfd_arch_w65: bfd_architecture = 28;
pub const bfd_arch_ns32k: bfd_architecture = 27;
pub const bfd_arch_arm: bfd_architecture = 26;
pub const bfd_arch_alpha: bfd_architecture = 25;
pub const bfd_arch_sh: bfd_architecture = 24;
pub const bfd_arch_h8500: bfd_architecture = 23;
pub const bfd_arch_z8k: bfd_architecture = 22;
pub const bfd_arch_d10v: bfd_architecture = 21;
pub const bfd_arch_hppa: bfd_architecture = 20;
pub const bfd_arch_rs6000: bfd_architecture = 19;
pub const bfd_arch_powerpc: bfd_architecture = 18;
pub const bfd_arch_h8300: bfd_architecture = 17;
pub const bfd_arch_pyramid: bfd_architecture = 16;
pub const bfd_arch_m88k: bfd_architecture = 15;
pub const bfd_arch_convex: bfd_architecture = 14;
pub const bfd_arch_alliant: bfd_architecture = 13;
pub const bfd_arch_romp: bfd_architecture = 12;
pub const bfd_arch_i860: bfd_architecture = 11;
pub const bfd_arch_tahoe: bfd_architecture = 10;
pub const bfd_arch_we32k: bfd_architecture = 9;
pub const bfd_arch_i386: bfd_architecture = 8;
pub const bfd_arch_mips: bfd_architecture = 7;
pub const bfd_arch_sparc: bfd_architecture = 6;
pub const bfd_arch_a29k: bfd_architecture = 5;
pub const bfd_arch_i960: bfd_architecture = 4;
pub const bfd_arch_vax: bfd_architecture = 3;
pub const bfd_arch_m68k: bfd_architecture = 2;
pub const bfd_arch_obscure: bfd_architecture = 1;
pub const bfd_arch_unknown: bfd_architecture = 0;
pub type bfd_flavour = ::core::ffi::c_uint;
pub const bfd_target_evax_flavour: bfd_flavour = 15;
pub const bfd_target_msdos_flavour: bfd_flavour = 14;
pub const bfd_target_versados_flavour: bfd_flavour = 13;
pub const bfd_target_os9k_flavour: bfd_flavour = 12;
pub const bfd_target_som_flavour: bfd_flavour = 11;
pub const bfd_target_ihex_flavour: bfd_flavour = 10;
pub const bfd_target_srec_flavour: bfd_flavour = 9;
pub const bfd_target_tekhex_flavour: bfd_flavour = 8;
pub const bfd_target_oasys_flavour: bfd_flavour = 7;
pub const bfd_target_nlm_flavour: bfd_flavour = 6;
pub const bfd_target_ieee_flavour: bfd_flavour = 5;
pub const bfd_target_elf_flavour: bfd_flavour = 4;
pub const bfd_target_ecoff_flavour: bfd_flavour = 3;
pub const bfd_target_coff_flavour: bfd_flavour = 2;
pub const bfd_target_aout_flavour: bfd_flavour = 1;
pub const bfd_target_unknown_flavour: bfd_flavour = 0;
pub type fprintf_function =
    Option<unsafe extern "C" fn(*mut FILE, *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int>;
pub type S390CpuEntitlement = ::core::ffi::c_uint;
pub const S390_CPU_ENTITLEMENT__MAX: S390CpuEntitlement = 4;
pub const S390_CPU_ENTITLEMENT_HIGH: S390CpuEntitlement = 3;
pub const S390_CPU_ENTITLEMENT_MEDIUM: S390CpuEntitlement = 2;
pub const S390_CPU_ENTITLEMENT_LOW: S390CpuEntitlement = 1;
pub const S390_CPU_ENTITLEMENT_AUTO: S390CpuEntitlement = 0;
pub type S390CpuState = ::core::ffi::c_uint;
pub const S390_CPU_STATE__MAX: S390CpuState = 5;
pub const S390_CPU_STATE_LOAD: S390CpuState = 4;
pub const S390_CPU_STATE_OPERATING: S390CpuState = 3;
pub const S390_CPU_STATE_CHECK_STOP: S390CpuState = 2;
pub const S390_CPU_STATE_STOPPED: S390CpuState = 1;
pub const S390_CPU_STATE_UNINITIALIZED: S390CpuState = 0;
pub type SysEmuTarget = ::core::ffi::c_uint;
pub const SYS_EMU_TARGET__MAX: SysEmuTarget = 29;
pub const SYS_EMU_TARGET_XTENSAEB: SysEmuTarget = 28;
pub const SYS_EMU_TARGET_XTENSA: SysEmuTarget = 27;
pub const SYS_EMU_TARGET_X86_64: SysEmuTarget = 26;
pub const SYS_EMU_TARGET_TRICORE: SysEmuTarget = 25;
pub const SYS_EMU_TARGET_SPARC64: SysEmuTarget = 24;
pub const SYS_EMU_TARGET_SPARC: SysEmuTarget = 23;
pub const SYS_EMU_TARGET_SH4EB: SysEmuTarget = 22;
pub const SYS_EMU_TARGET_SH4: SysEmuTarget = 21;
pub const SYS_EMU_TARGET_S390X: SysEmuTarget = 20;
pub const SYS_EMU_TARGET_RX: SysEmuTarget = 19;
pub const SYS_EMU_TARGET_RISCV64: SysEmuTarget = 18;
pub const SYS_EMU_TARGET_RISCV32: SysEmuTarget = 17;
pub const SYS_EMU_TARGET_PPC64: SysEmuTarget = 16;
pub const SYS_EMU_TARGET_PPC: SysEmuTarget = 15;
pub const SYS_EMU_TARGET_OR1K: SysEmuTarget = 14;
pub const SYS_EMU_TARGET_MIPSEL: SysEmuTarget = 13;
pub const SYS_EMU_TARGET_MIPS64EL: SysEmuTarget = 12;
pub const SYS_EMU_TARGET_MIPS64: SysEmuTarget = 11;
pub const SYS_EMU_TARGET_MIPS: SysEmuTarget = 10;
pub const SYS_EMU_TARGET_MICROBLAZE: SysEmuTarget = 9;
pub const SYS_EMU_TARGET_M68K: SysEmuTarget = 8;
pub const SYS_EMU_TARGET_LOONGARCH64: SysEmuTarget = 7;
pub const SYS_EMU_TARGET_I386: SysEmuTarget = 6;
pub const SYS_EMU_TARGET_HPPA: SysEmuTarget = 5;
pub const SYS_EMU_TARGET_HEXAGON: SysEmuTarget = 4;
pub const SYS_EMU_TARGET_AVR: SysEmuTarget = 3;
pub const SYS_EMU_TARGET_ARM: SysEmuTarget = 2;
pub const SYS_EMU_TARGET_ALPHA: SysEmuTarget = 1;
pub const SYS_EMU_TARGET_AARCH64: SysEmuTarget = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DeviceClass {
    pub parent_class: ObjectClass,
    pub categories: [::core::ffi::c_ulong; 1],
    pub fw_name: *const ::core::ffi::c_char,
    pub desc: *const ::core::ffi::c_char,
    pub props_: *const Property,
    pub props_count_: uint16_t,
    pub user_creatable: bool,
    pub hotpluggable: bool,
    pub legacy_reset: DeviceReset,
    pub realize: DeviceRealize,
    pub unrealize: DeviceUnrealize,
    pub sync_config: DeviceSyncConfig,
    pub vmsd: *const VMStateDescription,
    pub bus_type: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VMStateDescription {
    pub name: *const ::core::ffi::c_char,
    pub unmigratable: bool,
    pub early_setup: bool,
    pub version_id: ::core::ffi::c_int,
    pub minimum_version_id: ::core::ffi::c_int,
    pub priority: MigrationPriority,
    pub pre_load: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
    pub pre_load_errp:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut *mut Error) -> bool>,
    pub post_load: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, ::core::ffi::c_int) -> ::core::ffi::c_int,
    >,
    pub post_load_errp: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, ::core::ffi::c_int, *mut *mut Error) -> bool,
    >,
    pub pre_save: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
    pub pre_save_errp:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut *mut Error) -> bool>,
    pub post_save: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()>,
    pub needed: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> bool>,
    pub dev_unplug_pending: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> bool>,
    pub fields: *const VMStateField,
    pub subsections: *const *const VMStateDescription,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VMStateField {
    pub name: *const ::core::ffi::c_char,
    pub offset: size_t,
    pub size: size_t,
    pub size_indirect: VMStateStructMember,
    pub num_indirect: VMStateStructMember,
    pub start: size_t,
    pub num: ::core::ffi::c_int,
    pub info: *const VMStateInfo,
    pub flags: VMStateFlags,
    pub vmsd: *const VMStateDescription,
    pub version_id: ::core::ffi::c_int,
    pub struct_version_id: ::core::ffi::c_int,
    pub field_exists:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, ::core::ffi::c_int) -> bool>,
}
pub type VMStateFlags = ::core::ffi::c_uint;
pub const VMS_END: VMStateFlags = 131072;
pub const VMS_ARRAY_OF_POINTER_AUTO_ALLOC: VMStateFlags = 65536;
pub const VMS_VSTRUCT: VMStateFlags = 32768;
pub const VMS_ALLOC: VMStateFlags = 8192;
pub const VMS_MUST_EXIST: VMStateFlags = 4096;
pub const VMS_MULTIPLY: VMStateFlags = 512;
pub const VMS_VBUFFER: VMStateFlags = 256;
pub const VMS_NO_STATE: VMStateFlags = 128;
pub const VMS_ARRAY_OF_POINTER: VMStateFlags = 64;
pub const VMS_BUFFER: VMStateFlags = 32;
pub const VMS_VARRAY: VMStateFlags = 16;
pub const VMS_STRUCT: VMStateFlags = 8;
pub const VMS_ARRAY: VMStateFlags = 4;
pub const VMS_POINTER: VMStateFlags = 2;
pub const VMS_SINGLE: VMStateFlags = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VMStateInfo {
    pub name: *const ::core::ffi::c_char,
    pub get: Option<
        unsafe extern "C" fn(
            *mut QEMUFile,
            *mut ::core::ffi::c_void,
            size_t,
            *const VMStateField,
        ) -> ::core::ffi::c_int,
    >,
    pub put: Option<
        unsafe extern "C" fn(
            *mut QEMUFile,
            *mut ::core::ffi::c_void,
            size_t,
            *const VMStateField,
            *mut JSONWriter,
        ) -> ::core::ffi::c_int,
    >,
    pub load: Option<
        unsafe extern "C" fn(
            *mut QEMUFile,
            *mut ::core::ffi::c_void,
            size_t,
            *const VMStateField,
            *mut *mut Error,
        ) -> bool,
    >,
    pub save: Option<
        unsafe extern "C" fn(
            *mut QEMUFile,
            *mut ::core::ffi::c_void,
            size_t,
            *const VMStateField,
            *mut JSONWriter,
            *mut *mut Error,
        ) -> bool,
    >,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VMStateStructMember {
    pub offset: uint32_t,
    pub size: uint8_t,
}
pub type MigrationPriority = ::core::ffi::c_uint;
pub const MIG_PRI_MAX: MigrationPriority = 10;
pub const MIG_PRI_GICV3: MigrationPriority = 9;
pub const MIG_PRI_GICV3_ITS: MigrationPriority = 8;
pub const MIG_PRI_APIC: MigrationPriority = 7;
pub const MIG_PRI_VIRTIO_MEM: MigrationPriority = 6;
pub const MIG_PRI_PCI_BUS: MigrationPriority = 5;
pub const MIG_PRI_IOMMU: MigrationPriority = 4;
pub const MIG_PRI_BACKEND: MigrationPriority = 3;
pub const MIG_PRI_DEFAULT: MigrationPriority = 2;
pub const MIG_PRI_LOW: MigrationPriority = 1;
pub const MIG_PRI_UNINITIALIZED: MigrationPriority = 0;
pub type DeviceSyncConfig =
    Option<unsafe extern "C" fn(*mut DeviceState, *mut *mut Error) -> ::core::ffi::c_int>;
pub type DeviceUnrealize = Option<unsafe extern "C" fn(*mut DeviceState) -> ()>;
pub type DeviceRealize = Option<unsafe extern "C" fn(*mut DeviceState, *mut *mut Error) -> ()>;
pub type DeviceReset = Option<unsafe extern "C" fn(*mut DeviceState) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Property {
    pub name: *const ::core::ffi::c_char,
    pub info: *const PropertyInfo,
    pub offset: ptrdiff_t,
    pub link_type: *const ::core::ffi::c_char,
    pub bitmask: uint64_t,
    pub defval: C2RustUnnamed_81,
    pub arrayinfo: *const PropertyInfo,
    pub arrayoffset: ::core::ffi::c_int,
    pub arrayfieldsize: ::core::ffi::c_int,
    pub bitnr: uint8_t,
    pub set_default: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PropertyInfo {
    pub type_0: *const ::core::ffi::c_char,
    pub description: *const ::core::ffi::c_char,
    pub qapi_type: *const QAPITypeInfo,
    pub realized_set_allowed: bool,
    pub print:
        Option<unsafe extern "C" fn(*mut Object, *const Property) -> *mut ::core::ffi::c_char>,
    pub set_default_value: Option<unsafe extern "C" fn(*mut ObjectProperty, *const Property) -> ()>,
    pub create: Option<
        unsafe extern "C" fn(
            *mut ObjectClass,
            *const ::core::ffi::c_char,
            *const Property,
        ) -> *mut ObjectProperty,
    >,
    pub get: Option<ObjectPropertyAccessor>,
    pub set: Option<ObjectPropertyAccessor>,
    pub release: Option<ObjectPropertyRelease>,
}
pub type ObjectPropertyRelease =
    unsafe extern "C" fn(*mut Object, *const ::core::ffi::c_char, *mut ::core::ffi::c_void) -> ();
pub type ObjectPropertyAccessor = unsafe extern "C" fn(
    *mut Object,
    *mut Visitor,
    *const ::core::ffi::c_char,
    *mut ::core::ffi::c_void,
    *mut *mut Error,
) -> ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ObjectProperty {
    pub name: *mut ::core::ffi::c_char,
    pub type_0: *mut ::core::ffi::c_char,
    pub description: *mut ::core::ffi::c_char,
    pub qapi_type: *const QAPITypeInfo,
    pub get: Option<ObjectPropertyAccessor>,
    pub set: Option<ObjectPropertyAccessor>,
    pub resolve: Option<ObjectPropertyResolve>,
    pub release: Option<ObjectPropertyRelease>,
    pub init: Option<ObjectPropertyInit>,
    pub opaque: *mut ::core::ffi::c_void,
    pub defval: *mut QObject,
}
pub type ObjectPropertyInit = unsafe extern "C" fn(*mut Object, *mut ObjectProperty) -> ();
pub type ObjectPropertyResolve = unsafe extern "C" fn(
    *mut Object,
    *mut ::core::ffi::c_void,
    *const ::core::ffi::c_char,
) -> *mut Object;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QAPITypeInfo {
    pub name: *const ::core::ffi::c_char,
    pub masked_name: *const ::core::ffi::c_char,
    pub lookup: *const QEnumLookup,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QEnumLookup {
    pub array: *const *const ::core::ffi::c_char,
    pub features: *const uint64_t,
    pub size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_81 {
    pub i: int64_t,
    pub u: uint64_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QEMUCursor {
    pub width: uint16_t,
    pub height: uint16_t,
    pub hot_x: ::core::ffi::c_int,
    pub hot_y: ::core::ffi::c_int,
    pub refcount: ::core::ffi::c_int,
    pub data: [uint32_t; 0],
}
pub type pixman_format_code_t = ::core::ffi::c_uint;
pub const PIXMAN_yv12: pixman_format_code_t = 201785344;
pub const PIXMAN_yuy2: pixman_format_code_t = 268828672;
pub const PIXMAN_g1: pixman_format_code_t = 17104896;
pub const PIXMAN_a1: pixman_format_code_t = 16846848;
pub const PIXMAN_g4: pixman_format_code_t = 67436544;
pub const PIXMAN_c4: pixman_format_code_t = 67371008;
pub const PIXMAN_a1b1g1r1: pixman_format_code_t = 67309841;
pub const PIXMAN_a1r1g1b1: pixman_format_code_t = 67244305;
pub const PIXMAN_b1g2r1: pixman_format_code_t = 67305761;
pub const PIXMAN_r1g2b1: pixman_format_code_t = 67240225;
pub const PIXMAN_a4: pixman_format_code_t = 67190784;
pub const PIXMAN_x4g4: pixman_format_code_t = 134545408;
pub const PIXMAN_x4c4: pixman_format_code_t = 134479872;
pub const PIXMAN_x4a4: pixman_format_code_t = 134299648;
pub const PIXMAN_g8: pixman_format_code_t = 134545408;
pub const PIXMAN_c8: pixman_format_code_t = 134479872;
pub const PIXMAN_a2b2g2r2: pixman_format_code_t = 134423074;
pub const PIXMAN_a2r2g2b2: pixman_format_code_t = 134357538;
pub const PIXMAN_b2g3r3: pixman_format_code_t = 134415154;
pub const PIXMAN_r3g3b2: pixman_format_code_t = 134349618;
pub const PIXMAN_a8: pixman_format_code_t = 134316032;
pub const PIXMAN_x4b4g4r4: pixman_format_code_t = 268633156;
pub const PIXMAN_a4b4g4r4: pixman_format_code_t = 268649540;
pub const PIXMAN_x4r4g4b4: pixman_format_code_t = 268567620;
pub const PIXMAN_a4r4g4b4: pixman_format_code_t = 268584004;
pub const PIXMAN_x1b5g5r5: pixman_format_code_t = 268633429;
pub const PIXMAN_a1b5g5r5: pixman_format_code_t = 268637525;
pub const PIXMAN_x1r5g5b5: pixman_format_code_t = 268567893;
pub const PIXMAN_a1r5g5b5: pixman_format_code_t = 268571989;
pub const PIXMAN_b5g6r5: pixman_format_code_t = 268633445;
pub const PIXMAN_r5g6b5: pixman_format_code_t = 268567909;
pub const PIXMAN_b8g8r8: pixman_format_code_t = 402851976;
pub const PIXMAN_r8g8b8: pixman_format_code_t = 402786440;
pub const PIXMAN_r8g8b8_sRGB: pixman_format_code_t = 403310728;
pub const PIXMAN_a8r8g8b8_sRGB: pixman_format_code_t = 537561224;
pub const PIXMAN_a2b10g10r10: pixman_format_code_t = 537078442;
pub const PIXMAN_x2b10g10r10: pixman_format_code_t = 537070250;
pub const PIXMAN_a2r10g10b10: pixman_format_code_t = 537012906;
pub const PIXMAN_x2r10g10b10: pixman_format_code_t = 537004714;
pub const PIXMAN_x14r6g6b6: pixman_format_code_t = 537003622;
pub const PIXMAN_r8g8b8x8: pixman_format_code_t = 537462920;
pub const PIXMAN_r8g8b8a8: pixman_format_code_t = 537495688;
pub const PIXMAN_b8g8r8x8: pixman_format_code_t = 537397384;
pub const PIXMAN_b8g8r8a8: pixman_format_code_t = 537430152;
pub const PIXMAN_x8b8g8r8: pixman_format_code_t = 537069704;
pub const PIXMAN_a8b8g8r8: pixman_format_code_t = 537102472;
pub const PIXMAN_x8r8g8b8: pixman_format_code_t = 537004168;
pub const PIXMAN_a8r8g8b8: pixman_format_code_t = 537036936;
pub const PIXMAN_a16b16g16r16: pixman_format_code_t = 147005986;
pub const PIXMAN_rgb_float: pixman_format_code_t = 214631492;
pub const PIXMAN_rgba_float: pixman_format_code_t = 281756740;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DisplaySurface {
    pub image: *mut pixman_image_t,
    pub flags: uint8_t,
    pub texture: uint32_t,
    pub share_handle: qemu_pixman_shareable,
    pub share_handle_offset: uint32_t,
}
pub type qemu_pixman_shareable = ::core::ffi::c_int;
pub type pixman_image_t = pixman_image;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct I2CBus {
    pub qbus: BusState,
    pub current_devs: I2CNodeList,
    pub pending_masters: I2CPendingMasters,
    pub saved_address: uint8_t,
    pub broadcast: bool,
    pub bh: *mut QEMUBH,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct I2CPendingMasters {
    pub sqh_first: *mut I2CPendingMaster,
    pub sqh_last: *mut *mut I2CPendingMaster,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct I2CPendingMaster {
    pub bh: *mut QEMUBH,
    pub entry: C2RustUnnamed_82,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_82 {
    pub sqe_next: *mut I2CPendingMaster,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct I2CNodeList {
    pub lh_first: *mut I2CNode,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct I2CNode {
    pub elt: *mut I2CSlave,
    pub next: C2RustUnnamed_83,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_83 {
    pub le_next: *mut I2CNode,
    pub le_prev: *mut *mut I2CNode,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct I2CSlave {
    pub qdev: DeviceState,
    pub address: uint8_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIDevice {
    pub qdev: DeviceState,
    pub partially_hotplugged: bool,
    pub enabled: bool,
    pub loadparm: *mut ::core::ffi::c_char,
    pub config: *mut uint8_t,
    pub cmask: *mut uint8_t,
    pub wmask: *mut uint8_t,
    pub w1cmask: *mut uint8_t,
    pub used: *mut uint8_t,
    pub devfn: int32_t,
    pub requester_id_cache: PCIReqIDCache,
    pub name: [::core::ffi::c_char; 64],
    pub io_regions: [PCIIORegion; 7],
    pub bus_master_as: AddressSpace,
    pub is_master: bool,
    pub bus_master_container_region: MemoryRegion,
    pub bus_master_enable_region: MemoryRegion,
    pub config_read: Option<PCIConfigReadFunc>,
    pub config_write: Option<PCIConfigWriteFunc>,
    pub vga_regions: [*mut MemoryRegion; 3],
    pub has_vga: bool,
    pub irq_state: uint8_t,
    pub cap_present: uint32_t,
    pub pm_cap: uint8_t,
    pub msix_cap: uint8_t,
    pub msix_entries_nr: ::core::ffi::c_int,
    pub msix_table: *mut uint8_t,
    pub msix_pba: *mut uint8_t,
    pub irq_opaque: *mut ::core::ffi::c_void,
    pub msi_trigger: Option<MSITriggerFunc>,
    pub msi_prepare_message: Option<MSIPrepareMessageFunc>,
    pub msix_prepare_message: Option<MSIxPrepareMessageFunc>,
    pub msix_exclusive_bar: MemoryRegion,
    pub msix_table_mmio: MemoryRegion,
    pub msix_pba_mmio: MemoryRegion,
    pub msix_entry_used: *mut ::core::ffi::c_uint,
    pub msix_function_masked: bool,
    pub version_id: int32_t,
    pub msi_cap: uint8_t,
    pub exp: PCIExpressDevice,
    pub shpc: *mut SHPCDevice,
    pub romfile: *mut ::core::ffi::c_char,
    pub romsize: uint32_t,
    pub has_rom: bool,
    pub rom: MemoryRegion,
    pub rom_bar: int32_t,
    pub intx_routing_notifier: PCIINTxRoutingNotifier,
    pub msix_vector_use_notifier: MSIVectorUseNotifier,
    pub msix_vector_release_notifier: MSIVectorReleaseNotifier,
    pub msix_vector_poll_notifier: MSIVectorPollNotifier,
    pub spdm_port: uint16_t,
    pub spdm_trans: SpdmTransportType,
    pub doe_spdm: DOECap,
    pub failover_pair_id: *mut ::core::ffi::c_char,
    pub acpi_index: uint32_t,
    pub max_bounce_buffer_size: uint32_t,
    pub sriov_pf: *mut ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DOECap {
    pub pdev: *mut PCIDevice,
    pub offset: uint16_t,
    pub cap: C2RustUnnamed_86,
    pub ctrl: C2RustUnnamed_85,
    pub status: C2RustUnnamed_84,
    pub write_mbox: *mut uint32_t,
    pub read_mbox: *mut uint32_t,
    pub read_mbox_idx: uint32_t,
    pub read_mbox_len: uint32_t,
    pub write_mbox_len: uint32_t,
    pub protocols: *mut DOEProtocol,
    pub protocol_num: uint16_t,
    pub spdm_socket: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DOEProtocol {
    pub vendor_id: uint16_t,
    pub data_obj_type: uint8_t,
    pub handle_request: Option<unsafe extern "C" fn(*mut DOECap) -> bool>,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_84 {
    pub busy: bool,
    pub intr: bool,
    pub error: bool,
    pub ready: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_85 {
    pub abort: bool,
    pub intr: bool,
    pub go: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_86 {
    pub intr: bool,
    pub vec: uint16_t,
}
pub type SpdmTransportType = ::core::ffi::c_uint;
pub const SPDM_TRANSPORT_TYPE__MAX: SpdmTransportType = 5;
pub const SPDM_TRANSPORT_TYPE_NVME: SpdmTransportType = 4;
pub const SPDM_TRANSPORT_TYPE_SCSI: SpdmTransportType = 3;
pub const SPDM_TRANSPORT_TYPE_DOE: SpdmTransportType = 2;
pub const SPDM_TRANSPORT_TYPE_MCTP: SpdmTransportType = 1;
pub const SPDM_TRANSPORT_TYPE_UNSPECIFIED: SpdmTransportType = 0;
pub type MSIVectorPollNotifier =
    Option<unsafe extern "C" fn(*mut PCIDevice, ::core::ffi::c_uint, ::core::ffi::c_uint) -> ()>;
pub type MSIVectorReleaseNotifier =
    Option<unsafe extern "C" fn(*mut PCIDevice, ::core::ffi::c_uint) -> ()>;
pub type MSIVectorUseNotifier = Option<
    unsafe extern "C" fn(*mut PCIDevice, ::core::ffi::c_uint, MSIMessage) -> ::core::ffi::c_int,
>;
pub type PCIINTxRoutingNotifier = Option<unsafe extern "C" fn(*mut PCIDevice) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIExpressDevice {
    pub exp_cap: uint8_t,
    pub hpev_notified: bool,
    pub aer_cap: uint16_t,
    pub aer_log: PCIEAERLog,
    pub ats_cap: uint16_t,
    pub pasid_cap: uint16_t,
    pub pri_cap: uint16_t,
    pub acs_cap: uint16_t,
    pub sriov_cap: uint16_t,
    pub sriov_pf: PCIESriovPF,
    pub sriov_vf: PCIESriovVF,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIESriovVF {
    pub pf: *mut PCIDevice,
    pub vf_number: uint16_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIESriovPF {
    pub vf_bar_type: [uint8_t; 6],
    pub vf: *mut *mut PCIDevice,
    pub vf_user_created: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIEAERLog {
    pub log_num: uint16_t,
    pub log_max: uint16_t,
    pub log: *mut PCIEAERErr,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIEAERErr {
    pub status: uint32_t,
    pub source_id: uint16_t,
    pub flags: uint16_t,
    pub header: [uint32_t; 4],
    pub prefix: [uint32_t; 4],
}
pub type MSIxPrepareMessageFunc =
    unsafe extern "C" fn(*mut PCIDevice, ::core::ffi::c_uint) -> MSIMessage;
pub type MSIPrepareMessageFunc =
    unsafe extern "C" fn(*mut PCIDevice, ::core::ffi::c_uint) -> MSIMessage;
pub type MSITriggerFunc = unsafe extern "C" fn(*mut PCIDevice, MSIMessage) -> ();
pub type PCIConfigWriteFunc =
    unsafe extern "C" fn(*mut PCIDevice, uint32_t, uint32_t, ::core::ffi::c_int) -> ();
pub type PCIConfigReadFunc =
    unsafe extern "C" fn(*mut PCIDevice, uint32_t, ::core::ffi::c_int) -> uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIIORegion {
    pub addr: pcibus_t,
    pub size: pcibus_t,
    pub type_0: uint8_t,
    pub memory: *mut MemoryRegion,
    pub address_space: *mut MemoryRegion,
}
pub type pcibus_t = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIReqIDCache {
    pub dev: *mut PCIDevice,
    pub type_0: PCIReqIDType,
}
pub type PCIReqIDType = ::core::ffi::c_uint;
pub const PCI_REQ_ID_MAX: PCIReqIDType = 3;
pub const PCI_REQ_ID_SECONDARY_BUS: PCIReqIDType = 2;
pub const PCI_REQ_ID_BDF: PCIReqIDType = 1;
pub const PCI_REQ_ID_INVALID: PCIReqIDType = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QEMUTimer {
    pub expire_time: int64_t,
    pub timer_list: *mut QEMUTimerList,
    pub cb: Option<QEMUTimerCB>,
    pub opaque: *mut ::core::ffi::c_void,
    pub next: *mut QEMUTimer,
    pub attributes: ::core::ffi::c_int,
    pub scale: ::core::ffi::c_int,
}
pub type QEMUTimerCB = unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ();
pub type module_init_type = ::core::ffi::c_uint;
pub const MODULE_INIT_MAX: module_init_type = 9;
pub const MODULE_INIT_FUZZ_TARGET: module_init_type = 8;
pub const MODULE_INIT_LIBQOS: module_init_type = 7;
pub const MODULE_INIT_XEN_BACKEND: module_init_type = 6;
pub const MODULE_INIT_TRACE: module_init_type = 5;
pub const MODULE_INIT_QOM: module_init_type = 4;
pub const MODULE_INIT_TARGET_INFO: module_init_type = 3;
pub const MODULE_INIT_OPTS: module_init_type = 2;
pub const MODULE_INIT_BLOCK: module_init_type = 1;
pub const MODULE_INIT_MIGRATION: module_init_type = 0;
pub type QEMUClockType = ::core::ffi::c_uint;
pub const QEMU_CLOCK_MAX: QEMUClockType = 4;
pub const QEMU_CLOCK_VIRTUAL_RT: QEMUClockType = 3;
pub const QEMU_CLOCK_HOST: QEMUClockType = 2;
pub const QEMU_CLOCK_VIRTUAL: QEMUClockType = 1;
pub const QEMU_CLOCK_REALTIME: QEMUClockType = 0;
pub type QemuFileType = ::core::ffi::c_uint;
pub const QEMU_FILE_TYPE_KEYMAP: QemuFileType = 2;
pub const QEMU_FILE_TYPE_DTB: QemuFileType = 1;
pub const QEMU_FILE_TYPE_BIOS: QemuFileType = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TypeInfo {
    pub name: *const ::core::ffi::c_char,
    pub parent: *const ::core::ffi::c_char,
    pub instance_size: size_t,
    pub instance_align: size_t,
    pub instance_init: Option<unsafe extern "C" fn(*mut Object) -> ()>,
    pub instance_post_init: Option<unsafe extern "C" fn(*mut Object) -> ()>,
    pub instance_finalize: Option<unsafe extern "C" fn(*mut Object) -> ()>,
    pub abstract_0: bool,
    pub secure: bool,
    pub class_size: size_t,
    pub class_init:
        Option<unsafe extern "C" fn(*mut ObjectClass, *const ::core::ffi::c_void) -> ()>,
    pub class_base_init:
        Option<unsafe extern "C" fn(*mut ObjectClass, *const ::core::ffi::c_void) -> ()>,
    pub class_data: *const ::core::ffi::c_void,
    pub is_available: Option<unsafe extern "C" fn() -> bool>,
    pub interfaces: *const InterfaceInfo,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct InterfaceInfo {
    pub type_0: *const ::core::ffi::c_char,
}
pub type RCUReadAuto = ();
pub type RCUReadAuto_autoptr = *mut ::core::ffi::c_void;
pub type QEMUBHFunc = unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MemoryRegionPortio {
    pub offset: uint32_t,
    pub len: uint32_t,
    pub size: ::core::ffi::c_uint,
    pub read: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, uint32_t) -> uint32_t>,
    pub write: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, uint32_t, uint32_t) -> ()>,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PortioList {
    pub ports: *const MemoryRegionPortio,
    pub owner: *mut Object,
    pub address_space: *mut MemoryRegion,
    pub addr: uint32_t,
    pub nr: ::core::ffi::c_uint,
    pub regions: *mut *mut MemoryRegion,
    pub opaque: *mut ::core::ffi::c_void,
    pub name: *const ::core::ffi::c_char,
    pub flush_coalesced_mmio: bool,
}
pub type DeviceCategory = ::core::ffi::c_uint;
pub const DEVICE_CATEGORY_MAX: DeviceCategory = 10;
pub const DEVICE_CATEGORY_WATCHDOG: DeviceCategory = 9;
pub const DEVICE_CATEGORY_CPU: DeviceCategory = 8;
pub const DEVICE_CATEGORY_MISC: DeviceCategory = 7;
pub const DEVICE_CATEGORY_SOUND: DeviceCategory = 6;
pub const DEVICE_CATEGORY_DISPLAY: DeviceCategory = 5;
pub const DEVICE_CATEGORY_INPUT: DeviceCategory = 4;
pub const DEVICE_CATEGORY_NETWORK: DeviceCategory = 3;
pub const DEVICE_CATEGORY_STORAGE: DeviceCategory = 2;
pub const DEVICE_CATEGORY_USB: DeviceCategory = 1;
pub const DEVICE_CATEGORY_BRIDGE: DeviceCategory = 0;
pub type PCIUnregisterFunc = unsafe extern "C" fn(*mut PCIDevice) -> ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PCIDeviceClass {
    pub parent_class: DeviceClass,
    pub realize: Option<unsafe extern "C" fn(*mut PCIDevice, *mut *mut Error) -> ()>,
    pub exit: Option<PCIUnregisterFunc>,
    pub config_read: Option<PCIConfigReadFunc>,
    pub config_write: Option<PCIConfigWriteFunc>,
    pub vendor_id: uint16_t,
    pub device_id: uint16_t,
    pub revision: uint8_t,
    pub class_id: uint16_t,
    pub subsystem_vendor_id: uint16_t,
    pub subsystem_id: uint16_t,
    pub romfile: *const ::core::ffi::c_char,
    pub sriov_vf_user_creatable: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct QemuUIInfo {
    pub width_mm: uint16_t,
    pub height_mm: uint16_t,
    pub xoff: ::core::ffi::c_int,
    pub yoff: ::core::ffi::c_int,
    pub width: uint32_t,
    pub height: uint32_t,
    pub refresh_rate: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GraphicHwOps {
    pub get_flags: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
    pub invalidate: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()>,
    pub gfx_update: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> bool>,
    pub text_update: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut uint32_t) -> ()>,
    pub ui_info:
        Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, uint32_t, *mut QemuUIInfo) -> ()>,
    pub gl_block: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, bool) -> ()>,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct vga_precise_retrace {
    pub ticks_per_char: int64_t,
    pub total_chars: int64_t,
    pub htotal: ::core::ffi::c_int,
    pub hstart: ::core::ffi::c_int,
    pub hend: ::core::ffi::c_int,
    pub vstart: ::core::ffi::c_int,
    pub vend: ::core::ffi::c_int,
    pub freq: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union vga_retrace {
    pub precise: vga_precise_retrace,
}
