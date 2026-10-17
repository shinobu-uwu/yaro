use core::marker::PhantomData;

use crate::address::{PhysicalAddress, VirtualAddress};

pub mod arch;
pub mod mapper;

#[derive(Debug, Clone)]
pub struct PageTable<A: Arch> {
    base: PhysicalAddress,
    level: usize,
    _arch: PhantomData<A>,
}

impl<A: Arch> PageTable<A> {
    pub const fn new(base: PhysicalAddress, level: usize) -> Self {
        Self {
            base,
            level,
            _arch: PhantomData,
        }
    }

    pub fn root() -> Self {
        Self::new(A::table(), A::root_level())
    }

    pub unsafe fn entry(&self, index: usize) -> Option<A::Entry> {
        assert!(index < A::PAGE_TABLE_ENTRIES);

        let offset = index.checked_mul(size_of::<A::Entry>())?;
        let address = self.base.checked_add(offset)?.to_hhdm();

        Some(unsafe { A::read_entry(address) })
    }

    pub unsafe fn next_table(&self, index: usize) -> Option<PageTable<A>> {
        // entry() checks the index before reading page-table memory.
        debug_assert!(index < A::PAGE_TABLE_ENTRIES);

        let child_level = self.level.checked_sub(1)?;
        let entry = unsafe { self.entry(index) }?;

        if !entry.is_present() || entry.is_leaf(self.level) {
            return None;
        }

        Some(Self::new(entry.address(), child_level))
    }

    pub fn index_of(&self, address: VirtualAddress) -> usize {
        A::table_index(address, self.level)
    }
}

pub trait Arch: Clone + Copy {
    const PAGE_LEVELS: usize;
    const PAGE_TABLE_ENTRIES: usize;

    /// The hardware entry representation used by this architecture.
    type Entry: Entry;

    fn table() -> PhysicalAddress;
    fn root_level() -> usize;
    fn is_address_valid(addr: VirtualAddress) -> bool;

    /// Size in bytes of a leaf page at this level; must be a nonzero power of two.
    fn page_size(level: usize) -> usize;

    /// Reads an entry from its virtual address.
    ///
    /// # Safety
    /// The address must be aligned, mapped, and readable as a hardware
    /// entry, with access appropriately synchronized.
    unsafe fn read_entry(address: VirtualAddress) -> Self::Entry;

    fn table_index(address: VirtualAddress, level: usize) -> usize;
}

pub trait Entry: Clone {
    fn is_present(&self) -> bool;
    fn address(&self) -> PhysicalAddress;
    fn is_leaf(&self, level: usize) -> bool;
}
