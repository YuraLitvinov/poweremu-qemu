//! Vulkan renderer for the emulated ATI Radeon's R300 3D engine.
//!
//! This implements the interface in `ppc_mac_gpu_renderer.h`: the device
//! asks for VRAM from [`ppc_mac_gpu_vulkan_alloc_vram`], creates the
//! renderer with [`ppc_mac_gpu_renderer_vulkan`], and hands it the draw
//! packets `r300/r300_draw.c` assembles.  It was ported from the C backend
//! `ppc_mac_gpu_vulkan.c` (kept beside it, unbuilt, as the reference) and
//! uses `ash`.  Every C type comes from bindgen over QEMU's own headers.

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case, dead_code, clippy::all)]

mod batch;
mod cache;
mod ctx;
mod draw;
mod pipe;
mod record;
mod state;
mod tex;

#[allow(clippy::all)]
pub mod sys {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::ffi::{c_char, c_int, c_void, CString};

use ash::vk;

use state::*;
use sys::*;

#[no_mangle]
pub unsafe extern "C" fn ppc_mac_gpu_vulkan_alloc_vram(vram_size: u64, opaque_out: *mut *mut c_void) -> *mut c_void {
    let v = match vk_opt() {
        Some(v) => v,
        None => match Vk::create() {
            Ok(v) => v,
            Err(e) => {
                fail(&e);
                return std::ptr::null_mut();
            }
        },
    };
    match v.alloc_vram(vram_size) {
        Ok(p) => {
            *opaque_out = v as *mut Vk as *mut c_void;
            p as *mut c_void
        }
        Err(e) => {
            fail(&e);
            std::ptr::null_mut()
        }
    }
}

/// VRAM lives as long as the process: images and batches refer to it.
#[no_mangle]
pub unsafe extern "C" fn ppc_mac_gpu_vulkan_free_vram(_opaque: *mut c_void) {}

#[no_mangle]
pub unsafe extern "C" fn ppc_mac_gpu_vulkan_error() -> *const c_char {
    let e = last_error();
    let e = if e.is_empty() { "unknown Vulkan error".to_string() } else { e };
    // Leaked on purpose: the caller keeps the pointer for an error message.
    CString::new(e).unwrap_or_default().into_raw()
}

unsafe extern "C" fn r_init(vram_ptr: *mut u8, _vram_size: u64) -> *mut c_void {
    let Some(v) = vk_opt() else {
        fail("VRAM was not allocated by the Vulkan backend");
        return std::ptr::null_mut();
    };
    if v.vram.is_null() || vram_ptr != v.vram {
        fail("VRAM was not allocated by the Vulkan backend");
        return std::ptr::null_mut();
    }
    if v.zpass_buf == vk::Buffer::null() {
        match v.buffer(
            16,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            vk::MemoryPropertyFlags::empty(),
        ) {
            Ok((b, m, p)) => {
                v.zpass_buf = b;
                v.zpass_mem = m;
                v.zpass = p as *mut u32;
                std::ptr::write_bytes(p, 0, 16);
            }
            Err(e) => {
                fail(&e);
                return std::ptr::null_mut();
            }
        }
    }
    v as *mut Vk as *mut c_void
}

unsafe extern "C" fn r_fini(opaque: *mut c_void) {
    if let Some(v) = vk_opt() {
        let _ = opaque;
        v.flush();
    }
}

unsafe extern "C" fn r_draw_r300(
    _opaque: *mut c_void,
    vram_ptr: *mut u8,
    _vram_size: u64,
    pkt: *const R300DrawPacket,
) -> c_int {
    match vk_opt() {
        Some(v) => v.draw_r300(vram_ptr, &*pkt),
        None => -1,
    }
}

unsafe extern "C" fn r_zpass_r300(_opaque: *mut c_void, reset: bool, value: u32) -> u32 {
    let Some(v) = vk_opt() else {
        return 0;
    };
    if v.zpass.is_null() {
        return 0;
    }
    v.flush();
    let r = *v.zpass;
    if reset {
        *v.zpass = value;
    }
    r
}

unsafe extern "C" fn r_flush_r200(_opaque: *mut c_void) -> bool {
    match vk_opt() {
        Some(v) => v.flush(),
        None => false,
    }
}

unsafe extern "C" fn r_submit_r200(
    _opaque: *mut c_void,
    done: Option<unsafe extern "C" fn(*mut c_void, u32)>,
    arg: *mut c_void,
) -> u32 {
    match vk_opt() {
        Some(v) => v.commit(done, arg),
        None => 0,
    }
}

unsafe extern "C" fn r_range_busy_r200(_opaque: *mut c_void, lo: u64, hi: u64, write_access: bool) -> bool {
    match vk_opt() {
        Some(v) => v.range_busy(lo, hi, write_access),
        None => false,
    }
}

unsafe extern "C" fn r_set_dirty_source(_opaque: *mut c_void, f: PPCMacGPUDirtyFn, arg: *mut c_void) {
    if let Some(v) = vk_opt() {
        v.dirty_fn = f;
        v.dirty_arg = arg;
    }
}

unsafe extern "C" fn r_get_caps(_opaque: *mut c_void) -> u32 {
    (PPC_MAC_GPU_RENDERER_CAP_3D | PPC_MAC_GPU_RENDERER_CAP_ACCEL) as u32
}

#[no_mangle]
pub unsafe extern "C" fn ppc_mac_gpu_renderer_vulkan() -> *mut PPCMacGPURenderer {
    let mut r: PPCMacGPURenderer = std::mem::zeroed();
    r.name = c"vulkan".as_ptr();
    let fb_image = match vk_opt() {
        Some(v) => v.fb_image,
        None => match Vk::create() {
            Ok(v) => v.fb_image,
            Err(e) => {
                fail(&e);
                false
            }
        },
    };
    r.r300_glsl_flags = R300_GLSL_VRAM_SSBO | if fb_image { R300_GLSL_FB_IMAGE } else { 0 };
    r.init = Some(r_init);
    r.fini = Some(r_fini);
    r.draw_r300 = Some(r_draw_r300);
    r.zpass_r300 = Some(r_zpass_r300);
    r.flush_r200 = Some(r_flush_r200);
    r.submit_r200 = Some(r_submit_r200);
    r.range_busy_r200 = Some(r_range_busy_r200);
    r.set_dirty_source = Some(r_set_dirty_source);
    r.get_caps = Some(r_get_caps);
    Box::into_raw(Box::new(r))
}
