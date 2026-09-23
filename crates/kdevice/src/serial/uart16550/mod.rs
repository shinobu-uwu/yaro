#[cfg(not(target_arch = "x86_64"))]
mod mmio;
#[cfg(target_arch = "x86_64")]
mod port;

use core::fmt::Write;

#[cfg(not(target_arch = "x86_64"))]
use crate::serial::uart16550::mmio::Mmio;

#[cfg(target_arch = "x86_64")]
use crate::serial::uart16550::port::Port;

/// Receiver Buffer Register: read received data when DLAB is clear.
pub const RBR: u8 = 0;
/// Transmitter Holding Register: write outgoing data when DLAB is clear.
pub const THR: u8 = 0;
/// Divisor Latch Low: low baud-rate divisor byte when DLAB is set.
pub const DLL: u8 = 0;

/// Interrupt Enable Register: available when DLAB is clear.
pub const IER: u8 = 1;
/// Divisor Latch High: high baud-rate divisor byte when DLAB is set.
pub const DLM: u8 = 1;

/// Interrupt Identification Register: read-only.
pub const IIR: u8 = 2;
/// FIFO Control Register: write-only.
pub const FCR: u8 = 2;

/// Line Control Register: includes DLAB at bit 7.
pub const LCR: u8 = 3;
/// Modem Control Register.
pub const MCR: u8 = 4;
/// Line Status Register: read-only.
pub const LSR: u8 = 5;
/// Modem Status Register: read-only.
pub const MSR: u8 = 6;
/// Scratch Register: general-purpose byte storage.
pub const SCR: u8 = 7;

#[allow(private_bounds)] // Backends are internal to this mod
#[derive(Debug)]
pub struct Uart16550<T: Backend> {
    backend: T,
}

/// A 16550 UART accessed through x86-64 I/O ports.
#[cfg(target_arch = "x86_64")]
pub type PortUart16550 = Uart16550<Port>;

/// A 16550 UART accessed through memory-mapped registers.
#[cfg(not(target_arch = "x86_64"))]
pub type MmioUart16550 = Uart16550<Mmio>;

#[cfg(target_arch = "x86_64")]
impl Uart16550<Port> {
    pub fn new_port() -> Self {
        let backend = Port::new(0x3F8);
        Self::new(backend)
    }
}

#[allow(private_bounds)] // Backends are internal to this mod
impl<T: Backend> Uart16550<T> {
    fn new(backend: T) -> Self {
        Self { backend }
    }

    pub fn init(&mut self) {
        self.backend.write(IER, 0x00); // Disable interrupts
        self.backend.write(LCR, 0x80); // Enable DLAB
        self.backend.write(DLL, 0x03); // Set divisor (lo byte) 38400 baud
        self.backend.write(DLM, 0x00); //             (hi byte)
        self.backend.write(LCR, 0x03); // 8 bits, no parity, one stop bit
        self.backend.write(FCR, 0xc7); // Enable FIFO, clear them, with 14-byte threshold
        self.backend.write(MCR, 0x0B); // IRQs enabled, RTS/DSR set
        self.backend.write(MCR, 0x1E); // Set in loopback mode, test the serial chip
        self.backend.write(THR, 0xAE); // Test serial chip (send byte 0xAE and check if serial returns same byte)

        assert!(self.backend.read(RBR) == 0xAE);

        // If serial is not faulty set it in normal operation mode
        // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
        self.backend.write(MCR, 0x0F);
    }

    fn is_transmit_buffer_empty(&self) -> bool {
        self.backend.read(LSR) & (1 << 5) != 0
    }

    fn write(&mut self, byte: u8) {
        while !self.is_transmit_buffer_empty() {
            core::hint::spin_loop();
        }

        self.backend.write(THR, byte);
    }
}

trait Backend {
    /// Reads a logical register at offset 0..=7.
    fn read(&self, offset: u8) -> u8;

    /// Writes a logical register at offset 0..=7.
    fn write(&mut self, offset: u8, value: u8);
}

impl<T: Backend> Write for Uart16550<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write(byte);
        }

        Ok(())
    }
}
