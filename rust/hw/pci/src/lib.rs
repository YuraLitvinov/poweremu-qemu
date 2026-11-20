// SPDX-License-Identifier: GPL-2.0-or-later

//! Safe bindings to the QEMU PCI device API, so that a Rust QOM struct can
//! use `PCIDevice` as its parent type.

pub use pci_sys as bindings;

mod config;
pub use config::*;

mod ids;
pub use ids::*;

mod layout;

mod qdev;
pub use qdev::*;

// preserve one-item-per-"use" syntax, it is clearer
// for prelude-like modules
#[rustfmt::skip]
pub mod prelude;
