use super::*;
pub fn r300_reg(st: & R300State, addr: uint32_t) -> uint32_t {
    return st.regs[addr
        .wrapping_sub(0x1c00 as uint32_t)
        .wrapping_div(4 as uint32_t) as usize];
}
pub fn r300_state_owns(addr: uint32_t) -> bool {
    return addr >= 0x1c00 as uint32_t && addr < 0x5000 as uint32_t;
}
pub fn r300_front_ccw(cull: uint32_t) -> bool {
    return cull as ::core::ffi::c_uint & (1 as ::core::ffi::c_uint) << 2 as ::core::ffi::c_int
        == 0;
}
/// The C-ABI read callback the R300 draw builder calls: reads `len` bytes of guest memory at
/// `gpu_addr` into the buffer `dst` (see `r300_read_gpu_bytes`). `opaque` is the device state.
/// The signature is fixed by the C caller; the buffer is the one it allocated for `len` bytes.
pub unsafe extern "C" fn r300_read_raw(
    opaque: *mut ::core::ffi::c_void,
    gpu_addr: uint32_t,
    dst: *mut ::core::ffi::c_void,
    len: uint32_t,
) -> bool {
    // SAFETY: `opaque` is the device state QEMU registered with this callback.
    let Some(s) = (unsafe { crate::src::qemu::state_of_opaque(opaque) }) else {
        return false;
    };
    // SAFETY: the C caller passes `dst` as the `len`-byte buffer it allocated for this call, and
    // nothing else touches it until we return.
    let Some(out) = (unsafe { crate::src::qemu::c_buf_mut(dst, len as usize) }) else {
        return false;
    };
    r300_read_gpu_bytes(s, gpu_addr, out)
}
/// Reads `dst.len()` bytes of guest memory at GPU address `gpu_addr` into `dst`: from VRAM when
/// the range lies there, otherwise page by page through the GART/AGP tables. Returns false if a
/// page cannot be reached; the bytes read before that stay in `dst`.
pub fn r300_read_gpu_bytes(s: &mut PPCMacGPUState, mut gpu_addr: uint32_t, mut dst: &mut [u8]) -> bool {
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    if gpu_addr >= fb_base
        && (gpu_addr as uint64_t)
            .wrapping_sub(fb_base as uint64_t)
            .wrapping_add(dst.len() as uint64_t)
            <= s.vram_size
    {
        let off: uint64_t = gpu_addr.wrapping_sub(fb_base) as uint64_t;
        return crate::src::qemu::Vram::of(s).read_bytes(off, dst);
    }
    while !dst.is_empty() {
        let in_page: uint32_t = gpu_addr & 0xfff as uint32_t;
        let n: usize = (dst.len() as uint64_t).min((0x1000 as uint64_t).wrapping_sub(in_page as uint64_t)) as usize;
        let page: *mut uint8_t = gpu_agp_page(s, gpu_addr);
        if page.is_null() {
            return false;
        }
        let (head, rest) = dst.split_at_mut(n);
        if !crate::src::qemu::agp_page_read(s, page.wrapping_add(in_page as usize), head) {
            return false;
        }
        dst = rest;
        gpu_addr = gpu_addr.wrapping_add(n as uint32_t);
    }
    true
}
/// Writes the 4 bytes `bytes` to GPU address `gpu_addr`: into VRAM when it lies there (after
/// flushing the renderer), otherwise through the AGP or GART translation to guest memory.
/// Returns false if the address is mapped nowhere.
pub fn r300_write_raw(s: &mut PPCMacGPUState, gpu_addr: uint32_t, bytes: &[u8; 4]) -> bool {
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let mut phys: hwaddr = 0;
    if gpu_addr >= fb_base
        && (gpu_addr as uint64_t)
            .wrapping_sub(fb_base as uint64_t)
            .wrapping_add(4 as uint64_t)
            <= s.vram_size
    {
        if !s.renderer.is_null() && s.renderer_ref().flush_sync.is_some() {
            crate::src::qemu::renderer_flush_sync(s);
        }
        let off: uint64_t = gpu_addr.wrapping_sub(fb_base) as uint64_t;
        if !crate::src::qemu::Vram::of(s).write_bytes(off, bytes) {
            return false;
        }
        gpu_vram_dirty(s, off, 4 as uint64_t);
        return true;
    }
    if ppc_mac_gpu_agp_translate(s, gpu_addr, &mut phys)
        || ppc_mac_gpu_gart_translate(s, gpu_addr, &mut phys)
    {
        crate::src::qemu::guest_write(phys as u64, bytes);
        return true;
    }
    false
}
pub fn r300_swap_mode(
    v: uint32_t,
    mode: ::core::ffi::c_uint,
) -> uint32_t {
    match mode & 3 as ::core::ffi::c_uint {
        1 => {
            return (v & 0xff00ff as uint32_t) << 8 as ::core::ffi::c_int
                | v >> 8 as ::core::ffi::c_int & 0xff00ff as uint32_t;
        }
        2 => return v.swap_bytes(),
        3 => return v << 16 as ::core::ffi::c_int | v >> 16 as ::core::ffi::c_int,
        _ => return v,
    };
}
pub fn r300_zpass_log() -> bool {
    static on: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(-(1 as ::core::ffi::c_int));
    if on.get() < 0 as ::core::ffi::c_int {
        on.set(crate::src::qemu::env_flag(c"PPCGPU_ZPASS_LOG") as ::core::ffi::c_int);
    }
    return on.get() != 0;
}
pub fn r300_zpass_write(s: &mut PPCMacGPUState, addr: uint32_t, val: uint32_t) {
    if r300_zpass_log() {
        crate::src::logs::err(format_args!("ZPASS {} {:08x} active {}\n", crate::src::qemu::cstr(if addr == 0x4f58 as uint32_t {
                c"DATA".as_ptr()
            } else {
                c"ADDR".as_ptr()
            }), ((val) as u32), ((s.r3_zpass_active as ::core::ffi::c_int) as i32)));
    }
    if addr == 0x4f58 as uint32_t {
        crate::src::qemu::renderer_zpass(s, true, val);
        s.r3_zpass_active = true;
        return;
    }
    let dest: uint32_t = r300_reg(s.r3_ref(), 0x42c8 as uint32_t);
    let n: uint32_t = crate::src::qemu::renderer_zpass(s, false, 0).unwrap_or(0);
    let w: uint32_t = r300_swap_mode(
        if dest & 1 as uint32_t != 0 || dest == 0 {
            n
        } else {
            0 as uint32_t
        },
        r300_reg(s.r3_ref(), 0x4f24 as uint32_t) as ::core::ffi::c_uint >> 19 as ::core::ffi::c_int
            & 3 as ::core::ffi::c_uint,
    );
    let b: [uint8_t; 4] = [
        w as uint8_t,
        (w >> 8 as ::core::ffi::c_int) as uint8_t,
        (w >> 16 as ::core::ffi::c_int) as uint8_t,
        (w >> 24 as ::core::ffi::c_int) as uint8_t,
    ];
    s.r3_zpass_active = false;
    if !r300_write_raw(s, val & !(3 as uint32_t), &b) {
        crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: ZPASS_ADDR {:08x} not mapped\n", ((val) as u32)));
    }
}
pub fn r300_cb_samples(
    s: &mut PPCMacGPUState,
    off: uint32_t,
) -> uint32_t {
    let ns: uint32_t = crate::src::qemu::aa_samples(s.r3_ref());
    return if ns == 1 as uint32_t && off == s.r3_cb_aa_off {
        s.r3_cb_aa_ns
    } else {
        ns
    };
}
pub fn r300_zb_samples(
    s: &mut PPCMacGPUState,
    gpu_off: uint32_t,
) -> uint32_t {
    let ns: uint32_t = crate::src::qemu::aa_samples(s.r3_ref());
    return if ns == 1 as uint32_t && gpu_off == s.r3_zb_offset {
        {
            let mut _a43: ::core::ffi::c_uint = s.r3_zb_ns as ::core::ffi::c_uint;
            let mut _b44: ::core::ffi::c_uint = 1 as ::core::ffi::c_uint;
            if _a43 > _b44 {
                _a43 as ::core::ffi::c_uint
            } else {
                _b44 as ::core::ffi::c_uint
            }
        }
    } else {
        ns
    };
}
pub fn r300_zmask_clear(s: &mut PPCMacGPUState) {
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let mut off: uint32_t = r300_reg(s.r3_ref(), 0x4f20 as uint32_t) & !(0x1f as uint32_t);
    let pitch: uint32_t = r300_reg(s.r3_ref(), 0x4f24 as uint32_t);
    let bpp: uint32_t =
        (if r300_reg(s.r3_ref(), 0x4f10 as uint32_t) & 0xf as uint32_t == 2 as uint32_t {
            4 as ::core::ffi::c_int
        } else {
            2 as ::core::ffi::c_int
        }) as uint32_t;
    let mut rows: uint32_t =
        r300_reg(s.r3_ref(), 0x43e4 as uint32_t) >> 13 as ::core::ffi::c_int & 0x1fff as uint32_t;
    let v: uint32_t = r300_swap_mode(
        r300_reg(s.r3_ref(), 0x4f28 as uint32_t),
        pitch as ::core::ffi::c_uint >> 19 as ::core::ffi::c_int & 3 as ::core::ffi::c_uint,
    );
    let bpr: uint64_t =
        ((pitch & 0x3ffc as uint32_t) as uint64_t).wrapping_mul(bpp as uint64_t);
    let vram = crate::src::qemu::Vram::of(s);
    rows = if rows > 1440 as uint32_t {
        rows.wrapping_sub(1440 as uint32_t)
            .wrapping_add(1 as uint32_t)
    } else {
        0 as uint32_t
    };
    if s.r3_zb_offset == off && s.r3_zb_height > rows {
        rows = s.r3_zb_height;
    }
    rows = rows.wrapping_mul(r300_zb_samples(s, off));
    if off < fb_base || bpr == 0 || rows == 0 {
        return;
    }
    off = off.wrapping_sub(fb_base);
    if (off as uint64_t).wrapping_add(bpr.wrapping_mul(rows as uint64_t)) > s.vram_size {
        rows = s
            .vram_size
            .wrapping_sub(off as uint64_t)
            .wrapping_div(bpr) as uint32_t;
    }
    if !s.renderer.is_null() && s.renderer_ref().flush_sync.is_some() {
        crate::src::qemu::renderer_flush_sync(s);
    }
    let mut i: uint64_t = 0 as uint64_t;
    while i < bpr.wrapping_mul(rows as uint64_t) {
        if bpp == 4 as uint32_t {
            vram.put32(((off) as u64).wrapping_add((i) as u64), v);
        } else {
            vram.put16(((off) as u64).wrapping_add((i) as u64), v as uint16_t);
        }
        i = i.wrapping_add(bpp as uint64_t);
    }
    gpu_vram_dirty(s, off as uint64_t, bpr.wrapping_mul(rows as uint64_t));
    static logged: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
    let fresh13 = logged.get();
    logged.set(logged.get() + 1);
    if fresh13 < 4 as ::core::ffi::c_int {
        crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: 3D_CLEAR_ZMASK: depth at 0x{:x}, {} rows of {} bytes <- {:08x}\n", ((off) as u32), ((rows) as u32), ((bpr as ::core::ffi::c_ulonglong) as u64), ((r300_reg(s.r3_ref(), 0x4f28 as uint32_t)) as u32)));
    }
}
pub fn r300_cmask_clear(s: &mut PPCMacGPUState) {
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    let mut off: uint32_t = r300_reg(s.r3_ref(), 0x4e28 as uint32_t) & !(0x1f as uint32_t);
    let pitch: uint32_t = r300_reg(s.r3_ref(), 0x4e38 as uint32_t);
    let cf: uint32_t = pitch >> 21 as ::core::ffi::c_int & 0xf as uint32_t;
    let endian: uint32_t = pitch >> 19 as ::core::ffi::c_int & 3 as uint32_t;
    let mut rows: uint32_t =
        r300_reg(s.r3_ref(), 0x43e4 as uint32_t) >> 13 as ::core::ffi::c_int & 0x1fff as uint32_t;
    let mut v: uint32_t = r300_reg(s.r3_ref(), 0x4e14 as uint32_t);
    let vram = crate::src::qemu::Vram::of(s);
    let mut bpp: uint32_t = 0;
    let mut bpr: uint64_t = 0;
    match cf {
        6 => {
            bpp = 4 as uint32_t;
        }
        4 => {
            bpp = 2 as uint32_t;
            v = v >> 8 as ::core::ffi::c_int & 0xf800 as uint32_t
                | v >> 5 as ::core::ffi::c_int & 0x7e0 as uint32_t
                | v >> 3 as ::core::ffi::c_int & 0x1f as uint32_t;
        }
        _ => {
            if crate::src::qemu::loglevel_enabled(
                ((1 as ::core::ffi::c_uint) << 10 as ::core::ffi::c_int) as ::core::ffi::c_int,
            ) as ::core::ffi::c_int as ::core::ffi::c_long
                != 0
            {
                crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: 3D_CLEAR_CMASK on colour format {}\n", ((cf) as u32)));
            }
            return;
        }
    }
    bpr = ((pitch & 0x3ffe as uint32_t) as uint64_t).wrapping_mul(bpp as uint64_t);
    rows = if rows > 1440 as uint32_t {
        rows.wrapping_sub(1440 as uint32_t)
            .wrapping_add(1 as uint32_t)
    } else {
        0 as uint32_t
    };
    if off < fb_base || bpr == 0 || rows == 0 {
        return;
    }
    off = off.wrapping_sub(fb_base);
    rows = rows.wrapping_mul(r300_cb_samples(s, off));
    if (off as uint64_t).wrapping_add(bpr.wrapping_mul(rows as uint64_t)) > s.vram_size {
        rows = s
            .vram_size
            .wrapping_sub(off as uint64_t)
            .wrapping_div(bpr) as uint32_t;
    }
    if !s.renderer.is_null() && s.renderer_ref().flush_sync.is_some() {
        crate::src::qemu::renderer_flush_sync(s);
    }
    if bpp == 4 as uint32_t {
        let w: uint32_t = r300_swap_mode(v, endian as ::core::ffi::c_uint);
        let mut i: uint64_t = 0 as uint64_t;
        while i < bpr.wrapping_mul(rows as uint64_t) {
            vram.put32(((off) as u64).wrapping_add((i) as u64), w);
            i = i.wrapping_add(4 as uint64_t);
        }
    } else {
        let w_0: uint16_t = (if endian == 1 as uint32_t || endian == 2 as uint32_t {
            (v as ::core::ffi::c_ushort).swap_bytes() as uint32_t
        } else {
            v
        }) as uint16_t;
        let mut i_0: uint64_t = 0 as uint64_t;
        while i_0 < bpr.wrapping_mul(rows as uint64_t) {
            vram.put16(((off) as u64).wrapping_add((i_0) as u64), w_0);
            i_0 = i_0.wrapping_add(2 as uint64_t);
        }
    }
    gpu_vram_dirty(s, off as uint64_t, bpr.wrapping_mul(rows as uint64_t));
    static logged: crate::src::global::Global<::core::ffi::c_int> = crate::src::global::Global::<::core::ffi::c_int>::new(0);
    let fresh14 = logged.get();
    logged.set(logged.get() + 1);
    if fresh14 < 4 as ::core::ffi::c_int {
        crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: 3D_CLEAR_CMASK: colour at 0x{:x}, {} rows of {} bytes <- {:08x}\n", ((off) as u32), ((rows) as u32), ((bpr as ::core::ffi::c_ulonglong) as u64), ((v) as u32)));
    }
}
/// If the `len`-byte range at the GPU address `*addr` lies in VRAM, rewrites `*addr` to the VRAM
/// offset and returns true; otherwise leaves it alone and returns false.
pub fn r300_to_vram(s: &mut PPCMacGPUState, addr: &mut uint32_t, len: uint64_t) -> bool {
    let fb_base: uint32_t =
        (s.regs.mc_fb_location & 0xffff as uint32_t) << 16 as ::core::ffi::c_int;
    if *addr < fb_base
        || (*addr as uint64_t)
            .wrapping_sub(fb_base as uint64_t)
            .wrapping_add(len)
            > s.vram_size
    {
        return false;
    }
    *addr = (*addr).wrapping_sub(fb_base);
    true
}
/// Logs `what` (with an optional `detail`) the first time it is seen.
pub fn r300_warn_once(what: &str, detail: Option<&str>) {
    static SEEN: crate::src::qemu::SeenSet = crate::src::qemu::SeenSet::new();
    if SEEN.insert(what.to_owned()) {
        match detail {
            Some(d) => crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: {}: {}\n", what, d)),
            None => crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: {}\n", what)),
        }
    }
}
/// The R300 draw log: the file named by R300_DRAWLOG, created on first use.
struct DrawLogState {
    init: bool,
    file: Option<std::fs::File>,
    lines: u32,
}

static DRAWLOG: std::sync::Mutex<DrawLogState> = std::sync::Mutex::new(DrawLogState {
    init: false,
    file: None,
    lines: 0,
});

fn drawlog_state() -> std::sync::MutexGuard<'static, DrawLogState> {
    // A poisoned lock only means an earlier writer panicked; the state is still valid.
    DRAWLOG.lock().unwrap_or_else(|e| e.into_inner())
}

/// True if the draw log is open for this call. Each call on an open log counts one line; the
/// log closes after 200000 lines and stays closed.
pub fn r300_drawlog() -> bool {
    let mut st = drawlog_state();
    if !st.init {
        st.init = true;
        st.file = crate::src::qemu::env_string(c"R300_DRAWLOG")
            .and_then(|path| std::fs::File::create(path).ok());
    }
    if st.file.is_some() {
        st.lines = st.lines.wrapping_add(1);
        if st.lines > 200000 {
            st.file = None;
        }
    }
    st.file.is_some()
}

/// Flushes the draw log, if it is open.
pub fn r300_drawlog_flush() {
    if let Some(f) = drawlog_state().file.as_mut() {
        let _ = std::io::Write::flush(f);
    }
}
/// The text callback of the R300 register dump: writes `len` bytes at `text` to the `FILE` in
/// `ctx`. The signature is fixed by the C caller (QEMU's dump writer).
pub unsafe extern "C" fn r300_text_to_file(
    ctx: *mut ::core::ffi::c_void,
    text: *const uint8_t,
    len: size_t,
) {
    // SAFETY: QEMU's dump writer passes the `FILE` it opened as `ctx` and a `len`-byte text buffer.
    unsafe { crate::src::qemu::write_c_bytes(ctx as *mut FILE, text, len as usize) };
}
pub fn r300_draw_release(pkt: &mut R300DrawPacket) {
    for td in pkt.tex.iter_mut() {
        crate::src::qemu::c_free_slot(&mut td.host_data);
    }
    crate::src::qemu::r300_draw_free_packet(pkt);
}
/// Logs one R300 draw (register snapshot, vertex words, and the microcode disassembly when
/// R300_USDISASM is set) to the draw log. Does nothing unless the draw log is open.
pub fn r300_drawlog_draw(
    s: &PPCMacGPUState,
    opcode: uint32_t,
    d: &[uint32_t],
    body_dw: uint32_t,
) {
    if !r300_drawlog() {
        return;
    }
    let r: &R300State = s.r3_ref();
    let reg = |a: uint32_t| r300_reg(r, a);
    let vram = crate::src::qemu::Vram::of(s);
    let fb = (s.regs.mc_fb_location & 0xffff as uint32_t) << 16;
    // Non-zero words among `n` words at VRAM offset `base`; 0 if that range is not in VRAM
    // (the guest picks the offsets).
    let nonzero_words = |base: uint32_t, n: uint32_t| -> uint32_t {
        let end = base as u64 + 4 * n as u64;
        if end > s.vram_size || n > 1 << 20 {
            return 0;
        }
        (0..n as u64)
            .filter(|k| vram.get32(base as u64 + 4 * k).is_some_and(|w| w != 0))
            .count() as uint32_t
    };
    crate::src::logs::err(format_args!("D{} op{:02x} vf={:08x} n={} rt={:08x}/{:08x} zb={:x} zs={:08x} rm={:08x} zf={:x} zo={:08x} zp={:08x} bw={:x} cull={:x} pm={:x} vte={:x} vc={:x} cb={:08x} ab={:08x} cm={:x} af={:x} ten={:x} aa={:x} ms={:08x}/{:08x} res={:x} of={:08x}", r.draws, opcode, if body_dw != 0 { dw(d, 0) } else { 0 }, body_dw, reg(0x4e28), reg(0x4e38), reg(0x4f00), reg(0x4f04), reg(0x4f08), reg(0x4f10), reg(0x4f20), reg(0x4f24), reg(0x4f1c), reg(0x42b8), reg(0x4288), reg(0x20b0), reg(0x2140), reg(0x4e04), reg(0x4e08), reg(0x4e0c), reg(0x4bd4), reg(0x4104), reg(0x4020), reg(0x4010), reg(0x4014), reg(0x4e88), reg(0x46a4)));
    for t in 0..16u32 {
        if reg(0x4104) & (1 << t) == 0 {
            continue;
        }
        let xo = reg(0x4540 + 4 * t).wrapping_sub(fb);
        let wh = reg(0x4480 + 4 * t);
        let xw = (wh & 0x7ff).wrapping_add(1);
        let xh = (wh >> 11 & 0x7ff).wrapping_add(1);
        let xnz = nonzero_words(xo & !0xf, xw * xh);
        crate::src::logs::err(format_args!(" t{}={:08x}/{:08x}/{:08x}/{:08x}/{:08x} nz={}/{}", t, reg(0x4540 + 4 * t), reg(0x4480 + 4 * t), reg(0x44c0 + 4 * t), reg(0x4500 + 4 * t), reg(0x4400 + 4 * t), xnz, xw * xh));
    }
    crate::src::logs::err(format_args!(" rs={:08x}/{:08x} ip={:08x},{:08x},{:08x},{:08x} vo={:08x}/{:08x} ps={:08x}", reg(0x4300), reg(0x4304), reg(0x4310), reg(0x4314), reg(0x4318), reg(0x431c), reg(0x2090), reg(0x2094), reg(0x421c)));
    crate::src::logs::err(format_args!(" pt={:08x},{:08x},{:08x},{:08x} mm={:08x} pm={:08x} gb={:08x}", reg(0x4200), reg(0x4204), reg(0x4208), reg(0x420c), reg(0x4230), reg(0x4288), reg(0x4008)));
    let toff = reg(0x4540).wrapping_sub(fb);
    let tw = (reg(0x4480) & 0x7ff).wrapping_add(1);
    let th = (reg(0x4480) >> 11 & 0x7ff).wrapping_add(1);
    let nz = if reg(0x4104) & 1 != 0 { nonzero_words(toff, tw * th) } else { 0 };
    crate::src::logs::err(format_args!(" t0nz={}/{}", nz, tw * th));
    crate::src::logs::err(format_args!(" vtx:"));
    for k in 1..body_dw.min(25) {
        crate::src::logs::err(format_args!(" {:08x}", dw(d, k as usize)));
    }
    crate::src::logs::err(format_args!("\n"));
    if crate::src::qemu::env_flag(c"R300_USDISASM") {
        let dis = crate::src::qemu::r300_us_disasm_to_string(r);
        crate::src::logs::err(format_args!("{}\n", dis));
    }
    r300_drawlog_flush();
}
/// The formats already logged by `r300_note_formats`, at most 128 of them.
static SEEN_FORMATS: std::sync::Mutex<Vec<uint32_t>> = std::sync::Mutex::new(Vec::new());
const MAX_SEEN_FORMATS: usize = 128;

/// Logs the colour-buffer and texture formats of this draw the first time each is seen.
pub fn r300_note_formats(s: &mut PPCMacGPUState, pkt: &R300DrawPacket) {
    let mut keys: [uint32_t; 17] = [0; 17];
    let mut nk: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let fresh15 = nk;
    nk = nk.wrapping_add(1);
    keys[fresh15 as usize] = 0x80000000 as uint32_t
        | pkt.rt_format << 24 as ::core::ffi::c_int
        | (r300_reg(s.r3_ref(), 0x4e38 as uint32_t) >> 19 as ::core::ffi::c_int & 3 as uint32_t)
            << 20 as ::core::ffi::c_int
        | r300_reg(s.r3_ref(), 0x46a4 as uint32_t) & 0xf001f as ::core::ffi::c_int as uint32_t;
    let mut t: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while t < 16 as ::core::ffi::c_int {
        let td: & R300TexDesc = & pkt.tex[(t) as usize];
        let f1: uint32_t = r300_reg(
            s.r3_ref(),
            (0x44c0 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
        );
        if td.bound {
            let fresh16 = nk;
            nk = nk.wrapping_add(1);
            keys[fresh16 as usize] = (f1 & 0x1f as uint32_t) << 24 as ::core::ffi::c_int
                | f1 & 0x1e0 as uint32_t
                | (r300_reg(
                    s.r3_ref(),
                    (0x4540 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                ) & 3 as uint32_t)
                    << 20 as ::core::ffi::c_int
                | (f1 >> 21 as ::core::ffi::c_int & 1 as uint32_t) << 19 as ::core::ffi::c_int
                | (f1 >> 22 as ::core::ffi::c_int & 1 as uint32_t) << 18 as ::core::ffi::c_int
                | (if !td.host_data.is_null() {
                    (1 as uint32_t) << 17 as ::core::ffi::c_int
                } else {
                    0 as uint32_t
                })
                | r300_reg(
                    s.r3_ref(),
                    (0x4400 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
                ) >> 9 as ::core::ffi::c_int
                    & 0xf as uint32_t;
        }
        t += 1;
    }
    let mut i: usize = 0;
    while i < nk as usize {
        let key = keys[i];
        let mut seen = SEEN_FORMATS.lock().unwrap_or_else(|e| e.into_inner());
        if seen.len() >= MAX_SEEN_FORMATS {
            break;
        }
        if !seen.contains(&key) {
            seen.push(key);
            drop(seen);
            if key & 0x80000000 != 0 {
                crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: colour buffer format {} endian {} US_OUT_FMT {:05x} ({}x{} at {:08x})\n", pkt.rt_format, (key >> 20 & 3), key & 0xf001f, pkt.rt_pitch, pkt.rt_height, pkt.rt_gpu_addr));
            } else {
                let space = if key >> 17 & 1 != 0 { "GART" } else { "VRAM" };
                crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: texture format 0x{:02x} swap {} sign {:x} gamma {} yuv {} {} filter {:x}\n", key >> 24, key >> 20 & 3, key >> 5 & 0xf, key >> 19 & 1, key >> 18 & 1, space, key & 0xf));
            }
        }
        i += 1;
    }
}
pub fn r300_note_features(s: &mut PPCMacGPUState, pkt: &R300DrawPacket) {
    let r: &R300State = s.r3_ref();
    static seen: crate::src::global::Global<uint32_t> = crate::src::global::Global::<uint32_t>::new(0);
    let mut f: uint32_t = 0 as uint32_t;
    const NAMES: [&str; 22] = [
        "mipmapped texture",
        "3D texture",
        "cube map",
        "border colour wrap",
        "anisotropic filter",
        "LOD bias",
        "fog",
        "polygon mode",
        "polygon offset",
        "wide lines",
        "line stipple",
        "flat/solid shading",
        "two-sided colours",
        "user clip planes",
        "multiple render targets",
        "logic op",
        "depth from the fragment program",
        "discard src pixels",
        "vertex program flow control",
        "per-vertex point size",
        "signed texture",
        "sRGB texture",
    ];
    let mut t: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while t < 16 as ::core::ffi::c_int {
        let td: & R300TexDesc = & pkt.tex[(t) as usize];
        let f0: uint32_t = td.filter0;
        let f1: uint32_t = r300_reg(
            r,
            (0x44c0 as ::core::ffi::c_int + 4 as ::core::ffi::c_int * t) as uint32_t,
        );
        if td.bound {
            f |= (((td.levels > 1 as uint32_t) as ::core::ffi::c_int) << 0 as ::core::ffi::c_int
                | ((td.dim == R300_TEXDIM_3D as ::core::ffi::c_int as uint32_t)
                    as ::core::ffi::c_int)
                    << 1 as ::core::ffi::c_int
                | ((td.dim == R300_TEXDIM_CUBE as ::core::ffi::c_int as uint32_t)
                    as ::core::ffi::c_int)
                    << 2 as ::core::ffi::c_int) as uint32_t;
            let mut a: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while a < 3 as ::core::ffi::c_int {
                let m: ::core::ffi::c_uint = f0 as ::core::ffi::c_uint
                    >> 3 as ::core::ffi::c_int * a
                    & 7 as ::core::ffi::c_uint;
                f |= (((m == 6 as ::core::ffi::c_uint || m == 7 as ::core::ffi::c_uint)
                    as ::core::ffi::c_int)
                    << 3 as ::core::ffi::c_int) as uint32_t;
                a += 1;
            }
            f |= (((f0 >> 9 as ::core::ffi::c_int & 3 as uint32_t == 3 as uint32_t
                || f0 >> 11 as ::core::ffi::c_int & 3 as uint32_t == 3 as uint32_t)
                as ::core::ffi::c_int)
                << 4 as ::core::ffi::c_int) as uint32_t;
            f |= (((td.filter1 >> 3 as ::core::ffi::c_int & 0x3ff as uint32_t != 0 as uint32_t)
                as ::core::ffi::c_int)
                << 5 as ::core::ffi::c_int) as uint32_t;
            f |= (((f1 >> 5 as ::core::ffi::c_int & 0xf as uint32_t != 0 as uint32_t)
                as ::core::ffi::c_int)
                << 20 as ::core::ffi::c_int) as uint32_t
                | (f1 >> 21 as ::core::ffi::c_int & 1 as uint32_t) << 21 as ::core::ffi::c_int;
        }
        t += 1;
    }
    f |= (r300_reg(r, 0x4bc0 as uint32_t) & 1 as uint32_t) << 6 as ::core::ffi::c_int;
    f |= (((r300_reg(r, 0x4288 as uint32_t) & 3 as uint32_t == 1 as uint32_t)
        as ::core::ffi::c_int)
        << 7 as ::core::ffi::c_int) as uint32_t;
    f |= (((r300_reg(r, 0x42b4 as uint32_t) & 3 as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int)
        << 8 as ::core::ffi::c_int) as uint32_t;
    f |= (((r300_reg(r, 0x4234 as uint32_t) & 0xffff as uint32_t > 9 as uint32_t)
        as ::core::ffi::c_int)
        << 9 as ::core::ffi::c_int) as uint32_t;
    f |= (((r300_reg(r, 0x4238 as uint32_t) as ::core::ffi::c_uint & !(3 as ::core::ffi::c_uint)
        != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int)
        << 10 as ::core::ffi::c_int) as uint32_t;
    let mut c: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while c < 4 as ::core::ffi::c_int {
        let cc: uint32_t = r300_reg(r, 0x4278 as uint32_t);
        f |= (((cc >> 4 as ::core::ffi::c_int * c & 3 as uint32_t != 2 as uint32_t
            || cc >> 4 as ::core::ffi::c_int * c + 2 as ::core::ffi::c_int & 3 as uint32_t
                != 2 as uint32_t) as ::core::ffi::c_int)
            << 11 as ::core::ffi::c_int) as uint32_t;
        c += 1;
    }
    f |= (((r300_reg(r, 0x2090 as uint32_t) as ::core::ffi::c_uint
        & (3 as ::core::ffi::c_uint) << 3 as ::core::ffi::c_int
        != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int)
        << 12 as ::core::ffi::c_int) as uint32_t;
    f |= (((r300_reg(r, 0x221c as uint32_t) & 0x3f as uint32_t != 0 as uint32_t)
        as ::core::ffi::c_int)
        << 13 as ::core::ffi::c_int) as uint32_t;
    f |= (((pkt.num_cb > 1 as uint32_t) as ::core::ffi::c_int) << 14 as ::core::ffi::c_int)
        as uint32_t;
    f |= (r300_reg(r, 0x4e18 as uint32_t) >> 2 as ::core::ffi::c_int & 1 as uint32_t)
        << 15 as ::core::ffi::c_int;
    f |= (((r300_reg(r, 0x4bd8 as uint32_t) & 1 as uint32_t != 0
        && crate::src::qemu::c_arg(pkt.glsl).is_some_and(|g| String::from_utf8_lossy(g.to_bytes()).contains("ow = ar"))) as ::core::ffi::c_int)
        << 16 as ::core::ffi::c_int) as uint32_t;
    f |= (((r300_reg(r, 0x4e04 as uint32_t) & 1 as uint32_t != 0
        && r300_reg(r, 0x4e04 as uint32_t) >> 3 as ::core::ffi::c_int & 7 as uint32_t != 0)
        as ::core::ffi::c_int)
        << 17 as ::core::ffi::c_int) as uint32_t;
    f |= (((r300_reg(r, 0x22dc as uint32_t) != 0 as uint32_t) as ::core::ffi::c_int)
        << 18 as ::core::ffi::c_int) as uint32_t;
    f |= (r300_reg(r, 0x2090 as uint32_t) >> 16 as ::core::ffi::c_int & 1 as uint32_t)
        << 19 as ::core::ffi::c_int;
    f &= !seen.get();
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while f != 0 {
        if f & 1 as uint32_t != 0 {
            seen.set(seen.get() | (1 as ::core::ffi::c_uint) << i);
            crate::src::logs::qemu(format_args!("ppc-mac-gpu r300: first use of {} (draw {})\n", NAMES[i as usize], ((r.draws as ::core::ffi::c_ulonglong) as u64)));
        }
        i = i.wrapping_add(1);
        f >>= 1 as ::core::ffi::c_int;
    }
}
pub fn r300_aa_resolve(s: &mut PPCMacGPUState, pkt: &R300DrawPacket) {
    let ctl: u32 = r300_reg(s.r3_ref(), 0x4e88);
    let mut dst: u32 = r300_reg(s.r3_ref(), 0x4e80) & !0x1f;
    let dpitch: u32 = (r300_reg(s.r3_ref(), 0x4e84) & 0x3ffe).wrapping_mul(4);
    let spitch: u32 = pkt.rt_pitch.wrapping_mul(pkt.rt_bpp);
    // r300_cb_samples, inlined so that no FFI callee is needed here.
    let ns: u32 = {
        let n = crate::src::qemu::aa_samples(s.r3_ref());
        if n == 1 && pkt.rt_gpu_addr == s.r3_cb_aa_off {
            s.r3_cb_aa_ns
        } else {
            n
        }
    };
    let srow: u64 = u64::from(spitch).wrapping_mul(u64::from(ns));
    let mut x0 = pkt.rt_width as f32;
    let mut y0 = pkt.rt_height as f32;
    let mut x1 = 0.0f32;
    let mut y1 = 0.0f32;
    for v in pkt.verts() {
        let p = &v.pos;
        let w = if p[3] != 0.0 { p[3] } else { 1.0 };
        let x = (p[0] / w + 1.0) * pkt.rt_width as f32 * 0.5;
        let y = (1.0 - p[1] / w) * pkt.rt_height as f32 * 0.5;
        // `a < b ? a : b` and `a > b ? a : b` rather than f32::min/max, so NaN coordinates
        // select the same operand as before.
        x0 = if x0 < x { x0 } else { x };
        x1 = if x1 > x { x1 } else { x };
        y0 = if y0 < y { y0 } else { y };
        y1 = if y1 > y { y1 } else { y };
    }
    let ix0 = ((x0 + 0.5).floor() as i32).max(pkt.scissor[0] as i32);
    let iy0 = ((y0 + 0.5).floor() as i32).max(pkt.scissor[1] as i32);
    let ix1 = ((x1 + 0.5).floor() as i32).min(pkt.scissor[2].min(pkt.rt_width) as i32);
    let iy1 = ((y1 + 0.5).floor() as i32).min(pkt.scissor[3].min(pkt.rt_height) as i32);
    let w_0: u32 = if ix1 > ix0 {
        (i64::from(ix1) - i64::from(ix0)) as u32
    } else {
        0
    };
    let h: u32 = if iy1 > iy0 {
        (i64::from(iy1) - i64::from(iy0)) as u32
    } else {
        0
    };
    if w_0 == 0 || h == 0 || dpitch == 0 || (ix1 as u32).wrapping_mul(4) > dpitch {
        return;
    }
    // r300_to_vram, inlined: `dst` becomes a VRAM offset if the whole span lies in VRAM.
    let fb_base: u32 = (s.regs.mc_fb_location & 0xffff) << 16;
    let dst_span: u64 = u64::from(dpitch).wrapping_mul(iy1 as u64);
    let dst_in_vram = dst >= fb_base
        && (u64::from(dst) - u64::from(fb_base)).wrapping_add(dst_span) <= s.vram_size;
    if !dst_in_vram {
        r300_warn_once("AA resolve buffer outside VRAM", None);
        return;
    }
    dst = dst.wrapping_sub(fb_base);
    if u64::from(pkt.rt_gpu_addr).wrapping_add(srow.wrapping_mul(iy1 as u64)) > s.vram_size {
        r300_warn_once("multisampled colour buffer outside VRAM", None);
        return;
    }
    if !s.renderer.is_null() && s.renderer_ref().flush_sync.is_some() {
        let _bql = crate::src::qemu::BqlGuard::acquire();
        crate::src::qemu::renderer_flush_sync(s);
    }
    let avg: bool = ns > 1 && pkt.rt_bpp == 4 && pkt.rt_format == 6;
    if ns > 1 && !avg {
        r300_warn_once("AA resolve of a colour format other than 8888: sample 0 kept", None);
    }
    let vram = crate::src::qemu::Vram::of(s);
    // ix0 * 4 as a wrapping u64 offset (same bits as the C sign-extended value).
    let col: u64 = (ix0 as u64).wrapping_mul(4);
    if !avg {
        let n = (w_0 as usize).wrapping_mul(4);
        for y in iy0..iy1 {
            let yu = y as u64;
            let src = u64::from(pkt.rt_gpu_addr)
                .wrapping_add(yu.wrapping_mul(srow))
                .wrapping_add(col);
            let d = u64::from(dst)
                .wrapping_add(yu.wrapping_mul(u64::from(dpitch)))
                .wrapping_add(col);
            // The checked copy refuses spans outside VRAM instead of reading past them.
            vram.copy_within(src, d, n);
        }
    } else {
        static GAMMA: std::sync::OnceLock<([f32; 256], [u8; 4096])> = std::sync::OnceLock::new();
        let gamma = ctl & 2 != 0;
        let alpha_avg = ctl & 4 != 0;
        let amask: u32 = r300_swap_mode(
            0xff00_0000,
            (r300_reg(s.r3_ref(), 0x4e38) >> 19) & 3,
        );
        let ab: u32 = amask.trailing_zeros() / 8;
        let tables = gamma.then(|| {
            GAMMA.get_or_init(|| {
                let mut degamma = [0.0f32; 256];
                for (i, d) in degamma.iter_mut().enumerate() {
                    *d = (i as f32 / 255.0f32).powf(2.2f32);
                }
                let mut regamma = [0u8; 4096];
                for (i, r) in regamma.iter_mut().enumerate() {
                    let v = 255.0f32 * (i as f32 / 4095.0f32).powf(1.0f32 / 2.2f32);
                    *r = v.round_ties_even() as i64 as u8;
                }
                (degamma, regamma)
            })
        });
        for y in iy0..iy1 {
            let yu = y as u64;
            let sp_row = u64::from(pkt.rt_gpu_addr)
                .wrapping_add(yu.wrapping_mul(srow))
                .wrapping_add(col);
            let dp_row = u64::from(dst)
                .wrapping_add(yu.wrapping_mul(u64::from(dpitch)))
                .wrapping_add(col);
            for x in 0..u64::from(w_0) {
                let sp = sp_row.wrapping_add(x * 4);
                let dp = dp_row.wrapping_add(x * 4);
                for c in 0..4u32 {
                    let c_off = u64::from(c);
                    let value: Option<u8> = if c == ab && !alpha_avg {
                        vram.get8(sp.wrapping_add(c_off))
                    } else if let (Some((degamma, regamma)), true) = (tables, c != ab) {
                        let mut sum = 0.0f32;
                        for k in 0..ns {
                            let off = k.wrapping_mul(spitch).wrapping_add(c);
                            if let Some(v) = vram.get8(sp.wrapping_add(u64::from(off))) {
                                sum += degamma[usize::from(v)];
                            }
                        }
                        let idx = (sum / ns as f32 * 4095.0f32).round_ties_even() as i64;
                        let out = usize::try_from(idx)
                            .ok()
                            .and_then(|i| regamma.get(i))
                            .copied()
                            .unwrap_or(0);
                        Some(out)
                    } else {
                        let mut sum: u32 = ns / 2;
                        for k in 0..ns {
                            let off = k.wrapping_mul(spitch).wrapping_add(c);
                            if let Some(v) = vram.get8(sp.wrapping_add(u64::from(off))) {
                                sum = sum.wrapping_add(u32::from(v));
                            }
                        }
                        sum.checked_div(ns).map(|v| v as u8)
                    };
                    if let Some(v) = value {
                        vram.put8(dp.wrapping_add(c_off), v);
                    }
                }
            }
        }
    }
    gpu_vram_dirty(
        s,
        u64::from(dst).wrapping_add((iy0 as u64).wrapping_mul(u64::from(dpitch))),
        u64::from(dpitch).wrapping_mul(u64::from(h)),
    );
    static LOGGED: crate::src::global::Global<i32> = crate::src::global::Global::<i32>::new(0);
    let n = LOGGED.get();
    LOGGED.set(n.wrapping_add(1));
    if n < 3 {
        crate::src::logs::qemu(format_args!(
            "ppc-mac-gpu r300: AA resolve {:06x}/{} -> {:06x}/{} ({},{} {}x{}), {} samples{}\n",
            pkt.rt_gpu_addr,
            spitch,
            dst,
            dpitch,
            ix0,
            iy0,
            w_0,
            h,
            ns,
            if avg && ctl & 2 != 0 { ", gamma 2.2" } else { "" }
        ));
    }
}
