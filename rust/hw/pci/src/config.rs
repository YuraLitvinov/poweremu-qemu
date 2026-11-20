// SPDX-License-Identifier: GPL-2.0-or-later

//! Safe wrappers for the methods of `PCIDevice` used by Rust devices.

use std::{ffi::c_int, ptr};

use qom::prelude::*;
use system::MemoryRegion;
use util::{Error, Result};

use crate::{bindings, PCIDevice};

/// Trait for methods exposed by the [`PCIDevice`] class.  The methods can be
/// called on all objects that have the trait `IsA<PCIDevice>`.
///
/// The trait should only be used through the blanket implementation,
/// which guarantees safety via `IsA`.
pub trait PCIDeviceMethods: ObjectDeref
where
    Self::Target: IsA<PCIDevice>,
{
    /// Add a capability of `size` bytes with id `cap_id` at `offset` in the
    /// config space, or let QEMU pick a free offset when `offset` is 0.
    /// Returns the offset that was used.
    fn add_capability(&self, cap_id: u8, offset: u8, size: u8) -> Result<u8> {
        assert!(bql::is_locked());
        let dev = self.upcast::<PCIDevice>().as_mut_ptr();
        // SAFETY: the BQL is held and `dev` is a valid PCIDevice.
        let ret = unsafe {
            Error::with_errp(|errp| {
                bindings::pci_add_capability(dev, cap_id, offset, size, errp)
            })
        }?;
        u8::try_from(ret).map_err(|_| Error::msg("pci_add_capability failed"))
    }

    /// Read `N` bytes of the config space at `offset`.
    fn config_get<const N: usize>(&self, offset: usize) -> [u8; N] {
        let src = config_ptr(self.upcast::<PCIDevice>().as_mut_ptr(), offset, N);
        let mut out = [0u8; N];
        // SAFETY: config_ptr checked that offset..offset+N is inside the
        // conventional config header, which is always allocated.
        unsafe { ptr::copy_nonoverlapping(src, out.as_mut_ptr(), N) };
        out
    }

    /// Write `N` bytes to the config space at `offset`.
    fn config_set<const N: usize>(&self, offset: usize, bytes: [u8; N]) {
        let dst = config_ptr(self.upcast::<PCIDevice>().as_mut_ptr(), offset, N);
        // SAFETY: see `config_get`.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), dst, N) };
    }

    /// Little-endian config space accessors, as the C `pci_get_*` helpers.
    fn config_get_u8(&self, offset: usize) -> u8 {
        self.config_get::<1>(offset)[0]
    }

    fn config_get_u16(&self, offset: usize) -> u16 {
        u16::from_le_bytes(self.config_get::<2>(offset))
    }

    fn config_get_u32(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.config_get::<4>(offset))
    }

    fn config_set_u8(&self, offset: usize, val: u8) {
        self.config_set::<1>(offset, [val]);
    }

    fn config_set_u16(&self, offset: usize, val: u16) {
        self.config_set::<2>(offset, val.to_le_bytes());
    }

    fn config_set_u32(&self, offset: usize, val: u32) {
        self.config_set::<4>(offset, val.to_le_bytes());
    }

    /// Map `mem` as BAR `region` of the device; `attr` is one of the
    /// `PCI_BASE_ADDRESS_SPACE_*` flags.
    fn register_bar(&self, region: u32, attr: u8, mem: &MemoryRegion) {
        assert!(bql::is_locked());
        assert!(region < bindings::PCI_NUM_REGIONS);
        let dev = self.upcast::<PCIDevice>().as_mut_ptr();
        // SAFETY: the BQL is held, `dev` is a valid PCIDevice and `mem`
        // outlives the device, as required by pci_register_bar().
        unsafe {
            bindings::pci_register_bar(dev, region as c_int, attr, mem.as_mut_ptr());
        }
    }

    /// Drive the device's INTx pin to `level`.
    fn set_irq(&self, level: bool) {
        assert!(bql::is_locked());
        let dev = self.upcast::<PCIDevice>().as_mut_ptr();
        // SAFETY: the BQL is held and `dev` is a valid PCIDevice.
        unsafe { bindings::pci_set_irq(dev, c_int::from(level)) };
    }
}

impl<R: ObjectDeref> PCIDeviceMethods for R where R::Target: IsA<PCIDevice> {}

/// Return a pointer to config byte `offset` after checking that the
/// `len` bytes starting there lie in the 256-byte conventional header.
fn config_ptr(dev: *mut bindings::PCIDevice, offset: usize, len: usize) -> *mut u8 {
    assert!(bql::is_locked());
    let end = offset.checked_add(len).expect("PCI config access overflows");
    assert!(
        end <= bindings::PCI_CONFIG_SPACE_SIZE as usize,
        "PCI config access beyond the conventional config header"
    );
    // SAFETY: `dev` comes from a live PCIDevice whose config buffer is at
    // least PCI_CONFIG_SPACE_SIZE bytes long.
    unsafe { (*dev).config.add(offset) }
}
