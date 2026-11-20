
/// Declares an incomplete C type that is only ever used behind a pointer.
macro_rules! opaque {
    ($($name:ident);* $(;)?) => { $(#[repr(C)] pub struct $name { _p: [u8; 0] })* };
}
// QEMU's own Rust crate `system` supplies MemTxAttrs (same layout as the C header).
pub use system::MemTxAttrs;
pub mod ffi;
pub use self::ffi::*;
pub mod types_qemu_1;
pub use self::types_qemu_1::*;
pub mod types_qemu_2;
pub use self::types_qemu_2::*;
pub mod types_gpu;
pub use self::types_gpu::*;
pub mod fn_r300_1;
pub use self::fn_r300_1::*;
pub mod fn_r300_2;
pub use self::fn_r300_2::*;
pub mod fn_misc;
pub use self::fn_misc::*;
pub mod fn_frame;
pub use self::fn_frame::*;
pub mod fn_gpu;
pub use self::fn_gpu::*;
pub mod statics;
pub use self::statics::*;
pub mod fn_blit;
pub use self::fn_blit::*;
pub mod fn_pe;
pub use self::fn_pe::*;
pub mod fn_ppc_mac_gpu_1;
pub use self::fn_ppc_mac_gpu_1::*;
pub mod fn_ppc_mac_gpu_2;
pub use self::fn_ppc_mac_gpu_2::*;
pub mod fn_ppc_mac_gpu_3;
pub use self::fn_ppc_mac_gpu_3::*;
pub mod fn_ppc_mac_gpu_4;
pub use self::fn_ppc_mac_gpu_4::*;
pub mod fn_r200;
pub use self::fn_r200::*;
pub mod fn_mc;
pub use self::fn_mc::*;
pub mod fn_2d;
pub use self::fn_2d::*;
pub mod fn_r300_render;
pub use self::fn_r300_render::*;
pub mod fn_r300_trace;
pub use self::fn_r300_trace::*;
