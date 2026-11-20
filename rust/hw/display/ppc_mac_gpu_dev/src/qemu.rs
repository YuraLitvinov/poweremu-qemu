//! Safe layer over QEMU's C API.  `unsafe` for the device's C calls lives
//! here, each use with its justification.
use super::dev_ns::{memory_region_get_ram_ptr, r300_us_disasm, size_t, PPCMacGPUState, R300State};
use core::ffi::{c_char, CStr};

/// Handle to the card's VRAM (the RAM block behind `s.vram`).
///
/// The block is allocated in `realize` and lives as long as the device, so
/// the handle holds the host pointer and length rather than borrowing the
/// device state, which the draw and 2D code mutate at the same time.  Every
/// access is bounds-checked against `len`; accesses are unaligned-safe.
#[derive(Clone, Copy)]
pub struct Vram {
    ptr: *mut u8,
    len: usize,
}

impl Vram {
    pub fn of(s: &PPCMacGPUState) -> Vram {
        // SAFETY: `s.vram` is an initialised RAM MemoryRegion once realized;
        // the call only reads the region's host mapping.
        let ptr = unsafe { memory_region_get_ram_ptr(&s.vram) } as *mut u8;
        Vram { ptr, len: s.vram_size as usize }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Raw base pointer, for code not yet converted to the checked accessors.
    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.ptr
    }

    fn span(&self, off: u64, n: usize) -> Option<usize> {
        let off = usize::try_from(off).ok()?;
        (off.checked_add(n)? <= self.len).then_some(off)
    }

    pub fn get8(&self, off: u64) -> Option<u8> {
        let o = self.span(off, 1)?;
        // SAFETY: o + 1 <= len and ptr..ptr+len is the live RAM block.
        Some(unsafe { self.ptr.add(o).read() })
    }

    pub fn get16(&self, off: u64) -> Option<u16> {
        let o = self.span(off, 2)?;
        // SAFETY: o + 2 <= len, see get8.
        Some(u16::from_le(unsafe { self.ptr.add(o).cast::<u16>().read_unaligned() }))
    }

    pub fn get32(&self, off: u64) -> Option<u32> {
        let o = self.span(off, 4)?;
        // SAFETY: o + 4 <= len, see get8.
        Some(u32::from_le(unsafe { self.ptr.add(o).cast::<u32>().read_unaligned() }))
    }

    pub fn put8(&self, off: u64, v: u8) -> bool {
        let Some(o) = self.span(off, 1) else { return false };
        // SAFETY: o + 1 <= len, see get8.
        unsafe { self.ptr.add(o).write(v) };
        true
    }

    pub fn put16(&self, off: u64, v: u16) -> bool {
        let Some(o) = self.span(off, 2) else { return false };
        // SAFETY: o + 2 <= len, see get8.
        unsafe { self.ptr.add(o).cast::<u16>().write_unaligned(v.to_le()) };
        true
    }

    pub fn put32(&self, off: u64, v: u32) -> bool {
        let Some(o) = self.span(off, 4) else { return false };
        // SAFETY: o + 4 <= len, see get8.
        unsafe { self.ptr.add(o).cast::<u32>().write_unaligned(v.to_le()) };
        true
    }

    /// Copy `out.len()` bytes starting at `off` into `out`.
    pub fn read_bytes(&self, off: u64, out: &mut [u8]) -> bool {
        let Some(o) = self.span(off, out.len()) else { return false };
        // SAFETY: o + out.len() <= len; `out` is a distinct Rust buffer.
        unsafe { core::ptr::copy_nonoverlapping(self.ptr.add(o), out.as_mut_ptr(), out.len()) };
        true
    }

    /// Copy `data` into VRAM at `off`.
    pub fn write_bytes(&self, off: u64, data: &[u8]) -> bool {
        let Some(o) = self.span(off, data.len()) else { return false };
        // SAFETY: o + data.len() <= len; `data` is a distinct Rust buffer.
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(o), data.len()) };
        true
    }

    /// memmove within VRAM (ranges may overlap).
    pub fn copy_within(&self, src: u64, dst: u64, n: usize) -> bool {
        let (Some(s), Some(d)) = (self.span(src, n), self.span(dst, n)) else { return false };
        // SAFETY: both ranges are inside the block; `copy` handles overlap.
        unsafe { core::ptr::copy(self.ptr.add(s), self.ptr.add(d), n) };
        true
    }
}

/// A C string argument of a log message, as text.  Callers pass either a
/// string literal or a pointer from the device's static name tables, both
/// NUL-terminated and alive for the call; a null pointer prints "(null)".
pub fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "(null)".into();
    }
    // SAFETY: see above: NUL-terminated, valid for the duration of the call.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// `getenv` with a checked name.  The result points into the process
/// environment and is only valid until the environment changes; callers read
/// it immediately (it is never stored).
pub fn getenv(name: &CStr) -> *const c_char {
    // SAFETY: `name` is a valid NUL-terminated string.
    unsafe { super::dev_ns::getenv(name.as_ptr()) }
}

/// True if the environment variable is set (to anything).
pub fn env_flag(name: &CStr) -> bool {
    !getenv(name).is_null()
}

/// First byte of the environment variable's value, if it is set and non-empty.
pub fn env_first_byte(name: &CStr) -> Option<u8> {
    let p = getenv(name);
    if p.is_null() {
        return None;
    }
    // SAFETY: getenv returned a non-null NUL-terminated string; reading the first byte is
    // always in bounds (an empty string yields the NUL, mapped to None below).
    let b = unsafe { *p } as u8;
    (b != 0).then_some(b)
}

/// A static C string as `&str` for log formatting (names here are all ASCII).
pub fn cs(c: &CStr) -> &str {
    c.to_str().unwrap_or("?")
}

/// glib's monotonic clock in microseconds.
pub fn monotonic_us() -> i64 {
    // SAFETY: g_get_monotonic_time has no preconditions.
    unsafe { super::dev_ns::g_get_monotonic_time() as i64 }
}

/// A set of strings for "log this once" bookkeeping.
pub struct SeenSet(std::sync::Mutex<Option<std::collections::HashSet<String>>>);

impl SeenSet {
    pub const fn new() -> Self {
        SeenSet(std::sync::Mutex::new(None))
    }

    /// Adds `key`; returns true if it was not already present.
    pub fn insert(&self, key: String) -> bool {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(Default::default).insert(key)
    }
}

/// Appends `text` to the file at `path`, creating it if needed; errors are ignored (this is
/// only used for best-effort diagnostics).
pub fn append_file(path: &str, text: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(text.as_bytes());
    }
}

/// The value of an environment variable, if set.
pub fn env_string(name: &CStr) -> Option<String> {
    let p = getenv(name);
    (!p.is_null()).then(|| cstr(p))
}

/// QEMU's virtual-clock time in nanoseconds.
pub fn clock_virtual_ns() -> i64 {
    // SAFETY: qemu_clock_get_ns has no preconditions for the built-in clock types.
    unsafe { super::dev_ns::qemu_clock_get_ns(super::dev_ns::QEMU_CLOCK_VIRTUAL) }
}

/// QEMU's real-time clock in nanoseconds.
pub fn clock_realtime_ns() -> i64 {
    // SAFETY: as above.
    unsafe { super::dev_ns::qemu_clock_get_ns(super::dev_ns::QEMU_CLOCK_REALTIME) }
}

// QOM requires the parent object at offset 0 of the instance, and the instance_size registered with the
// TypeInfo is size_of::<PPCMacGPUState>(); PCIDevice itself is pinned to the C layout in the `pci` crate.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    type S = super::dev_ns::PPCMacGPUState;
    assert!(offset_of!(S, pci) == 0);
    assert!(size_of::<super::dev_ns::PCIDevice>() == 2768);
    assert!(size_of::<S>() >= size_of::<super::dev_ns::PCIDevice>());
    assert!(align_of::<S>() >= align_of::<super::dev_ns::PCIDevice>());
};

// Layout of QEMU's MemTxAttrs (include/exec/memattrs.h): a 32-bit bitfield word, then `unspecified` (bool),
// `_reserved1` (u8) and `_reserved2` (u16).  The device passes it by value to address_space_*, so the
// bindgen type from QEMU's `system` crate must match this exactly.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    type A = super::dev_ns::MemTxAttrs;
    assert!(size_of::<A>() == 8);
    assert!(align_of::<A>() == 4);
    assert!(offset_of!(A, unspecified) == 4);
};

fn unspecified_attrs() -> super::dev_ns::MemTxAttrs {
    system::MEMTXATTRS_UNSPECIFIED
}

/// Reads guest physical memory into `buf`; returns QEMU's MemTxResult (0 = OK).
pub fn guest_read(addr: u64, buf: &mut [u8]) -> super::dev_ns::MemTxResult {
    // SAFETY: address_space_memory is QEMU's global system address space; `buf` is a live
    // mutable slice of exactly the length passed.
    unsafe {
        super::dev_ns::address_space_read(
            &raw mut super::dev_ns::address_space_memory,
            addr,
            unspecified_attrs(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            buf.len() as _,
        )
    }
}

/// Writes `buf` to guest physical memory; returns QEMU's MemTxResult (0 = OK).
pub fn guest_write(addr: u64, buf: &[u8]) -> super::dev_ns::MemTxResult {
    // SAFETY: as guest_read; `buf` is a live slice of exactly the length passed.
    unsafe {
        super::dev_ns::address_space_write(
            &raw mut super::dev_ns::address_space_memory,
            addr,
            unspecified_attrs(),
            buf.as_ptr() as *const core::ffi::c_void,
            buf.len() as _,
        )
    }
}

/// Reads one native-endian dword from guest memory into `out`.
pub fn guest_read32(addr: u64, out: &mut u32) -> super::dev_ns::MemTxResult {
    let mut b = [0u8; 4];
    let r = guest_read(addr, &mut b);
    if r == 0 {
        *out = u32::from_ne_bytes(b);
    }
    r
}

/// Writes one native-endian dword to guest memory.
pub fn guest_write32(addr: u64, v: u32) -> super::dev_ns::MemTxResult {
    guest_write(addr, &v.to_ne_bytes())
}

/// Reads `out.len()` native-endian dwords from guest memory.
pub fn guest_read_dwords(addr: u64, out: &mut [u32]) -> super::dev_ns::MemTxResult {
    let mut b = vec![0u8; out.len() * 4];
    let r = guest_read(addr, &mut b);
    if r == 0 {
        for (o, c) in out.iter_mut().zip(b.chunks_exact(4)) {
            *o = u32::from_ne_bytes([c[0], c[1], c[2], c[3]]);
        }
    }
    r
}

/// A zeroed byte buffer of `n` bytes, or None if the allocation fails (like g_try_malloc).
pub fn try_alloc_bytes(n: usize) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).ok()?;
    v.resize(n, 0);
    Some(v)
}

/// Parses an unsigned integer the way C `strtoull(s, NULL, 0)` does: optional `0x` hex or
/// leading-`0` octal prefix, otherwise decimal; stops at the first invalid character and
/// returns 0 if there are no digits.
pub fn parse_c_uint(s: &str) -> u64 {
    let s = s.trim_start();
    let s = s.strip_prefix('+').unwrap_or(s);
    let (radix, digits) = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (16, h)
    } else if s.len() > 1 && s.starts_with('0') {
        (8, &s[1..])
    } else {
        (10, s)
    };
    let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
    u64::from_str_radix(&digits[..end], radix).unwrap_or(0)
}

/// Holds QEMU's big lock for its lifetime, unless the caller already held it.
pub struct BqlGuard {
    took_lock: bool,
}

impl BqlGuard {
    pub fn acquire() -> Self {
        // SAFETY: bql_locked/bql_lock_impl are QEMU's own lock API, callable from any thread.
        let took_lock = unsafe { !super::dev_ns::bql_locked() };
        if took_lock {
            unsafe { super::dev_ns::bql_lock_impl(c"ppc_mac_gpu_dev".as_ptr(), line!() as _) };
        }
        BqlGuard { took_lock }
    }
}

impl Drop for BqlGuard {
    fn drop(&mut self) {
        if self.took_lock {
            // SAFETY: this guard took the lock and is the only one releasing it.
            unsafe { super::dev_ns::bql_unlock() };
        }
    }
}

// BEGIN run_static_initializers
/// Publishes the device's property table and vmstate description. Only the
/// `.init_array` constructor calls this, before QEMU's main() starts, so the
/// two statics are written once while no other thread can observe them.
pub fn install_ppc_mac_gpu_tables(
    props: [super::dev_ns::Property; 5],
    vmsd: super::dev_ns::VMStateDescription,
) {
    // SAFETY: single writer in the constructor phase (see above); the statics
    // are only read after this returns.
    unsafe {
        super::dev_ns::ppc_mac_gpu_properties = props;
        super::dev_ns::vmstate_ppc_mac_gpu = vmsd;
    }
}
// END run_static_initializers

// BEGIN mmio_read_impl
/// QEMU's real-time clock in milliseconds (qemu_clock_get_ms).
pub fn clock_realtime_ms() -> i64 {
    clock_realtime_ns() / 1_000_000
}

/// True if any log category in `mask` is enabled (qemu_loglevel_mask).
pub fn loglevel_enabled(mask: core::ffi::c_int) -> bool {
    // SAFETY: a plain query of QEMU's log mask; no preconditions.
    unsafe { super::dev_ns::qemu_loglevel_mask(mask) }
}

/// True if the mmio-read trace event is compiled in and enabled.
pub fn trace_mmio_read_enabled() -> bool {
    // SAFETY: reads QEMU's trace-state globals, which are only written at init.
    unsafe {
        super::dev_ns::trace_events_enabled_count != 0
            && super::dev_ns::_TRACE_PPC_MAC_GPU_MMIO_READ_DSTATE != 0
    }
}

/// Emits the mmio-read trace event.  Callers check `trace_mmio_read_enabled` first.
pub fn trace_mmio_read(size: u32, addr: u64, name: &CStr, val: u64) {
    // SAFETY: `name` is a valid NUL-terminated string that lives for the call.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_mmio_read(size, addr, name.as_ptr(), val) }
}
// END mmio_read_impl
// BEGIN r300_aa_resolve
/// `r300_aa_samples` of the R300 register block: the colour buffer's sample count.
pub fn aa_samples(st: &super::dev_ns::R300State) -> u32 {
    // SAFETY: `st` is a live R300State; the C helper only reads it.
    unsafe { super::dev_ns::r300_aa_samples(st as *const super::dev_ns::R300State) }
}

/// Calls the renderer's `flush_sync` callback if it registered one.  The caller holds the
/// big lock (see `BqlGuard`).
pub fn renderer_flush_sync(s: &super::dev_ns::PPCMacGPUState) {
    if s.renderer.is_null() {
        return;
    }
    if let Some(flush) = s.renderer_ref().flush_sync {
        // SAFETY: the renderer table is static and set at realize; `flush` is the callback it
        // registered and is called with the device's opaque pointer, as before.
        unsafe { flush(s.renderer_opaque) };
    }
}
// END r300_aa_resolve
// BEGIN w1_r200
/// Calls the renderer's `fill_notify` callback if one is registered.  A device without a
/// renderer table does nothing.  The caller holds the big lock (see `BqlGuard`).
#[allow(clippy::too_many_arguments)]
pub fn renderer_fill_notify(
    s: &super::dev_ns::PPCMacGPUState,
    offset: u32,
    pitch: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    bpp: u32,
    value: u32,
) {
    if s.renderer.is_null() {
        return;
    }
    if let Some(notify) = s.renderer_ref().fill_notify {
        // SAFETY: the renderer table is static and set at realize; `notify` is the callback it
        // registered and is called with the device's opaque pointer, as before.
        unsafe { notify(s.renderer_opaque, offset, pitch, x, y, w, h, bpp, value) };
    }
}
// END w1_r200
// BEGIN w1_ppc1
/// The AGP GART table base the northbridge reports (uninorth_get_agp_gart_base).
pub fn agp_gart_base() -> super::dev_ns::hwaddr {
    // SAFETY: a plain query of the northbridge model's state; no arguments, no preconditions.
    unsafe { super::dev_ns::uninorth_get_agp_gart_base() }
}

// END w1_ppc1
// BEGIN w1_mc
// END w1_mc
// BEGIN w1_ppc3
/// True while the timer is armed (timer_pending).
pub fn timer_is_pending(t: &super::dev_ns::QEMUTimer) -> bool {
    // SAFETY: `t` is a live QEMUTimer; timer_pending only reads its state.
    unsafe { super::dev_ns::timer_pending(t) }
}

/// Reads `len` bytes of PCI config space at `address` (pci_default_read_config).
pub fn pci_config_read(dev: &mut super::dev_ns::PCIDevice, address: u32, len: core::ffi::c_int) -> u32 {
    // SAFETY: `dev` is a live PCIDevice borrowed exclusively for the call; the default config
    // read touches only its config space at the given address and length.
    unsafe { super::dev_ns::pci_default_read_config(dev, address, len) }
}
// END w1_ppc3
// BEGIN w5_ppc3
/// Emits the unimplemented-register read trace event for `name`, the register's log name.
pub fn trace_mmio_read_unimpl(size: u32, addr: u64, name: &CStr) {
    // SAFETY: `name` is a valid NUL-terminated string that lives for the call.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_mmio_read_unimpl(size, addr, name.as_ptr()) }
}
// END w5_ppc3
// BEGIN w1_ringdump
/// A `FILE *` opened by `fopen` for the R300 ring dump.  Only `cfile_open_write` makes one,
/// so the pointer is never null and is closed at most once (by `close`, which consumes it).
pub struct CFile(core::ptr::NonNull<super::dev_ns::FILE>);

// SAFETY: the handle is only touched while the caller holds its module-level mutex.
unsafe impl Send for CFile {}

/// Opens `path` for writing (truncating), like `fopen(path, "w")`.  `None` on failure.
pub fn cfile_open_write(path: &CStr) -> Option<CFile> {
    // SAFETY: `path` is NUL-terminated and lives for the call; the mode literal is static.
    let p = unsafe { super::dev_ns::fopen(path.as_ptr(), c"w".as_ptr()) };
    core::ptr::NonNull::new(p).map(CFile)
}

impl CFile {
    /// The raw `FILE *` for C-side writers (`fprintf`, `fputc`).  Valid while `self` lives.
    pub fn as_ptr(&self) -> *mut super::dev_ns::FILE {
        self.0.as_ptr()
    }

    /// Current stream position in bytes, as `ftell`.
    pub fn tell(&self) -> core::ffi::c_long {
        // SAFETY: the handle came from `fopen` and has not been closed (`close` consumes it).
        unsafe { super::dev_ns::ftell(self.0.as_ptr()) }
    }

    /// Closes the stream, as `fclose`.
    pub fn close(self) {
        // SAFETY: the handle came from `fopen`, is not used again after this call, and the
        // consumed `self` guarantees it is closed exactly once.
        unsafe {
            super::dev_ns::fclose(self.0.as_ptr());
        }
    }
}
// END w1_ringdump
// BEGIN w1_gpu
/// The register file of the current vCPU via `KVM_GET_REGS`, or None if there is no current
/// CPU or the ioctl fails.
pub fn cur_cpu_kvm_regs() -> Option<super::dev_ns::kvm_regs> {
    // _IOR(KVMIO = 0xae, 0x81, struct kvm_regs)
    const KVM_GET_REGS: core::ffi::c_ulong = (2 << 30)
        | ((core::mem::size_of::<super::dev_ns::kvm_regs>() as core::ffi::c_ulong) << 16)
        | (0xae << 8)
        | 0x81;
    let mut kr = super::dev_ns::kvm_regs {
        rax: 0,
        rbx: 0,
        rcx: 0,
        rdx: 0,
        rsi: 0,
        rdi: 0,
        rsp: 0,
        rbp: 0,
        r8: 0,
        r9: 0,
        r10: 0,
        r11: 0,
        r12: 0,
        r13: 0,
        r14: 0,
        r15: 0,
        rip: 0,
        rflags: 0,
    };
    // SAFETY: shim_current_cpu returns the running vCPU or null, and the null case is checked
    // before its kvm_fd is read. KVM_GET_REGS writes exactly one kvm_regs, and `kr` is a live
    // local of that size.
    unsafe {
        let cpu = super::dev_ns::shim_current_cpu();
        if cpu.is_null() {
            return None;
        }
        let rc = super::dev_ns::ioctl((*cpu).kvm_fd, KVM_GET_REGS, &raw mut kr);
        (rc == 0).then_some(kr)
    }
}
/// Debug-reads guest memory through the current vCPU's MMU into `buf`. Does nothing if there
/// is no current CPU; a failed read leaves the bytes it could not fetch untouched.
pub fn cur_cpu_read_mem(addr: u64, buf: &mut [u8]) {
    // SAFETY: the cpu pointer is checked for null; cpu_memory_rw_debug writes at most
    // buf.len() bytes, into `buf`, a live mutable slice of exactly that length.
    unsafe {
        let cpu = super::dev_ns::shim_current_cpu();
        if !cpu.is_null() {
            super::dev_ns::cpu_memory_rw_debug(
                cpu,
                addr as super::dev_ns::vaddr,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                buf.len() as super::dev_ns::size_t,
                false,
            );
        }
    }
}
// END w1_gpu
// BEGIN w2_mc
/// The name of the call site tagged for VRAM watch logging (`mc_watch_fn`), as text.
pub fn mc_watch_name() -> String {
    // SAFETY: mc_watch_fn is only assigned NUL-terminated string literals (`c"...".as_ptr()`),
    // which live for the whole program; it is read on the device's single thread under the BQL.
    let p = unsafe { super::dev_ns::mc_watch_fn };
    cstr(p)
}
// END w2_mc
// BEGIN w2_gpu
/// Host pointer to the RAM byte backing guest-physical `phys` (the page-aligned base is
/// translated, as the AGP path always did). `None` when the address is not plain RAM or the
/// translated run is shorter than one page. The RCU read section opened here ends before
/// return, exactly as the old function-scope guard did.
pub fn guest_ram_page_host(phys: super::dev_ns::hwaddr) -> Option<*mut u8> {
    // SAFETY: address_space_memory is QEMU's global system address space and translate accepts
    // any guest address. The returned region is null-checked and tested with
    // memory_region_is_ram before its RAM pointer is read; the offset is applied with
    // wrapping_add, so no out-of-bounds pointer arithmetic happens in this call.
    unsafe {
        let _rcu = super::dev_ns::rcu_read_auto_lock();
        let mut xlat: super::dev_ns::hwaddr = 0;
        let mut plen: super::dev_ns::hwaddr = 0x1000;
        let mr = super::dev_ns::address_space_translate(
            &raw mut super::dev_ns::address_space_memory,
            phys & !0xfff,
            &raw mut xlat,
            &raw mut plen,
            false,
            unspecified_attrs(),
        );
        if mr.is_null() || !super::dev_ns::memory_region_is_ram(mr) || plen < 0x1000 {
            return None;
        }
        let base = super::dev_ns::memory_region_get_ram_ptr(mr) as *mut u8;
        Some(base.wrapping_add(xlat as usize))
    }
}
// END w2_gpu
// BEGIN w3_ppc3
/// Recovers the device state from the opaque pointer QEMU hands to a callback registered with
/// the device (the ring bottom half).  Returns `None` for a null pointer.
/// # Safety
/// `opaque` must be null or point to the live device state, with no other live reference to it.
pub unsafe fn state_of_opaque<'a>(opaque: *mut core::ffi::c_void) -> Option<&'a mut PPCMacGPUState> {
    if opaque.is_null() {
        return None;
    }
    // SAFETY: the only caller-visible pointer is the `PPCMacGPUState` passed as `opaque` when the
    // device registered the callback; QEMU keeps that device alive and unmoved for as long as the
    // bottom half can run, and the callback runs on the single QEMU main loop thread, so no other
    // live `&mut` to the state exists while the returned reference is used.
    Some(unsafe { &mut *(opaque as *mut PPCMacGPUState) })
}
/// Disarms the timer (timer_del).
pub fn timer_disarm(t: &mut super::dev_ns::QEMUTimer) {
    // SAFETY: as timer_arm.
    unsafe { super::dev_ns::timer_del(t) }
}
/// Emits the soft-reset trace event with the written value.
pub fn trace_soft_reset(val: u64) {
    // SAFETY: the trace stub takes a plain integer and has no preconditions.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_soft_reset(val) }
}
/// Sends an explicit mouse position and visibility to the console.
pub fn console_set_mouse_at(s: &super::dev_ns::PPCMacGPUState, x: i32, y: i32, visible: bool) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: as console_update_full.
    unsafe { super::dev_ns::qemu_console_set_mouse(s.con, x, y, visible) };
}
/// Sets the 1x1 placeholder cursor on the console; the console keeps its own reference.
pub fn console_set_blank_cursor(s: &super::dev_ns::PPCMacGPUState) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: cursor_alloc returns a new cursor that this call owns; the console takes its own
    // reference in qemu_console_set_cursor, and cursor_unref drops ours.  con is live.
    unsafe {
        let c = super::dev_ns::cursor_alloc(1, 1);
        super::dev_ns::qemu_console_set_cursor(s.con, c);
        super::dev_ns::cursor_unref(c);
    }
}
// END w3_ppc3
// BEGIN w4_r200
/// The fence-done counter as an atomic.  The field is written by the fence callback (run on
/// the main loop) and read by the device under the big lock; `from_mut` needs no unsafe code
/// because the exclusive borrow of the state already guarantees the access is unshared.
pub fn fence_done_atomic(s: &mut super::dev_ns::PPCMacGPUState) -> &std::sync::atomic::AtomicU32 {
    std::sync::atomic::AtomicU32::from_mut(&mut s.regs.r200_fence_done)
}

/// Creates the fence bottom half on first use, with the device state as its opaque pointer.
pub fn fence_bh_ensure(s: &mut super::dev_ns::PPCMacGPUState, cb: unsafe extern "C" fn(*mut core::ffi::c_void)) {
    let opaque = s as *mut super::dev_ns::PPCMacGPUState as *mut core::ffi::c_void;
    // SAFETY: `r200_fence_bh` is only touched on the device thread under the big lock; the bottom
    // half is created once, never freed, and `opaque` is the device state that owns it.  The name
    // is a static NUL-terminated string.
    unsafe {
        if (&raw const super::dev_ns::r200_fence_bh).read().is_null() {
            let bh = super::dev_ns::qemu_bh_new_full(
                Some(cb),
                opaque,
                c"r200_fence_bh_cb".as_ptr(),
                core::ptr::null_mut(),
            );
            (&raw mut super::dev_ns::r200_fence_bh).write(bh);
        }
    }
}

/// Schedules the fence bottom half if it has been created.
pub fn fence_bh_schedule() {
    // SAFETY: the static is only touched under the big lock.  A non-null value was stored by
    // `fence_bh_ensure` and the bottom half is never freed, so it is valid to schedule.
    unsafe {
        let bh = (&raw const super::dev_ns::r200_fence_bh).read();
        if !bh.is_null() {
            super::dev_ns::qemu_bh_schedule(bh);
        }
    }
}

/// Submits a fence through the renderer table and returns its sequence number, or `None` when
/// the renderer registered no fence support.  `done` is called with the device state as its
/// opaque pointer once the fence completes.
pub fn renderer_submit_fence(
    s: &mut super::dev_ns::PPCMacGPUState,
    done: unsafe extern "C" fn(*mut core::ffi::c_void, u32),
) -> Option<u32> {
    if s.renderer.is_null() {
        return None;
    }
    let submit = s.renderer_ref().submit_fence?;
    let opaque = s.renderer_opaque;
    let arg = s as *mut super::dev_ns::PPCMacGPUState as *mut core::ffi::c_void;
    // SAFETY: the renderer table is static and set at realize; `submit` is the callback it
    // registered and takes the device opaque pointer, as before.  `arg` is the device state,
    // which outlives the fence because QEMU keeps the device alive until its callbacks run.
    Some(unsafe { submit(opaque, Some(done), arg) })
}

/// Sleeps the calling thread for `us` microseconds (the device's fence wait poll).
pub fn sleep_us(us: u64) {
    std::thread::sleep(std::time::Duration::from_micros(us));
}

/// Yields the calling thread to the scheduler (the device's fence wait poll).
pub fn yield_now() {
    std::thread::yield_now();
}

/// `atoi`-style parse of an environment value: leading blanks, an optional sign, then decimal
/// digits; anything after the digits is ignored and a non-numeric value gives 0.  Overflow
/// saturates instead of being undefined.
pub fn parse_atoi(s: &str) -> i32 {
    let t = s.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
    let (neg, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut v: i64 = 0;
    for b in digits.bytes().take_while(|b| b.is_ascii_digit()) {
        v = (v * 10 + i64::from(b - b'0')).min(i64::from(i32::MAX) + 1);
    }
    if neg {
        v = -v;
    }
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
// END w4_r200
// BEGIN w4_ppc2
/// Tags the call site for VRAM watch logging: `name` becomes `mc_watch_fn` and `line` becomes
/// `mc_watch_line`.
pub fn mc_watch_tag(name: &'static CStr, line: core::ffi::c_int) {
    // SAFETY: mc_watch_fn only ever holds a pointer to a `&'static CStr` (NUL-terminated, alive for
    // the whole program), and it is read and written on the device's single thread under the BQL,
    // as in `mc_watch_name`.
    unsafe { super::dev_ns::mc_watch_fn = name.as_ptr() };
    super::dev_ns::mc_watch_line.set(line);
}
// END w4_ppc2
// BEGIN w4_gpu
/// True if `[p, p + len)` lies entirely inside VRAM, or entirely inside one page held in the
/// AGP translation cache (`r200_agp_tc`, whose host pointers are the guest RAM pages that
/// `gpu_agp_page` resolved). A null pointer, a zero length, or a range that crosses the end of
/// its block is rejected.
pub fn guest_px_ptr_valid(s: &super::dev_ns::PPCMacGPUState, p: *mut u8, len: usize) -> bool {
    if p.is_null() || len == 0 {
        return false;
    }
    let start = p as usize;
    let Some(end) = start.checked_add(len) else {
        return false;
    };
    let vram = Vram::of(s);
    let vbase = vram.as_mut_ptr() as usize;
    if vbase != 0 {
        if let Some(vend) = vbase.checked_add(vram.len()) {
            if start >= vbase && end <= vend {
                return true;
            }
        }
    }
    // Not in VRAM: the pointer can only be a guest-RAM page that `gpu_px_ptr` took from
    // `gpu_agp_page` plus an in-page offset.  The access must stay inside that 4 KiB page
    // (host pages are 4 KiB aligned), which is O(1); this is what bounds the access by default.
    if (start & 0xfff) + len > 0x1000 {
        return false;
    }
    // Debug aid, off by default (PPCGPU_AGP_CHECK=1): additionally prove the page is one the
    // AGP translation cache really holds.  It scans the whole 1024-entry table per access.
    if !agp_check_enabled() {
        return true;
    }
    super::dev_ns::r200_agp_tc_mut().iter().any(|e| {
        let h = e.host as usize;
        !e.host.is_null()
            && h.checked_add(0x1000)
                .is_some_and(|hend| start >= h && end <= hend)
    })
}

/// True when `PPCGPU_AGP_CHECK` is set: enables the slow AGP-cache membership check in
/// `guest_px_ptr_valid`.  Read once.
fn agp_check_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| env_flag(c"PPCGPU_AGP_CHECK"))
}

/// Access width for a pixel of `bytes` bytes: 4 and 2 are dword and word, anything else is
/// one byte (the same split the device's pixel accessors have always made).
fn px_width(bytes: usize) -> usize {
    match bytes {
        4 => 4,
        2 => 2,
        _ => 1,
    }
}

/// Little-endian read of a pixel at `p` (see `px_width`). Returns 0 if the range is not a
/// valid guest pixel range (see `guest_px_ptr_valid`).
pub fn px_read(s: &super::dev_ns::PPCMacGPUState, p: *mut u8, bytes: usize) -> u32 {
    let n = px_width(bytes);
    if !guest_px_ptr_valid(s, p, n) {
        return 0;
    }
    // SAFETY: guest_px_ptr_valid checked that [p, p + n) is non-null and lies inside VRAM or
    // one live AGP page, so every unaligned read below stays in bounds of that block.
    unsafe {
        match n {
            4 => u32::from_le(p.cast::<u32>().read_unaligned()),
            2 => u32::from(u16::from_le(p.cast::<u16>().read_unaligned())),
            _ => u32::from(p.read()),
        }
    }
}

/// Little-endian write of the low `bytes`-width part of `v` at `p` (see `px_width`). Does
/// nothing if the range is not a valid guest pixel range (see `guest_px_ptr_valid`).
pub fn px_write(s: &super::dev_ns::PPCMacGPUState, p: *mut u8, bytes: usize, v: u32) {
    let n = px_width(bytes);
    if !guest_px_ptr_valid(s, p, n) {
        return;
    }
    // SAFETY: guest_px_ptr_valid checked that [p, p + n) is non-null and lies inside VRAM or
    // one live AGP page, so every unaligned write below stays in bounds of that block.
    unsafe {
        match n {
            4 => p.cast::<u32>().write_unaligned(v.to_le()),
            2 => p.cast::<u16>().write_unaligned((v as u16).to_le()),
            _ => p.write(v as u8),
        }
    }
}

/// memmove of `len` bytes from `src` to `dst`, both pixel ranges (see `guest_px_ptr_valid`).
/// Does nothing unless both ranges are valid; overlapping ranges are handled.
pub fn px_move(s: &super::dev_ns::PPCMacGPUState, dst: *mut u8, src: *mut u8, len: usize) {
    if !guest_px_ptr_valid(s, dst, len) || !guest_px_ptr_valid(s, src, len) {
        return;
    }
    // SAFETY: guest_px_ptr_valid checked that both [dst, dst + len) and [src, src + len) are
    // non-null and lie inside VRAM or one live AGP page each; `copy` permits overlap.
    unsafe { core::ptr::copy(src, dst, len) };
}
// END w4_gpu
// BEGIN w6_display
/// Drag state the renderer reports: drag origin, size and generation counter.
pub struct DragState {
    pub ox: u32,
    pub oy: u32,
    pub w: u32,
    pub h: u32,
    pub generation: u32,
}

/// Pixman format of a 32-bit little-endian guest scanout (ppc_mac_gpu_fmt32_le).
pub fn guest_fmt32() -> super::dev_ns::pixman_format_code_t {
    // SAFETY: a plain query of a static format code; no preconditions.
    unsafe { super::dev_ns::ppc_mac_gpu_fmt32_le() }
}

/// Pixman format of a 24-bit little-endian guest scanout (ppc_mac_gpu_fmt24_le).
pub fn guest_fmt24() -> super::dev_ns::pixman_format_code_t {
    // SAFETY: as guest_fmt32.
    unsafe { super::dev_ns::ppc_mac_gpu_fmt24_le() }
}

/// Makes the device's shadow framebuffer at least `need` bytes.  A block that is missing or too
/// small is freed and replaced; returns true when a new block was allocated.  `need` must fit
/// the 32-bit `shadow_buf_size` field (callers bound it).
pub fn shadow_ensure(s: &mut super::dev_ns::PPCMacGPUState, need: u64) -> bool {
    if !s.shadow_buf.is_null() && u64::from(s.shadow_buf_size) >= need {
        return false;
    }
    debug_assert!(need <= u64::from(u32::MAX));
    // SAFETY: the old block (null is fine for g_free) came from g_malloc and is replaced
    // right here, so it is freed exactly once; g_malloc aborts rather than returning null.
    unsafe {
        super::dev_ns::g_free(s.shadow_buf as super::dev_ns::gpointer);
        s.shadow_buf = super::dev_ns::g_malloc(need as super::dev_ns::gsize) as *mut u8;
    }
    s.shadow_buf_size = need as u32;
    true
}

/// The shadow framebuffer as bytes: `shadow_buf_size` bytes from `shadow_buf`, or an empty slice
/// if none is allocated.  The slice borrows the device mutably, so nothing else reads it meanwhile.
pub fn shadow_bytes(s: &mut super::dev_ns::PPCMacGPUState) -> &mut [u8] {
    if s.shadow_buf.is_null() {
        return &mut [];
    }
    let len = s.shadow_buf_size as usize;
    // SAFETY: shadow_buf is a g_malloc block of at least shadow_buf_size bytes (shadow_ensure keeps
    // that invariant), and the returned slice holds the only live borrow of it.
    unsafe { core::slice::from_raw_parts_mut(s.shadow_buf, len) }
}

/// Creates a display surface over the shadow framebuffer and installs it on the console.
pub fn console_set_shadow_surface(
    s: &super::dev_ns::PPCMacGPUState,
    width: u32,
    height: u32,
    format: super::dev_ns::pixman_format_code_t,
    linesize: u32,
) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: con is the console created at realize and lives as long as the device.  The surface
    // reads the shadow block, which is replaced only by shadow_ensure, and the device then installs
    // a new surface before the next update.
    unsafe {
        let ds = super::dev_ns::qemu_create_displaysurface_from(
            width as core::ffi::c_int,
            height as core::ffi::c_int,
            format,
            linesize as core::ffi::c_int,
            s.shadow_buf,
        );
        super::dev_ns::qemu_console_set_surface(s.con, ds);
    }
}

/// Creates a display surface over VRAM starting at byte `offset` and installs it on the console.
/// Does nothing if `offset` lies beyond the VRAM block.
pub fn console_set_vram_surface(
    s: &super::dev_ns::PPCMacGPUState,
    width: u32,
    height: u32,
    format: super::dev_ns::pixman_format_code_t,
    linesize: u32,
    offset: u64,
) {
    let vram = Vram::of(s);
    if s.con.is_null() || offset > vram.len() as u64 {
        return;
    }
    let data = vram.as_mut_ptr().wrapping_add(offset as usize);
    // SAFETY: con as in console_set_shadow_surface.  `data` is inside the VRAM block (offset <= len,
    // so one-past-the-end is allowed), and VRAM lives as long as the device.
    unsafe {
        let ds = super::dev_ns::qemu_create_displaysurface_from(
            width as core::ffi::c_int,
            height as core::ffi::c_int,
            format,
            linesize as core::ffi::c_int,
            data,
        );
        super::dev_ns::qemu_console_set_surface(s.con, ds);
    }
}

/// Marks the whole console surface for redraw.
pub fn console_update_full(s: &super::dev_ns::PPCMacGPUState) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: con is the live console of this device (see console_set_shadow_surface).
    unsafe { super::dev_ns::qemu_console_update_full(s.con) };
}

/// Marks a rectangle of the console surface for redraw.
pub fn console_update_rect(s: &super::dev_ns::PPCMacGPUState, x: i32, y: i32, w: i32, h: i32) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: as console_update_full.
    unsafe { super::dev_ns::qemu_console_update(s.con, x, y, w, h) };
}

/// Sends the hardware mouse position and visibility to the console.
pub fn console_set_mouse(s: &super::dev_ns::PPCMacGPUState) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: as console_update_full.
    unsafe { super::dev_ns::qemu_console_set_mouse(s.con, s.hwc_x, s.hwc_y, s.hwc_visible) };
}

/// Publishes the hardware cursor image (`hwc_w` x `hwc_h` ARGB pixels from `hwc_pix`) to the
/// console.  An image larger than the `hwc_pix` buffer is not published.
pub fn cursor_publish_hwc(s: &super::dev_ns::PPCMacGPUState) {
    let (w, h) = (s.hwc_w, s.hwc_h);
    let Some(n) = w.checked_mul(h).filter(|&n| n as usize <= s.hwc_pix.len()) else {
        return;
    };
    if s.con.is_null() {
        return;
    }
    // SAFETY: cursor_alloc returns a new cursor with room for w*h pixels in its flexible `data`
    // array; the copy writes exactly n*4 bytes (n <= hwc_pix.len()) into it and reads n*4 bytes
    // of hwc_pix.  The cursor is handed to the console, which takes its own reference, and then
    // released here.
    unsafe {
        let c = super::dev_ns::cursor_alloc(w as super::dev_ns::uint16_t, h as super::dev_ns::uint16_t);
        if c.is_null() {
            return;
        }
        (*c).hot_x = 0;
        (*c).hot_y = 0;
        core::ptr::copy_nonoverlapping(
            s.hwc_pix.as_ptr().cast::<u8>(),
            core::ptr::addr_of_mut!((*c).data).cast::<u8>(),
            n as usize * 4,
        );
        super::dev_ns::qemu_console_set_cursor(s.con, c);
        super::dev_ns::cursor_unref(c);
    }
}

/// Asks the renderer whether `[lo, hi)` is busy (pending GPU work).  False when no renderer or no
/// such callback is registered.
pub fn renderer_range_busy_sync(s: &super::dev_ns::PPCMacGPUState, lo: u64, hi: u64, write: bool) -> bool {
    if s.renderer.is_null() {
        return false;
    }
    let Some(f) = s.renderer_ref().range_busy_sync else {
        return false;
    };
    // SAFETY: the renderer table is static and set at realize; `f` is the callback it registered,
    // called with the device's opaque pointer as before.
    unsafe { f(s.renderer_opaque, lo, hi, write) }
}

/// The renderer's current drag state, if it has one.  None when no renderer or callback exists,
/// or when the callback reports no drag.
pub fn renderer_drag_state(s: &super::dev_ns::PPCMacGPUState) -> Option<DragState> {
    if s.renderer.is_null() {
        return None;
    }
    let f = s.renderer_ref().get_drag_state?;
    let (mut ox, mut oy, mut w, mut h, mut generation) = (0u32, 0u32, 0u32, 0u32, 0u32);
    // SAFETY: `f` is the renderer's callback, called with its paired opaque pointer; the five
    // out-parameters are live locals of the right type for the duration of the call.
    let ok = unsafe {
        f(
            s.renderer_opaque,
            &raw mut ox,
            &raw mut oy,
            &raw mut w,
            &raw mut h,
            &raw mut generation,
        )
    };
    ok.then_some(DragState { ox, oy, w, h, generation })
}

/// Lets the renderer paste the drag buffer into VRAM.  Does nothing without a renderer callback.
pub fn renderer_flush_drag_paste(s: &super::dev_ns::PPCMacGPUState, vram: Vram) {
    if s.renderer.is_null() {
        return;
    }
    let Some(f) = s.renderer_ref().flush_drag_paste else {
        return;
    };
    // SAFETY: as renderer_range_busy_sync; the renderer may read and write the VRAM block, which is
    // the device's live RAM for the duration of the call.
    unsafe { f(s.renderer_opaque, vram.as_mut_ptr()) };
}

/// Runs the VGA core's graphics update hook (the legacy VGA hand-off).
pub fn vga_gfx_update(s: &super::dev_ns::PPCMacGPUState) {
    if s.vga.is_null() {
        return;
    }
    // SAFETY: `vga` is the VGA core embedded in this device; its hw_ops table is static and set by
    // the VGA init, and the hook receives the same VGA pointer the core expects.
    unsafe {
        let ops = (*s.vga).hw_ops;
        if ops.is_null() {
            return;
        }
        if let Some(update) = (*ops).gfx_update {
            update(s.vga as *mut core::ffi::c_void);
        }
    }
}

/// Bumps the frame and row counters the device publishes for FPS readout (when registered).
pub fn fps_count(s: &super::dev_ns::PPCMacGPUState, rows: u32) {
    if s.fps_frames_ptr.is_null() || s.fps_rows_ptr.is_null() {
        return;
    }
    // SAFETY: both counters are host words registered for the FPS readout; they stay valid while the
    // device lives, and only the device thread writes them, under the big lock.
    unsafe {
        *s.fps_frames_ptr = (*s.fps_frames_ptr).wrapping_add(1);
        *s.fps_rows_ptr = (*s.fps_rows_ptr).wrapping_add(rows);
    }
}

/// Emits the mode-change trace event (the trace stub checks its own enable state).
pub fn trace_mode_change(width: u32, height: u32, bpp: u32, stride: u32) {
    // SAFETY: the trace stub takes only values and has no preconditions.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_mode_change(width, height, bpp, stride) };
}
// END w6_display
// BEGIN w6_pm4
/// Trace hook for one unhandled 3D command (`shim_trace_ppc_mac_gpu_3d_cmd`).
pub fn trace_3d_cmd(opcode: u32, body_dw: u32) {
    // SAFETY: the hook takes two plain integers and no pointers; it has no preconditions.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_3d_cmd(opcode, body_dw) };
}
// END w6_pm4
// BEGIN w6_trace
/// The NUL-terminated string QEMU passed to a property setter, or `None` for a null pointer.
pub fn c_arg<'a>(p: *const c_char) -> Option<&'a CStr> {
    if p.is_null() {
        return None;
    }
    // SAFETY: QEMU passes property-setter strings that are NUL-terminated and alive for the
    // whole setter call; the null case was handled above.
    Some(unsafe { CStr::from_ptr(p) })
}

/// Sets the environment variable `name` (bytes without NUL) to "1", or removes it when `on`
/// is false. Names that are empty or contain '=' are ignored, as g_setenv would reject them.
pub fn set_trace_env(name: &[u8], on: bool) {
    use std::os::unix::ffi::OsStrExt;
    if name.is_empty() || name.contains(&b'=') {
        return;
    }
    let key = std::ffi::OsStr::from_bytes(name);
    if on {
        std::env::set_var(key, "1");
    } else {
        std::env::remove_var(key);
    }
}

/// Reports a QEMU error through `errp` as `error_setg` does (file, line and function are the
/// C-side location strings). `errp` is the out-parameter QEMU passed to the caller; it is only
/// handed back to C and never dereferenced here.
pub fn error_setg_at(
    errp: *mut *mut super::dev_ns::Error,
    file: &CStr,
    line: core::ffi::c_int,
    func: &CStr,
    msg: &str,
) {
    let msg = std::ffi::CString::new(msg).unwrap_or_default();
    // SAFETY: errp is the QEMU-owned out-parameter (or NULL, which error_setg ignores); the
    // format "%s" consumes the NUL-terminated msg that lives across the call.
    unsafe {
        super::dev_ns::error_setg_internal(
            errp,
            file.as_ptr(),
            line,
            func.as_ptr(),
            c"%s".as_ptr(),
            msg.as_ptr(),
        );
    }
}

/// A glib-allocated copy of `s`; QEMU frees property getter results with g_free.
pub fn glib_strdup(s: &str) -> *mut c_char {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    // SAFETY: c is a valid NUL-terminated string for the duration of the call; the returned
    // buffer is allocated by glib and owned by the caller (QEMU).
    unsafe { super::dev_ns::g_strdup_inline(c.as_ptr()) }
}
// END w6_trace
// BEGIN w6_drawlog
/// The microcode disassembly of the R300 state, as text (the R300_USDISASM log). Text after
/// the first NUL byte is dropped, as the C string it replaces was read up to its NUL.
pub fn r300_us_disasm_to_string(st: &R300State) -> String {
    unsafe extern "C" fn collect(ctx: *mut ::core::ffi::c_void, text: *const u8, len: size_t) {
        if text.is_null() || len == 0 {
            return;
        }
        // SAFETY: ctx is the &mut Vec<u8> passed below and outlives the call; the callback
        // contract gives `len` readable bytes at `text`.
        unsafe {
            let out = &mut *(ctx as *mut Vec<u8>);
            out.extend_from_slice(std::slice::from_raw_parts(text, len as usize));
        }
    }
    let mut buf: Vec<u8> = Vec::new();
    // SAFETY: `st` is a valid R300State for the call and the C side only reads it; `collect`
    // is the write callback and `buf` is the only context, live until the call returns.
    unsafe {
        r300_us_disasm(st, Some(collect), &mut buf as *mut Vec<u8> as *mut ::core::ffi::c_void);
    }
    let text = buf.split(|&b| b == 0).next().unwrap_or_default();
    String::from_utf8_lossy(text).into_owned()
}
// END w6_drawlog
// BEGIN w6_realize
pub use self::realize_wrap::*;

impl CFile {
    /// Line-buffers the stream (`setvbuf` with `_IOLBF` and a library-owned buffer).  Must be
    /// called before the first write.
    pub fn set_line_buffered(&self) {
        // SAFETY: the handle came from `fopen` and is open; a null buffer asks stdio to allocate.
        unsafe {
            super::dev_ns::setvbuf(self.0.as_ptr(), core::ptr::null_mut(), 1, 0);
        }
    }
}

/// Safe wrappers for the device's realize path: PCI config and BARs, memory regions, timers,
/// bottom halves, the console, the DDC bus, the renderer hooks and the BIOS file.  Each wrapper
/// makes one QEMU call; the references it takes are the live objects the call needs.
mod realize_wrap {
    use super::Vram;
    use crate::src::dev_ns::*;
    use core::ffi::{c_int, c_void, CStr};
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    /// Size of a conventional PCI function's config space; the cmask and wmask buffers match.
    const PCI_CONF_LEN: usize = 256;

    /// Writes `bytes` into the device buffer `p` at `off`, if they fit inside it.
    fn pci_buf_put(p: *mut u8, off: usize, bytes: &[u8]) {
        if p.is_null() {
            return;
        }
        // SAFETY: `p` is one of the PCIDevice's PCI_CONF_LEN-byte buffers (config or wmask),
        // allocated by the PCI core for the device's lifetime; the `&mut PCIDevice` the callers
        // hold makes this the only access.
        let buf = unsafe { std::slice::from_raw_parts_mut(p, PCI_CONF_LEN) };
        if let Some(dst) = buf.get_mut(off..).and_then(|d| d.get_mut(..bytes.len())) {
            dst.copy_from_slice(bytes);
        }
    }

    /// `pci_set_byte` on the config space.
    pub fn pci_set_u8(dev: &mut PCIDevice, off: usize, v: u8) {
        pci_buf_put(dev.config, off, &[v]);
    }
    /// `pci_set_word` on the config space (little-endian, as QEMU stores it).
    pub fn pci_set_u16(dev: &mut PCIDevice, off: usize, v: u16) {
        pci_buf_put(dev.config, off, &v.to_le_bytes());
    }
    /// `pci_set_long` on the config space (little-endian).
    pub fn pci_set_u32(dev: &mut PCIDevice, off: usize, v: u32) {
        pci_buf_put(dev.config, off, &v.to_le_bytes());
    }
    /// `pci_set_long` on the write mask.
    pub fn pci_set_wmask_u32(dev: &mut PCIDevice, off: usize, v: u32) {
        pci_buf_put(dev.wmask, off, &v.to_le_bytes());
    }

    /// `pci_add_capability`: on failure QEMU has stored the error in `errp`.
    pub fn pci_add_capability(
        dev: &mut PCIDevice,
        id: u8,
        off: u8,
        size: u8,
        errp: &mut *mut Error,
    ) -> c_int {
        // SAFETY: `dev` is the live device, `errp` is QEMU's out-parameter slot; both are only
        // used for the duration of the call.
        unsafe { crate::src::dev_ns::pci_add_capability(dev, id, off, size, errp) }
    }

    /// `pci_register_bar`: `mr` becomes BAR `region` of the device for the device's lifetime.
    pub fn pci_register_bar(dev: &mut PCIDevice, region: c_int, attr: u8, mr: &mut MemoryRegion) {
        // SAFETY: `dev` and `mr` are live for the call; `mr` is part of the device state.
        unsafe { crate::src::dev_ns::pci_register_bar(dev, region, attr, mr) }
    }

    /// The device's QOM object: its embedded `Object`, at the address QEMU's C code uses.
    fn owner_obj(dev: &mut PCIDevice) -> *mut Object {
        &raw mut dev.qdev.parent_obj
    }

    /// The QOM type name of the device, for error messages.
    pub fn object_typename(dev: &PCIDevice) -> String {
        // SAFETY: `dev` is a live device object; object_get_typename only reads its class.
        super::cstr(unsafe { crate::src::dev_ns::object_get_typename(&raw const dev.qdev.parent_obj) })
    }

    /// `memory_region_init`: a container region of `size` bytes owned by the device.
    pub fn mr_init(mr: &mut MemoryRegion, owner: &mut PCIDevice, name: &CStr, size: uint64_t) {
        // SAFETY: `mr` lives inside the device state being realized; `name` is NUL-terminated
        // and alive for the call.
        unsafe { crate::src::dev_ns::memory_region_init(mr, owner_obj(owner), name.as_ptr(), size) }
    }

    /// `memory_region_init_io`: MMIO region whose accesses go to `ops` with `opaque`.  The
    /// opaque pointer is stored for the callbacks, not dereferenced here.
    pub fn mr_init_io(
        mr: &mut MemoryRegion,
        owner: &mut PCIDevice,
        ops: &'static MemoryRegionOps,
        opaque: *mut c_void,
        name: &CStr,
        size: uint64_t,
    ) {
        // SAFETY: as mr_init; `ops` is a static table; `opaque` is only stored by QEMU.
        unsafe {
            crate::src::dev_ns::memory_region_init_io(mr, owner_obj(owner), ops, opaque, name.as_ptr(), size)
        }
    }

    /// `memory_region_init_ram_ptr`: RAM region backed by the host block `ptr`.
    pub fn mr_init_ram_ptr(
        mr: &mut MemoryRegion,
        owner: &mut PCIDevice,
        name: &CStr,
        size: uint64_t,
        ptr: *mut c_void,
    ) {
        // SAFETY: `ptr` is the VRAM block allocated by the renderer, at least `size` bytes, and
        // outlives the device.
        unsafe {
            crate::src::dev_ns::memory_region_init_ram_ptr(mr, owner_obj(owner), name.as_ptr(), size, ptr)
        }
    }

    /// `memory_region_init_ram` with QEMU's fatal error slot: allocation failure aborts.
    pub fn mr_init_ram_fatal(mr: &mut MemoryRegion, owner: &mut PCIDevice, name: &CStr, size: uint64_t) {
        // SAFETY: as mr_init; error_fatal is QEMU's global abort-on-error slot.
        unsafe {
            memory_region_init_ram(mr, owner_obj(owner), name.as_ptr(), size, &raw mut error_fatal);
        }
    }

    /// `memory_region_init_alias`: a window of `orig` at `off`, `size` bytes long.
    pub fn mr_init_alias(
        mr: &mut MemoryRegion,
        owner: &mut PCIDevice,
        name: &CStr,
        orig: &mut MemoryRegion,
        off: hwaddr,
        size: uint64_t,
    ) {
        // SAFETY: `orig` is a region the device owns and outlives the alias.
        unsafe {
            crate::src::dev_ns::memory_region_init_alias(mr, owner_obj(owner), name.as_ptr(), orig, off, size)
        }
    }

    /// `memory_region_add_subregion`.
    pub fn mr_add_subregion(mr: &mut MemoryRegion, off: hwaddr, sub: &mut MemoryRegion) {
        // SAFETY: both regions are live device-owned regions.
        unsafe { crate::src::dev_ns::memory_region_add_subregion(mr, off, sub) }
    }

    /// `memory_region_add_subregion_overlap` into the PCI memory space of `dev`.
    pub fn pci_mem_add_subregion_overlap(dev: &mut PCIDevice, off: hwaddr, sub: &mut MemoryRegion, prio: c_int) {
        // SAFETY: pci_address_space returns the bus memory region of this device, alive while the
        // device is; `sub` is device-owned.
        unsafe {
            let space = crate::src::dev_ns::pci_address_space(dev);
            crate::src::dev_ns::memory_region_add_subregion_overlap(space, off, sub, prio)
        }
    }

    /// `memory_region_set_enabled`.
    pub fn mr_set_enabled(mr: &mut MemoryRegion, on: bool) {
        // SAFETY: `mr` is live.
        unsafe { crate::src::dev_ns::memory_region_set_enabled(mr, on) }
    }

    /// `memory_region_set_log` for the dirty-tracking client `client`.
    pub fn mr_set_log(mr: &mut MemoryRegion, on: bool, client: u32) {
        // SAFETY: `mr` is live.
        unsafe { crate::src::dev_ns::memory_region_set_log(mr, on, client) }
    }

    /// `vmstate_register_ram`: the region's contents are migrated with the device.
    pub fn vmstate_register_ram(mr: &mut MemoryRegion, dev: &mut PCIDevice) {
        // SAFETY: both are live device-owned objects.
        unsafe { crate::src::dev_ns::vmstate_register_ram(mr, &raw mut dev.qdev) }
    }

    /// `timer_init_ns` on the virtual clock.  `opaque` is handed back to `cb`, not dereferenced.
    pub fn timer_init_virtual(t: &mut QEMUTimer, cb: QEMUTimerCB, opaque: *mut c_void) {
        // SAFETY: `t` is part of the device state, which outlives the timer's use.
        unsafe { crate::src::dev_ns::timer_init_ns(t, QEMU_CLOCK_VIRTUAL, Some(cb), opaque) }
    }

    /// `qemu_bh_new_full` with no reentrancy guard.  `opaque` is stored, not dereferenced.
    pub fn bh_new(cb: QEMUBHFunc, opaque: *mut c_void, name: &CStr) -> *mut QEMUBH {
        // SAFETY: `name` is NUL-terminated and static; QEMU owns the bottom half.
        unsafe { crate::src::dev_ns::qemu_bh_new_full(Some(cb), opaque, name.as_ptr(), core::ptr::null_mut()) }
    }

    /// `qemu_graphic_console_create` for the device's head 0.
    pub fn graphic_console_create(
        dev: &mut PCIDevice,
        ops: &'static GraphicHwOps,
        opaque: *mut c_void,
    ) -> *mut QemuConsole {
        // SAFETY: the device's DeviceState is the first-class owner of the console; `ops` is a
        // static table; `opaque` is stored for the console callbacks.
        unsafe { crate::src::dev_ns::qemu_graphic_console_create(&raw mut dev.qdev, 0, ops, opaque) }
    }

    /// Zeroed storage from glib (`g_malloc0`) for a C-layout struct the device owns; QEMU frees
    /// it with `g_free`, which is why it is not a Rust allocation.
    pub fn glib_zeroed<T>() -> *mut T {
        // SAFETY: g_malloc0 returns a zeroed block of size_of::<T>() bytes or aborts.
        unsafe { crate::src::dev_ns::g_malloc0(::core::mem::size_of::<T>() as gsize) as *mut T }
    }

    /// As `glib_zeroed`, borrowed for the caller's use.
    pub fn glib_zeroed_ref<'a, T>() -> &'a mut T {
        // SAFETY: the block is fresh, zeroed, never null and not aliased anywhere else.
        unsafe { &mut *glib_zeroed::<T>() }
    }

    /// Legacy VGA setup on the device: `vga_common_init` then `vga_init` on the PCI spaces.
    /// False when `vga_common_init` fails; its error is then in `errp`.
    pub fn vga_setup(vga: &mut VGACommonState, dev: &mut PCIDevice, errp: &mut *mut Error) -> bool {
        // SAFETY: `vga` is the device's zeroed VGA state, `dev` the live device, `errp` QEMU's slot.
        unsafe {
            if !crate::src::dev_ns::vga_common_init(vga, owner_obj(dev), errp) {
                return false;
            }
            let mem = crate::src::dev_ns::pci_address_space(dev);
            let io = crate::src::dev_ns::pci_address_space_io(dev);
            crate::src::dev_ns::vga_init(vga, owner_obj(dev), mem, io, true);
        }
        true
    }

    /// Creates the DDC bus on the device, its bit-bang interface, and the EDID slave at 0x50.
    pub fn i2c_ddc_init(dev: &mut PCIDevice, bbi: &mut bitbang_i2c_interface) {
        // SAFETY: the bus and slave are created here and owned by the device; `dev` is live.
        unsafe {
            let bus = crate::src::dev_ns::i2c_init_bus(&raw mut dev.qdev, c"ppc-mac-gpu.ddc".as_ptr());
            crate::src::dev_ns::bitbang_i2c_init(bbi, bus);
            let slave = crate::src::dev_ns::I2C_SLAVE(
                crate::src::dev_ns::qdev_new(c"i2c-ddc".as_ptr()) as *const c_void,
            );
            crate::src::dev_ns::i2c_slave_set_address(slave, 0x50);
            crate::src::dev_ns::qdev_realize_and_unref(
                crate::src::dev_ns::DEVICE(slave as *const c_void),
                crate::src::dev_ns::BUS(bus as *const c_void),
                &raw mut error_abort,
            );
        }
    }

    /// The Vulkan renderer's static table.
    pub fn vulkan_renderer() -> *mut PPCMacGPURenderer {
        // SAFETY: no arguments; returns a pointer to QEMU's static renderer table.
        unsafe { crate::src::dev_ns::ppc_mac_gpu_renderer_vulkan() }
    }

    /// Allocates the host VRAM block the Vulkan renderer exports; the renderer's opaque
    /// handle is written to `opaque_out`.  Null on failure.
    pub fn vulkan_alloc_vram(size: uint64_t, opaque_out: &mut *mut c_void) -> *mut c_void {
        // SAFETY: `opaque_out` is a live out-slot for the call.
        unsafe { crate::src::dev_ns::ppc_mac_gpu_vulkan_alloc_vram(size, opaque_out) }
    }

    /// The Vulkan renderer's last error, as text.
    pub fn vulkan_error_text() -> String {
        // SAFETY: no arguments; the returned string is static or NUL-terminated.
        super::cstr(unsafe { crate::src::dev_ns::ppc_mac_gpu_vulkan_error() })
    }

    /// Calls the renderer's `init` on the VRAM block.  Null if it has no init hook or fails.
    pub fn renderer_init(r: &PPCMacGPURenderer, vram: Vram, size: uint64_t) -> *mut c_void {
        match r.init {
            // SAFETY: `f` is a renderer hook from the static table; the VRAM block is
            // `size` bytes and lives as long as the device.
            Some(f) => unsafe { f(vram.as_mut_ptr(), size) },
            None => core::ptr::null_mut(),
        }
    }

    /// Registers the VRAM dirty callback with the renderer, if it takes one.
    pub fn renderer_set_dirty_source(r: &PPCMacGPURenderer, renderer_opaque: *mut c_void, state: *mut c_void) {
        if let Some(f) = r.set_dirty_source {
            // SAFETY: `f` is the renderer's hook; `state` is the device state, which the
            // callback receives back and which outlives the renderer.
            unsafe { f(renderer_opaque, Some(ppc_mac_gpu_vram_dirty), state) }
        }
    }

    /// `trace_ppc_mac_gpu_realize`.
    pub fn trace_realize(vram_mb: uint32_t, vram_bytes: uint64_t) {
        // SAFETY: trace emission has no preconditions beyond QEMU being initialised.
        unsafe { crate::src::dev_ns::trace_ppc_mac_gpu_realize(vram_mb, vram_bytes) }
    }

    /// Reads the BIOS image `name` the way QEMU finds it: through `qemu_find_file` when that
    /// resolves it, else the name itself.  Returns the path tried (for logs) and the bytes, or
    /// None when the file cannot be read.
    pub fn read_bios_file(name: &CStr) -> (String, Option<Vec<u8>>) {
        // SAFETY: `name` is NUL-terminated and lives for the call.
        let resolved = unsafe { crate::src::dev_ns::qemu_find_file(QEMU_FILE_TYPE_BIOS, name.as_ptr()) };
        let path: Vec<u8> = if resolved.is_null() {
            name.to_bytes().to_vec()
        } else {
            // SAFETY: qemu_find_file returns a glib-allocated, NUL-terminated string that this
            // function owns; it is copied out before being freed.
            let bytes = unsafe { CStr::from_ptr(resolved) }.to_bytes().to_vec();
            unsafe { crate::src::dev_ns::g_free(resolved as gpointer) };
            bytes
        };
        let data = std::fs::read(OsStr::from_bytes(&path)).ok();
        (String::from_utf8_lossy(&path).into_owned(), data)
    }

    /// Opens `path` for the R300 dump with `fopen(path, "w")`, line-buffered like the C code.
    pub fn open_r3_dump(path: &CStr) -> *mut FILE {
        match super::cfile_open_write(path) {
            Some(f) => {
                f.set_line_buffered();
                f.as_ptr()
            }
            None => core::ptr::null_mut(),
        }
    }

    /// Stores the error in `errp` (QEMU's out-parameter), or a scratch slot when none was given.
    pub fn errp_slot<'a>(errp: *mut *mut Error) -> Option<&'a mut *mut Error> {
        if errp.is_null() {
            return None;
        }
        // SAFETY: QEMU passes a live out-parameter slot to realize; it is only written through
        // this reference for the duration of the caller's use.
        Some(unsafe { &mut *errp })
    }
}
// END w6_realize
// BEGIN w7_ringdump
/// The R300 ring dump: opened once from `R300_RINGDUMP` and dropped (closed) for good once it
/// passes 2000 MiB.
struct RingState {
    opened: bool,
    file: Option<CFile>,
}

static RINGDUMP: std::sync::Mutex<RingState> =
    std::sync::Mutex::new(RingState { opened: false, file: None });

/// True while the ring dump is open for writing.  The first call opens it, and a dump past
/// 2000 MiB is closed here, so later calls return false.
pub fn ringdump_active() -> bool {
    let mut st = RINGDUMP.lock().unwrap_or_else(|e| e.into_inner());
    if !st.opened {
        st.opened = true;
        st.file = env_string(c"R300_RINGDUMP")
            .and_then(|path| std::ffi::CString::new(path).ok())
            .and_then(|path| cfile_open_write(&path));
    }
    if st.file.as_ref().is_some_and(|f| f.tell() > 2000 * 1024 * 1024) {
        if let Some(f) = st.file.take() {
            f.close();
        }
    }
    st.file.is_some()
}

/// Appends `s` to the ring dump.  Does nothing when no dump is active.
pub fn ringdump_write(s: &[u8]) {
    let st = RINGDUMP.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(f) = st.file.as_ref() {
        // SAFETY: `f` holds a live `FILE *` from `fopen` that is not closed while the lock
        // guard `st` is held, and `s` is valid for reads of `s.len()` bytes for the call.
        unsafe {
            super::dev_ns::fwrite(s.as_ptr().cast(), 1, s.len() as size_t, f.as_ptr());
        }
    }
}
// END w7_ringdump
// BEGIN w8_packet3
/// Calls `ppc_mac_gpu_mmio_write` for the device: QEMU passes the device state as the opaque
/// pointer, so `s` is what it would receive.
pub fn mmio_write_dev(s: &mut super::dev_ns::PPCMacGPUState, addr: super::dev_ns::hwaddr, val: u64, size: u32) {
    super::dev_ns::ppc_mac_gpu_mmio_write(s, addr, val, size)
}

/// `fflush` on the device's R300 dump file, if one is open.
pub fn r3_dump_flush(s: &super::dev_ns::PPCMacGPUState) {
    if s.r3_dump.is_null() {
        return;
    }
    // SAFETY: `r3_dump` is the FILE the device opened for its dump and keeps open until
    // unrealize; the device holds the big lock, so nothing else closes it during the call.
    unsafe { super::dev_ns::fflush(s.r3_dump) };
}

/// `r300_state_write` on the device's R300 register block (no-op if the block is absent).
pub fn r3_state_write(s: &mut super::dev_ns::PPCMacGPUState, addr: u32, val: u32) {
    if s.r3.is_null() {
        return;
    }
    // SAFETY: `s.r3` is the heap R300State owned by this device; `s` is borrowed exclusively.
    unsafe { super::dev_ns::r300_state_write(s.r3, addr, val) };
}

/// Dumps the R300 register block to the device's dump file, then flushes it. No-op unless both
/// the register block and the dump file exist.
pub fn r3_state_dump_to_file(s: &super::dev_ns::PPCMacGPUState) {
    if s.r3.is_null() || s.r3_dump.is_null() {
        return;
    }
    // SAFETY: `s.r3` is the device's live R300State and `s.r3_dump` its open dump FILE; the
    // callback writes into that FILE (`r300_text_to_file`) and only while this call runs.
    unsafe {
        super::dev_ns::r300_state_dump(
            s.r3,
            Some(
                super::dev_ns::r300_text_to_file
                    as unsafe extern "C" fn(*mut core::ffi::c_void, *const u8, super::dev_ns::size_t),
            ),
            s.r3_dump as *mut core::ffi::c_void,
        );
        super::dev_ns::fflush(s.r3_dump);
    }
}

/// Copies `out.len()` bytes from the host page `p` (an AGP/GART page from `gpu_agp_page`) into
/// `out`. Returns false, leaving `out` untouched, unless the range passes `guest_px_ptr_valid`.
pub fn agp_page_read(s: &super::dev_ns::PPCMacGPUState, p: *mut u8, out: &mut [u8]) -> bool {
    if !guest_px_ptr_valid(s, p, out.len()) {
        return false;
    }
    // SAFETY: guest_px_ptr_valid checked that [p, p + len) is non-null and lies inside VRAM or
    // one live guest-RAM page; `out` is a distinct Rust buffer, so the ranges cannot overlap.
    unsafe { core::ptr::copy_nonoverlapping(p, out.as_mut_ptr(), out.len()) };
    true
}
// END w8_packet3
// BEGIN wF_misc
/// Checked QOM downcast of a device object: `object_dynamic_cast_assert` with the caller's
/// type name and source location.  Returns null for a null `obj`; a type mismatch aborts inside
/// QEMU, exactly as the C cast macro does.  The result is handed back, never dereferenced here.
pub fn qom_object_cast<T>(
    obj: *const core::ffi::c_void,
    typename: &CStr,
    file: &CStr,
    line: core::ffi::c_int,
    func: &CStr,
) -> *mut T {
    if obj.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: `obj` is the live QOM object the caller was handed (as in `state_of_opaque`);
    // QEMU's cast reads only its class and type chain, and returns it unchanged on success.
    unsafe {
        super::dev_ns::object_dynamic_cast_assert(
            obj as *mut super::dev_ns::Object,
            typename.as_ptr(),
            file.as_ptr(),
            line,
            func.as_ptr(),
        ) as *mut T
    }
}

/// Registers a QOM type-registration constructor with QEMU's module-init list (QOM stage).
pub fn register_qom_module_init(f: unsafe extern "C" fn()) {
    // SAFETY: register_module_init only appends the callback to QEMU's init list; `f` is an
    // ordinary extern "C" function that QEMU calls once, before main().
    unsafe { super::dev_ns::register_module_init(Some(f), super::dev_ns::MODULE_INIT_QOM) };
}

/// A zero-filled `FrameTracker`.  Every field is an integer, a bool, a `c_uint` alias
/// (`SurfaceClass`, `PassEventType`) or a plain struct of those, so all-zero bytes are valid
/// (the C code initialised it with memset(0)).
pub fn zeroed_frame_tracker() -> super::dev_ns::FrameTracker {
    // SAFETY: see above; no field has a niche or a reference that zero would violate.
    unsafe { core::mem::zeroed() }
}

/// The R300 frame tracker registered in `g_frame_tracker`, or None when none is set.  The
/// pointer is null or the live tracker set at device realize; it is used only under QEMU's
/// big lock, and each caller holds the borrow for one call.
pub fn frame_tracker_global<'a>() -> Option<&'a mut super::dev_ns::FrameTracker> {
    // SAFETY: see above; reading the pointer does not alias any other live reference.
    unsafe { super::dev_ns::g_frame_tracker.as_mut() }
}

/// Bytes of a NUL-terminated C string (without the NUL); empty for a null pointer.
pub fn c_bytes<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    // SAFETY: `p` is a NUL-terminated string alive for the call (the same contract as `cstr`).
    unsafe { CStr::from_ptr(p) }.to_bytes()
}

/// `len` bytes of a host-side texture buffer at `p`, or None for a null pointer.  Callers pass
/// the length the buffer was allocated with (`r300_read_gart_texture` mallocs exactly the
/// texture's byte count, and `R300TexDesc::size_bytes` is that count for uploaded textures).
pub fn host_bytes<'a>(p: *const u8, len: usize) -> Option<&'a [u8]> {
    if p.is_null() {
        return None;
    }
    // SAFETY: `p` is a live host buffer of `len` bytes, per the allocation contract above.
    Some(unsafe { core::slice::from_raw_parts(p, len) })
}

/// Length of the harmony tile map `pe_harm_tiles` points at; 0 while none is allocated.
static HARM_TILES_LEN: super::global::Global<usize> = super::global::Global::<usize>::new(0);

/// Replaces the harmony tile map with `n` zeroed bytes, freeing the previous block.  Called
/// under QEMU's big lock, which also guards every reader of the map.
pub fn harm_tiles_replace(n: usize) {
    // SAFETY: `pe_harm_tiles` is null or the block g_malloc0 returned for HARM_TILES_LEN bytes by
    // an earlier call here; it is freed once and overwritten below.  g_malloc0 aborts on failure,
    // so the new pointer is non-null.
    unsafe {
        super::dev_ns::g_free(super::dev_ns::pe_harm_tiles as super::dev_ns::gpointer);
        super::dev_ns::pe_harm_tiles = super::dev_ns::g_malloc0(n as super::dev_ns::gsize) as *mut u8;
    }
    HARM_TILES_LEN.set(n);
}

/// The harmony tile map as its `HARM_TILES_LEN` bytes, or None if none is allocated.
pub fn harm_tiles_mut<'a>() -> Option<&'a mut [u8]> {
    // SAFETY: a non-null `pe_harm_tiles` is the block `harm_tiles_replace` allocated with
    // HARM_TILES_LEN bytes; the big lock serialises every user, and the caller uses the slice
    // for one update only.
    let p = unsafe { super::dev_ns::pe_harm_tiles };
    if p.is_null() {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts_mut(p, HARM_TILES_LEN.get()) })
}
// END wF_misc
// BEGIN wC_r300_1
/// The `len`-byte C buffer at `ptr` as a Rust slice, for a C callback that is handed a buffer it
/// owns. `None` for a null pointer with a non-zero length.
///
/// # Safety
/// A non-null `ptr` must point to `len` bytes that are valid for reads and writes for `'a` and
/// are not accessed through any other pointer while the slice lives.
pub unsafe fn c_buf_mut<'a>(ptr: *mut core::ffi::c_void, len: usize) -> Option<&'a mut [u8]> {
    if len == 0 {
        return Some(&mut []);
    }
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the callers pass a buffer of `len` bytes that they allocated (a malloc'd texture
    // copy or a VRAM span checked by `r300_to_vram`), and nothing else borrows it during the call.
    Some(unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, len) })
}

/// Writes the `len` bytes at `ptr` to the open C `FILE` (the R300 register dump). Does nothing
/// for a null file or buffer.
///
/// # Safety
/// A non-null `ptr` must point to `len` readable bytes, and a non-null `file` must be an open
/// C stream.
pub unsafe fn write_c_bytes(file: *mut super::dev_ns::FILE, ptr: *const u8, len: usize) {
    if file.is_null() || ptr.is_null() || len == 0 {
        return;
    }
    // SAFETY: QEMU's dump writer passes the text buffer it owns, valid for `len` bytes for the
    // duration of the call; `file` is the stream the dump was opened on (see `open_r3_dump`).
    let data = unsafe { core::slice::from_raw_parts(ptr, len) };
    // SAFETY: `file` is an open stream and `data` is a live slice of `data.len()` bytes.
    unsafe { super::dev_ns::fwrite(data.as_ptr() as *const core::ffi::c_void, 1, data.len() as size_t, file) };
}

/// Frees the C allocation held in `slot` (null, or a pointer from malloc set by the GART texture
/// reader) and clears the slot, so it cannot be freed twice through it.
pub fn c_free_slot(slot: &mut *mut u8) {
    // SAFETY: `*slot` is null or was returned by malloc and has not been freed, because the slot
    // is cleared immediately after this call.
    unsafe { super::dev_ns::free(*slot as *mut core::ffi::c_void) };
    *slot = core::ptr::null_mut();
}

/// Releases what an R300 draw packet owns through the C helper `r300_draw_free`.
pub fn r300_draw_free_packet(pkt: &mut super::dev_ns::R300DrawPacket) {
    // SAFETY: `pkt` is exclusively borrowed for the call and is a valid packet; the helper only
    // releases what the packet owns.
    unsafe { super::dev_ns::r300_draw_free(pkt) };
}

/// Calls the renderer's `zpass_r300` callback if one is registered. Returns its count for a
/// query (`enable` false), or `None` when there is no callback.
pub fn renderer_zpass(s: &super::dev_ns::PPCMacGPUState, enable: bool, val: u32) -> Option<u32> {
    if s.renderer.is_null() {
        return None;
    }
    let zpass = s.renderer_ref().zpass_r300?;
    // SAFETY: the renderer table is static and set at realize; `zpass` is the callback it
    // registered and is called with the device's opaque pointer, as before.
    Some(unsafe { zpass(s.renderer_opaque, enable, val) })
}
// END wC_r300_1
// BEGIN wE_r300_2
/// Safe wrappers for the R300 surface watch region (`fn_r300_2.rs`): the region is mapped into
/// VRAM and its callbacks reach the device state through the opaque pointer.
pub use self::watch_wrap::*;

mod watch_wrap {
    use crate::src::dev_ns::*;
    use core::ffi::c_int;

    /// Initialises `s.r300_watch` as an MMIO region of `size` bytes that dispatches to `ops`,
    /// with the device state as both its owner and its opaque pointer (the callbacks read the
    /// state back through `state_of_opaque`).
    pub fn r300_watch_region_init(s: &mut PPCMacGPUState, ops: &'static MemoryRegionOps, size: uint64_t) {
        let owner = s as *mut PPCMacGPUState as *mut Object;
        let opaque = s as *mut PPCMacGPUState as *mut ::core::ffi::c_void;
        let mr = &raw mut s.r300_watch;
        // SAFETY: `s` is the live device state, which outlives the region; `mr` is a field of
        // `s`, so it is valid to initialise; `ops` is a static table and the name is a static
        // C string. QEMU only stores the owner and opaque pointers.
        unsafe { memory_region_init_io(mr, owner, ops, opaque, c"ppc-mac-gpu-surfwatch".as_ptr(), size) }
    }

    /// `memory_region_del_subregion`: removes `sub` from its container `mr`.
    pub fn mr_del_subregion(mr: &mut MemoryRegion, sub: &mut MemoryRegion) {
        // SAFETY: both regions are live device-owned regions; QEMU asserts the parent link itself.
        unsafe { memory_region_del_subregion(mr, sub) }
    }

    /// `object_unparent` on an initialised region: unlinks it from its owner.
    pub fn mr_unparent(mr: &mut MemoryRegion) {
        // SAFETY: `mr` is a live region; a MemoryRegion begins with its QOM object header, as the
        // device code already relies on, and QOM only unlinks it from its parent.
        unsafe { object_unparent(mr as *mut MemoryRegion as *mut Object) }
    }

    /// `memory_region_add_subregion_overlap`: maps `sub` into `mr` at `off` with priority `prio`.
    pub fn mr_add_subregion_overlap(mr: &mut MemoryRegion, off: hwaddr, sub: &mut MemoryRegion, prio: c_int) {
        // SAFETY: both regions are live device-owned regions; `mr` keeps `sub` until it is removed.
        unsafe { memory_region_add_subregion_overlap(mr, off, sub, prio) }
    }
}
// END wE_r300_2
// BEGIN wB_ppc1
/// `pci_set_irq`: raises or lowers the device's INTx line.
pub fn pci_irq_set(dev: &mut super::dev_ns::PCIDevice, level: bool) {
    // SAFETY: `dev` is a live PCIDevice borrowed exclusively for the call; pci_set_irq only
    // updates the interrupt line state of that device.
    unsafe { super::dev_ns::pci_set_irq(dev, level as core::ffi::c_int) }
}

/// `timer_mod`: arms `t` to fire at `expire` (nanoseconds on its clock).
pub fn timer_arm(t: &mut super::dev_ns::QEMUTimer, expire: i64) {
    // SAFETY: `t` is a live QEMUTimer borrowed exclusively for the call; timer_mod only
    // reschedules it.
    unsafe { super::dev_ns::timer_mod(t, expire) }
}

/// `trace_ppc_mac_gpu_irq`: emits the interrupt trace event.
pub fn trace_irq_event(status: u32, cntl: u32, level: bool) {
    // SAFETY: plain trace call with scalar arguments.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_irq(status, cntl, level) }
}

/// The harmony tile buffer pointer (`pe_harm_tiles`), null when none is allocated.  Only read
/// here; the pointer is not dereferenced.
pub fn harm_tiles_ptr() -> *mut u8 {
    // SAFETY: reads the static pointer on the device thread under the big lock.
    unsafe { super::dev_ns::pe_harm_tiles }
}

/// Runs `f` on the global frame tracker (`g_frame_tracker`) when one is set.
pub fn with_frame_tracker(f: impl FnOnce(&mut super::dev_ns::FrameTracker)) {
    // SAFETY: g_frame_tracker is set once at device realize and used only on the device thread
    // under QEMU's big lock; `as_mut` yields None for null, and the reference lives only in `f`.
    if let Some(ft) = unsafe { super::dev_ns::g_frame_tracker.as_mut() } {
        f(ft);
    }
}

/// `g_malloc` of `src.len()` dwords holding a copy of `src`.  The caller frees it with
/// `g_free`.  Null for an empty slice (g_malloc(0) returns NULL).
pub fn g_malloc_dwords(src: &[u32]) -> *mut u32 {
    let n = src.len();
    let Some(bytes) = n.checked_mul(4) else {
        return core::ptr::null_mut();
    };
    if n == 0 {
        return core::ptr::null_mut();
    }
    // SAFETY: g_malloc returns a fresh block of `bytes` bytes (it aborts rather than returning
    // null for a nonzero size); the copy writes exactly `n` u32 values, that is `bytes` bytes,
    // into it from `src`, which is a distinct live slice.
    unsafe {
        let p = super::dev_ns::g_malloc(bytes as super::dev_ns::gsize) as *mut u32;
        core::ptr::copy_nonoverlapping(src.as_ptr(), p, n);
        p
    }
}
/// `bitbang_i2c_set`: drives one line (`line` 0 = SDA, 1 = SCL) of the bit-banged I2C bus to
/// `level`, returning the resulting line state.
pub fn bitbang_line_set(i2c: &mut super::dev_ns::bitbang_i2c_interface, line: core::ffi::c_int, level: core::ffi::c_int) -> core::ffi::c_int {
    // SAFETY: `i2c` is a live bit-bang interface borrowed exclusively for the call.
    unsafe { super::dev_ns::bitbang_i2c_set(i2c, line, level) }
}

/// `deposit32`: replaces the `length`-bit field of `value` at bit `start` with the low bits of
/// `fieldval`.
pub fn deposit_field(value: u32, start: core::ffi::c_int, length: core::ffi::c_int, fieldval: u32) -> u32 {
    // SAFETY: deposit32 is a pure bit operation on scalar arguments.
    unsafe { super::dev_ns::deposit32(value, start, length, fieldval) }
}
// END wB_ppc1
// BEGIN wA_ppc4
/// Traces a CP ring base/control write (`trace_ppc_mac_gpu_cp_ring_setup`).
pub fn trace_cp_ring_setup(base: u64, cntl: u32) {
    // SAFETY: the trace hook takes two plain integers and no pointers.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_cp_ring_setup(base, cntl) };
}

/// Schedules the ring bottom half, if the device has created one (`qemu_bh_schedule`).
pub fn ring_bh_schedule(s: &super::dev_ns::PPCMacGPUState) {
    if s.ring_bh.is_null() {
        return;
    }
    // SAFETY: ring_bh is created by bh_new at realize and never freed while the device lives; it
    // is non-null here.  The bottom half runs on the main loop under the big lock.
    unsafe { super::dev_ns::qemu_bh_schedule(s.ring_bh) };
}

/// Traces a 3D register write (`trace_ppc_mac_gpu_3d_reg_write`).
pub fn trace_3d_reg_write(addr: u64, val: u64) {
    // SAFETY: the trace hook takes two plain integers and no pointers.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_3d_reg_write(addr, val) };
}

/// Traces an MMIO write with its register name (`trace_ppc_mac_gpu_mmio_write`).
pub fn trace_mmio_write(size: u32, addr: u64, name: &CStr, val: u64) {
    // SAFETY: `name` is NUL-terminated and alive for the call; the hook reads it only during the call.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_mmio_write(size, addr, name.as_ptr(), val) };
}

/// Traces an unimplemented MMIO write (`trace_ppc_mac_gpu_mmio_write_unimpl`).
pub fn trace_mmio_write_unimpl(size: u32, addr: u64, name: &CStr, val: u64) {
    // SAFETY: as trace_mmio_write.
    unsafe { super::dev_ns::trace_ppc_mac_gpu_mmio_write_unimpl(size, addr, name.as_ptr(), val) };
}

/// The name of a register in the R300 name table, or `None` when the table has no entry
/// (`r300_reg_name` returns NULL).
pub fn r300_reg_cstr(addr: u32) -> Option<&'static CStr> {
    // SAFETY: r300_reg_name has no preconditions; a non-null result points at a static
    // NUL-terminated name.
    let p = unsafe { super::dev_ns::r300_reg_name(addr) };
    if p.is_null() {
        return None;
    }
    // SAFETY: see above; the name table entries are static C strings.
    Some(unsafe { CStr::from_ptr(p) })
}

/// Stops a QEMU timer (`timer_del`).
pub fn timer_delete(t: &mut super::dev_ns::QEMUTimer) {
    // SAFETY: t is a live timer borrowed exclusively; timer_del only unlinks it from its list.
    unsafe { super::dev_ns::timer_del(t) };
}

/// Frees the device's shadow framebuffer and clears its pointer (`g_free` on `shadow_buf`).
pub fn shadow_free(s: &mut PPCMacGPUState) {
    // SAFETY: shadow_buf is NULL or a g_malloc block owned by the device, and g_free accepts NULL.
    // The pointer is cleared straight after, so the block is freed once.
    unsafe { super::dev_ns::g_free(s.shadow_buf as super::dev_ns::gpointer) };
    s.shadow_buf = core::ptr::null_mut();
}

/// Dirty-page snapshot of the VRAM region, freed when dropped (`DirtyBitmapSnapshot`).
pub struct VramDirtySnapshot(*mut super::dev_ns::DirtyBitmapSnapshot);

impl VramDirtySnapshot {
    /// Snapshots and clears the dirty state of all of VRAM.
    pub fn take(s: &mut PPCMacGPUState) -> Self {
        let size = s.vram_size as super::dev_ns::hwaddr;
        // SAFETY: &raw mut s.vram is the device's VRAM MemoryRegion, live for the device's lifetime;
        // the returned snapshot is owned by the caller and freed by Drop.
        let snap = unsafe {
            super::dev_ns::memory_region_snapshot_and_clear_dirty(&raw mut s.vram, 0, size, 0)
        };
        Self(snap)
    }

    /// True if any page in `[addr, addr + len)` was dirty when the snapshot was taken.
    pub fn get_dirty(&self, s: &mut PPCMacGPUState, addr: u64, len: u64) -> bool {
        if self.0.is_null() {
            return false;
        }
        // SAFETY: self.0 is the live snapshot of this region (see take) and is freed only by Drop;
        // &raw mut s.vram is the region the snapshot came from.
        unsafe {
            super::dev_ns::memory_region_snapshot_get_dirty(
                &raw mut s.vram,
                self.0,
                addr as super::dev_ns::hwaddr,
                len as super::dev_ns::hwaddr,
            )
        }
    }
}

impl Drop for VramDirtySnapshot {
    fn drop(&mut self) {
        // SAFETY: self.0 came from g_malloc in take (or is NULL, which g_free accepts) and is freed
        // exactly once, here.
        unsafe { super::dev_ns::g_free(self.0 as super::dev_ns::gpointer) };
    }
}

/// The dirty-log words QEMU passes to a VRAM sync callback: one bit per page of the `npages`-page
/// region, rounded up to whole words.  A null bitmap gives an empty slice.
pub fn dirty_log_words<'a>(bitmap: *mut core::ffi::c_ulong, npages: u64) -> &'a mut [core::ffi::c_ulong] {
    if bitmap.is_null() {
        return &mut [];
    }
    let words = usize::try_from(npages.div_ceil(64)).unwrap_or(usize::MAX);
    // SAFETY: QEMU's dirty-log sync allocates the bitmap for the region's `npages` pages, which is
    // the `words` longs here; the slice is used only inside the sync callback that received it.
    unsafe { core::slice::from_raw_parts_mut(bitmap, words) }
}

/// Sets bit `nr` of a dirty bitmap; a bit past the end of the slice is ignored.
pub fn set_bit_in(words: &mut [core::ffi::c_ulong], nr: usize) {
    if let Some(w) = words.get_mut(nr / 64) {
        *w |= 1 << (nr % 64);
    }
}

/// The PCI device a config callback was called for, or `None` for a null pointer.
pub fn pci_device_mut<'a>(dev: *mut super::dev_ns::PCIDevice) -> Option<&'a mut super::dev_ns::PCIDevice> {
    // SAFETY: QEMU calls the config callbacks with the device they belong to, which is live for the
    // duration of the call and is not otherwise borrowed while the reference is used.
    unsafe { dev.as_mut() }
}

/// Writes `len` bytes of PCI config space at `address` (pci_default_write_config).
pub fn pci_config_write(dev: &mut super::dev_ns::PCIDevice, address: u32, val: u32, len: core::ffi::c_int) {
    // SAFETY: dev is a live PCIDevice borrowed exclusively for the call; the default write touches
    // only its config space at the given address and length.
    unsafe { super::dev_ns::pci_default_write_config(dev, address, val, len) };
}

/// The PCI device class QEMU is initialising for this type, or `None` for a null pointer
/// (`PCI_DEVICE_CLASS`, which QOM checks).
pub fn pci_device_class<'a>(klass: *mut super::dev_ns::ObjectClass) -> Option<&'a mut super::dev_ns::PCIDeviceClass> {
    // SAFETY: class_init gives the live class being initialised; PCI_DEVICE_CLASS is the QOM cast
    // that returns the same pointer (or asserts), and the reference is used only within class_init.
    unsafe { super::dev_ns::PCI_DEVICE_CLASS(klass as *const core::ffi::c_void).as_mut() }
}

/// Sets the device class reset callback (`device_class_set_legacy_reset`).
pub fn class_set_reset(dc: &mut super::dev_ns::DeviceClass, reset: super::dev_ns::DeviceReset) {
    // SAFETY: dc is the class being initialised; the call stores the callback pointer only.
    unsafe { super::dev_ns::device_class_set_legacy_reset(dc, reset) };
}

/// Installs the device's property table on its class.  The table is a static of `N` entries whose
/// last entry must be filled in (QEMU's build-time check, a call that cannot be reached otherwise).
pub fn class_set_ppc_mac_gpu_props(dc: &mut super::dev_ns::DeviceClass) {
    use super::dev_ns::Property;
    let count = (core::mem::size_of::<[Property; 5]>() / core::mem::size_of::<Property>()) as super::dev_ns::size_t;
    // SAFETY: the property table is a static read here only, during class_init on the QEMU thread
    // that owns it; `count - 1` is inside the array.  device_class_set_props_n keeps the pointer.
    unsafe {
        let props = &raw const super::dev_ns::ppc_mac_gpu_properties;
        if (*props)[count as usize - 1].name.is_null() {
            super::dev_ns::qemu_build_not_reached_always();
        }
        super::dev_ns::device_class_set_props_n(dc, props.cast::<Property>(), count);
    }
}

/// Registers a QOM string property called `trace` on the class (`object_class_property_add_str`).
pub fn class_add_trace_property(
    klass: &mut super::dev_ns::ObjectClass,
    get: Option<unsafe extern "C" fn(*mut super::dev_ns::Object, *mut *mut super::dev_ns::Error) -> *mut c_char>,
    set: Option<unsafe extern "C" fn(*mut super::dev_ns::Object, *const c_char, *mut *mut super::dev_ns::Error)>,
) {
    // SAFETY: klass is the class being initialised; QOM copies the name and keeps the two plain
    // accessor functions for the property's lifetime.
    unsafe { super::dev_ns::object_class_property_add_str(klass, c"trace".as_ptr(), get, set) };
}

/// Registers the device type and its companion type with QOM (`type_register_static`).
pub fn register_device_types() {
    // SAFETY: both TypeInfo statics are complete and fixed after their constructor; QOM copies
    // them during registration.
    unsafe {
        super::dev_ns::type_register_static(&raw const super::dev_ns::ppc_mac_gpu_type_info);
        super::dev_ns::type_register_static(&raw const super::dev_ns::ati_radeon_9700_type_info);
    }
}

/// The host display size and depth globals (`graphic_width`, `graphic_height`, `graphic_depth`).
pub fn host_graphic_size() -> (core::ffi::c_int, core::ffi::c_int, core::ffi::c_int) {
    // SAFETY: the display globals are written on the main loop under the big lock and read here on
    // the device's thread under the same lock.
    unsafe {
        (
            super::dev_ns::graphic_width,
            super::dev_ns::graphic_height,
            super::dev_ns::graphic_depth,
        )
    }
}

/// Sends an explicit mouse position and visibility to the device's console.
pub fn console_set_mouse_to(s: &PPCMacGPUState, x: i32, y: i32, on: bool) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: as console_update_full: con is the live console created at realize.
    unsafe { super::dev_ns::qemu_console_set_mouse(s.con, x, y, on) };
}

/// Resets the device's VGA state (`vga_common_reset`).
pub fn vga_reset(s: &PPCMacGPUState) {
    if s.vga.is_null() {
        return;
    }
    // SAFETY: vga is the VGA state realize created on the legacy VGA path, live for the device's
    // lifetime.
    unsafe { super::dev_ns::vga_common_reset(s.vga) };
}

/// Closes the device's graphic console (`qemu_graphic_console_close`).
pub fn graphic_console_close(s: &PPCMacGPUState) {
    if s.con.is_null() {
        return;
    }
    // SAFETY: as console_update_full: con is the live console created at realize.
    unsafe { super::dev_ns::qemu_graphic_console_close(s.con) };
}

/// Zeroes the device's register block (`memset` of `PPCMacGPURegs`).
pub fn zero_regs(regs: &mut super::dev_ns::PPCMacGPURegs) {
    // SAFETY: PPCMacGPURegs holds only integers, bools and arrays of them, for which all-zero bytes
    // are a valid value.
    unsafe { core::ptr::write_bytes(regs, 0, 1) };
}

/// Zeroes the tiled-surface table (`memset` of `tiled_surfaces`).
pub fn zero_tiled_surfaces(tiles: &mut [super::dev_ns::C2RustUnnamed_91; 8]) {
    // SAFETY: each entry holds integers, a bool and a byte label, for which all-zero bytes are valid.
    unsafe { core::ptr::write_bytes(tiles.as_mut_ptr(), 0, tiles.len()) };
}
// END wA_ppc4
// BEGIN wG_render
/// A `malloc`ed copy of `src` (freed with `c_free_slot`), for buffers a C consumer frees. Null
/// for an empty slice or when the allocation fails.
pub fn c_malloc_copy(src: &[u8]) -> *mut u8 {
    if src.is_empty() {
        return core::ptr::null_mut();
    }
    // SAFETY: malloc returns null or a fresh block of `src.len()` bytes; the copy writes exactly
    // that many bytes into it from `src`, a distinct live slice.
    unsafe {
        let p = super::dev_ns::malloc(src.len() as super::dev_ns::size_t) as *mut u8;
        if !p.is_null() {
            core::ptr::copy_nonoverlapping(src.as_ptr(), p, src.len());
        }
        p
    }
}

/// Copies `data` into the host page `p` (an AGP/GART page from `gpu_agp_page`, offset already
/// applied). Returns false, writing nothing, unless the range passes `guest_px_ptr_valid`.
pub fn agp_page_write(s: &super::dev_ns::PPCMacGPUState, p: *mut u8, data: &[u8]) -> bool {
    if !guest_px_ptr_valid(s, p, data.len()) {
        return false;
    }
    // SAFETY: guest_px_ptr_valid checked that [p, p + len) lies inside VRAM or one live guest-RAM
    // page; `data` is a distinct Rust buffer, so the ranges cannot overlap.
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len()) };
    true
}

/// Loads the vertex-array (AOS) registers `aos` into `arr` (`r300_load_vbpntr`).
pub fn r300_load_arrays(arr: &mut super::dev_ns::R300Arrays, aos: &[u32]) {
    // SAFETY: `arr` is exclusively borrowed; the helper reads `aos.len()` dwords from the slice.
    unsafe { super::dev_ns::r300_load_vbpntr(arr, aos.as_ptr(), aos.len() as u32) };
}

/// Builds the draw packet `pkt` from the CP state (`r300_draw_build`, or `_indexed` when `idx`
/// holds the index dwords). Returns whether it worked and the builder's error message (a static
/// C string, or null) when it did not. Guest data is read back through `r300_read_raw`.
pub fn r300_build_packet(
    s: &mut super::dev_ns::PPCMacGPUState,
    pkt: &mut super::dev_ns::R300DrawPacket,
    opcode: u32,
    d: &[u32],
    body_dw: u32,
    idx: Option<&[u32]>,
) -> (bool, *const core::ffi::c_char) {
    let mut err: *const core::ffi::c_char = core::ptr::null();
    let st = s.r3;
    let arrays = &raw mut s.r3_arrays;
    let opaque = s as *mut super::dev_ns::PPCMacGPUState as *mut core::ffi::c_void;
    let read: super::dev_ns::R300ReadFn = Some(super::dev_ns::r300_read_raw);
    // SAFETY: `st` is the device's R300 state (set at realize) and `arrays` its array block, both
    // alive for the call; `d` and `idx` are live slices whose lengths are passed (the dword count
    // is clamped to the slice); `pkt` and `err` are exclusive locals; the read callback and
    // `opaque` are the device pair `r300_read_raw` expects.
    let ok = unsafe {
        match idx {
            Some(ix) => {
                let ind = super::dev_ns::R300Indices { dw: ix.as_ptr(), ndw: ix.len() as u32 };
                super::dev_ns::r300_draw_build_indexed(st, arrays, d.first().copied().unwrap_or(0), &ind, read, opaque, pkt, &mut err)
            }
            None => {
                let ndw = body_dw.min(d.len() as u32);
                super::dev_ns::r300_draw_build(st, arrays, opcode, d.as_ptr(), ndw, read, opaque, pkt, &mut err)
            }
        }
    };
    (ok, err)
}

/// Hands `pkt` to the renderer's `draw_r300` with the VRAM base. Negative when there is no
/// renderer or callback, or when the renderer rejects the draw.
pub fn renderer_draw_r300(s: &super::dev_ns::PPCMacGPUState, vram: &Vram, pkt: &mut super::dev_ns::R300DrawPacket) -> i32 {
    if s.renderer.is_null() {
        return -1;
    }
    let Some(f) = s.renderer_ref().draw_r300 else { return -1 };
    // SAFETY: the callback is paired with `renderer_opaque`; `vram` is the device's live RAM block
    // of `s.vram_size` bytes; `pkt` is a built packet borrowed for the call.
    unsafe { f(s.renderer_opaque, vram.as_mut_ptr(), s.vram_size, pkt) }
}
// END wG_render
