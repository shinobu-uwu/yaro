use core::fmt::{self, Debug};

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
#[repr(transparent)]
pub struct PhysicalAddress(usize);

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl PhysicalAddress {
    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }
}

impl VirtualAddress {
    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }
}

impl Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PhysicalAddress")
            .field(&format_args!("PhysAddr({:#x})", self.0))
            .finish()
    }
}

impl Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PhysicalAddress")
            .field(&format_args!("VirtAddr({:#x})", self.0))
            .finish()
    }
}
