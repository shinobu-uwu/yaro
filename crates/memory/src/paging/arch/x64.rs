use core::arch::asm;

use crate::{
    address::{PhysicalAddress, VirtualAddress},
    paging::{self, Arch, Entry},
};

const PAGE_TABLE_ENTRIES: usize = 512;
const CR3_ADDRESS_MASK: usize = 0x000f_ffff_ffff_f000;
const CANONICAL_ADDRESS_MASK: u64 = 0xFFFF_8000_0000_0000;
const ENTRY_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;

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

        PhysicalAddress::new(addr & CR3_ADDRESS_MASK)
    }

    #[inline(always)]
    fn root_level() -> usize {
        Self::PAGE_LEVELS - 1
    }

    #[inline(always)]
    fn is_address_valid(address: VirtualAddress) -> bool {
        // On x64 addresses must be canonical
        let masked = address.as_u64() & CANONICAL_ADDRESS_MASK;

        masked == CANONICAL_ADDRESS_MASK || masked == 0
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
        (address.as_u64() >> (12 + 9 * level)) as usize & 0x1ff
    }
}

impl Entry for X86Entry {
    fn is_present(&self) -> bool {
        self.0 & 1 != 0
    }

    fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new((self.0 & ENTRY_ADDRESS_MASK) as usize)
    }

    fn is_leaf(&self, level: usize) -> bool {
        level == 0 || (matches!(level, 1 | 2) && self.0 & (1 << 7) != 0)
    }
}

impl X86Entry {
    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }
}
