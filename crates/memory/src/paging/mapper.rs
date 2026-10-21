use core::{error::Error, fmt::Display};

use bit_field::BitField;

use crate::{
    address::{PhysicalAddress, VirtualAddress},
    allocator::physical::FrameAllocator,
    paging::{Arch, Entry, PageFlagsFor, PageFlush, PageTable},
};

pub struct PageMapper<A: Arch> {
    root: PageTable<A>,
}

impl<A: Arch> PageMapper<A> {
    pub fn new() -> Self {
        let root = PageTable::root();

        Self { root }
    }

    /// Maps one level-0 page to an existing physical frame.
    /// Returns a pending invalidation for the mapped virtual address; this method
    /// does not flush translations. Linked intermediate tables remain allocated
    /// for reuse if a later allocation fails.
    ///
    /// # Panics
    /// Panics if a physical frame is unaligned or unrepresentable, flags are
    /// invalid for a level-0 leaf, or an intermediate table level is invalid.
    ///
    /// # Safety
    /// Memory initialization must be complete. The table hierarchy must remain
    /// valid and accessible through the HHDM, with exclusive software access for
    /// this operation. Allocated frames must be uniquely owned, correctly sized
    /// and aligned for a table, and writable through the HHDM. The physical frame
    /// and requested permissions must be suitable for their intended use and
    /// supported by the CPU, including its physical-address width and enabled
    /// features. The caller must handle the returned invalidation before relying
    /// on the mapping and synchronize other CPUs using this hierarchy separately.
    pub unsafe fn map_to(
        &mut self,
        phys: PhysicalAddress,
        virt: VirtualAddress,
        flags: PageFlagsFor<A>,
        allocator: &mut impl FrameAllocator,
    ) -> Result<PageFlush<A>, MapToError> {
        if !A::is_address_valid(virt) {
            return Err(MapToError::InvalidVirtualAddress);
        }

        if !virt.is_aligned(A::page_size(0)) {
            return Err(MapToError::UnalignedVirtualAddress);
        }

        let leaf = A::Entry::new_page(phys, flags);
        let mut table = self.root.clone();

        while table.level > 0 {
            let index = table.index_of(virt);
            let entry = unsafe { table.entry(index) };

            let child_frame = if entry.is_present() {
                if entry.is_leaf(table.level) {
                    return Err(MapToError::ParentEntryHugePage);
                }

                if !entry.allows_mapping(flags, table.level) {
                    return Err(MapToError::ParentEntryPermissions);
                }

                entry.address()
            } else {
                let frame = allocator
                    .allocate()
                    .ok_or(MapToError::FrameAllocationFailed)?;
                let parent_entry = A::Entry::new_table(frame, flags, table.level);
                let mut child = PageTable::<A>::new(frame, table.level - 1);

                // SAFETY: The caller guarantees exclusive, writable table frames
                // and synchronized hierarchy access. Initialize before publishing.
                unsafe {
                    child.clear();
                    table.set_entry(index, parent_entry);
                }

                frame
            };

            table = PageTable::new(child_frame, table.level - 1);
        }

        let index = table.index_of(virt);
        // SAFETY: The caller guarantees readable tables and exclusive software access.
        let entry = unsafe { table.entry(index) };

        if entry.is_present() {
            return Err(MapToError::PageAlreadyMapped(entry.address()));
        }

        // SAFETY: The leaf was validated above. The caller guarantees writable
        // table memory and appropriately synchronized hierarchy access.
        unsafe {
            table.set_entry(index, leaf);
        }

        Ok(PageFlush::new(virt))
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
            let entry = unsafe { table.entry(index) };

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

#[derive(Debug, Clone)]
pub enum MapToError {
    /// The virtual address is not valid for this architecture.
    InvalidVirtualAddress,
    /// The virtual address is not aligned to a level-0 page boundary.
    UnalignedVirtualAddress,
    /// An additional frame was needed for the mapping process, but the frame allocator
    /// returned `None`.
    FrameAllocationFailed,
    /// An upper level page table entry has the `HUGE_PAGE` flag set, which means that the
    /// given page is part of an already mapped huge page.
    ParentEntryHugePage,
    /// An existing parent would block the requested mapping permissions.
    ParentEntryPermissions,
    /// The given page is already mapped to a physical frame.
    PageAlreadyMapped(PhysicalAddress),
}

impl Display for MapToError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MapToError::InvalidVirtualAddress => write!(f, "Invalid virtual address"),
            MapToError::UnalignedVirtualAddress => write!(f, "Unaligned virtual address"),
            MapToError::FrameAllocationFailed => write!(f, "Frame allocation failed"),
            MapToError::ParentEntryHugePage => write!(f, "Parent is huge page"),
            MapToError::ParentEntryPermissions => write!(f, "Parent permissions block the mapping"),
            MapToError::PageAlreadyMapped(p) => write!(f, "Page already mapped to: {:#?}", p),
        }
    }
}

impl Error for MapToError {}
