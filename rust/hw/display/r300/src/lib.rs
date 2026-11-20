//! Idiomatic Rust port of the R300 (Radeon 9700) helpers that QEMU's
//! `hw/display/r300/*.c` provides to the renderers.
//!
//! The crate is little-endian-guest only: the big-endian, PCI and PowerPC
//! paths of the C original are not ported.  Everything outside [`capi`] is safe
//! Rust on slices; `capi` holds the `#[unsafe(no_mangle)]` entry points the C
//! device calls.  The shader compiler is the safe `shaderc` crate.
//!
//! This work is licensed under the terms of the GNU GPL, version 2 or later.

pub mod capi;
pub mod draw;
pub mod cfmt;
pub mod pvs;
pub mod pvs_glsl;
pub mod regnames;
pub mod spirv;
pub mod state;
pub mod texel;
pub mod types;
pub mod us;
mod us_text;
