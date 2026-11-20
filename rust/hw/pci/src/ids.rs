// SPDX-License-Identifier: GPL-2.0-or-later

//! Setters for the identification fields of `PCIDeviceClass`.

use crate::bindings;

/// The PCI identification of a device type, as stored in `PCIDeviceClass`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PCIIds {
    pub vendor_id: u16,
    pub device_id: u16,
    pub revision: u8,
    pub class_id: u16,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
}

/// Safe setters for the identification fields of [`bindings::PCIDeviceClass`].
pub trait PCIDeviceClassIds {
    fn set_vendor_id(&mut self, id: u16);
    fn set_device_id(&mut self, id: u16);
    fn set_revision(&mut self, revision: u8);
    fn set_class_id(&mut self, id: u16);
    fn set_subsystem_ids(&mut self, vendor_id: u16, id: u16);

    /// Set every identification field at once.
    fn set_ids(&mut self, ids: PCIIds) {
        self.set_vendor_id(ids.vendor_id);
        self.set_device_id(ids.device_id);
        self.set_revision(ids.revision);
        self.set_class_id(ids.class_id);
        self.set_subsystem_ids(ids.subsystem_vendor_id, ids.subsystem_id);
    }
}

impl PCIDeviceClassIds for bindings::PCIDeviceClass {
    fn set_vendor_id(&mut self, id: u16) {
        self.vendor_id = id;
    }

    fn set_device_id(&mut self, id: u16) {
        self.device_id = id;
    }

    fn set_revision(&mut self, revision: u8) {
        self.revision = revision;
    }

    fn set_class_id(&mut self, id: u16) {
        self.class_id = id;
    }

    fn set_subsystem_ids(&mut self, vendor_id: u16, id: u16) {
        self.subsystem_vendor_id = vendor_id;
        self.subsystem_id = id;
    }
}
