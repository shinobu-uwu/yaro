use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use yaro_log::info;
use yaro_memory::paging::{arch::x64::X86Arch, mapper::PageMapper};

use crate::x64::{
    device::apic::{LocalApic, TIMER_VECTOR},
    port::outb,
};

pub mod apic;
pub mod tsc;

lazy_static! {
    // Ordinary access must disable local interrupts before taking this lock:
    // the timer handler also takes it to acknowledge interrupts.
    static ref LAPIC: Mutex<LocalApic> = Mutex::new(LocalApic::new());
}

/// Initializes the boot CPU's LAPIC and starts its periodic timer.
///
/// # Safety
/// Call once at ring 0 with local interrupts disabled, memory initialized,
/// exclusive access to the kernel page tables, and the architecture IDT loaded.
pub unsafe fn init(mapper: &mut PageMapper<X86Arch>) {
    assert!(LocalApic::is_supported(), "Local APIC is required");
    // SAFETY: Early boot has exclusive access to both legacy PICs. Mask all
    // their IRQs before enabling CPU interrupts; no PIC handlers are installed.
    unsafe {
        outb(0x21, 0xFF);
        outb(0xA1, 0xFF);
    }

    let mut lapic = LAPIC.lock();
    lapic.init(mapper);
    // SAFETY: The caller guarantees disabled interrupts and the loaded IDT,
    // which installs TIMER_VECTOR. The count is deliberately uncalibrated.
    unsafe { lapic.start_timer(TIMER_VECTOR, 0x000F_FFFF) };
}

pub(crate) fn timer_tick() {
    info!("TIMER!");
    // SAFETY: Called by the boot CPU's timer interrupt after LAPIC setup.
    // Interrupt gates disable local interrupts, and ordinary lock holders must
    // also disable them, so this cannot interrupt a holder of this lock.
    unsafe { LAPIC.lock().eoi() };
}
