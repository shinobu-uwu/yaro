use core::arch::asm;

use bit_field::BitField;

use crate::{
    address::{PhysicalAddress, VirtualAddress},
    paging::{Arch, Entry},
};

const PAGE_TABLE_ENTRIES: usize = 512;

const _: () = {
    assert!(size_of::<X86Entry>() == 8);
};

#[derive(Debug, Clone)]
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [X86Entry; PAGE_TABLE_ENTRIES],
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct X86Entry(u64);

#[derive(Debug, Clone, Copy)]
pub struct X86Arch;

impl Arch for X86Arch {
    const PAGE_LEVELS: usize = 4;
    const PAGE_TABLE_ENTRIES: usize = PAGE_TABLE_ENTRIES;

    type Entry = X86Entry;

    #[inline(always)]
    fn table() -> PhysicalAddress {
        let addr: usize;

        unsafe {
            asm!("mov {}, cr3", out(reg) addr);
        }

        PhysicalAddress::new(addr.get_bits(12..52) << 12)
    }

    #[inline(always)]
    fn root_level() -> usize {
        Self::PAGE_LEVELS - 1
    }

    #[inline(always)]
    fn is_address_valid(address: VirtualAddress) -> bool {
        // Four-level paging requires bits 48..64 to sign-extend bit 47.
        let address = address.as_u64();
        let upper = if address.get_bit(47) { 0xffff } else { 0 };
        address.get_bits(48..64) == upper
    }

    fn page_size(level: usize) -> usize {
        // Leaf levels: 0 = 4 KiB, 1 = 2 MiB, 2 = 1 GiB.
        1usize << (12 + 9 * level)
    }

    #[inline(always)]
    unsafe fn read_entry(address: VirtualAddress) -> Self::Entry {
        X86Entry(unsafe { core::ptr::read_volatile(address.as_ptr()) })
    }

    fn table_index(address: VirtualAddress, level: usize) -> usize {
        let start = 12 + 9 * level;
        address.as_u64().get_bits(start..start + 9) as usize
    }
}

impl Entry for X86Entry {
    fn is_present(&self) -> bool {
        self.0.get_bit(0)
    }

    fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new((self.0.get_bits(12..52) << 12) as usize)
    }

    fn is_leaf(&self, level: usize) -> bool {
        level == 0 || (matches!(level, 1 | 2) && self.0.get_bit(7))
    }
}

impl X86Entry {
    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }
}
