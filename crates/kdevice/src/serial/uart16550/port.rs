use karch::x64::port::{inb, outb};

use crate::serial::uart16550::Backend;

#[derive(Debug)]
pub struct Port(u16);

impl Port {
    /// Creates a backend for eight consecutive UART I/O ports.
    ///
    /// # Panics
    ///
    /// Panics if the register range exceeds the 16-bit I/O address space.
    pub fn new(base: u16) -> Self {
        base.checked_add(7).expect("UART port address overflow");
        Self(base)
    }

    fn address(&self, offset: u8) -> u16 {
        assert!(offset < 8, "invalid UART register offset");
        self.0
            .checked_add(u16::from(offset))
            .expect("UART port address overflow")
    }
}

impl Backend for Port {
    fn read(&self, offset: u8) -> u8 {
        // SAFETY: Construction establishes valid, exclusively owned UART ports
        // and I/O permission. address() checks the offset, and &mut self
        // serializes access through this backend.
        unsafe { inb(self.address(offset)) }
    }

    fn write(&mut self, offset: u8, value: u8) {
        // SAFETY: Construction establishes valid, exclusively owned UART ports
        // and I/O permission. address() checks the offset, and &mut self
        // serializes access through this backend.
        unsafe {
            outb(self.address(offset), value);
        }
    }
}
