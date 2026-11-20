//! Foreign libraries: the only module that declares `extern "C"` functions
//! the crate imports.  Today that is shaderc (GLSL to SPIR-V); the safe
//! wrapper [`compile_glsl`] is what the rest of the crate uses.

use core::ffi::{c_char, c_int, c_uint, CStr};
use core::ptr::NonNull;
use std::sync::Mutex;

#[repr(C)]
struct CompilerRaw {
    _private: [u8; 0],
}
#[repr(C)]
struct OptionsRaw {
    _private: [u8; 0],
}
#[repr(C)]
struct ResultRaw {
    _private: [u8; 0],
}

// Values from shaderc/shaderc.h, env.h and status.h.
const SHADERC_VERTEX_SHADER: c_int = 0;
const SHADERC_FRAGMENT_SHADER: c_int = 1;
const SHADERC_TARGET_ENV_VULKAN: c_int = 0;
const SHADERC_ENV_VERSION_VULKAN_1_1: c_uint = (1 << 22) | (1 << 12);
const SHADERC_SPIRV_VERSION_1_3: c_uint = 0x0001_0300;
const SHADERC_COMPILATION_STATUS_SUCCESS: c_int = 0;

#[link(name = "shaderc_shared")]
extern "C" {
    fn shaderc_compiler_initialize() -> *mut CompilerRaw;
    fn shaderc_compile_options_initialize() -> *mut OptionsRaw;
    fn shaderc_compile_options_release(options: *mut OptionsRaw);
    fn shaderc_compile_options_set_target_env(options: *mut OptionsRaw, target: c_int, version: c_uint);
    fn shaderc_compile_options_set_target_spirv(options: *mut OptionsRaw, version: c_uint);
    fn shaderc_compile_options_add_macro_definition(
        options: *mut OptionsRaw,
        name: *const c_char,
        name_length: usize,
        value: *const c_char,
        value_length: usize,
    );
    fn shaderc_compile_into_spv(
        compiler: *const CompilerRaw,
        source_text: *const c_char,
        source_text_size: usize,
        shader_kind: c_int,
        input_file_name: *const c_char,
        entry_point_name: *const c_char,
        options: *const OptionsRaw,
    ) -> *mut ResultRaw;
    fn shaderc_result_release(result: *mut ResultRaw);
    fn shaderc_result_get_length(result: *const ResultRaw) -> usize;
    fn shaderc_result_get_bytes(result: *const ResultRaw) -> *const c_char;
    fn shaderc_result_get_compilation_status(result: *const ResultRaw) -> c_int;
    fn shaderc_result_get_error_message(result: *const ResultRaw) -> *const c_char;
}

/// The shader stage a compilation is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderKind {
    Vertex,
    Fragment,
}

struct Compiler(NonNull<CompilerRaw>);

// SAFETY: a shaderc compiler is not tied to the thread that made it, and every
// use goes through the `Mutex` below.
unsafe impl Send for Compiler {}

static COMPILER: Mutex<Option<Compiler>> = Mutex::new(None);

/// Owns the options object so that it is released on every path.
struct Options(NonNull<OptionsRaw>);

impl Drop for Options {
    fn drop(&mut self) {
        // SAFETY: made by `shaderc_compile_options_initialize`, released once.
        unsafe { shaderc_compile_options_release(self.0.as_ptr()) };
    }
}

/// Owns a compilation result.
struct CompileResult(NonNull<ResultRaw>);

impl Drop for CompileResult {
    fn drop(&mut self) {
        // SAFETY: made by `shaderc_compile_into_spv`, released once.
        unsafe { shaderc_result_release(self.0.as_ptr()) };
    }
}

/// Compiles GLSL `source` to SPIR-V 1.3 for Vulkan 1.1.  `defines` are
/// defined to `1`; `file_name` is what error messages name.  The error is the
/// compiler's message.
pub fn compile_glsl(source: &str, kind: ShaderKind, file_name: &CStr, defines: &[&str]) -> Result<Vec<u32>, String> {
    let mut guard = COMPILER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_none() {
        // SAFETY: no preconditions.
        let raw = unsafe { shaderc_compiler_initialize() };
        *guard = Some(Compiler(NonNull::new(raw).ok_or("shaderc: cannot initialise the compiler")?));
    }
    let compiler = guard.as_ref().expect("initialised above").0;

    // SAFETY: no preconditions.
    let options = Options(NonNull::new(unsafe { shaderc_compile_options_initialize() }).ok_or("shaderc: out of memory")?);
    // SAFETY: `options` is live; the strings are valid for the stated lengths
    // and shaderc copies them.
    unsafe {
        shaderc_compile_options_set_target_env(options.0.as_ptr(), SHADERC_TARGET_ENV_VULKAN, SHADERC_ENV_VERSION_VULKAN_1_1);
        shaderc_compile_options_set_target_spirv(options.0.as_ptr(), SHADERC_SPIRV_VERSION_1_3);
        for name in defines {
            shaderc_compile_options_add_macro_definition(
                options.0.as_ptr(),
                name.as_ptr().cast(),
                name.len(),
                c"1".as_ptr(),
                1,
            );
        }
    }
    let shader_kind = match kind {
        ShaderKind::Vertex => SHADERC_VERTEX_SHADER,
        ShaderKind::Fragment => SHADERC_FRAGMENT_SHADER,
    };
    // SAFETY: the compiler and options are live, `source` is valid for its
    // length, and the names are NUL-terminated.
    let result = unsafe {
        shaderc_compile_into_spv(
            compiler.as_ptr(),
            source.as_ptr().cast(),
            source.len(),
            shader_kind,
            file_name.as_ptr(),
            c"main".as_ptr(),
            options.0.as_ptr(),
        )
    };
    let result = CompileResult(NonNull::new(result).ok_or("shaderc: out of memory")?);
    // SAFETY: `result` is live; shaderc returns a NUL-terminated message and a
    // byte buffer of the reported length that stays valid until the release.
    unsafe {
        if shaderc_result_get_compilation_status(result.0.as_ptr()) != SHADERC_COMPILATION_STATUS_SUCCESS {
            let msg = shaderc_result_get_error_message(result.0.as_ptr());
            let msg = if msg.is_null() { String::new() } else { CStr::from_ptr(msg).to_string_lossy().into_owned() };
            return Err(format!("GLSL: {msg}"));
        }
        let len = shaderc_result_get_length(result.0.as_ptr());
        let bytes = shaderc_result_get_bytes(result.0.as_ptr());
        let bytes: &[u8] = if bytes.is_null() { &[] } else { core::slice::from_raw_parts(bytes.cast(), len) };
        Ok(bytes.as_chunks::<4>().0.iter().map(|w| u32::from_ne_bytes([w[0], w[1], w[2], w[3]])).collect())
    }
}
