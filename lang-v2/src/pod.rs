//! Alignment-1 Pod integer types for zero-copy Solana account access.
//!
//! Pod types (`PodU64`, `PodU32`, etc.) wrap native integers in `[u8; N]`
//! arrays, guaranteeing alignment 1. This allows direct pointer casts from
//! account data without alignment concerns — critical for `#[repr(C)]`
//! zero-copy structs on Solana.
//!
//! Arithmetic operators (`+`, `-`, `*`) use wrapping semantics in release
//! builds for CU efficiency and panic on overflow in debug builds. Use
//! `checked_add`, `checked_sub`, `checked_mul`, `checked_div` where overflow
//! must be detected.

use core::fmt;

macro_rules! define_pod_unsigned {
    ($name:ident, $native:ty, $size:expr) => {
        define_pod_common!($name, $native, $size);
        define_pod_arithmetic!($name, $native);
    };
}

macro_rules! define_pod_signed {
    ($name:ident, $native:ty, $size:expr) => {
        define_pod_common!($name, $native, $size);
        define_pod_arithmetic!($name, $native);

        impl core::ops::Neg for $name {
            type Output = Self;
            #[inline(always)]
            fn neg(self) -> Self {
                #[cfg(debug_assertions)]
                { Self::from(self.get().checked_neg().expect("attempt to negate with overflow")) }
                #[cfg(not(debug_assertions))]
                { Self::from(self.get().wrapping_neg()) }
            }
        }
    };
}

macro_rules! define_pod_common {
    ($name:ident, $native:ty, $size:expr) => {
        #[repr(transparent)]
        #[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct $name([u8; $size]);

        impl $name {
            pub const ZERO: Self = Self([0u8; $size]);
            pub const MAX: Self = Self(<$native>::MAX.to_le_bytes());
            pub const MIN: Self = Self(<$native>::MIN.to_le_bytes());

            #[inline(always)]
            pub fn get(&self) -> $native { <$native>::from_le_bytes(self.0) }

            #[inline(always)]
            pub fn is_zero(&self) -> bool { self.0 == [0u8; $size] }

            #[inline(always)]
            pub fn checked_add(self, rhs: impl Into<$name>) -> Option<Self> {
                self.get().checked_add(rhs.into().get()).map(Self::from)
            }
            #[inline(always)]
            pub fn checked_sub(self, rhs: impl Into<$name>) -> Option<Self> {
                self.get().checked_sub(rhs.into().get()).map(Self::from)
            }
            #[inline(always)]
            pub fn checked_mul(self, rhs: impl Into<$name>) -> Option<Self> {
                self.get().checked_mul(rhs.into().get()).map(Self::from)
            }
            #[inline(always)]
            pub fn checked_div(self, rhs: impl Into<$name>) -> Option<Self> {
                self.get().checked_div(rhs.into().get()).map(Self::from)
            }
            #[inline(always)]
            pub fn saturating_add(self, rhs: impl Into<$name>) -> Self {
                Self::from(self.get().saturating_add(rhs.into().get()))
            }
            #[inline(always)]
            pub fn saturating_sub(self, rhs: impl Into<$name>) -> Self {
                Self::from(self.get().saturating_sub(rhs.into().get()))
            }
            #[inline(always)]
            pub fn saturating_mul(self, rhs: impl Into<$name>) -> Self {
                Self::from(self.get().saturating_mul(rhs.into().get()))
            }
        }

        impl From<$native> for $name {
            #[inline(always)]
            fn from(v: $native) -> Self { Self(v.to_le_bytes()) }
        }
        impl From<$name> for $native {
            #[inline(always)]
            fn from(v: $name) -> Self { v.get() }
        }

        impl PartialEq for $name {
            #[inline(always)]
            fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
        }
        impl Eq for $name {}

        impl PartialEq<$native> for $name {
            #[inline(always)]
            fn eq(&self, other: &$native) -> bool { self.get() == *other }
        }

        impl PartialOrd for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(other)) }
        }
        impl Ord for $name {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering { self.get().cmp(&other.get()) }
        }
        impl PartialOrd<$native> for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &$native) -> Option<core::cmp::Ordering> { self.get().partial_cmp(other) }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.get().fmt(f) }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}({})", stringify!($name), self.get()) }
        }
    };
}

macro_rules! define_pod_arithmetic {
    ($name:ident, $native:ty) => {
        impl core::ops::Add<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: $native) -> Self {
                #[cfg(debug_assertions)]
                { Self::from(self.get().checked_add(rhs).expect("attempt to add with overflow")) }
                #[cfg(not(debug_assertions))]
                { Self::from(self.get().wrapping_add(rhs)) }
            }
        }
        impl core::ops::Sub<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: $native) -> Self {
                #[cfg(debug_assertions)]
                { Self::from(self.get().checked_sub(rhs).expect("attempt to subtract with overflow")) }
                #[cfg(not(debug_assertions))]
                { Self::from(self.get().wrapping_sub(rhs)) }
            }
        }
        impl core::ops::Mul<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn mul(self, rhs: $native) -> Self {
                #[cfg(debug_assertions)]
                { Self::from(self.get().checked_mul(rhs).expect("attempt to multiply with overflow")) }
                #[cfg(not(debug_assertions))]
                { Self::from(self.get().wrapping_mul(rhs)) }
            }
        }
        impl core::ops::Div<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn div(self, rhs: $native) -> Self { Self::from(self.get() / rhs) }
        }
        impl core::ops::Rem<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn rem(self, rhs: $native) -> Self { Self::from(self.get() % rhs) }
        }
        impl core::ops::Add for $name {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: Self) -> Self { self + rhs.get() }
        }
        impl core::ops::Sub for $name {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: Self) -> Self { self - rhs.get() }
        }
        impl core::ops::Mul for $name {
            type Output = Self;
            #[inline(always)]
            fn mul(self, rhs: Self) -> Self { self * rhs.get() }
        }
        impl core::ops::Div for $name {
            type Output = Self;
            #[inline(always)]
            fn div(self, rhs: Self) -> Self { self / rhs.get() }
        }
        impl core::ops::Rem for $name {
            type Output = Self;
            #[inline(always)]
            fn rem(self, rhs: Self) -> Self { self % rhs.get() }
        }
        impl core::ops::AddAssign<$native> for $name { #[inline(always)] fn add_assign(&mut self, rhs: $native) { *self = *self + rhs; } }
        impl core::ops::SubAssign<$native> for $name { #[inline(always)] fn sub_assign(&mut self, rhs: $native) { *self = *self - rhs; } }
        impl core::ops::MulAssign<$native> for $name { #[inline(always)] fn mul_assign(&mut self, rhs: $native) { *self = *self * rhs; } }
        impl core::ops::DivAssign<$native> for $name { #[inline(always)] fn div_assign(&mut self, rhs: $native) { *self = *self / rhs; } }
        impl core::ops::RemAssign<$native> for $name { #[inline(always)] fn rem_assign(&mut self, rhs: $native) { *self = *self % rhs; } }
        impl core::ops::AddAssign for $name { #[inline(always)] fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; } }
        impl core::ops::SubAssign for $name { #[inline(always)] fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; } }
        impl core::ops::MulAssign for $name { #[inline(always)] fn mul_assign(&mut self, rhs: Self) { *self = *self * rhs; } }
        impl core::ops::DivAssign for $name { #[inline(always)] fn div_assign(&mut self, rhs: Self) { *self = *self / rhs; } }
        impl core::ops::RemAssign for $name { #[inline(always)] fn rem_assign(&mut self, rhs: Self) { *self = *self % rhs; } }
        impl core::ops::BitAnd<$native> for $name { type Output = Self; #[inline(always)] fn bitand(self, rhs: $native) -> Self { Self::from(self.get() & rhs) } }
        impl core::ops::BitOr<$native> for $name { type Output = Self; #[inline(always)] fn bitor(self, rhs: $native) -> Self { Self::from(self.get() | rhs) } }
        impl core::ops::BitXor<$native> for $name { type Output = Self; #[inline(always)] fn bitxor(self, rhs: $native) -> Self { Self::from(self.get() ^ rhs) } }
        impl core::ops::BitAnd for $name { type Output = Self; #[inline(always)] fn bitand(self, rhs: Self) -> Self { self & rhs.get() } }
        impl core::ops::BitOr for $name { type Output = Self; #[inline(always)] fn bitor(self, rhs: Self) -> Self { self | rhs.get() } }
        impl core::ops::BitXor for $name { type Output = Self; #[inline(always)] fn bitxor(self, rhs: Self) -> Self { self ^ rhs.get() } }
        impl core::ops::BitAndAssign<$native> for $name { #[inline(always)] fn bitand_assign(&mut self, rhs: $native) { *self = *self & rhs; } }
        impl core::ops::BitOrAssign<$native> for $name { #[inline(always)] fn bitor_assign(&mut self, rhs: $native) { *self = *self | rhs; } }
        impl core::ops::BitXorAssign<$native> for $name { #[inline(always)] fn bitxor_assign(&mut self, rhs: $native) { *self = *self ^ rhs; } }
        impl core::ops::BitAndAssign for $name { #[inline(always)] fn bitand_assign(&mut self, rhs: Self) { *self = *self & rhs; } }
        impl core::ops::BitOrAssign for $name { #[inline(always)] fn bitor_assign(&mut self, rhs: Self) { *self = *self | rhs; } }
        impl core::ops::BitXorAssign for $name { #[inline(always)] fn bitxor_assign(&mut self, rhs: Self) { *self = *self ^ rhs; } }
        impl core::ops::Shl<u32> for $name { type Output = Self; #[inline(always)] fn shl(self, rhs: u32) -> Self { Self::from(self.get() << rhs) } }
        impl core::ops::Shr<u32> for $name { type Output = Self; #[inline(always)] fn shr(self, rhs: u32) -> Self { Self::from(self.get() >> rhs) } }
        impl core::ops::ShlAssign<u32> for $name { #[inline(always)] fn shl_assign(&mut self, rhs: u32) { *self = *self << rhs; } }
        impl core::ops::ShrAssign<u32> for $name { #[inline(always)] fn shr_assign(&mut self, rhs: u32) { *self = *self >> rhs; } }
        impl core::ops::Not for $name { type Output = Self; #[inline(always)] fn not(self) -> Self { Self::from(!self.get()) } }
    };
}

define_pod_unsigned!(PodU128, u128, 16);
define_pod_unsigned!(PodU64, u64, 8);
define_pod_unsigned!(PodU32, u32, 4);
define_pod_unsigned!(PodU16, u16, 2);
define_pod_signed!(PodI128, i128, 16);
define_pod_signed!(PodI64, i64, 8);
define_pod_signed!(PodI32, i32, 4);
define_pod_signed!(PodI16, i16, 2);

const _: () = assert!(core::mem::align_of::<PodU128>() == 1);
const _: () = assert!(core::mem::size_of::<PodU128>() == 16);
const _: () = assert!(core::mem::align_of::<PodU64>() == 1);
const _: () = assert!(core::mem::size_of::<PodU64>() == 8);
const _: () = assert!(core::mem::align_of::<PodU32>() == 1);
const _: () = assert!(core::mem::size_of::<PodU32>() == 4);
const _: () = assert!(core::mem::align_of::<PodU16>() == 1);
const _: () = assert!(core::mem::size_of::<PodU16>() == 2);
const _: () = assert!(core::mem::align_of::<PodI128>() == 1);
const _: () = assert!(core::mem::size_of::<PodI128>() == 16);
const _: () = assert!(core::mem::align_of::<PodI64>() == 1);
const _: () = assert!(core::mem::size_of::<PodI64>() == 8);
const _: () = assert!(core::mem::align_of::<PodI32>() == 1);
const _: () = assert!(core::mem::size_of::<PodI32>() == 4);
const _: () = assert!(core::mem::align_of::<PodI16>() == 1);
const _: () = assert!(core::mem::size_of::<PodI16>() == 2);
const _: () = assert!(core::mem::align_of::<PodBool>() == 1);
const _: () = assert!(core::mem::size_of::<PodBool>() == 1);

/// An alignment-1 boolean stored as a single `[u8; 1]`.
///
/// Any non-zero byte is considered `true`, matching Solana program conventions.
#[repr(transparent)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PodBool([u8; 1]);

impl PodBool {
    #[inline(always)]
    pub fn get(&self) -> bool { self.0[0] != 0 }
}

impl From<bool> for PodBool {
    #[inline(always)]
    fn from(v: bool) -> Self { Self([v as u8]) }
}
impl From<PodBool> for bool {
    #[inline(always)]
    fn from(v: PodBool) -> Self { v.get() }
}
impl PartialEq for PodBool {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool { self.get() == other.get() }
}
impl Eq for PodBool {}
impl PartialEq<bool> for PodBool {
    #[inline(always)]
    fn eq(&self, other: &bool) -> bool { self.get() == *other }
}
impl core::ops::Not for PodBool {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self { Self::from(!self.get()) }
}
impl fmt::Display for PodBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.get().fmt(f) }
}
impl fmt::Debug for PodBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "PodBool({})", self.get()) }
}

// ---------------------------------------------------------------------------
// PodVec — fixed-capacity, variable-length array with u16 length header
// ---------------------------------------------------------------------------

/// A fixed-capacity array with a `PodU16` length prefix.
///
/// `PodVec<T, MAX>` stores up to `MAX` elements of type `T` inline.
/// The in-memory size is always `2 + size_of::<T>() * MAX` regardless
/// of how many elements are populated. Use `.as_slice()` to access
/// only the populated elements.
///
/// This type is `Pod` when `T: Pod`, so it can be used directly inside
/// `#[account]` structs for zero-copy account access.
///
/// # Layout
///
/// ```text
/// [len: u16 LE (2 bytes)][data: T × MAX]
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PodVec<T: bytemuck::Pod, const MAX: usize> {
    len: PodU16,
    data: [T; MAX],
}

// Safety: #[repr(C)], all fields are Pod (PodU16 is alignment 1, T: Pod).
// The const assert below catches any padding at compile time.
unsafe impl<T: bytemuck::Pod, const MAX: usize> bytemuck::Zeroable for PodVec<T, MAX> {}
unsafe impl<T: bytemuck::Pod, const MAX: usize> bytemuck::Pod for PodVec<T, MAX> {}

impl<T: bytemuck::Pod, const MAX: usize> PodVec<T, MAX> {
    // Compile-time check: no padding between len and data.
    // If T has alignment > 1, repr(C) would insert padding after the 2-byte
    // PodU16, violating Pod. This catches it at monomorphization time.
    const _NO_PADDING: () = assert!(
        core::mem::size_of::<Self>() == 2 + core::mem::size_of::<T>() * MAX,
        "PodVec<T, MAX>: T must have alignment 1 (no padding allowed)"
    );

    /// Returns the populated elements as a slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        let _ = Self::_NO_PADDING;
        let len = self.len.get() as usize;
        &self.data[..len]
    }

    /// Returns the populated elements as a mutable slice.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len.get() as usize;
        &mut self.data[..len]
    }

    /// Returns the number of populated elements.
    #[inline(always)]
    pub fn len(&self) -> usize { self.len.get() as usize }

    /// Returns `true` if no elements are populated.
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.len.is_zero() }

    /// Returns a reference to the element at `idx`, or `None` if out of bounds.
    #[inline(always)]
    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.len() { Some(&self.data[idx]) } else { None }
    }

    /// Sets the contents from a slice. Panics if `src.len() > MAX`.
    #[inline]
    pub fn set_from_slice(&mut self, src: &[T]) {
        assert!(src.len() <= MAX, "PodVec: slice length exceeds capacity");
        self.len = PodU16::from(src.len() as u16);
        self.data[..src.len()].copy_from_slice(src);
    }

    /// Returns the maximum capacity.
    #[inline(always)]
    pub const fn capacity(&self) -> usize { MAX }
}

impl<T: bytemuck::Pod, const MAX: usize> Default for PodVec<T, MAX> {
    fn default() -> Self {
        // Safety: PodVec is Pod, so all-zeros is a valid representation.
        unsafe { core::mem::zeroed() }
    }
}
