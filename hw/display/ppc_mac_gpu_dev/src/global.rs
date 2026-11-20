//! Safe replacement for scalar `static mut`: a relaxed-atomic cell.
//!
//! The device state lives under QEMU's big lock, so ordering is irrelevant;
//! atomics are used only because they make a shared static safe without an
//! `unsafe impl Sync`.
use core::sync::atomic::{AtomicBool, AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, AtomicIsize, Ordering::Relaxed};

pub trait Atom: Copy {
    type A;
    fn new(v: Self) -> Self::A;
    fn load(a: &Self::A) -> Self;
    fn store(a: &Self::A, v: Self);
}

macro_rules! atom_int {
    ($($t:ty => $a:ty),*) => {$(
        impl Atom for $t {
            type A = $a;
            fn new(v: Self) -> $a { <$a>::new(v) }
            fn load(a: &$a) -> Self { a.load(Relaxed) }
            fn store(a: &$a, v: Self) { a.store(v, Relaxed) }
        }
    )*};
}
atom_int!(i8 => AtomicI8, u8 => AtomicU8, i16 => AtomicI16, u16 => AtomicU16,
          i32 => AtomicI32, u32 => AtomicU32, i64 => AtomicI64, u64 => AtomicU64,
          bool => AtomicBool, usize => AtomicUsize, isize => AtomicIsize);

impl Atom for f32 {
    type A = AtomicU32;
    fn new(v: Self) -> AtomicU32 { AtomicU32::new(v.to_bits()) }
    fn load(a: &AtomicU32) -> Self { f32::from_bits(a.load(Relaxed)) }
    fn store(a: &AtomicU32, v: Self) { a.store(v.to_bits(), Relaxed) }
}
impl Atom for f64 {
    type A = AtomicU64;
    fn new(v: Self) -> AtomicU64 { AtomicU64::new(v.to_bits()) }
    fn load(a: &AtomicU64) -> Self { f64::from_bits(a.load(Relaxed)) }
    fn store(a: &AtomicU64, v: Self) { a.store(v.to_bits(), Relaxed) }
}

#[repr(transparent)]
pub struct Global<T: Atom>(T::A);

impl<T: Atom> Global<T> {
    pub fn get(&self) -> T { T::load(&self.0) }
    pub fn set(&self, v: T) { T::store(&self.0, v) }
}

macro_rules! global_new {
    ($($t:ty),*) => {$(
        impl Global<$t> {
            pub const fn new(v: $t) -> Self { Global(<<$t as Atom>::A>::new(v)) }
        }
    )*};
}
global_new!(i8, u8, i16, u16, i32, u32, i64, u64, bool, usize, isize);
impl Global<f32> { pub const fn new(v: f32) -> Self { Global(AtomicU32::new(v.to_bits())) } }
impl Global<f64> { pub const fn new(v: f64) -> Self { Global(AtomicU64::new(v.to_bits())) } }
