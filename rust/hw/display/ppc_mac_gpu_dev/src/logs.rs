// Logging for the device: the three trace channels (seq, blit path, debug), stderr and
// QEMU's own log.  The three `*_msg` functions are the C entry points used by the
// variadic wrappers in ppc_mac_gpu_inline_shims.c (stable Rust cannot define C-variadic
// functions); they only convert the C string and delegate to the safe functions below.
use super::dev_ns::*;
use ::core::ffi::{c_char, c_int};
use core::fmt::Arguments;
use std::ffi::{CStr, CString};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

/// A lazily opened log file; `None` after a failed open means "keep trying next time".
struct LogFile(Mutex<Option<File>>);

impl LogFile {
    const fn new() -> Self {
        LogFile(Mutex::new(None))
    }

    /// Runs `f` with the file, opening it with `open` on first use.
    fn with(&self, open: impl FnOnce() -> Option<File>, f: impl FnOnce(&mut File)) {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            *g = open();
        }
        if let Some(file) = g.as_mut() {
            f(file);
        }
    }
}

fn open_append(path: &str) -> Option<File> {
    OpenOptions::new().create(true).append(true).open(path).ok()
}

fn open_truncate(path: &str) -> Option<File> {
    File::create(path).ok()
}

static SEQ_FILE: LogFile = LogFile::new();
static BLIT_FILE: LogFile = LogFile::new();
static DEBUG_FILE: LogFile = LogFile::new();
static DEBUG_ENABLED: crate::src::global::Global<c_int> = crate::src::global::Global::<c_int>::new(-1);

fn seq_enabled() -> bool {
    if g_seq_log_enabled.get() < 0 {
        let on = crate::src::qemu::env_string(c"PPCGPU_SEQ_LOG")
            .is_some_and(|v| v.as_bytes().first() != Some(&b'0'));
        g_seq_log_enabled.set(on as c_int);
    }
    g_seq_log_enabled.get() != 0
}

fn debug_on() -> bool {
    if gpu_track_only() && !gpu_track_pass.get() {
        return false;
    }
    if DEBUG_ENABLED.get() < 0 {
        let on = crate::src::qemu::env_string(c"PPCGPU_DEBUG_LOG").is_some_and(|v| v.starts_with('1'));
        DEBUG_ENABLED.set(on as c_int);
    }
    DEBUG_ENABLED.get() != 0
}

/// Sequence log (/tmp/gpu_seq.log), one numbered line per message.
pub fn seq(a: Arguments) {
    if !seq_enabled() {
        return;
    }
    SEQ_FILE.with(
        || open_append("/tmp/gpu_seq.log"),
        |f| {
            g_seq.set(g_seq.get() + 1);
            let who = if g_in_pm4.get() != 0 { "RING" } else { "CPU " };
            let _ = writeln!(f, "{} {} {}", g_seq.get(), who, a);
        },
    );
}

/// Blit path log (/tmp/gpu_blit_paths.log) with periodic per-path counters.
pub fn blit_path(path: &str, a: Arguments) {
    BLIT_FILE.with(
        || {
            let mut f = open_truncate("/tmp/gpu_blit_paths.log")?;
            let _ = writeln!(f, "=== Blit Path Instrumentation ===");
            Some(f)
        },
        |f| {
            let s = g_blit_stats_mut();
            let total = s.sep_count + s.mmio_count + s.bbm_count + s.paint_count + s.hostdata_count;
            let _ = writeln!(f, "[{} #{}] {}", path, total, a);
            if total - s.last_dump >= 500 {
                let _ = writeln!(
                    f,
                    "--- COUNTS: SEP={} MMIO={} BBM={} PAINT={} HD={} ---",
                    s.sep_count, s.mmio_count, s.bbm_count, s.paint_count, s.hostdata_count
                );
                s.last_dump = total;
                let _ = f.flush();
            }
        },
    );
}

/// Debug log of all register accesses (/tmp/gpu_all_access.log).
pub fn debug(a: Arguments) {
    if !debug_on() {
        return;
    }
    DEBUG_FILE.with(
        || open_truncate("/tmp/gpu_all_access.log"),
        |f| {
            let n = gpu_debug_seq.get();
            gpu_debug_seq.set(n + 1);
            let _ = writeln!(f, "[{:06}] {}", n, a);
            let _ = f.flush();
        },
    );
}

fn cmsg(a: Arguments) -> CString {
    CString::new(a.to_string().replace('\0', "")).unwrap_or_default()
}

pub fn qemu(a: Arguments) {
    // SAFETY: "%s" with a valid NUL-terminated string.
    unsafe { qemu_log(c"%s".as_ptr(), cmsg(a).as_ptr()) }
}

/// stderr like C's fprintf(stderr): a failed write is ignored, never a panic.
pub fn err(a: Arguments) {
    let _ = std::io::stderr().write_fmt(a);
}

fn lossy<'a>(p: *const c_char) -> std::borrow::Cow<'a, str> {
    if p.is_null() {
        return "".into();
    }
    // SAFETY: the C callers pass NUL-terminated strings valid for the call.
    unsafe { CStr::from_ptr(p) }.to_string_lossy()
}

// ---- C entry points (called from ppc_mac_gpu_inline_shims.c) ----

#[no_mangle]
pub extern "C" fn seq_log_msg(msg: *const c_char) {
    seq(format_args!("{}", lossy(msg)));
}

#[no_mangle]
pub extern "C" fn blit_path_log_msg(path: *const c_char, msg: *const c_char) {
    blit_path(&lossy(path), format_args!("{}", lossy(msg)));
}

#[no_mangle]
pub extern "C" fn gpu_debug_log_msg(msg: *const c_char) {
    debug(format_args!("{}", lossy(msg)));
}
