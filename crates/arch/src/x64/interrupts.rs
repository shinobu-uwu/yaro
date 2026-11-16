use core::arch::asm;

use crate::x64::registers::rflags::RFlags;

#[inline]
pub fn are_enabled() -> bool {
    let rflags: u64;

    unsafe {
        asm!(
            "pushfq",
                "pop {}",
                out(reg) rflags,
        );
    }

    RFlags::from_bits_truncate(rflags).contains(RFlags::INTERRUPT_FLAG)
}

/// Enable interrupts.
///
/// This is a wrapper around the `sti` instruction.
///
/// This function executes `sti; nop` to ensure that the interrupt shadow
/// caused by the `sti` instruction doesn't last beyond this function.
/// Use [`enable_and_hlt`] to execute `hlt` in `sti`'s interrupt shadow.
#[inline]
pub fn enable() {
    unsafe {
        asm!("sti", "nop", options(preserves_flags, nostack));
    }
}

/// Disable interrupts.
///
/// This is a wrapper around the `cli` instruction.
#[inline]
pub fn disable() {
    unsafe {
        asm!("cli", options(preserves_flags, nostack));
    }
}

#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let enabled = are_enabled();

    if enabled {
        disable();
    }

    let ret = f();

    if enabled {
        enable();
    }

    ret
}
