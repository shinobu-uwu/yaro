use core::marker::PhantomData;

use crate::address::{PhysicalAddress, VirtualAddress};

pub mod arch;
pub mod mapper;

pub use arch::CurrentArch;

/// The hardware entry flags used by the current compilation target.
pub type PageFlags = PageFlagsFor<CurrentArch>;

/// The hardware entry flags used by architecture `A`.
pub type PageFlagsFor<A> = <<A as Arch>::Entry as Entry>::Flags;

/// A pending local translation invalidation for a changed virtual mapping.
/// Dropping this token does not flush translations or perform a TLB shootdown.
#[must_use = "Mapping changes require flushing or explicitly ignoring"]
pub struct PageFlush<A: Arch> {
    address: VirtualAddress,
    _phantom: PhantomData<A>,
}

/// Common mapping permissions, encoded by each architecture's hardware flags type.
pub trait MappingFlags: Copy {
    /// Creates supervisor-only, read-only, non-executable mapping flags.
    /// Entry constructors supply presence and any other required hardware bits.
    fn new() -> Self;

    /// Enables or disables user-mode access, subject to parent permissions.
    fn user(self, enabled: bool) -> Self;

    /// Enables or disables writes, subject to parent permissions.
    fn write(self, enabled: bool) -> Self;

    /// Enables or disables instruction fetches, subject to parent permissions.
    fn execute(self, enabled: bool) -> Self;
}

#[derive(Debug, Clone)]
pub struct PageTable<A: Arch> {
    base: PhysicalAddress,
    level: usize,
    _arch: PhantomData<A>,
}

/// Architecture-specific page-table layout, entry access, and translation maintenance.
pub trait Arch: Clone + Copy {
    /// Number of table levels supported by this implementation.
    const PAGE_LEVELS: usize;
    /// Number of contiguous hardware entry slots in one table.
    const PAGE_TABLE_ENTRIES: usize;
    /// Leaf-entry page size.
    const PAGE_SIZE: usize;

    /// The hardware entry representation used by this architecture.
    type Entry: Entry;

    /// Returns the physical base address of the current CPU's active root table.
    /// Must be called at the privilege level required to read the paging registers.
    fn table() -> PhysicalAddress;

    /// Returns the active root table's level, with level 0 being the final leaf table.
    fn root_level() -> usize;

    /// Checks whether a virtual address is valid for the supported paging mode.
    /// Does not check whether the address is mapped or accessible.
    fn is_address_valid(addr: VirtualAddress) -> bool;

    /// Size in bytes of a leaf page at this level; must be a nonzero power of two.
    fn page_size(level: usize) -> usize;

    /// Reads an entry from its virtual address.
    ///
    /// # Safety
    /// The address must be aligned, mapped, and readable as a hardware
    /// entry, with access appropriately synchronized.
    unsafe fn read_entry(address: VirtualAddress) -> Self::Entry;

    /// Writes an entry at its virtual address.
    ///
    /// # Safety
    /// The address must be aligned, mapped, and writable as a hardware
    /// entry, with access appropriately synchronized. The entry must be
    /// valid for its level and the enabled processor features. The caller
    /// must perform any required translation-cache invalidation before
    /// relying on the new mapping or reclaiming previously mapped memory.
    unsafe fn write_entry(address: VirtualAddress, entry: Self::Entry);

    /// Synchronizes page-table writes and invalidates the address's cached
    /// translation on the local CPU for the current address space.
    ///
    /// # Safety
    /// Must run at the required privilege level with a valid virtual address.
    /// The caller must arrange synchronization with other CPUs separately.
    unsafe fn invalidate(address: VirtualAddress);

    /// Extracts the entry index for a virtual address at the given table level.
    /// The level must be valid for the supported paging mode. The result must
    /// be less than [`Self::PAGE_TABLE_ENTRIES`]. Does not access table memory.
    fn table_index(address: VirtualAddress, level: usize) -> usize;
}

/// A hardware page-table entry value, independent of its location in memory.
pub trait Entry: Clone {
    /// Hardware flags encoding the common mapping permissions for this entry type.
    type Flags: MappingFlags;

    /// Constructs a present entry pointing to a child table at `level - 1`.
    /// Derives table permissions from the requested level-0 leaf flags rather
    /// than copying leaf attributes onto the table entry.
    ///
    /// The caller must ensure the processor supports the address and features
    /// used by the entry before installing it.
    ///
    /// # Panics
    /// Panics for an unaligned or unrepresentable frame, invalid parent level,
    /// or invalid level-0 leaf flags.
    fn new_table(frame: PhysicalAddress, requested_flags: Self::Flags, level: usize) -> Self;

    /// Constructs a present level-0 leaf entry.
    ///
    /// The caller must ensure the processor supports the address and features
    /// used by the entry before installing it.
    ///
    /// # Panics
    /// Panics for an unaligned or unrepresentable frame or invalid level-0 leaf flags.
    fn new_page(frame: PhysicalAddress, flags: Self::Flags) -> Self;

    /// Whether this present non-leaf entry permits the requested leaf permissions.
    ///
    /// # Panics
    /// Panics for an invalid parent level or invalid level-0 leaf flags.
    fn allows_mapping(&self, flags: Self::Flags, level: usize) -> bool;

    /// Returns whether the entry is marked present for address translation.
    fn is_present(&self) -> bool;

    /// Extracts the physical address field without checking presence.
    /// For a leaf, align the result down to its page size to remove any
    /// level-specific attribute bits occupying address-field positions.
    fn address(&self) -> PhysicalAddress;

    /// Interprets the entry at `level` as a leaf mapping rather than a child table.
    /// The level must be valid for this architecture. Check [`Self::is_present`]
    /// separately before treating the entry as a usable mapping.
    fn is_leaf(&self, level: usize) -> bool;

    /// Returns an absent entry suitable for initializing an unused table slot.
    fn empty() -> Self;

    /// Extracts the defined hardware flags using the entry format at `level`.
    /// Flags that share bits may still need to be interpreted according to the
    /// level and whether the entry is a leaf.
    ///
    /// # Panics
    /// Panics if the level is outside the supported paging hierarchy.
    fn flags(&self, level: usize) -> Self::Flags;
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

    /// Reads the entry at the given index, including entries that are not present.
    ///
    /// # Panics
    /// Panics if the index is out of bounds or the entry address calculation overflows.
    ///
    /// # Safety
    /// The table must be aligned, mapped, and readable through the HHDM,
    /// with access appropriately synchronized.
    pub unsafe fn entry(&self, index: usize) -> A::Entry {
        let address = self.entry_address(index);

        // SAFETY: The caller guarantees readable table memory and synchronized access.
        unsafe { A::read_entry(address) }
    }

    pub fn entry_address(&self, index: usize) -> VirtualAddress {
        assert!(index < A::PAGE_TABLE_ENTRIES);

        let offset = index
            .checked_mul(size_of::<A::Entry>())
            .expect("page-table entry offset overflow");
        self.base
            .checked_add(offset)
            .expect("page-table entry address overflow")
            .to_hhdm()
    }

    pub fn index_of(&self, address: VirtualAddress) -> usize {
        A::table_index(address, self.level)
    }

    /// Writes the entry at the given index without invalidating cached translations.
    ///
    /// # Panics
    /// Panics if the index is out of bounds or the entry address calculation overflows.
    ///
    /// # Safety
    /// The table must be aligned, mapped, and writable through the HHDM,
    /// with access appropriately synchronized. The entry must be valid for
    /// this level and the enabled processor features. The caller must perform
    /// any required translation-cache invalidation before relying on the new
    /// mapping or reclaiming previously mapped memory.
    pub unsafe fn set_entry(&mut self, index: usize, entry: A::Entry) {
        let address = self.entry_address(index);

        // SAFETY: The caller upholds the hardware entry write requirements.
        unsafe {
            A::write_entry(address, entry);
        }
    }

    /// Initializes every entry as absent. Does not free frames or invalidate translations.
    ///
    /// # Panics
    /// Panics if an entry address calculation overflows.
    ///
    /// # Safety
    /// The entire table must be aligned, mapped, and writable through the HHDM,
    /// with access appropriately synchronized. Clearing its mappings must not
    /// invalidate memory needed during this operation. The caller must perform
    /// any required translation-cache invalidation before relying on the cleared
    /// mappings or reclaiming previously mapped memory.
    pub unsafe fn clear(&mut self) {
        for index in 0..A::PAGE_TABLE_ENTRIES {
            // SAFETY: The caller guarantees writable table memory, synchronized
            // access, and appropriate invalidation. empty() produces an absent entry.
            unsafe { self.set_entry(index, A::Entry::empty()) };
        }
    }
}

impl<A: Arch> PageFlush<A> {
    /// Records the virtual address whose mapping changed, not the entry's address.
    pub(crate) fn new(address: VirtualAddress) -> Self {
        Self {
            address,
            _phantom: PhantomData,
        }
    }

    /// Invalidates the changed address on the local CPU and consumes this token.
    ///
    /// # Safety
    /// The affected address space must be current on this CPU, running at the
    /// privilege level required for invalidation. The caller must synchronize
    /// other CPUs using the mapping separately.
    pub unsafe fn flush(self) {
        // SAFETY: The mapper validated the address, and the caller guarantees
        // the correct current address space and required privilege level.
        unsafe {
            A::invalidate(self.address);
        }
    }

    /// Consumes this token without invalidating any translations.
    ///
    /// # Safety
    /// Invalidation must be unnecessary or arranged separately before relying
    /// on the changed mapping or reclaiming memory reachable by stale translations.
    pub unsafe fn ignore(self) {}
}
