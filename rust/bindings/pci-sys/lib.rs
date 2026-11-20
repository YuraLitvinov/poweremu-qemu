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

use hwcore_sys::{qemu_irq, BusClass, BusState, DeviceClass, DeviceState};
use migration_sys::{VMStateDescription, VMStateInfo};
use qom_sys::{Object, ObjectClass};
use system_sys::{
    hwaddr, AddressSpace, IOMMUNotifier, IOMMUNotify, IOMMUTLBEntry, MemoryRegion, MemoryRegionOps,
    MemoryRegionSection, SysBusDevice,
};
use util_sys::{Error, QEMUFile};

/// `FWCfgState` is only named in a pci.h prototype (pci_bus_add_fw_cfg_extra_pci_roots) and only
/// used behind a pointer, so it is left opaque instead of binding hw/nvram/fw_cfg.h.
#[repr(C)]
pub struct FWCfgState {
    _p: [u8; 0],
}

#[cfg(MESON)]
include!("bindings.inc.rs");

#[cfg(not(MESON))]
include!(concat!(env!("OUT_DIR"), "/bindings.inc.rs"));
