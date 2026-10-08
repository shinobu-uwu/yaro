use crate::address::PhysicalAddress;

pub struct MemoryRegion {
    pub base: PhysicalAddress,
    pub len: usize,
    pub memory_type: MemoryType,
}

#[repr(u8)]
pub enum MemoryType {
    /// The memory region is freely usable.
    Usable,
    /// The memory region is permanently reserved.
    Reserved,
    /// The memory region is currently used by ACPI, but can be reclaimed once
    /// ACPI structures are no longer needed.
    AcpiReclaimable,
    /// The memory region is permanently reserved by ACPI, and must not be used.
    AcpiNvs,
    /// The memory region is unusable due to physical damage or similar errors.
    Bad,
    /// The memory region is used by the bootloader, but can be reclaimed once
    /// all responses have been processed and will no longer be used.
    BootloaderReclaimable,
    /// The memory region is used by the executable and modules, and as such is
    /// permanently reserved.
    Executable,
    /// The memory region is used by the framebuffer, and as such is permanently
    /// reserved.
    FrameBuffer,
}
