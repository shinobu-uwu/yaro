use core::mem;
use klog::info;
use kmemory::{PhysicalAddress, VirtualAddress};

use crate::x64::{
    gdt::{Descriptor, GlobalDescriptorTable},
    idt::InterruptDescriptorTable,
    registers::InterruptStackFrame,
    tss::TaskStateSegment,
};

pub mod gdt;
pub mod idt;
pub mod port;
pub mod registers;
pub mod tss;

const _: () = {
    assert!(mem::size_of::<VirtualAddress>() == 8);
    assert!(mem::size_of::<PhysicalAddress>() == 8);
    assert!(mem::size_of::<TablePointer>() == 10);
    assert!(mem::offset_of!(TablePointer, base) == 2);
};

static TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::empty();

/// Installs the boot CPU's GDT and TSS.
///
/// # Safety
/// Must be called exactly once, at ring 0 on the boot CPU, with interrupts
/// disabled and exclusive access to the GDT. No other CPU may use this GDT/TSS.
/// The GDT must remain mapped and writable, and the TSS must remain mapped,
/// for as long as they are active. The GDT must not be modified afterward.
/// The caller must arrange compatible segment selectors and interrupt handlers
/// before enabling interrupts, and configure TSS stacks before using them.
#[allow(static_mut_refs)] // Early boot provides exclusive access under the contract above.
pub unsafe fn init() {
    unsafe {
        let tss_selector = GDT.push_back(Descriptor::tss(&TSS));
        let kernel_code = GDT.push_back(Descriptor::kernel_code());
        let kernel_data = GDT.push_back(Descriptor::kernel_data());
        let _user_code = GDT.push_back(Descriptor::user_code());
        let _user_data = GDT.push_back(Descriptor::user_data());
        GDT.load();

        // SAFETY: These selectors refer to the present ring-0 code and data
        // descriptors just installed. The caller guarantees a valid stack and
        // disabled interrupts. The far return reloads CS without changing RSP
        // overall; the remaining moves reload the stack and data selectors.
        core::arch::asm!(
            "push {code}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            "mov ss, {data:x}",
            "mov ds, {data:x}",
            "mov es, {data:x}",
            code = in(reg) u64::from(kernel_code.as_u16()),
            data = in(reg) kernel_data.as_u16(),
            out("rax") _,
            options(preserves_flags),
        );

        TaskStateSegment::load(tss_selector);

        IDT.breakpoint.set_handler(debug_interrupt);
        IDT.load();
    }
}

extern "x86-interrupt" fn debug_interrupt(_: InterruptStackFrame) {
    info!("Breakpoint interrupt handler called!");
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TablePointer {
    pub limit: u16,
    pub base: VirtualAddress,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

impl PrivilegeLevel {
    #[inline]
    pub const fn from_u16(value: u16) -> Self {
        match value {
            0 => Self::Ring0,
            1 => Self::Ring1,
            2 => Self::Ring2,
            3 => Self::Ring3,
            _ => panic!("invalid privilege level"),
        }
    }
}
