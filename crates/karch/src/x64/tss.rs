use core::arch::asm;

use kmemory::address::VirtualAddress;

use crate::x64::gdt::SegmentSelector;

const _: () = {
    assert!(core::mem::size_of::<TaskStateSegment>() == 104);
};

#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    pub rsp: [VirtualAddress; 3],
    reserved_2: u32,
    reserved_3: u32,
    pub ist: [VirtualAddress; 7],
    reserved_4: u64,
    reserved_5: u16,
    pub iopb: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved_1: 0,
            rsp: [const { VirtualAddress::new(0) }; 3],
            reserved_2: 0,
            reserved_3: 0,
            ist: [const { VirtualAddress::new(0) }; 7],
            reserved_4: 0,
            reserved_5: 0,
            // No I/O bitmap: place its base beyond the descriptor's inclusive limit.
            iopb: core::mem::size_of::<Self>() as u16,
        }
    }

    /// # Safety
    /// `selector` must identify a present, available TSS in the loaded GDT.
    /// Its backing storage must remain valid, and the descriptor must be
    /// writable so the CPU can mark it busy. Must execute at ring 0.
    pub unsafe fn load(selector: SegmentSelector) {
        unsafe {
            asm!(
                "ltr {0:x}",
                in(reg) selector.as_u16(),
                options(nostack, preserves_flags),
            );
        }
    }
}
