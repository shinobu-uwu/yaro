use yaro_memory::paging::{CurrentArch, mapper::PageMapper};

#[cfg(target_arch = "x86_64")]
use crate::x64;

/// Initializes architecture devices and enables local CPU interrupts.
///
/// # Safety
/// Call once on the boot CPU at kernel privilege with interrupts disabled,
/// memory initialized, exclusive page-table access, and the architecture IDT
/// loaded. Unhandled external interrupt sources must remain masked.
pub unsafe fn init(mapper: &mut PageMapper<CurrentArch>) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        x64::device::init(mapper);
        // Device initialization has returned and released the LAPIC lock.
        // core::arch::asm!("sti", options(nostack));
    }
}
