// SPDX-License-Identifier: GPL-2.0-or-later
#![allow(
    dead_code,
    improper_ctypes_definitions,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unnecessary_transmutes,
    unsafe_op_in_unsafe_fn,
    clippy::complexity,
    clippy::pedantic,
    clippy::restriction,
    clippy::style,
    clippy::missing_const_for_fn,
    clippy::ptr_offset_with_cast,
    clippy::useless_transmute,
    clippy::missing_safety_doc,
    clippy::too_many_arguments
)]

use glib_sys::{GArray, GSList};
use hwcore_sys::{qemu_irq, BusState, DeviceClass, DeviceState};
use migration_sys::VMStateDescription;
use qom_sys::{Object, ObjectClass};
use system_sys::{hwaddr, MemoryRegion, MemoryRegionOps, MemoryRegionSection};
use util_sys::{Error, QemuOpts, QEMUBH};

/// `QemuConsole` is defined in ui/console-priv.h, a private header; the public API only passes it
/// by pointer, so it is left opaque here.
#[repr(C)]
pub struct QemuConsole {
    _p: [u8; 0],
}

#[cfg(MESON)]
include!("bindings.inc.rs");

#[cfg(not(MESON))]
include!(concat!(env!("OUT_DIR"), "/bindings.inc.rs"));
