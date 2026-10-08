//! Initial exception handlers: report the exception and halt through panic.

use super::registers::InterruptStackFrame;

pub(super) extern "x86-interrupt" fn divide_error(frame: InterruptStackFrame) {
    panic!("Divide error\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn debug(frame: InterruptStackFrame) {
    panic!("Debug exception\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn nmi(frame: InterruptStackFrame) {
    panic!("Non-maskable interrupt\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn breakpoint(frame: InterruptStackFrame) {
    panic!("Breakpoint\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn overflow(frame: InterruptStackFrame) {
    panic!("Overflow\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn bound_range_exceeded(frame: InterruptStackFrame) {
    panic!("BOUND range exceeded\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn invalid_opcode(frame: InterruptStackFrame) {
    panic!("Invalid opcode\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn device_not_available(frame: InterruptStackFrame) {
    panic!("Device not available\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn x87_floating_point(frame: InterruptStackFrame) {
    panic!("x87 floating-point exception\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn simd_floating_point(frame: InterruptStackFrame) {
    panic!("SIMD floating-point exception\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn virtualization(frame: InterruptStackFrame) {
    panic!("Virtualization exception\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn hv_injection_exception(frame: InterruptStackFrame) {
    panic!("Hypervisor injection exception\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn invalid_tss(frame: InterruptStackFrame, error: u64) {
    panic!("Invalid TSS; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn segment_not_present(frame: InterruptStackFrame, error: u64) {
    panic!("Segment not present; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn stack_segment_fault(frame: InterruptStackFrame, error: u64) {
    panic!("Stack segment fault; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn general_protection_fault(
    frame: InterruptStackFrame,
    error: u64,
) {
    panic!("General protection fault; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, error: u64) {
    panic!("Page fault; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn alignment_check(frame: InterruptStackFrame, error: u64) {
    panic!("Alignment check; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn cp_protection_exception(
    frame: InterruptStackFrame,
    error: u64,
) {
    panic!("Control protection exception; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn vmm_communication_exception(
    frame: InterruptStackFrame,
    error: u64,
) {
    panic!("VMM communication exception; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn security_exception(frame: InterruptStackFrame, error: u64) {
    panic!("Security exception; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn double_fault(frame: InterruptStackFrame, error: u64) -> ! {
    panic!("Double fault; error code: {error:#x}\n{frame:#?}");
}

pub(super) extern "x86-interrupt" fn machine_check(frame: InterruptStackFrame) -> ! {
    panic!("Machine check\n{frame:#?}");
}
