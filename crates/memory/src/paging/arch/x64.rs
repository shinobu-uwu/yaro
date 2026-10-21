use core::arch::asm;

use bit_field::BitField;
use bitflags::bitflags;

use crate::{
    address::{PhysicalAddress, VirtualAddress},
    paging::{Arch, Entry, MappingFlags},
};

const PAGE_TABLE_ENTRIES: usize = 512;
const ENTRY_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
const _: () = {
    assert!(size_of::<X86Entry>() == 8);
    assert!(size_of::<PageTable>() == 4096);
    assert!(align_of::<PageTable>() == 4096);
    assert!(PAGE_TABLE_ENTRIES == 512);
};

bitflags! {
    /// Hardware flags for x86-64 page-table entries.
    ///
    /// Valid flags depend on the entry's level, whether it maps a leaf page,
    /// and the enabled processor features.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PageFlags: u64 {
        /// The entry participates in address translation.
        const PRESENT = 1 << 0;
        /// Allows writes, subject to permissions in parent entries.
        const WRITABLE = 1 << 1;
        /// Allows user-mode access, subject to permissions in parent entries.
        const USER_ACCESSIBLE = 1 << 2;
        /// Page-level write-through caching control (PWT).
        const WRITE_THROUGH = 1 << 3;
        /// Page-level cache-disable control (PCD).
        const NO_CACHE = 1 << 4;
        /// Set by hardware when the entry is used for translation.
        const ACCESSED = 1 << 5;
        /// Set by hardware when a leaf page is written.
        const DIRTY = 1 << 6;
        /// Maps a 2 MiB or 1 GiB leaf at level 1 or 2, respectively.
        const HUGE_PAGE = 1 << 7;
        /// PAT index bit for a 4 KiB leaf; shares bit 7 with HUGE_PAGE.
        const PAT_4K = 1 << 7;
        /// Keeps a leaf translation across CR3 reloads when CR4.PGE is enabled.
        const GLOBAL = 1 << 8;
        /// PAT index bit for a 2 MiB or 1 GiB leaf.
        /// In other entry formats, bit 12 belongs to the physical address.
        const PAT_HUGE = 1 << 12;
        /// Disallows instruction fetches; requires EFER.NXE to be enabled.
        const NO_EXECUTE = 1 << 63;
    }
}

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
    const PAGE_SIZE: usize = 4096;

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

    #[inline(always)]
    unsafe fn write_entry(address: VirtualAddress, entry: Self::Entry) {
        // SAFETY: The caller guarantees an 8-byte-aligned, writable entry
        // address and appropriately synchronized access.
        unsafe {
            core::ptr::write_volatile(address.as_mut_ptr::<u64>(), entry.0);
        }
    }

    #[inline]
    unsafe fn invalidate(address: VirtualAddress) {
        // SAFETY: The caller guarantees kernel privilege and a valid address.
        // Keep the default memory effects so entry stores cannot be reordered
        // across the invalidation by the compiler.
        unsafe {
            asm!("invlpg [{}]", in(reg) address.as_u64(), options(nostack, preserves_flags));
        }
    }

    fn table_index(address: VirtualAddress, level: usize) -> usize {
        let start = 12 + 9 * level;
        address.as_u64().get_bits(start..start + 9) as usize
    }
}

impl Entry for X86Entry {
    type Flags = PageFlags;

    fn new_table(frame: PhysicalAddress, requested_flags: PageFlags, level: usize) -> Self {
        assert!(
            (1..X86Arch::PAGE_LEVELS).contains(&level),
            "invalid parent table level"
        );

        assert!(
            frame.is_aligned(X86Arch::page_size(0)),
            "page-table entry frame is not aligned"
        );
        assert!(
            frame.as_u64() & !ENTRY_ADDRESS_MASK == 0,
            "physical address cannot be encoded in the entry"
        );
        assert!(
            requested_flags.bits() & !PageFlags::all().difference(PageFlags::PAT_HUGE).bits() == 0,
            "invalid flags for a level-0 leaf entry"
        );

        // Leave writes and execution available at the parent so leaf entries
        // can select their own permissions. User access must be allowed at
        // every level, but leaf cache/PAT/global attributes do not belong here.
        let flags = PageFlags::PRESENT
            | PageFlags::WRITABLE
            | (requested_flags & PageFlags::USER_ACCESSIBLE);

        Self(frame.as_u64() | flags.bits())
    }

    fn new_page(frame: PhysicalAddress, flags: PageFlags) -> Self {
        assert!(
            frame.is_aligned(X86Arch::page_size(0)),
            "page-table entry frame is not aligned"
        );
        assert!(
            frame.as_u64() & !ENTRY_ADDRESS_MASK == 0,
            "physical address cannot be encoded in the entry"
        );
        // Bit 7 is PAT at level 0; bit 12 belongs to the physical address.
        assert!(
            flags.bits() & !PageFlags::all().difference(PageFlags::PAT_HUGE).bits() == 0,
            "invalid flags for a level-0 leaf entry"
        );

        Self(frame.as_u64() | (flags | PageFlags::PRESENT).bits())
    }

    fn allows_mapping(&self, flags: PageFlags, level: usize) -> bool {
        assert!(
            (1..X86Arch::PAGE_LEVELS).contains(&level),
            "invalid parent table level"
        );
        assert!(
            flags.bits() & !PageFlags::all().difference(PageFlags::PAT_HUGE).bits() == 0,
            "invalid flags for a level-0 leaf entry"
        );

        if !self.is_present() || self.is_leaf(level) {
            return false;
        }

        let parent = self.flags(level);

        (!flags.contains(PageFlags::WRITABLE) || parent.contains(PageFlags::WRITABLE))
            && (!flags.contains(PageFlags::USER_ACCESSIBLE)
                || parent.contains(PageFlags::USER_ACCESSIBLE))
            && (flags.contains(PageFlags::NO_EXECUTE) || !parent.contains(PageFlags::NO_EXECUTE))
    }

    #[inline]
    fn flags(&self, level: usize) -> PageFlags {
        assert!(level < X86Arch::PAGE_LEVELS);

        let mut flags = PageFlags::from_bits_truncate(self.0);

        if !matches!(level, 1 | 2) || !flags.contains(PageFlags::HUGE_PAGE) {
            flags.remove(PageFlags::PAT_HUGE);
        }

        flags
    }

    fn is_present(&self) -> bool {
        self.0.get_bit(0)
    }

    fn address(&self) -> PhysicalAddress {
        PhysicalAddress::new((self.0.get_bits(12..52) << 12) as usize)
    }

    fn is_leaf(&self, level: usize) -> bool {
        level == 0 || (matches!(level, 1 | 2) && self.0.get_bit(7))
    }

    #[inline]
    fn empty() -> Self {
        Self(0)
    }
}

impl MappingFlags for PageFlags {
    #[inline]
    fn new() -> Self {
        Self::NO_EXECUTE
    }

    #[inline]
    fn user(mut self, enabled: bool) -> Self {
        self.set(Self::USER_ACCESSIBLE, enabled);
        self
    }

    #[inline]
    fn write(mut self, enabled: bool) -> Self {
        self.set(Self::WRITABLE, enabled);
        self
    }

    #[inline]
    fn execute(mut self, enabled: bool) -> Self {
        self.set(Self::NO_EXECUTE, !enabled);
        self
    }
}
