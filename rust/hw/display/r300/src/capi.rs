//! The C entry points of the crate: what `ppc_mac_gpu.c` and the renderers
//! call.  Raw pointers stop here; each function builds slices and calls the
//! safe code.
//!
//! The C original returned a `malloc`ed buffer that the caller `free`d.  Rust
//! cannot hand out memory the C allocator will accept, so the
//! caller owns the buffer instead: ask for the size, allocate, then convert.

use core::ffi::{c_char, c_void, CStr};
use core::{fmt, ptr, slice};
use std::ffi::CString;
use std::sync::Mutex;

use crate::draw::misc::{self, Arrays, MAX_ARRAYS};
use crate::draw::{draw_build, draw_build_indexed, DrawPacket, Indices};
use crate::state::{reg_name, R300State};
use crate::types::R300TexDesc;

/// A register write from MMIO or a PM4 type-0/1 packet.
///
/// # Safety
/// `st` must point to a valid `R300State`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_state_write(st: *mut R300State, addr: u32, val: u32) {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        (*st).write(addr, val);
    }
}

/// Name of the register at `addr` for traces, or NULL.  The string is static.
#[unsafe(no_mangle)]
pub extern "C" fn r300_reg_name(addr: u32) -> *const c_char {
    reg_name(addr).map_or(ptr::null(), CStr::as_ptr)
}

/// Receives the text of a state dump in pieces; the C caller turns it into a
/// `fwrite` to its `FILE *`.
pub type DumpWriteFn = unsafe extern "C" fn(ctx: *mut c_void, text: *const u8, len: usize);

struct CallbackWriter {
    write: DumpWriteFn,
    ctx: *mut c_void,
}

impl fmt::Write for CallbackWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // SAFETY: the caller of `r300_state_dump` guarantees the callback
        // accepts any (text, len) pair together with its `ctx`.
        unsafe { (self.write)(self.ctx, s.as_ptr(), s.len()) };
        Ok(())
    }
}

/// Dumps every non-zero register and the live part of PVS memory through
/// `write`.  Replaces the C `r300_state_dump(st, FILE *)`; the caller flushes.
///
/// # Safety
/// `st` must be valid and `write` must be safe to call with `ctx`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_state_dump(st: *const R300State, write: DumpWriteFn, ctx: *mut c_void) {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        // Writing to the callback cannot fail, so neither can the dump.
        let _ = (*st).dump(&mut CallbackWriter { write, ctx });
    }
}

// ---- small helpers -----------------------------------------------------------
// (Only what the C device calls is exported; the Vulkan crate uses the Rust API directly.)

/// `GB_AA_CONFIG`'s sample count (1 without antialiasing).
///
/// # Safety
/// `st` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_aa_samples(st: *const R300State) -> u32 {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        misc::aa_samples(&*st)
    }
}

/// Byte offset of a sample in a multisampled buffer, or `~0u`.
#[unsafe(no_mangle)]
pub extern "C" fn r300_msaa_offset(x: u32, y: u32, ns: u32, pitch_px: u32, bpp: u32, sample: u32) -> u32 {
    misc::msaa_offset(x, y, ns, pitch_px, bpp, sample).unwrap_or(u32::MAX)
}

/// Disassembles the active US program through `write`, for traces.
///
/// # Safety
/// `st` must be valid and `write` safe to call with `ctx`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_us_disasm(st: *const R300State, write: DumpWriteFn, ctx: *mut c_void) {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        let text = crate::us::disasm(&*st);
        write(ctx, text.as_ptr(), text.len());
    }
}

// ---- draw assembly -------------------------------------------------------------

/// `R300Arrays`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CArrays {
    pub count: u32,
    pub a: [CArray; MAX_ARRAYS],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CArray {
    pub addr: u32,
    pub size_dw: u32,
    pub stride_dw: u32,
}

/// `R300ColorDesc`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CColorDesc {
    pub gpu_addr: u32,
    pub pitch: u32,
    pub format: u32,
    pub bpp: u32,
    pub view: u32,
}

/// `R300DepthDesc`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CDepthDesc {
    pub attach: bool,
    pub gpu_addr: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub format: u32,
}

/// `R300DrawPacket`.  `glsl`, `verts` and `gpu_data` are owned by Rust: release
/// the packet with [`r300_draw_free`].  `tex[k].host_data` belongs to the
/// caller, which allocates and frees it.
#[repr(C)]
pub struct CDrawPacket {
    pub rt_gpu_addr: u32,
    pub rt_pitch: u32,
    pub rt_width: u32,
    pub rt_height: u32,
    pub rt_format: u32,
    pub rt_bpp: u32,
    pub rt_dummy: bool,
    pub harvest_hint: bool,
    pub depth_min: f32,
    pub depth_max: f32,
    pub depth_clip: bool,
    pub depth_dx: bool,
    pub rt_view: u32,
    pub scissor: [u32; 4],
    pub num_cb: u32,
    pub cb: [CColorDesc; crate::us::MAX_TARGETS],
    pub depth: CDepthDesc,
    pub cull: u32,
    pub aa_samples: u32,
    pub aa_pos: [[f32; 2]; 6],
    pub glsl: *mut c_char,
    pub uniforms: crate::us::R300FSUniforms,
    pub tex: [R300TexDesc; crate::us::NUM_TEX_UNITS],
    pub verts: *mut crate::us::R300Vertex,
    pub gpu_vs: bool,
    pub gpu_data: *mut u32,
    pub gpu_words: u32,
    pub num_verts: u32,
    pub num_line_verts: u32,
    pub prim_class: u32,
    pub warn: u32,
}

/// `R300Indices`.
#[repr(C)]
pub struct CIndices {
    pub dw: *const u32,
    pub ndw: u32,
}

/// `R300ReadFn`: reads guest GPU memory, false if the range is not mapped.
pub type CReadFn = unsafe extern "C" fn(opaque: *mut c_void, gpu_addr: u32, dst: *mut c_void, len: u32) -> bool;

fn arrays_from_c(a: &CArrays) -> Arrays {
    let mut out = Arrays { count: (a.count as usize).min(MAX_ARRAYS), ..Default::default() };
    for (o, c) in out.a.iter_mut().zip(&a.a) {
        *o = misc::ArrayDesc { addr: c.addr, size_dw: c.size_dw, stride_dw: c.stride_dw };
    }
    out
}

/// Error messages as C strings with static lifetime.
fn static_cstr(msg: &'static str) -> *const c_char {
    static CACHE: Mutex<Vec<(&'static str, CString)>> = Mutex::new(Vec::new());
    let mut cache = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, c)) = cache.iter().find(|(m, _)| *m == msg) {
        return c.as_ptr();
    }
    let c = CString::new(msg).unwrap_or_default();
    let p = c.as_ptr();
    cache.push((msg, c)); // the string's heap buffer does not move with the Vec
    p
}

/// Decodes a `3D_LOAD_VBPNTR` (type-3 0x2F) body.
///
/// # Safety
/// `arr` must be valid for writing and `d` readable for `ndw` dwords.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_load_vbpntr(arr: *mut CArrays, d: *const u32, ndw: u32) {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        let a = misc::load_vbpntr(if ndw == 0 { &[] } else { slice::from_raw_parts(d, ndw as usize) });
        let mut out = CArrays { count: a.count as u32, a: [CArray { addr: 0, size_dw: 0, stride_dw: 0 }; MAX_ARRAYS] };
        for (o, s) in out.a.iter_mut().zip(&a.a) {
            *o = CArray { addr: s.addr, size_dw: s.size_dw, stride_dw: s.stride_dw };
        }
        *arr = out;
    }
}

fn export_packet(p: DrawPacket, out: &mut CDrawPacket) {
    let verts = if p.verts.is_empty() { ptr::null_mut() } else { Box::into_raw(p.verts.into_boxed_slice()).cast() };
    let gpu_words = p.gpu_data.len() as u32;
    let gpu_data = if p.gpu_data.is_empty() { ptr::null_mut() } else { Box::into_raw(p.gpu_data.into_boxed_slice()).cast() };
    let glsl = CString::new(p.glsl).unwrap_or_default().into_raw();
    *out = CDrawPacket {
        rt_gpu_addr: p.rt_gpu_addr,
        rt_pitch: p.rt_pitch,
        rt_width: p.rt_width,
        rt_height: p.rt_height,
        rt_format: p.rt_format,
        rt_bpp: p.rt_bpp,
        rt_dummy: p.rt_dummy,
        harvest_hint: p.harvest_hint,
        depth_min: p.depth_min,
        depth_max: p.depth_max,
        depth_clip: p.depth_clip,
        depth_dx: p.depth_dx,
        rt_view: p.rt_view as u32,
        scissor: p.scissor,
        num_cb: p.num_cb,
        cb: p.cb.map(|c| CColorDesc { gpu_addr: c.gpu_addr, pitch: c.pitch, format: c.format, bpp: c.bpp, view: c.view }),
        depth: CDepthDesc {
            attach: p.depth.attach,
            gpu_addr: p.depth.gpu_addr,
            pitch: p.depth.pitch,
            bpp: p.depth.bpp,
            format: p.depth.format,
        },
        cull: p.cull,
        aa_samples: p.aa_samples,
        aa_pos: p.aa_pos,
        glsl,
        uniforms: p.uniforms,
        tex: p.tex,
        verts,
        gpu_vs: p.gpu_vs,
        gpu_data,
        gpu_words,
        num_verts: p.num_verts,
        num_line_verts: p.num_line_verts,
        prim_class: p.prim_class,
        warn: p.warn,
    };
}

fn empty_packet() -> CDrawPacket {
    let mut out = std::mem::MaybeUninit::<CDrawPacket>::uninit();
    // SAFETY: `export_packet` assigns every field of an owned packet.
    export_packet(DrawPacket::default(), unsafe { &mut *out.as_mut_ptr() });
    // SAFETY: initialised by `export_packet` above.
    unsafe { out.assume_init() }
}

/// Builds a draw from a `3D_DRAW_VBUF_2` (0x34), `3D_DRAW_IMMD_2` (0x35) or
/// `3D_DRAW_INDX_2` (0x36) body.  Returns false (with `*err` a static string)
/// when the draw cannot be assembled at all; smaller omissions are flagged in
/// `pkt->warn`.  On success the caller releases `pkt` with [`r300_draw_free`].
///
/// # Safety
/// All pointers must be valid; `d` readable for `ndw` dwords; `read` safe to
/// call with `opaque`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_draw_build(
    st: *const R300State,
    arr: *const CArrays,
    opcode: u32,
    d: *const u32,
    ndw: u32,
    read: CReadFn,
    opaque: *mut c_void,
    pkt: *mut CDrawPacket,
    err: *mut *const c_char,
) -> bool {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        *err = ptr::null();
        let body = if ndw == 0 { &[] } else { slice::from_raw_parts(d, ndw as usize) };
        let mut rd = |addr: u32, dst: &mut [u8]| read(opaque, addr, dst.as_mut_ptr().cast(), dst.len() as u32);
        finish(draw_build(&*st, &arrays_from_c(&*arr), opcode, body, &mut rd), pkt, err)
    }
}

/// The same for `DRAW_INDX_2` whose indices come from memory: `vf` is the
/// packet's `VAP_VF_CNTL` and `idx` the dwords `INDX_BUFFER` pointed at.
///
/// # Safety
/// As [`r300_draw_build`]; `idx.dw` must be readable for `idx.ndw` dwords.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_draw_build_indexed(
    st: *const R300State,
    arr: *const CArrays,
    vf: u32,
    idx: *const CIndices,
    read: CReadFn,
    opaque: *mut c_void,
    pkt: *mut CDrawPacket,
    err: *mut *const c_char,
) -> bool {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        *err = ptr::null();
        let idx = &*idx;
        let dw = if idx.ndw == 0 { &[] } else { slice::from_raw_parts(idx.dw, idx.ndw as usize) };
        let mut rd = |addr: u32, dst: &mut [u8]| read(opaque, addr, dst.as_mut_ptr().cast(), dst.len() as u32);
        finish(draw_build_indexed(&*st, &arrays_from_c(&*arr), vf, &Indices { dw }, &mut rd), pkt, err)
    }
}

unsafe fn finish(r: Result<DrawPacket, &'static str>, pkt: *mut CDrawPacket, err: *mut *const c_char) -> bool {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        match r {
            Ok(p) => {
                export_packet(p, &mut *pkt);
                true
            }
            Err(e) => {
                *pkt = empty_packet();
                *err = static_cstr(e);
                false
            }
        }
    }
}

/// Releases what [`r300_draw_build`] allocated: the shader text, the vertices
/// and the GPU vertex-stage buffer.  `tex[k].host_data` is the caller's.
///
/// # Safety
/// `pkt` must have come from `r300_draw_build*` and not been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn r300_draw_free(pkt: *mut CDrawPacket) {
    // SAFETY: the caller upholds the contract in the function documentation.
    unsafe {
        let p = &mut *pkt;
        if !p.glsl.is_null() {
            drop(CString::from_raw(p.glsl));
            p.glsl = ptr::null_mut();
        }
        if !p.verts.is_null() {
            let n = (p.num_verts + p.num_line_verts) as usize;
            drop(Box::from_raw(ptr::slice_from_raw_parts_mut(p.verts, n)));
            p.verts = ptr::null_mut();
        }
        if !p.gpu_data.is_null() {
            drop(Box::from_raw(ptr::slice_from_raw_parts_mut(p.gpu_data, p.gpu_words as usize)));
            p.gpu_data = ptr::null_mut();
        }
    }
}
