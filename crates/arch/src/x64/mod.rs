use core::{mem, ptr::addr_of};
use yaro_memory::address::{PhysicalAddress, VirtualAddress};

use crate::x64::{
    device::apic::{SPURIOUS_VECTOR_INDEX, TIMER_VECTOR},
    gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
    idt::InterruptDescriptorTable,
    registers::InterruptStackFrame,
    tss::TaskStateSegment,
};

mod exceptions;

pub(crate) mod device;
pub(crate) mod gdt;
pub(crate) mod idt;
pub(crate) mod interrupts;
pub mod port;
pub(crate) mod registers;
pub(crate) mod tss;

const _: () = {
    assert!(mem::size_of::<VirtualAddress>() == 8);
    assert!(mem::size_of::<PhysicalAddress>() == 8);
    assert!(mem::size_of::<TablePointer>() == 10);
    assert!(mem::offset_of!(TablePointer, base) == 2);
};
const STACK_SIZE: usize = 16 * 1024; // 16KiB

// Slot 0 is null; the 64-bit TSS descriptor occupies slots 1 and 2.
pub(crate) const GDT_TSS: u16 = 1;
pub(crate) const GDT_KERNEL_CODE: u16 = 3;
pub(crate) const GDT_KERNEL_DATA: u16 = 4;
pub(crate) const GDT_USER_CODE: u16 = 5;
pub(crate) const GDT_USER_DATA: u16 = 6;

pub(crate) const TSS_SELECTOR: SegmentSelector =
    SegmentSelector::new(GDT_TSS, PrivilegeLevel::Ring0);

/// Selectors for the fixed GDT layout. The GDT must be installed before use.
pub(crate) const SELECTORS: Selectors = Selectors {
    kernel_code: SegmentSelector::new(GDT_KERNEL_CODE, PrivilegeLevel::Ring0),
    kernel_data: SegmentSelector::new(GDT_KERNEL_DATA, PrivilegeLevel::Ring0),
    user_code: SegmentSelector::new(GDT_USER_CODE, PrivilegeLevel::Ring3),
    user_data: SegmentSelector::new(GDT_USER_DATA, PrivilegeLevel::Ring3),
};

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::empty();
static mut DOUBLE_FAULT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(crate) struct TablePointer {
    pub(crate) limit: u16,
    pub(crate) base: VirtualAddress,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Selectors {
    pub(crate) kernel_code: SegmentSelector,
    pub(crate) kernel_data: SegmentSelector,
    pub(crate) user_code: SegmentSelector,
    pub(crate) user_data: SegmentSelector,
}

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
pub(crate) unsafe fn init() {
    unsafe {
        let base = addr_of!(DOUBLE_FAULT_STACK) as usize;
        let top = VirtualAddress::new(base + STACK_SIZE);
        TSS.ist[0] = top;
        let tss_selector = GDT.push_back(Descriptor::tss(&TSS));
        let kernel_code = GDT.push_back(Descriptor::kernel_code());
        let kernel_data = GDT.push_back(Descriptor::kernel_data());
        let user_code = GDT.push_back(Descriptor::user_code());
        let user_data = GDT.push_back(Descriptor::user_data());
        // Check the builder against the fixed layout before loading the GDT.
        assert_eq!(tss_selector, TSS_SELECTOR);
        assert_eq!(kernel_code, SELECTORS.kernel_code);
        assert_eq!(kernel_data, SELECTORS.kernel_data);
        assert_eq!(user_code, SELECTORS.user_code);
        assert_eq!(user_data, SELECTORS.user_data);
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
            code = in(reg) u64::from(SELECTORS.kernel_code.as_u16()),
            data = in(reg) SELECTORS.kernel_data.as_u16(),
            out("rax") _,
            options(preserves_flags),
        );

        TaskStateSegment::load(TSS_SELECTOR);

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
        IDT.interrupt_entry(SPURIOUS_VECTOR_INDEX)
            .set_handler(spurious_interrupt);
        IDT.interrupt_entry(TIMER_VECTOR)
            .set_handler(timer_interrupt);
        IDT.load();
    }
}

impl PrivilegeLevel {
    #[inline]
    pub(crate) const fn from_u16(value: u16) -> Self {
        match value {
            0 => Self::Ring0,
            1 => Self::Ring1,
            2 => Self::Ring2,
            3 => Self::Ring3,
            _ => panic!("invalid privilege level"),
        }
    }
}

extern "x86-interrupt" fn spurious_interrupt(_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn timer_interrupt(_frame: InterruptStackFrame) {
    device::timer_tick();
}
