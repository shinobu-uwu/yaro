use bit_field::BitField;

use crate::{
    address::{PhysicalAddress, VirtualAddress},
    paging::{Arch, Entry, PageTable},
};

pub struct PageMapper<A: Arch> {
    root: PageTable<A>,
}

impl<A: Arch> PageMapper<A> {
    pub fn new() -> Self {
        let root = PageTable::root();

        Self { root }
    }

    /// Translates a virtual address using this mapper's page tables.
    ///
    /// # Safety
    /// Memory initialization must be complete. All traversed page tables
    /// must remain valid and readable through the HHDM, with concurrent
    /// modifications and reclamation appropriately synchronized.
    pub unsafe fn translate(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        if !A::is_address_valid(address) {
            return None;
        }

        let mut table = self.root.clone();

        loop {
            let index = table.index_of(address);

            // SAFETY: The caller guarantees valid, readable page tables
            // and appropriate synchronization throughout the walk.
            let entry = unsafe { table.entry(index) }?;

            if !entry.is_present() {
                return None;
            }

            if entry.is_leaf(table.level) {
                let page_size = A::page_size(table.level);
                let offset_bits = page_size.trailing_zeros() as usize;
                let offset = if offset_bits == 0 {
                    0
                } else {
                    address.as_u64().get_bits(0..offset_bits) as usize
                };

                // Align to the leaf's page size, removing flag bits such
                // as the x86 PAT bit in large-page entries.
                let base = entry.address().align_down(page_size);

                return base.checked_add(offset);
            }

            let child_level = table.level.checked_sub(1)?;
            table = PageTable::new(entry.address(), child_level);
        }
    }
}
