use core::arch::asm;

use crate::{address::PhysicalAddress, paging};

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [X86Entry; 512],
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct X86Entry(u64);

#[derive(Debug, Clone, Copy)]
pub struct X86Arch;

impl paging::Arch for X86Arch {
    type Entry = X86Entry;

    #[inline(always)]
    fn table() -> PhysicalAddress {
        let addr: usize;

        unsafe {
            asm!("mov {}, cr3", out(reg) addr);
        }

        PhysicalAddress::new(addr)
    }
}

impl paging::Entry for X86Entry {}
