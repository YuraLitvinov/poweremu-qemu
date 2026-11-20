// SPDX-License-Identifier: GPL-2.0-or-later

//! The `PCIDevice` QOM type and the `PCIDeviceImpl` trait for Rust devices.

use std::{ffi::CStr, ptr::NonNull};

use common::Opaque;
use hwcore::prelude::*;
use qom::prelude::*;
use util::{Error, Result};

use crate::{bindings, ids::PCIDeviceClassIds, ids::PCIIds};

/// A safe wrapper around [`bindings::PCIDevice`].
#[repr(transparent)]
#[derive(Debug, common::Wrapper)]
pub struct PCIDevice(Opaque<bindings::PCIDevice>);

unsafe impl Send for PCIDevice {}
unsafe impl Sync for PCIDevice {}

unsafe impl ObjectType for PCIDevice {
    type Class = bindings::PCIDeviceClass;
    const TYPE_NAME: &'static CStr =
        unsafe { CStr::from_bytes_with_nul_unchecked(bindings::TYPE_PCI_DEVICE) };
}

qom_isa!(PCIDevice: DeviceState, Object);

/// Trait providing the PCI-specific contents of [`bindings::PCIDeviceClass`]
/// for a Rust device, on top of the [`DeviceImpl`] contents.
pub trait PCIDeviceImpl: DeviceImpl + IsA<PCIDevice> {
    /// If not `None`, replaces `PCIDeviceClass::realize`. It is called by the
    /// C `pci_qdev_realize` after the common PCI setup, so it must not
    /// register the device on its own bus.
    const PCI_REALIZE: Option<fn(&Self) -> Result<()>> = None;

    /// If not `None`, replaces `PCIDeviceClass::exit`. It is called by the
    /// C `pci_qdev_unrealize` before the common PCI teardown.
    const PCI_EXIT: Option<fn(&Self)> = None;

    /// If not `None`, the PCI identification written to the class, so that
    /// the device does not have to touch the raw class struct.
    const IDS: Option<PCIIds> = None;
}

/// # Safety
///
/// We expect the FFI user of this function to pass a valid pointer that
/// can be downcasted to type `T`, since `PCIDevice` is the first field of
/// the Rust device struct.
unsafe extern "C" fn rust_pci_realize_fn<T: PCIDeviceImpl>(
    dev: *mut bindings::PCIDevice,
    errp: *mut *mut util::bindings::Error,
) {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    let result = T::PCI_REALIZE.unwrap()(unsafe { state.as_ref() });
    unsafe {
        Error::ok_or_propagate(result, errp);
    }
}

/// # Safety
///
/// Same requirements as [`rust_pci_realize_fn`].
unsafe extern "C" fn rust_pci_exit_fn<T: PCIDeviceImpl>(dev: *mut bindings::PCIDevice) {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::PCI_EXIT.unwrap()(unsafe { state.as_ref() });
}

/// Trait to fill in a [`bindings::PCIDeviceClass`] from a [`PCIDeviceImpl`].
pub trait PCIDeviceClassExt {
    fn class_init<T: PCIDeviceImpl>(&mut self);
}

impl PCIDeviceClassExt for bindings::PCIDeviceClass {
    fn class_init<T: PCIDeviceImpl>(&mut self) {
        if T::PCI_REALIZE.is_some() {
            self.realize = Some(rust_pci_realize_fn::<T>);
        }
        if T::PCI_EXIT.is_some() {
            self.exit = Some(rust_pci_exit_fn::<T>);
        }
        if let Some(ids) = T::IDS {
            self.set_ids(ids);
        }
        self.parent_class.class_init::<T>();
    }
}
