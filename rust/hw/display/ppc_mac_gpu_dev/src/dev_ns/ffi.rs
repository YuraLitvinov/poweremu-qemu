use super::*;
extern "C" {
    pub fn seq_log(fmt: *const ::core::ffi::c_char, ...);
    pub fn gpu_debug_log(fmt: *const ::core::ffi::c_char, ...);
    pub fn blit_path_log(path: *const ::core::ffi::c_char, fmt: *const ::core::ffi::c_char, ...);
    pub fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    pub fn strtoul(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    pub fn strtoll(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
    pub fn strtoull(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulonglong;
    pub fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    pub fn free(__ptr: *mut ::core::ffi::c_void);
    pub fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    pub fn abs(__x: ::core::ffi::c_int) -> ::core::ffi::c_int;
    pub static mut stderr: *mut FILE;
    pub fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    pub fn fflush(__stream: *mut FILE) -> ::core::ffi::c_int;
    pub fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    pub fn setvbuf(
        __stream: *mut FILE,
        __buf: *mut ::core::ffi::c_char,
        __modes: ::core::ffi::c_int,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    pub fn fputc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    pub fn fwrite(
        __ptr: *const ::core::ffi::c_void,
        __size: size_t,
        __n: size_t,
        __s: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    pub fn ftell(__stream: *mut FILE) -> ::core::ffi::c_long;
    pub fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    pub fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __n: size_t,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    pub fn fprintf(__stream: *mut FILE, __fmt: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    pub fn vfprintf(
        __stream: *mut FILE,
        __fmt: *const ::core::ffi::c_char,
        __ap: ::core::ffi::VaList,
    ) -> ::core::ffi::c_int;
    pub fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    pub fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    pub fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    pub fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    pub fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    pub fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    pub fn memset(
        __dest: *mut ::core::ffi::c_void,
        __ch: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    pub fn g_getenv(variable: *const gchar) -> *const gchar;
    pub fn g_setenv(variable: *const gchar, value: *const gchar, overwrite: gboolean) -> gboolean;
    pub fn g_unsetenv(variable: *const gchar);
    pub fn g_file_get_contents(
        filename: *const gchar,
        contents: *mut *mut gchar,
        length: *mut gsize,
        error: *mut *mut GError,
    ) -> gboolean;
    pub fn g_free(mem: gpointer);
    pub fn g_malloc(n_bytes: gsize) -> gpointer;
    pub fn g_malloc0(n_bytes: gsize) -> gpointer;
    pub fn g_try_malloc(n_bytes: gsize) -> gpointer;
    pub fn g_malloc_n(n_blocks: gsize, n_block_bytes: gsize) -> gpointer;
    pub fn g_malloc0_n(n_blocks: gsize, n_block_bytes: gsize) -> gpointer;
    pub fn g_hash_table_new(hash_func: GHashFunc, key_equal_func: GEqualFunc) -> *mut GHashTable;
    pub fn g_hash_table_add(hash_table: *mut GHashTable, key: gpointer) -> gboolean;
    pub fn g_hash_table_contains(hash_table: *mut GHashTable, key: gconstpointer) -> gboolean;
    pub fn g_str_equal(v1: gconstpointer, v2: gconstpointer) -> gboolean;
    pub fn g_str_hash(v: gconstpointer) -> guint;
    pub fn g_direct_hash(v: gconstpointer) -> guint;
    pub fn g_direct_equal(v1: gconstpointer, v2: gconstpointer) -> gboolean;
    pub fn g_get_monotonic_time() -> gint64;
    pub fn g_str_has_prefix(str: *const gchar, prefix: *const gchar) -> gboolean;
    pub fn g_strdup_printf(format: *const gchar, ...) -> *mut gchar;
    #[link_name = "shim_g_strdup_inline"]
    pub fn g_strdup_inline(str: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    pub fn g_string_new(init: *const gchar) -> *mut GString;
    pub fn g_string_free(string: *mut GString, free_segment: gboolean) -> *mut gchar;
    pub fn g_string_append_printf(string: *mut GString, format: *const gchar, ...);
    #[link_name = "shim_g_string_append_len_inline"]
    pub fn g_string_append_len_inline(
        gstring: *mut GString,
        val: *const ::core::ffi::c_char,
        len: gssize,
    ) -> *mut GString;
    pub fn g_usleep(microseconds: gulong);
    pub fn sched_yield() -> ::core::ffi::c_int;
    pub fn qemu_build_not_reached_always() -> !;
    pub fn r300_state_write(st: *mut R300State, addr: uint32_t, val: uint32_t);
    pub fn r300_state_dump(st: *const R300State, write: R300WriteFn, ctx: *mut ::core::ffi::c_void);
    pub fn r300_reg_name(addr: uint32_t) -> *const ::core::ffi::c_char;
    pub fn r300_us_disasm(st: *const R300State, write: R300WriteFn, ctx: *mut ::core::ffi::c_void);
    pub fn r300_load_vbpntr(arr: *mut R300Arrays, d: *const uint32_t, ndw: uint32_t);
    pub fn r300_draw_build(
        st: *const R300State,
        arr: *const R300Arrays,
        opcode: uint32_t,
        d: *const uint32_t,
        ndw: uint32_t,
        read: R300ReadFn,
        opaque: *mut ::core::ffi::c_void,
        pkt: *mut R300DrawPacket,
        err: *mut *const ::core::ffi::c_char,
    ) -> bool;
    pub fn r300_draw_build_indexed(
        st: *const R300State,
        arr: *const R300Arrays,
        vf: uint32_t,
        idx: *const R300Indices,
        read: R300ReadFn,
        opaque: *mut ::core::ffi::c_void,
        pkt: *mut R300DrawPacket,
        err: *mut *const ::core::ffi::c_char,
    ) -> bool;
    pub fn r300_aa_samples(st: *const R300State) -> uint32_t;
    pub fn r300_msaa_offset(
        x: uint32_t,
        y: uint32_t,
        ns: uint32_t,
        pitch_px: uint32_t,
        bpp: uint32_t,
        sample: uint32_t,
    ) -> uint32_t;
    pub fn r300_draw_free(pkt: *mut R300DrawPacket);
    pub fn powf(__x: ::core::ffi::c_float, __y: ::core::ffi::c_float) -> ::core::ffi::c_float;
    pub fn ceilf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
    pub fn floorf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
    pub fn lrintf(__x: ::core::ffi::c_float) -> ::core::ffi::c_long;
    #[link_name = "shim_qemu_loglevel_mask"]
    pub fn qemu_loglevel_mask(mask: ::core::ffi::c_int) -> bool;
    pub fn qemu_log(fmt: *const ::core::ffi::c_char, ...);
    #[link_name = "shim_ctz32"]
    pub fn ctz32(val: uint32_t) -> ::core::ffi::c_int;
    pub fn register_module_init(fn_0: Option<unsafe extern "C" fn() -> ()>, type_0: module_init_type);
    #[link_name = "shim_set_bit"]
    pub fn set_bit(nr: ::core::ffi::c_long, addr: *mut ::core::ffi::c_ulong);
    #[link_name = "shim_extract32"]
    pub fn extract32(
        value: uint32_t,
        start: ::core::ffi::c_int,
        length: ::core::ffi::c_int,
    ) -> uint32_t;
    #[link_name = "shim_deposit32"]
    pub fn deposit32(
        value: uint32_t,
        start: ::core::ffi::c_int,
        length: ::core::ffi::c_int,
        fieldval: uint32_t,
    ) -> uint32_t;
    pub fn qemu_clock_get_ns(type_0: QEMUClockType) -> int64_t;
    #[link_name = "shim_qemu_clock_get_ms"]
    pub fn qemu_clock_get_ms(type_0: QEMUClockType) -> int64_t;
    #[link_name = "shim_timer_init_ns"]
    pub fn timer_init_ns(
        ts: *mut QEMUTimer,
        type_0: QEMUClockType,
        cb: Option<QEMUTimerCB>,
        opaque: *mut ::core::ffi::c_void,
    );
    pub fn timer_del(ts: *mut QEMUTimer);
    pub fn timer_mod(ts: *mut QEMUTimer, expire_timer: int64_t);
    pub fn timer_pending(ts: *const QEMUTimer) -> bool;
    pub fn qemu_find_file(
        type_0: QemuFileType,
        name: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    pub fn object_dynamic_cast_assert(
        obj: *mut Object,
        typename: *const ::core::ffi::c_char,
        file: *const ::core::ffi::c_char,
        line: ::core::ffi::c_int,
        func: *const ::core::ffi::c_char,
    ) -> *mut Object;
    pub fn object_get_typename(obj: *const Object) -> *const ::core::ffi::c_char;
    pub fn type_register_static(info: *const TypeInfo) -> Type;
    pub fn object_unparent(obj: *mut Object);
    pub fn object_class_property_add_str(
        klass: *mut ObjectClass,
        name: *const ::core::ffi::c_char,
        get: Option<unsafe extern "C" fn(*mut Object, *mut *mut Error) -> *mut ::core::ffi::c_char>,
        set: Option<
            unsafe extern "C" fn(*mut Object, *const ::core::ffi::c_char, *mut *mut Error) -> (),
        >,
    ) -> *mut ObjectProperty;
    #[link_name = "shim_be32_to_cpu"]
    pub fn be32_to_cpu(v: uint32_t) -> uint32_t;
    #[link_name = "shim_cpu_to_be32"]
    pub fn cpu_to_be32(v: uint32_t) -> uint32_t;
    #[link_name = "shim_le32_to_cpu"]
    pub fn le32_to_cpu(v: uint32_t) -> uint32_t;
    #[link_name = "shim_cpu_to_le32"]
    pub fn cpu_to_le32(v: uint32_t) -> uint32_t;
    #[link_name = "shim_lduw_le_p"]
    pub fn lduw_le_p(ptr: *const ::core::ffi::c_void) -> ::core::ffi::c_int;
    #[link_name = "shim_ldl_le_p"]
    pub fn ldl_le_p(ptr: *const ::core::ffi::c_void) -> ::core::ffi::c_int;
    #[link_name = "shim_stw_le_p"]
    pub fn stw_le_p(ptr: *mut ::core::ffi::c_void, v: uint16_t);
    #[link_name = "shim_stl_le_p"]
    pub fn stl_le_p(ptr: *mut ::core::ffi::c_void, v: uint32_t);
    #[link_name = "shim_ldl_be_p"]
    pub fn ldl_be_p(ptr: *const ::core::ffi::c_void) -> ::core::ffi::c_int;
    #[link_name = "shim_rcu_read_auto_lock"]
    pub fn rcu_read_auto_lock() -> *mut ::core::ffi::c_void;
    pub fn memory_region_init(
        mr: *mut MemoryRegion,
        owner: *mut Object,
        name: *const ::core::ffi::c_char,
        size: uint64_t,
    );
    pub fn memory_region_init_io(
        mr: *mut MemoryRegion,
        owner: *mut Object,
        ops: *const MemoryRegionOps,
        opaque: *mut ::core::ffi::c_void,
        name: *const ::core::ffi::c_char,
        size: uint64_t,
    );
    pub fn memory_region_init_ram_ptr(
        mr: *mut MemoryRegion,
        owner: *mut Object,
        name: *const ::core::ffi::c_char,
        size: uint64_t,
        ptr: *mut ::core::ffi::c_void,
    );
    pub fn memory_region_init_alias(
        mr: *mut MemoryRegion,
        owner: *mut Object,
        name: *const ::core::ffi::c_char,
        orig: *mut MemoryRegion,
        offset: hwaddr,
        size: uint64_t,
    );
    pub fn memory_region_init_ram(
        mr: *mut MemoryRegion,
        owner: *mut Object,
        name: *const ::core::ffi::c_char,
        size: uint64_t,
        errp: *mut *mut Error,
    ) -> bool;
    #[link_name = "shim_memory_region_is_ram"]
    pub fn memory_region_is_ram(mr: *const MemoryRegion) -> bool;
    pub fn memory_region_get_ram_ptr(mr: *const MemoryRegion) -> *mut ::core::ffi::c_void;
    pub fn memory_region_set_log(mr: *mut MemoryRegion, log: bool, client: ::core::ffi::c_uint);
    pub fn memory_region_set_dirty(mr: *mut MemoryRegion, addr: hwaddr, size: hwaddr);
    pub fn memory_region_snapshot_and_clear_dirty(
        mr: *mut MemoryRegion,
        addr: hwaddr,
        size: hwaddr,
        client: ::core::ffi::c_uint,
    ) -> *mut DirtyBitmapSnapshot;
    pub fn memory_region_snapshot_get_dirty(
        mr: *mut MemoryRegion,
        snap: *mut DirtyBitmapSnapshot,
        addr: hwaddr,
        size: hwaddr,
    ) -> bool;
    pub fn memory_region_add_subregion(
        mr: *mut MemoryRegion,
        offset: hwaddr,
        subregion: *mut MemoryRegion,
    );
    pub fn memory_region_add_subregion_overlap(
        mr: *mut MemoryRegion,
        offset: hwaddr,
        subregion: *mut MemoryRegion,
        priority: ::core::ffi::c_int,
    );
    pub fn memory_region_del_subregion(mr: *mut MemoryRegion, subregion: *mut MemoryRegion);
    pub fn memory_region_set_enabled(mr: *mut MemoryRegion, enabled: bool);
    pub fn address_space_write(
        as_0: *const AddressSpace,
        addr: hwaddr,
        attrs: MemTxAttrs,
        buf: *const ::core::ffi::c_void,
        len: hwaddr,
    ) -> MemTxResult;
    #[link_name = "shim_address_space_translate"]
    pub fn address_space_translate(
        as_0: *mut AddressSpace,
        addr: hwaddr,
        xlat: *mut hwaddr,
        len: *mut hwaddr,
        is_write: bool,
        attrs: MemTxAttrs,
    ) -> *mut MemoryRegion;
    #[link_name = "shim_address_space_read"]
    pub fn address_space_read(
        as_0: *const AddressSpace,
        addr: hwaddr,
        attrs: MemTxAttrs,
        buf: *mut ::core::ffi::c_void,
        len: hwaddr,
    ) -> MemTxResult;
    pub static mut address_space_memory: AddressSpace;
    pub fn qemu_bh_schedule(bh: *mut QEMUBH);
    pub fn bql_locked() -> bool;
    pub fn bql_lock_impl(file: *const ::core::ffi::c_char, line: ::core::ffi::c_int);
    pub fn bql_unlock();
    pub fn qemu_bh_new_full(
        cb: Option<QEMUBHFunc>,
        opaque: *mut ::core::ffi::c_void,
        name: *const ::core::ffi::c_char,
        reentrancy_guard: *mut MemReentrancyGuard,
    ) -> *mut QEMUBH;
    pub fn error_setg_internal(
        errp: *mut *mut Error,
        src: *const ::core::ffi::c_char,
        line: ::core::ffi::c_int,
        func: *const ::core::ffi::c_char,
        fmt: *const ::core::ffi::c_char,
        ...
    );
    pub static mut error_abort: *mut Error;
    pub static mut error_fatal: *mut Error;
    #[link_name = "shim_DEVICE"]
    pub fn DEVICE(obj: *const ::core::ffi::c_void) -> *mut DeviceState;
    #[link_name = "shim_DEVICE_CLASS"]
    pub fn DEVICE_CLASS(klass: *const ::core::ffi::c_void) -> *mut DeviceClass;
    #[link_name = "shim_BUS"]
    pub fn BUS(obj: *const ::core::ffi::c_void) -> *mut BusState;
    pub fn qdev_new(name: *const ::core::ffi::c_char) -> *mut DeviceState;
    pub fn qdev_realize_and_unref(
        dev: *mut DeviceState,
        bus: *mut BusState,
        errp: *mut *mut Error,
    ) -> bool;
    pub fn device_class_set_props_n(dc: *mut DeviceClass, props: *const Property, n: size_t);
    pub fn device_class_set_legacy_reset(dc: *mut DeviceClass, dev_reset: DeviceReset);
    pub fn pci_register_bar(
        pci_dev: *mut PCIDevice,
        region_num: ::core::ffi::c_int,
        attr: uint8_t,
        memory: *mut MemoryRegion,
    );
    pub fn pci_add_capability(
        pdev: *mut PCIDevice,
        cap_id: uint8_t,
        offset: uint8_t,
        size: uint8_t,
        errp: *mut *mut Error,
    ) -> ::core::ffi::c_int;
    pub fn pci_default_read_config(
        d: *mut PCIDevice,
        address: uint32_t,
        len: ::core::ffi::c_int,
    ) -> uint32_t;
    pub fn pci_default_write_config(
        d: *mut PCIDevice,
        address: uint32_t,
        val: uint32_t,
        len: ::core::ffi::c_int,
    );
    pub fn pci_address_space(dev: *mut PCIDevice) -> *mut MemoryRegion;
    pub fn pci_address_space_io(dev: *mut PCIDevice) -> *mut MemoryRegion;
    #[link_name = "shim_pci_set_byte"]
    pub fn pci_set_byte(config: *mut uint8_t, val: uint8_t);
    #[link_name = "shim_pci_set_word"]
    pub fn pci_set_word(config: *mut uint8_t, val: uint16_t);
    #[link_name = "shim_pci_set_long"]
    pub fn pci_set_long(config: *mut uint8_t, val: uint32_t);
    pub fn pci_set_irq(pci_dev: *mut PCIDevice, level: ::core::ffi::c_int);
    #[link_name = "shim_PCI_DEVICE_CLASS"]
    pub fn PCI_DEVICE_CLASS(klass: *const ::core::ffi::c_void) -> *mut PCIDeviceClass;
    #[link_name = "shim_pci_get_address_space"]
    pub fn pci_get_address_space(dev: *mut PCIDevice) -> *mut AddressSpace;
    pub static vmstate_pci_device: VMStateDescription;
    pub static qdev_prop_bool: PropertyInfo;
    pub static qdev_prop_uint32: PropertyInfo;
    pub static qdev_prop_string: PropertyInfo;
    pub static vmstate_info_bool: VMStateInfo;
    pub static vmstate_info_int32: VMStateInfo;
    pub static vmstate_info_uint32_equal: VMStateInfo;
    pub static vmstate_info_uint32: VMStateInfo;
    pub static vmstate_info_buffer: VMStateInfo;
    pub fn vmstate_register_ram(memory: *mut MemoryRegion, dev: *mut DeviceState);
    pub fn shim_current_cpu() -> *mut CPUState;
    pub fn cpu_memory_rw_debug(
        cpu: *mut CPUState,
        addr: vaddr,
        ptr: *mut ::core::ffi::c_void,
        len: size_t,
        is_write: bool,
    ) -> ::core::ffi::c_int;
    pub fn ppc_mac_gpu_scan16_le(sp: *const uint8_t, dp: *mut uint32_t, width: uint32_t, is565: bool);
    pub fn ppc_mac_gpu_line32_le(dst: *mut uint32_t, src: *const uint32_t, width: ::core::ffi::c_int);
    pub fn ppc_mac_gpu_fmt32_le() -> pixman_format_code_t;
    pub fn ppc_mac_gpu_fmt24_le() -> pixman_format_code_t;
    pub fn qemu_create_displaysurface_from(
        width: ::core::ffi::c_int,
        height: ::core::ffi::c_int,
        format: pixman_format_code_t,
        linesize: ::core::ffi::c_int,
        data: *mut uint8_t,
    ) -> *mut DisplaySurface;
    pub fn cursor_alloc(width: uint16_t, height: uint16_t) -> *mut QEMUCursor;
    pub fn cursor_unref(c: *mut QEMUCursor);
    pub fn qemu_console_update(
        con: *mut QemuConsole,
        x: ::core::ffi::c_int,
        y: ::core::ffi::c_int,
        w: ::core::ffi::c_int,
        h: ::core::ffi::c_int,
    );
    pub fn qemu_console_update_full(con: *mut QemuConsole);
    pub fn qemu_console_set_surface(con: *mut QemuConsole, surface: *mut DisplaySurface);
    pub fn qemu_console_set_mouse(
        con: *mut QemuConsole,
        x: ::core::ffi::c_int,
        y: ::core::ffi::c_int,
        on: bool,
    );
    pub fn qemu_console_set_cursor(con: *mut QemuConsole, cursor: *mut QEMUCursor);
    pub fn qemu_graphic_console_create(
        dev: *mut DeviceState,
        head: uint32_t,
        ops: *const GraphicHwOps,
        opaque: *mut ::core::ffi::c_void,
    ) -> *mut QemuConsole;
    pub fn qemu_graphic_console_close(con: *mut QemuConsole);
    pub fn vga_common_init(s: *mut VGACommonState, obj: *mut Object, errp: *mut *mut Error) -> bool;
    pub fn vga_init(
        s: *mut VGACommonState,
        obj: *mut Object,
        address_space: *mut MemoryRegion,
        address_space_io: *mut MemoryRegion,
        init_vga_ports: bool,
    );
    pub fn vga_common_reset(s: *mut VGACommonState);
    #[link_name = "shim_I2C_SLAVE"]
    pub fn I2C_SLAVE(obj: *const ::core::ffi::c_void) -> *mut I2CSlave;
    pub fn i2c_init_bus(parent: *mut DeviceState, name: *const ::core::ffi::c_char) -> *mut I2CBus;
    pub fn i2c_slave_set_address(dev: *mut I2CSlave, address: uint8_t);
    pub fn bitbang_i2c_init(s: *mut bitbang_i2c_interface, bus: *mut I2CBus);
    pub fn bitbang_i2c_set(
        i2c: *mut bitbang_i2c_interface,
        line: ::core::ffi::c_int,
        level: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    pub fn ppc_mac_gpu_renderer_vulkan() -> *mut PPCMacGPURenderer;
    pub fn ppc_mac_gpu_vulkan_alloc_vram(
        vram_size: uint64_t,
        opaque_out: *mut *mut ::core::ffi::c_void,
    ) -> *mut ::core::ffi::c_void;
    pub fn ppc_mac_gpu_vulkan_error() -> *const ::core::ffi::c_char;
    pub static mut graphic_width: ::core::ffi::c_int;
    pub static mut graphic_height: ::core::ffi::c_int;
    pub static mut graphic_depth: ::core::ffi::c_int;
    pub fn ioctl(__fd: ::core::ffi::c_int, __request: ::core::ffi::c_ulong, ...) -> ::core::ffi::c_int;
    pub static mut trace_events_enabled_count: ::core::ffi::c_int;
    pub static mut _TRACE_PPC_MAC_GPU_MMIO_READ_DSTATE: uint8_t;
    #[link_name = "shim_trace_ppc_mac_gpu_mmio_read"]
    pub fn trace_ppc_mac_gpu_mmio_read(
        size: ::core::ffi::c_uint,
        addr: uint64_t,
        name: *const ::core::ffi::c_char,
        val: uint64_t,
    );
    #[link_name = "shim_trace_ppc_mac_gpu_mmio_write"]
    pub fn trace_ppc_mac_gpu_mmio_write(
        size: ::core::ffi::c_uint,
        addr: uint64_t,
        name: *const ::core::ffi::c_char,
        val: uint64_t,
    );
    #[link_name = "shim_trace_ppc_mac_gpu_mmio_read_unimpl"]
    pub fn trace_ppc_mac_gpu_mmio_read_unimpl(
        size: ::core::ffi::c_uint,
        addr: uint64_t,
        name: *const ::core::ffi::c_char,
    );
    #[link_name = "shim_trace_ppc_mac_gpu_mmio_write_unimpl"]
    pub fn trace_ppc_mac_gpu_mmio_write_unimpl(
        size: ::core::ffi::c_uint,
        addr: uint64_t,
        name: *const ::core::ffi::c_char,
        val: uint64_t,
    );
    #[link_name = "shim_trace_ppc_mac_gpu_irq"]
    pub fn trace_ppc_mac_gpu_irq(status: uint32_t, cntl: uint32_t, level: bool);
    #[link_name = "shim_trace_ppc_mac_gpu_mode_change"]
    pub fn trace_ppc_mac_gpu_mode_change(
        width: uint32_t,
        height: uint32_t,
        bpp: uint32_t,
        stride: uint32_t,
    );
    #[link_name = "shim_trace_ppc_mac_gpu_soft_reset"]
    pub fn trace_ppc_mac_gpu_soft_reset(val: uint64_t);
    #[link_name = "shim_trace_ppc_mac_gpu_realize"]
    pub fn trace_ppc_mac_gpu_realize(vram_mb: uint32_t, vram_bytes: uint64_t);
    #[link_name = "shim_trace_ppc_mac_gpu_cp_ring_setup"]
    pub fn trace_ppc_mac_gpu_cp_ring_setup(base: uint64_t, cntl: uint32_t);
    #[link_name = "shim_trace_ppc_mac_gpu_3d_reg_write"]
    pub fn trace_ppc_mac_gpu_3d_reg_write(addr: uint64_t, val: uint64_t);
    #[link_name = "shim_trace_ppc_mac_gpu_3d_cmd"]
    pub fn trace_ppc_mac_gpu_3d_cmd(opcode: uint32_t, body_dw: uint32_t);
    pub fn uninorth_get_agp_gart_base() -> hwaddr;
}
