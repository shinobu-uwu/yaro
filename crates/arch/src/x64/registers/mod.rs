use core::fmt;

use yaro_memory::address::VirtualAddress;

use crate::x64::{gdt::SegmentSelector, registers::rflags::RFlags};

pub mod rflags;

#[derive(Clone, Copy)]
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

impl fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptStackFrame")
            .field("ip", &self.ip)
            .field("cs", &self.cs)
            .field("cf", &self.cf)
            .field("sp", &self.sp)
            .field("ss", &self.ss)
            .finish()
    }
}
