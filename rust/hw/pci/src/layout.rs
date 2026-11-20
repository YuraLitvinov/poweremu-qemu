// SPDX-License-Identifier: GPL-2.0-or-later

//! Compile-time checks that the bindgen mirror of the C PCI structs has the
//! same size and field offsets as the C definitions in
//! include/hw/pci/pci_device.h. The numbers were taken from the C compiler
//! for the x86_64 build of qemu-system-x86_64.

use std::mem::{offset_of, size_of};

use crate::bindings::{PCIDevice, PCIDeviceClass};

const _: () = {
    assert!(size_of::<PCIDevice>() == 2768);
    assert!(offset_of!(PCIDevice, qdev) == 0);
    assert!(offset_of!(PCIDevice, partially_hotplugged) == 152);
    assert!(offset_of!(PCIDevice, enabled) == 153);
    assert!(offset_of!(PCIDevice, loadparm) == 160);
    assert!(offset_of!(PCIDevice, config) == 168);
    assert!(offset_of!(PCIDevice, cmask) == 176);
    assert!(offset_of!(PCIDevice, wmask) == 184);
    assert!(offset_of!(PCIDevice, w1cmask) == 192);
    assert!(offset_of!(PCIDevice, used) == 200);
    assert!(offset_of!(PCIDevice, devfn) == 208);
    assert!(offset_of!(PCIDevice, name) == 232);
    assert!(offset_of!(PCIDevice, io_regions) == 296);
    assert!(offset_of!(PCIDevice, bus_master_as) == 576);
    assert!(offset_of!(PCIDevice, config_read) == 1296);
    assert!(offset_of!(PCIDevice, config_write) == 1304);
    assert!(offset_of!(PCIDevice, irq_state) == 1337);
    assert!(offset_of!(PCIDevice, msix_cap) == 1345);
    assert!(offset_of!(PCIDevice, intx_routing_notifier) == 2632);

    assert!(size_of::<PCIDeviceClass>() == 248);
    assert!(offset_of!(PCIDeviceClass, parent_class) == 0);
    assert!(offset_of!(PCIDeviceClass, realize) == 184);
    assert!(offset_of!(PCIDeviceClass, exit) == 192);
    assert!(offset_of!(PCIDeviceClass, config_read) == 200);
    assert!(offset_of!(PCIDeviceClass, config_write) == 208);
    assert!(offset_of!(PCIDeviceClass, vendor_id) == 216);
    assert!(offset_of!(PCIDeviceClass, device_id) == 218);
    assert!(offset_of!(PCIDeviceClass, revision) == 220);
    assert!(offset_of!(PCIDeviceClass, class_id) == 222);
    assert!(offset_of!(PCIDeviceClass, subsystem_vendor_id) == 224);
    assert!(offset_of!(PCIDeviceClass, subsystem_id) == 226);
    assert!(offset_of!(PCIDeviceClass, romfile) == 232);
    assert!(offset_of!(PCIDeviceClass, sriov_vf_user_creatable) == 240);
};
