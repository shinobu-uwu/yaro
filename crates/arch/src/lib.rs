#![no_std]
#![feature(abi_x86_interrupt)]

#[cfg(target_arch = "x86_64")]
pub mod x64;

/// Initializes the boot CPU's architecture state.
///
/// # Safety
/// The caller must uphold the target architecture's initialization requirements;
/// on x86_64, see [`x64::init`].
pub unsafe fn init() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        x64::init();
    }
}
