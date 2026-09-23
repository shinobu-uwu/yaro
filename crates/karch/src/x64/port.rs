use core::arch::asm;

/// Writes a byte to an x86 I/O port
///
/// # Safety
///
/// The caller must have permission to access this port and ensure that the
/// write is valid for the device's current state. Access must be coordinated
/// with other users of the device, including interrupt handlers
#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nostack, preserves_flags),
        );
    }
}

/// Reads a byte from an x86 I/O port
///
/// # Safety
///
/// The caller must have permission to access this port and ensure that the
/// read and its device side effects are valid. Access must be coordinated
/// with other users of the device, including interrupt handlers
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nostack, preserves_flags),
        );
    }
    value
}
