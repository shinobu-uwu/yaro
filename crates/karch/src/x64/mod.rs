use core::{mem, ptr::addr_of};
use kmemory::address::{PhysicalAddress, VirtualAddress};

use crate::x64::{
    gdt::{Descriptor, GlobalDescriptorTable},
    idt::InterruptDescriptorTable,
    tss::TaskStateSegment,
};

mod exceptions;

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
const STACK_SIZE: usize = 16 * 1024; // 16KiB

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::empty();
static mut DOUBLE_FAULT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

/// Installs the boot CPU's GDT, TSS, and exception handlers.
///
/// # Safety
/// Must be called exactly once, at ring 0 on the boot CPU, with interrupts
/// disabled and exclusive access to the GDT. No other CPU may use this GDT/TSS.
/// The GDT must remain mapped and writable, and the TSS must remain mapped,
/// for as long as they are active. The GDT must not be modified afterward.
/// The caller must arrange compatible segment selectors and interrupt handlers
/// before enabling interrupts, and configure TSS stacks before using them.
// Early boot provides exclusive access under the contract above. Also,
// for now the kenel is single-threaded, so no race condition can happen
// eventually this will be moved to a CPU-local structure
#[allow(static_mut_refs)]
pub unsafe fn init() {
    unsafe {
        let base = addr_of!(DOUBLE_FAULT_STACK) as usize;
        let top = VirtualAddress::new(base + STACK_SIZE);
        TSS.ist[0] = top;
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

        IDT.divide_error.set_handler(exceptions::divide_error);
        IDT.debug.set_handler(exceptions::debug);
        IDT.nmi.set_handler(exceptions::nmi);
        IDT.breakpoint.set_handler(exceptions::breakpoint);
        IDT.overflow.set_handler(exceptions::overflow);
        IDT.bound_range_exceeded
            .set_handler(exceptions::bound_range_exceeded);
        IDT.invalid_opcode.set_handler(exceptions::invalid_opcode);
        IDT.device_not_available
            .set_handler(exceptions::device_not_available);
        IDT.x87_floating_point
            .set_handler(exceptions::x87_floating_point);
        IDT.simd_floating_point
            .set_handler(exceptions::simd_floating_point);
        IDT.virtualization.set_handler(exceptions::virtualization);
        IDT.hv_injection_exception
            .set_handler(exceptions::hv_injection_exception);
        IDT.invalid_tss.set_handler(exceptions::invalid_tss);
        IDT.segment_not_present
            .set_handler(exceptions::segment_not_present);
        IDT.stack_segment_fault
            .set_handler(exceptions::stack_segment_fault);
        IDT.general_protection_fault
            .set_handler(exceptions::general_protection_fault);
        IDT.page_fault.set_handler(exceptions::page_fault);
        IDT.alignment_check.set_handler(exceptions::alignment_check);
        IDT.cp_protection_exception
            .set_handler(exceptions::cp_protection_exception);
        IDT.vmm_communication_exception
            .set_handler(exceptions::vmm_communication_exception);
        IDT.security_exception
            .set_handler(exceptions::security_exception);
        IDT.machine_check.set_handler(exceptions::machine_check);
        IDT.double_fault
            .set_handler(exceptions::double_fault)
            .set_stack_index(Some(0));
        IDT.load();
    }
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
