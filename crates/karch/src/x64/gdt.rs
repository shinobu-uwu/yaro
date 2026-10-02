use core::arch::asm;

use crate::x64::{PrivilegeLevel, TablePointer, tss::TaskStateSegment};
use bitflags::bitflags;
use kmemory::VirtualAddress;

#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable<const N: usize = 8> {
    entries: [Entry; N],
    len: usize,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub limit_low: u16,
    pub offset_low: u16,
    pub offset_mid: u8,
    pub access: u8,
    pub flags_limit_high: u8,
    pub offset_high: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum Descriptor {
    Segment(u64),
    SystemSegment(u64, u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

bitflags! {
    /// Flags for a GDT descriptor. Not all flags are valid for all descriptor types.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct DescriptorFlags: u64 {
        /// Set by the processor if this segment has been accessed. Only cleared by software.
        /// _Setting_ this bit in software prevents GDT writes on first use.
        const ACCESSED          = 1 << 40;
        /// For 32-bit data segments, sets the segment as writable. For 32-bit code segments,
        /// sets the segment as _readable_. In 64-bit mode, ignored for all segments.
        const WRITABLE          = 1 << 41;
        /// For code segments, sets the segment as “conforming”, influencing the
        /// privilege checks that occur on control transfers. For 32-bit data segments,
        /// sets the segment as "expand down". In 64-bit mode, ignored for data segments.
        const CONFORMING        = 1 << 42;
        /// This flag must be set for code segments and unset for data segments.
        const EXECUTABLE        = 1 << 43;
        /// This flag must be set for user segments (in contrast to system segments).
        const USER_SEGMENT      = 1 << 44;
        /// These two bits encode the Descriptor Privilege Level (DPL) for this descriptor.
        /// If both bits are set, the DPL is Ring 3, if both are unset, the DPL is Ring 0.
        const DPL_RING_3        = 3 << 45;
        /// Must be set for any segment, causes a segment not present exception if not set.
        const PRESENT           = 1 << 47;
        /// Available for use by the Operating System
        const AVAILABLE         = 1 << 52;
        /// Must be set for 64-bit code segments, unset otherwise.
        const LONG_MODE         = 1 << 53;
        /// Use 32-bit (as opposed to 16-bit) operands. If [`LONG_MODE`][Self::LONG_MODE] is set,
        /// this must be unset. In 64-bit mode, ignored for data segments.
        const DEFAULT_SIZE      = 1 << 54;
        /// Limit field is scaled by 4096 bytes. In 64-bit mode, ignored for all segments.
        const GRANULARITY       = 1 << 55;

        /// Bits `0..=15` of the limit field (ignored in 64-bit mode)
        const LIMIT_0_15        = 0xFFFF;
        /// Bits `16..=19` of the limit field (ignored in 64-bit mode)
        const LIMIT_16_19       = 0xF << 48;
        /// Bits `0..=23` of the base field (ignored in 64-bit mode, except for fs and gs)
        const BASE_0_23         = 0xFF_FFFF << 16;
        /// Bits `24..=31` of the base field (ignored in 64-bit mode, except for fs and gs)
        const BASE_24_31        = 0xFF << 56;
    }
}

impl<const N: usize> GlobalDescriptorTable<N> {
    #[inline]
    pub const fn new() -> Self {
        assert!(N > 0, "a GDT must have room for the null descriptor");
        assert!(N <= (1 << 13), "a GDT cannot exceed 8192 (2^13) entries");

        Self {
            entries: [Entry::null(); N],
            len: 1, // first entry is the null descriptor
        }
    }

    /// ## Safety
    ///
    /// This function is unsafe because the caller must ensure that the given
    /// `DescriptorTablePointer` points to a valid GDT and that loading this
    /// GDT is safe.
    pub unsafe fn load(&'static self) {
        unsafe {
            asm!("lgdt [{}]", in(reg) &self.pointer(), options(readonly, nostack));
        }
    }

    pub fn pointer(&self) -> TablePointer {
        let limit = (self.len * size_of::<u64>() - 1) as u16;
        let base = VirtualAddress::new(self.entries.as_ptr() as _);

        TablePointer { limit, base }
    }

    #[inline]
    const fn push(&mut self, value: u64) -> usize {
        let index = self.len;
        self.entries[index] = Entry::from_u64(value);
        self.len += 1;
        index
    }

    pub const fn push_back(&mut self, descriptor: Descriptor) -> SegmentSelector {
        let selector_index = match descriptor {
            Descriptor::Segment(value) => {
                assert!(self.len <= self.entries.len().saturating_sub(1));
                self.push(value)
            }
            Descriptor::SystemSegment(low, high) => {
                assert!(self.len <= self.entries.len().saturating_sub(2));
                let index = self.push(low);
                self.push(high);
                index
            }
        };

        SegmentSelector::new(selector_index as u16, descriptor.dpl())
    }
}

impl Entry {
    pub const fn from_u64(value: u64) -> Self {
        Self {
            limit_low: value as u16,
            offset_low: (value >> 16) as u16,
            offset_mid: (value >> 32) as u8,
            access: (value >> 40) as u8,
            flags_limit_high: (value >> 48) as u8,
            offset_high: (value >> 56) as u8,
        }
    }

    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            offset_low: 0,
            offset_mid: 0,
            access: 0,
            flags_limit_high: 0,
            offset_high: 0,
        }
    }
}

impl Descriptor {
    /// A present, available 64-bit TSS descriptor at ring 0.
    ///
    /// The limit covers only the [`TaskStateSegment`] structure, without an
    /// appended I/O permission bitmap. The static reference keeps the backing
    /// storage alive at a stable address.
    ///
    /// Before using this descriptor, the caller must ensure the TSS is correctly
    /// initialized and remains mapped for as long as the CPU uses it.
    #[inline]
    pub fn tss(tss: &'static TaskStateSegment) -> Self {
        let base = tss as *const TaskStateSegment as u64;
        let limit = (size_of::<TaskStateSegment>() - 1) as u64;

        let low = (limit & 0xFFFF)
            | ((base & 0xFF_FFFF) << 16)
            | (0b1001 << 40) // Available 64-bit TSS; system descriptor (S = 0).
            | DescriptorFlags::PRESENT.bits()
            | (((limit >> 16) & 0xF) << 48)
            | (((base >> 24) & 0xFF) << 56);
        let high = base >> 32; // The upper 32 reserved bits remain zero.

        Self::SystemSegment(low, high)
    }

    /// A present, readable 64-bit code segment at ring 0.
    /// The accessed bit is pre-set to avoid a hardware write on first use.
    #[inline]
    pub const fn kernel_code() -> Self {
        Self::Segment(
            DescriptorFlags::PRESENT.bits()
                | DescriptorFlags::USER_SEGMENT.bits()
                | DescriptorFlags::EXECUTABLE.bits()
                | DescriptorFlags::WRITABLE.bits()
                | DescriptorFlags::ACCESSED.bits()
                | DescriptorFlags::LONG_MODE.bits(),
        )
    }

    /// A present, writable data segment at ring 0 for use in 64-bit mode.
    /// Base and limit are zero; they are ignored for ordinary data addressing
    /// in 64-bit mode. The accessed bit is pre-set.
    #[inline]
    pub const fn kernel_data() -> Self {
        Self::Segment(
            DescriptorFlags::PRESENT.bits()
                | DescriptorFlags::USER_SEGMENT.bits()
                | DescriptorFlags::WRITABLE.bits()
                | DescriptorFlags::ACCESSED.bits(),
        )
    }

    /// A present, readable 64-bit code segment at ring 3.
    /// The accessed bit is pre-set to avoid a hardware write on first use.
    #[inline]
    pub const fn user_code() -> Self {
        Self::Segment(
            DescriptorFlags::PRESENT.bits()
                | DescriptorFlags::USER_SEGMENT.bits()
                | DescriptorFlags::EXECUTABLE.bits()
                | DescriptorFlags::WRITABLE.bits()
                | DescriptorFlags::ACCESSED.bits()
                | DescriptorFlags::LONG_MODE.bits()
                | DescriptorFlags::DPL_RING_3.bits(),
        )
    }

    /// A present, writable data segment at ring 3 for use in 64-bit mode.
    /// Base and limit are zero; the accessed bit is pre-set.
    #[inline]
    pub const fn user_data() -> Self {
        Self::Segment(
            DescriptorFlags::PRESENT.bits()
                | DescriptorFlags::USER_SEGMENT.bits()
                | DescriptorFlags::WRITABLE.bits()
                | DescriptorFlags::ACCESSED.bits()
                | DescriptorFlags::DPL_RING_3.bits(),
        )
    }

    #[inline]
    pub const fn dpl(self) -> PrivilegeLevel {
        let val = match self {
            Descriptor::Segment(v) => v,
            Descriptor::SystemSegment(v, _) => v,
        };

        let dpl = (val & DescriptorFlags::DPL_RING_3.bits()) >> 45;
        PrivilegeLevel::from_u16(dpl as u16)
    }
}

impl SegmentSelector {
    #[inline]
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        Self((index << 3) | (rpl as u16))
    }

    #[inline]
    pub const fn rpl(self) -> PrivilegeLevel {
        PrivilegeLevel::from_u16(self.0 & 0b11)
    }

    #[inline]
    pub const fn from_u16(value: u16) -> Self {
        Self(value)
    }

    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}
