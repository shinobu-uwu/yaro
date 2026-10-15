use core::{
    fmt::{self, Debug},
    ops::Add,
};

use crate::hhdm_offset;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
#[repr(transparent)]
pub struct PhysicalAddress(usize);

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl PhysicalAddress {
    /// Adds a byte offset, returning `None` on integer overflow.
    #[inline]
    pub const fn checked_add(self, offset: usize) -> Option<Self> {
        match self.0.checked_add(offset) {
            Some(address) => Some(Self(address)),
            None => None,
        }
    }

    /// Rounds down to the nearest multiple of `alignment`.
    ///
    /// # Panics
    /// Panics if `alignment` is not a nonzero power of two.
    #[inline]
    pub const fn align_down(self, alignment: usize) -> Self {
        assert!(
            alignment.is_power_of_two(),
            "Alignment must be a nonzero power of two"
        );
        Self(self.0 & !(alignment - 1))
    }

    /// Rounds up to the nearest multiple of `alignment`, returning `None`
    /// on integer overflow.
    ///
    /// # Panics
    /// Panics if `alignment` is not a nonzero power of two.
    #[inline]
    pub const fn align_up(self, alignment: usize) -> Option<Self> {
        assert!(
            alignment.is_power_of_two(),
            "Alignment must be a nonzero power of two"
        );
        match self.0.checked_add(alignment - 1) {
            Some(address) => Some(Self(address & !(alignment - 1))),
            None => None,
        }
    }

    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub const fn as_u64(&self) -> u64 {
        self.0 as _
    }

    #[inline(always)]
    pub const fn as_usize(&self) -> usize {
        self.0 as _
    }

    /// Returns the higher-half direct map (HHDM) virtual address by adding
    /// the offset supplied during memory initialization.
    ///
    /// This computes an address without checking that the physical memory is mapped.
    ///
    /// # Panics
    /// Panics if the memory subsystem has not been initialized.
    #[inline(always)]
    pub fn to_hhdm(&self) -> VirtualAddress {
        VirtualAddress(self.0 + hhdm_offset())
    }
}

impl VirtualAddress {
    /// Adds a byte offset, returning `None` on integer overflow.
    #[inline]
    pub const fn checked_add(self, offset: usize) -> Option<Self> {
        match self.0.checked_add(offset) {
            Some(address) => Some(Self(address)),
            None => None,
        }
    }

    /// Rounds down to the nearest multiple of `alignment`.
    ///
    /// # Panics
    /// Panics if `alignment` is not a nonzero power of two.
    #[inline]
    pub const fn align_down(self, alignment: usize) -> Self {
        assert!(
            alignment.is_power_of_two(),
            "Alignment must be a nonzero power of two"
        );
        Self(self.0 & !(alignment - 1))
    }

    /// Rounds up to the nearest multiple of `alignment`, returning `None`
    /// on integer overflow.
    ///
    /// # Panics
    /// Panics if `alignment` is not a nonzero power of two.
    #[inline]
    pub const fn align_up(self, alignment: usize) -> Option<Self> {
        assert!(
            alignment.is_power_of_two(),
            "Alignment must be a nonzero power of two"
        );
        match self.0.checked_add(alignment - 1) {
            Some(address) => Some(Self(address & !(alignment - 1))),
            None => None,
        }
    }

    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    /// Creates a virtual address from the given pointer
    #[inline]
    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        Self::new(ptr as *const () as _)
    }

    /// Converts the address to a raw pointer.
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.as_u64() as *const T
    }

    /// Converts the address to a mutable raw pointer.
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>().cast_mut()
    }

    /// Convenience method for checking if a virtual address is null.
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PhysAddr")
            .field(&format_args!("{:x}", self.0))
            .finish()
    }
}

impl Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VirtAddr")
            .field(&format_args!("{:x}", self.0))
            .finish()
    }
}

impl Add<usize> for PhysicalAddress {
    type Output = Self;

    fn add(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl Add for PhysicalAddress {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        self + rhs.0
    }
}

impl Add<usize> for VirtualAddress {
    type Output = Self;

    fn add(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl Add for VirtualAddress {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        self + rhs.0
    }
}
