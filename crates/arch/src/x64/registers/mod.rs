use yaro_memory::address::VirtualAddress;

use crate::x64::{gdt::SegmentSelector, registers::rflags::RFlags};

pub mod rflags;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub ip: VirtualAddress,
    pub cs: SegmentSelector,
    _reserved_1: [u8; 6],
    pub cf: RFlags,
    pub sp: VirtualAddress,
    pub ss: SegmentSelector,
    _reserved_2: [u8; 6],
}
