//! Alignment-1 Pod integer types for zero-copy Solana account access.
//!
//! Pod types (`PodU64`, `PodU32`, etc.) wrap native integers in `[u8; N]`
//! arrays, guaranteeing alignment 1. This allows direct pointer casts from
//! account data without alignment concerns — critical for `#[repr(C)]`
//! zero-copy structs on Solana.
//!
//! Arithmetic operators (`+`, `-`, `*`) are overflow-checked in debug builds
//! and wrapping in release. Use `checked_add`, `checked_sub`, `checked_mul`,
//! `checked_div` where overflow must be detected.

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
                {
                    Self::from(
                        self.get()
                            .checked_neg()
                            .expect("attempt to negate with overflow"),
                    )
                }
                #[cfg(not(debug_assertions))]
                {
                    Self::from(self.get().wrapping_neg())
                }
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
            pub fn get(&self) -> $native {
                <$native>::from_le_bytes(self.0)
            }

            #[inline(always)]
            pub fn is_zero(&self) -> bool {
                self.0 == [0u8; $size]
            }

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
            fn from(v: $native) -> Self {
                Self(v.to_le_bytes())
            }
        }
        impl From<$name> for $native {
            #[inline(always)]
            fn from(v: $name) -> Self {
                v.get()
            }
        }

        impl PartialEq for $name {
            #[inline(always)]
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
        impl Eq for $name {}

        impl PartialEq<$native> for $name {
            #[inline(always)]
            fn eq(&self, other: &$native) -> bool {
                self.get() == *other
            }
        }

        impl PartialOrd for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for $name {
            #[inline(always)]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }
        impl PartialOrd<$native> for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &$native) -> Option<core::cmp::Ordering> {
                self.get().partial_cmp(other)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.get().fmt(f)
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.get())
            }
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
                {
                    Self::from(
                        self.get()
                            .checked_add(rhs)
                            .expect("attempt to add with overflow"),
                    )
                }
                #[cfg(not(debug_assertions))]
                {
                    Self::from(self.get().wrapping_add(rhs))
                }
            }
        }
        impl core::ops::Sub<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: $native) -> Self {
                #[cfg(debug_assertions)]
                {
                    Self::from(
                        self.get()
                            .checked_sub(rhs)
                            .expect("attempt to subtract with overflow"),
                    )
                }
                #[cfg(not(debug_assertions))]
                {
                    Self::from(self.get().wrapping_sub(rhs))
                }
            }
        }
        impl core::ops::Mul<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn mul(self, rhs: $native) -> Self {
                #[cfg(debug_assertions)]
                {
                    Self::from(
                        self.get()
                            .checked_mul(rhs)
                            .expect("attempt to multiply with overflow"),
                    )
                }
                #[cfg(not(debug_assertions))]
                {
                    Self::from(self.get().wrapping_mul(rhs))
                }
            }
        }
        impl core::ops::Div<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn div(self, rhs: $native) -> Self {
                Self::from(self.get() / rhs)
            }
        }
        impl core::ops::Rem<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn rem(self, rhs: $native) -> Self {
                Self::from(self.get() % rhs)
            }
        }
        impl core::ops::Add for $name {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: Self) -> Self {
                self + rhs.get()
            }
        }
        impl core::ops::Sub for $name {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: Self) -> Self {
                self - rhs.get()
            }
        }
        impl core::ops::Mul for $name {
            type Output = Self;
            #[inline(always)]
            fn mul(self, rhs: Self) -> Self {
                self * rhs.get()
            }
        }
        impl core::ops::Div for $name {
            type Output = Self;
            #[inline(always)]
            fn div(self, rhs: Self) -> Self {
                self / rhs.get()
            }
        }
        impl core::ops::Rem for $name {
            type Output = Self;
            #[inline(always)]
            fn rem(self, rhs: Self) -> Self {
                self % rhs.get()
            }
        }
        impl core::ops::AddAssign<$native> for $name {
            #[inline(always)]
            fn add_assign(&mut self, rhs: $native) {
                *self = *self + rhs;
            }
        }
        impl core::ops::SubAssign<$native> for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: $native) {
                *self = *self - rhs;
            }
        }
        impl core::ops::MulAssign<$native> for $name {
            #[inline(always)]
            fn mul_assign(&mut self, rhs: $native) {
                *self = *self * rhs;
            }
        }
        impl core::ops::DivAssign<$native> for $name {
            #[inline(always)]
            fn div_assign(&mut self, rhs: $native) {
                *self = *self / rhs;
            }
        }
        impl core::ops::RemAssign<$native> for $name {
            #[inline(always)]
            fn rem_assign(&mut self, rhs: $native) {
                *self = *self % rhs;
            }
        }
        impl core::ops::AddAssign for $name {
            #[inline(always)]
            fn add_assign(&mut self, rhs: Self) {
                *self = *self + rhs;
            }
        }
        impl core::ops::SubAssign for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                *self = *self - rhs;
            }
        }
        impl core::ops::MulAssign for $name {
            #[inline(always)]
            fn mul_assign(&mut self, rhs: Self) {
                *self = *self * rhs;
            }
        }
        impl core::ops::DivAssign for $name {
            #[inline(always)]
            fn div_assign(&mut self, rhs: Self) {
                *self = *self / rhs;
            }
        }
        impl core::ops::RemAssign for $name {
            #[inline(always)]
            fn rem_assign(&mut self, rhs: Self) {
                *self = *self % rhs;
            }
        }
        impl core::ops::BitAnd<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, rhs: $native) -> Self {
                Self::from(self.get() & rhs)
            }
        }
        impl core::ops::BitOr<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, rhs: $native) -> Self {
                Self::from(self.get() | rhs)
            }
        }
        impl core::ops::BitXor<$native> for $name {
            type Output = Self;
            #[inline(always)]
            fn bitxor(self, rhs: $native) -> Self {
                Self::from(self.get() ^ rhs)
            }
        }
        impl core::ops::BitAnd for $name {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, rhs: Self) -> Self {
                self & rhs.get()
            }
        }
        impl core::ops::BitOr for $name {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, rhs: Self) -> Self {
                self | rhs.get()
            }
        }
        impl core::ops::BitXor for $name {
            type Output = Self;
            #[inline(always)]
            fn bitxor(self, rhs: Self) -> Self {
                self ^ rhs.get()
            }
        }
        impl core::ops::BitAndAssign<$native> for $name {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: $native) {
                *self = *self & rhs;
            }
        }
        impl core::ops::BitOrAssign<$native> for $name {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: $native) {
                *self = *self | rhs;
            }
        }
        impl core::ops::BitXorAssign<$native> for $name {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: $native) {
                *self = *self ^ rhs;
            }
        }
        impl core::ops::BitAndAssign for $name {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: Self) {
                *self = *self & rhs;
            }
        }
        impl core::ops::BitOrAssign for $name {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: Self) {
                *self = *self | rhs;
            }
        }
        impl core::ops::BitXorAssign for $name {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: Self) {
                *self = *self ^ rhs;
            }
        }
        impl core::ops::Shl<u32> for $name {
            type Output = Self;
            #[inline(always)]
            fn shl(self, rhs: u32) -> Self {
                Self::from(self.get() << rhs)
            }
        }
        impl core::ops::Shr<u32> for $name {
            type Output = Self;
            #[inline(always)]
            fn shr(self, rhs: u32) -> Self {
                Self::from(self.get() >> rhs)
            }
        }
        impl core::ops::ShlAssign<u32> for $name {
            #[inline(always)]
            fn shl_assign(&mut self, rhs: u32) {
                *self = *self << rhs;
            }
        }
        impl core::ops::ShrAssign<u32> for $name {
            #[inline(always)]
            fn shr_assign(&mut self, rhs: u32) {
                *self = *self >> rhs;
            }
        }
        impl core::ops::Not for $name {
            type Output = Self;
            #[inline(always)]
            fn not(self) -> Self {
                Self::from(!self.get())
            }
        }
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

// `u8` and `i8` are already alignment-1 in Rust, so no Pod wrapper is needed.
// These aliases let v1 ports referencing `PodU8`/`PodI8` resolve cleanly.
pub type PodU8 = u8;
pub type PodI8 = i8;

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
    pub fn get(&self) -> bool {
        self.0[0] != 0
    }
}

impl From<bool> for PodBool {
    #[inline(always)]
    fn from(v: bool) -> Self {
        Self([v as u8])
    }
}
impl From<PodBool> for bool {
    #[inline(always)]
    fn from(v: PodBool) -> Self {
        v.get()
    }
}
impl PartialEq for PodBool {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}
impl Eq for PodBool {}
impl PartialEq<bool> for PodBool {
    #[inline(always)]
    fn eq(&self, other: &bool) -> bool {
        self.get() == *other
    }
}
impl core::ops::Not for PodBool {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        Self::from(!self.get())
    }
}
impl fmt::Display for PodBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}
impl fmt::Debug for PodBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PodBool({})", self.get())
    }
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
/// # Validation contract
///
/// `len()` returns the raw u16 length prefix verbatim — no clamp to
/// `MAX`. On a freshly-constructed `PodVec` this invariant holds
/// naturally, but a `PodVec` cast from attacker-controlled account
/// bytes (via `bytemuck::from_bytes`) can carry a `len` prefix larger
/// than `MAX`. The unchecked accessors — `as_slice`, `as_mut_slice`,
/// `pop`, `iter`, `iter_mut`, `get`, `first`, `last`, and
/// `std::ops::Index` — panic (via Rust's bounds check, not UB) when
/// `len() > MAX`.
///
/// Two options for programs reading attacker-controlled data:
/// 1. Validate once at account load: `vec.validate()?`, then use the
///    unchecked accessors freely.
/// 2. Use the `try_*` variants (`try_as_slice`, `try_pop`, `try_get`)
///    which return `Err(CapacityError)` instead of panicking.
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

/// Error returned when pushing to a full `PodVec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityError;

impl core::fmt::Display for CapacityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PodVec capacity exceeded")
    }
}

impl<T: bytemuck::Pod, const MAX: usize> PodVec<T, MAX> {
    // Compile-time check: no padding between len and data.
    // If T has alignment > 1, repr(C) would insert padding after the 2-byte
    // PodU16, violating Pod. This catches it at monomorphization time.
    const _NO_PADDING: () = assert!(
        core::mem::size_of::<Self>() == 2 + core::mem::size_of::<T>() * MAX,
        "PodVec<T, MAX>: T must have alignment 1 (no padding allowed)"
    );

    // Compile-time check: MAX must fit in the u16 length prefix. Without
    // this guard, `try_push` / `pop` / `truncate` / `set_from_slice` /
    // `try_extend_from_slice` silently truncate `usize → u16` and corrupt
    // `len` when the in-memory vec exceeds u16::MAX items. Catch at
    // monomorphization so `PodVec<T, N>` with `N > 65_535` fails to compile.
    const _MAX_FITS_U16: () = assert!(
        MAX <= u16::MAX as usize,
        "PodVec<T, MAX>: MAX must be <= 65_535 (u16::MAX). Use a larger \
         length-prefix wrapper for capacities beyond this."
    );

    // --- Length / capacity ---

    /// Returns the number of populated elements.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len.get() as usize
    }

    /// Returns `true` if no elements are populated.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len.is_zero()
    }

    /// Returns `true` if the vector is at capacity.
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.len() == MAX
    }

    /// Returns the maximum capacity.
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        MAX
    }

    // --- Validation (for `PodVec`s cast from attacker-controlled bytes) ---

    /// Returns `true` iff the in-memory length prefix is at most `MAX`.
    ///
    /// A freshly-constructed `PodVec` always satisfies this, but a `PodVec`
    /// cast from account bytes may not. The unchecked accessors below rely
    /// on this invariant to avoid panicking — call this (or [`validate`])
    /// at load time, or use the `try_*` variants on every access.
    ///
    /// [`validate`]: Self::validate
    #[inline(always)]
    pub fn is_valid_len(&self) -> bool {
        self.len() <= MAX
    }

    /// Returns `Ok(())` iff the in-memory length prefix is at most `MAX`,
    /// `Err(CapacityError)` otherwise.
    ///
    /// `?`-operator-friendly shorthand for `is_valid_len()`.
    #[inline(always)]
    pub fn validate(&self) -> Result<(), CapacityError> {
        if self.is_valid_len() {
            Ok(())
        } else {
            Err(CapacityError)
        }
    }

    // --- Element access ---

    /// Returns the populated elements as a slice.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX` (can happen on a `PodVec` cast from
    /// attacker-controlled account bytes). Use [`try_as_slice`] for a
    /// non-panicking variant, or [`validate`] once at load time.
    ///
    /// [`try_as_slice`]: Self::try_as_slice
    /// [`validate`]: Self::validate
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len()]
    }

    /// Non-panicking variant of [`as_slice`]. Returns `Err(CapacityError)` if
    /// `len() > MAX`.
    ///
    /// [`as_slice`]: Self::as_slice
    #[inline]
    pub fn try_as_slice(&self) -> Result<&[T], CapacityError> {
        self.validate()?;
        Ok(self.as_slice())
    }

    /// Returns the populated elements as a mutable slice.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX`. See [`as_slice`] for the validation story;
    /// use [`try_as_mut_slice`] for a non-panicking variant.
    ///
    /// [`as_slice`]: Self::as_slice
    /// [`try_as_mut_slice`]: Self::try_as_mut_slice
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        &mut self.data[..len]
    }

    /// Non-panicking variant of [`as_mut_slice`]. Returns `Err(CapacityError)`
    /// if `len() > MAX`.
    ///
    /// [`as_mut_slice`]: Self::as_mut_slice
    #[inline]
    pub fn try_as_mut_slice(&mut self) -> Result<&mut [T], CapacityError> {
        self.validate()?;
        Ok(self.as_mut_slice())
    }

    /// Returns a reference to the element at `idx`, or `None` if out of bounds.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX` (propagates from [`as_slice`]). Use
    /// [`try_get`] for a non-panicking variant.
    ///
    /// [`as_slice`]: Self::as_slice
    /// [`try_get`]: Self::try_get
    #[inline(always)]
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.as_slice().get(idx)
    }

    /// Non-panicking variant of [`get`]. Returns `Err(CapacityError)` if
    /// `len() > MAX`; returns `Ok(None)` if `idx` is out of bounds but
    /// the length prefix is valid.
    ///
    /// [`get`]: Self::get
    #[inline]
    pub fn try_get(&self, idx: usize) -> Result<Option<&T>, CapacityError> {
        self.validate()?;
        Ok(self.get(idx))
    }

    /// Returns a mutable reference to the element at `idx`, or `None` if
    /// out of bounds.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX`.
    #[inline(always)]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(idx)
    }

    /// Returns a reference to the first element, or `None` if empty.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX`.
    #[inline(always)]
    pub fn first(&self) -> Option<&T> {
        self.as_slice().first()
    }

    /// Returns a reference to the last element, or `None` if empty.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX`.
    #[inline(always)]
    pub fn last(&self) -> Option<&T> {
        self.as_slice().last()
    }

    // --- Iteration ---

    /// Returns an iterator over the populated elements.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX`. For attacker-controlled data, [`validate`]
    /// once at load time or use [`try_as_slice`]`()?.iter()` instead.
    ///
    /// [`validate`]: Self::validate
    /// [`try_as_slice`]: Self::try_as_slice
    #[inline(always)]
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns a mutable iterator over the populated elements.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX`.
    #[inline(always)]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    // --- Mutation ---

    /// Centralised `len` write: every length-mutating method funnels
    /// through here so `Self::_MAX_FITS_U16` is forced at monomorphization
    /// time. Without this, a `PodVec<T, 70_000>` could compile and silently
    /// truncate its prefix through any direct `self.len = ...` write.
    #[inline(always)]
    fn write_len(&mut self, new_len: usize) {
        #[allow(clippy::let_unit_value)]
        let _ = Self::_MAX_FITS_U16;
        self.len = PodU16::from(new_len as u16);
    }

    /// Appends an element. Returns `Err(CapacityError)` if the vector is full.
    #[inline]
    pub fn try_push(&mut self, value: T) -> Result<(), CapacityError> {
        let len = self.len();
        if len >= MAX {
            return Err(CapacityError);
        }
        self.data[len] = value;
        self.write_len(len + 1);
        Ok(())
    }

    /// Appends an element. Panics if the vector is full.
    #[inline]
    pub fn push(&mut self, value: T) {
        self.try_push(value).expect("PodVec: push on full vector");
    }

    /// Removes and returns the last element, or `None` if empty.
    ///
    /// # Panics
    ///
    /// Panics if `len() > MAX` (indexes `self.data[len-1]` which is OOB
    /// on `[T; MAX]`). Use [`try_pop`] for a non-panicking variant.
    ///
    /// [`try_pop`]: Self::try_pop
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        let len = self.len();
        if len == 0 {
            return None;
        }
        let val = self.data[len - 1];
        self.write_len(len - 1);
        Some(val)
    }

    /// Non-panicking variant of [`pop`]. Returns `Err(CapacityError)` if
    /// `len() > MAX`; returns `Ok(None)` if the vector is validly empty.
    ///
    /// [`pop`]: Self::pop
    #[inline]
    pub fn try_pop(&mut self) -> Result<Option<T>, CapacityError> {
        self.validate()?;
        Ok(self.pop())
    }

    /// Removes all elements, setting length to 0.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.write_len(0);
    }

    /// Shortens the vector to `new_len`. No-op if `new_len >= len()`.
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len() {
            self.write_len(new_len);
        }
    }

    /// Appends every element from `src`. Returns `Err(CapacityError)` if it
    /// would exceed capacity; the vector is not modified on failure.
    #[inline]
    pub fn try_extend_from_slice(&mut self, src: &[T]) -> Result<(), CapacityError> {
        let len = self.len();
        if len + src.len() > MAX {
            return Err(CapacityError);
        }
        self.data[len..len + src.len()].copy_from_slice(src);
        self.write_len(len + src.len());
        Ok(())
    }

    /// Appends every element from `src`. Panics if it would exceed capacity.
    #[inline]
    pub fn extend_from_slice(&mut self, src: &[T]) {
        self.try_extend_from_slice(src)
            .expect("PodVec: extend exceeds capacity");
    }

    /// Replaces the contents with `src`. Panics if `src.len() > MAX`.
    #[inline]
    pub fn set_from_slice(&mut self, src: &[T]) {
        assert!(src.len() <= MAX, "PodVec: slice length exceeds capacity");
        self.data[..src.len()].copy_from_slice(src);
        self.write_len(src.len());
    }
}

// --- Indexing ---

impl<T: bytemuck::Pod, const MAX: usize> core::ops::Index<usize> for PodVec<T, MAX> {
    type Output = T;
    #[inline(always)]
    fn index(&self, idx: usize) -> &T {
        &self.as_slice()[idx]
    }
}

impl<T: bytemuck::Pod, const MAX: usize> core::ops::IndexMut<usize> for PodVec<T, MAX> {
    #[inline(always)]
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.as_mut_slice()[idx]
    }
}

// --- Deref to slice (so all slice methods work transparently) ---

impl<T: bytemuck::Pod, const MAX: usize> core::ops::Deref for PodVec<T, MAX> {
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: bytemuck::Pod, const MAX: usize> core::ops::DerefMut for PodVec<T, MAX> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

// --- IntoIterator for &PodVec and &mut PodVec ---

impl<'a, T: bytemuck::Pod, const MAX: usize> IntoIterator for &'a PodVec<T, MAX> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: bytemuck::Pod, const MAX: usize> IntoIterator for &'a mut PodVec<T, MAX> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: bytemuck::Pod, const MAX: usize> Default for PodVec<T, MAX> {
    fn default() -> Self {
        // Safety: PodVec is Pod, so all-zeros is a valid representation.
        unsafe { core::mem::zeroed() }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use {super::*, alloc::format};

    // ---- Unsigned integer Pod types ---------------------------------------

    #[test]
    fn pod_u64_round_trips_through_native() {
        let p = PodU64::from(42u64);
        assert_eq!(p.get(), 42);
        let back: u64 = p.into();
        assert_eq!(back, 42);
    }

    #[test]
    fn pod_unsigned_constants_match_native_bounds() {
        assert_eq!(PodU64::ZERO.get(), 0);
        assert_eq!(PodU64::MAX.get(), u64::MAX);
        assert_eq!(PodU64::MIN.get(), u64::MIN);
        assert_eq!(PodU32::MAX.get(), u32::MAX);
        assert_eq!(PodU16::MAX.get(), u16::MAX);
        assert_eq!(PodU128::MAX.get(), u128::MAX);
    }

    #[test]
    fn pod_is_zero_reflects_underlying_value() {
        assert!(PodU64::ZERO.is_zero());
        assert!(!PodU64::from(1u64).is_zero());
    }

    #[test]
    fn pod_arithmetic_ops_match_native() {
        let a = PodU64::from(10u64);
        let b = PodU64::from(3u64);
        assert_eq!((a + b).get(), 13);
        assert_eq!((a - b).get(), 7);
        assert_eq!((a * b).get(), 30);
        assert_eq!((a / b).get(), 3);
        assert_eq!((a % b).get(), 1);

        // Mixed native rhs.
        assert_eq!((a + 5u64).get(), 15);
        assert_eq!((a - 5u64).get(), 5);
        assert_eq!((a * 2u64).get(), 20);
        assert_eq!((a / 2u64).get(), 5);
        assert_eq!((a % 3u64).get(), 1);
    }

    #[test]
    fn pod_assign_ops_mutate_in_place() {
        let mut x = PodU64::from(5u64);
        x += 3u64;
        assert_eq!(x.get(), 8);
        x -= 2u64;
        assert_eq!(x.get(), 6);
        x *= 4u64;
        assert_eq!(x.get(), 24);
        x /= 3u64;
        assert_eq!(x.get(), 8);
        x %= 5u64;
        assert_eq!(x.get(), 3);

        // Pod-rhs variants.
        let mut y = PodU64::from(100u64);
        y += PodU64::from(10u64);
        assert_eq!(y.get(), 110);
        y -= PodU64::from(5u64);
        assert_eq!(y.get(), 105);
        y *= PodU64::from(2u64);
        assert_eq!(y.get(), 210);
        y /= PodU64::from(7u64);
        assert_eq!(y.get(), 30);
        y %= PodU64::from(7u64);
        assert_eq!(y.get(), 2);
    }

    #[test]
    fn pod_checked_arith_detects_overflow() {
        let max = PodU64::MAX;
        assert_eq!(max.checked_add(1u64), None);
        assert_eq!(PodU64::ZERO.checked_sub(1u64), None);
        assert_eq!(max.checked_mul(2u64), None);
        assert_eq!(PodU64::from(10u64).checked_div(0u64), None);

        assert_eq!(PodU64::from(10u64).checked_add(5u64).unwrap().get(), 15);
        assert_eq!(PodU64::from(10u64).checked_sub(3u64).unwrap().get(), 7);
        assert_eq!(PodU64::from(10u64).checked_mul(3u64).unwrap().get(), 30);
        assert_eq!(PodU64::from(10u64).checked_div(3u64).unwrap().get(), 3);
    }

    #[test]
    fn pod_saturating_arith_caps_at_bounds() {
        assert_eq!(PodU64::MAX.saturating_add(1u64), PodU64::MAX);
        assert_eq!(PodU64::ZERO.saturating_sub(1u64), PodU64::ZERO);
        assert_eq!(PodU64::MAX.saturating_mul(2u64), PodU64::MAX);

        assert_eq!(PodU64::from(10u64).saturating_add(5u64).get(), 15);
        assert_eq!(PodU64::from(10u64).saturating_sub(3u64).get(), 7);
        assert_eq!(PodU64::from(10u64).saturating_mul(3u64).get(), 30);
    }

    #[test]
    fn pod_comparisons_match_native_ordering() {
        let a = PodU64::from(5u64);
        let b = PodU64::from(10u64);

        assert!(a < b);
        assert!(b > a);
        assert!(a <= PodU64::from(5u64));
        assert!(a >= PodU64::from(5u64));
        assert_eq!(a, PodU64::from(5u64));
        assert_ne!(a, b);

        // Mixed native-rhs comparisons.
        assert_eq!(a, 5u64);
        assert!(a < 10u64);

        // Ord.
        assert_eq!(a.cmp(&b), core::cmp::Ordering::Less);
    }

    #[test]
    fn pod_display_and_debug_format_as_native() {
        let p = PodU64::from(42u64);
        assert_eq!(format!("{p}"), "42");
        assert_eq!(format!("{p:?}"), "PodU64(42)");
    }

    // ---- Signed integer Pod types -----------------------------------------

    #[test]
    fn pod_signed_handles_negative_values() {
        let p = PodI64::from(-42i64);
        assert_eq!(p.get(), -42);

        let a = PodI64::from(10i64);
        let b = PodI64::from(-3i64);
        assert_eq!((a + b).get(), 7);
        assert_eq!((a - b).get(), 13);
        assert_eq!((a * b).get(), -30);
    }

    #[test]
    fn pod_signed_neg_flips_sign() {
        let p = PodI32::from(7i32);
        assert_eq!((-p).get(), -7);
        assert_eq!((-(-p)).get(), 7);
    }

    #[test]
    fn pod_signed_min_max_roundtrip() {
        assert_eq!(PodI64::MAX.get(), i64::MAX);
        assert_eq!(PodI64::MIN.get(), i64::MIN);
    }

    // ---- PodBool ----------------------------------------------------------

    #[test]
    fn pod_bool_any_nonzero_byte_is_true() {
        assert!(!PodBool::from(false).get());
        assert!(PodBool::from(true).get());
        let back: bool = PodBool::from(true).into();
        assert!(back);
    }

    #[test]
    fn pod_bool_not_flips_value() {
        assert!((!PodBool::from(false)).get());
        assert!(!(!PodBool::from(true)).get());
    }

    #[test]
    fn pod_bool_equality_and_display() {
        assert_eq!(PodBool::from(true), PodBool::from(true));
        assert_ne!(PodBool::from(true), PodBool::from(false));
        assert_eq!(PodBool::from(true), true);
        assert_eq!(format!("{}", PodBool::from(true)), "true");
        assert_eq!(format!("{:?}", PodBool::from(false)), "PodBool(false)");
    }

    // ---- PodVec -----------------------------------------------------------

    #[test]
    fn pod_vec_default_is_empty() {
        let v: PodVec<PodU32, 8> = PodVec::default();
        assert_eq!(v.len(), 0);
        assert!(v.is_empty());
        assert!(!v.is_full());
        assert_eq!(v.as_slice().len(), 0);
    }

    #[test]
    fn pod_vec_push_pop_roundtrip() {
        let mut v: PodVec<PodU64, 4> = PodVec::default();
        v.push(PodU64::from(1u64));
        v.push(PodU64::from(2u64));
        v.push(PodU64::from(3u64));
        assert_eq!(v.len(), 3);
        assert_eq!(v.first().unwrap().get(), 1);
        assert_eq!(v.last().unwrap().get(), 3);
        assert_eq!(v.get(1).unwrap().get(), 2);

        assert_eq!(v.pop().unwrap().get(), 3);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn pod_vec_try_push_returns_err_at_capacity() {
        let mut v: PodVec<PodU32, 2> = PodVec::default();
        v.try_push(PodU32::from(1u32)).unwrap();
        v.try_push(PodU32::from(2u32)).unwrap();
        assert!(v.is_full());
        assert!(v.try_push(PodU32::from(3u32)).is_err());
    }

    #[test]
    fn pod_vec_clear_and_truncate() {
        let mut v: PodVec<PodU32, 8> = PodVec::default();
        for i in 0..5u32 {
            v.push(PodU32::from(i));
        }
        assert_eq!(v.len(), 5);

        v.truncate(2);
        assert_eq!(v.len(), 2);

        v.clear();
        assert_eq!(v.len(), 0);
        assert!(v.is_empty());
    }

    #[test]
    fn pod_vec_iter_yields_pushed_elements() {
        let mut v: PodVec<PodU16, 4> = PodVec::default();
        v.push(PodU16::from(10u16));
        v.push(PodU16::from(20u16));
        v.push(PodU16::from(30u16));

        let collected: alloc::vec::Vec<u16> = v.iter().map(|p| p.get()).collect();
        assert_eq!(collected, alloc::vec![10, 20, 30]);

        // Mutating iteration.
        for item in v.iter_mut() {
            *item = PodU16::from(item.get() * 2);
        }
        let doubled: alloc::vec::Vec<u16> = (&v).into_iter().map(|p| p.get()).collect();
        assert_eq!(doubled, alloc::vec![20, 40, 60]);
    }

    #[test]
    fn pod_vec_get_out_of_bounds_returns_none() {
        let mut v: PodVec<PodU64, 4> = PodVec::default();
        v.push(PodU64::from(1u64));
        assert!(v.get(0).is_some());
        assert!(v.get(1).is_none());
        assert!(v.get_mut(1).is_none());
    }
}
