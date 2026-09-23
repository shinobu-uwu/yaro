use core::fmt::Write;

#[cfg(not(target_arch = "x86_64"))]
use kdevice::serial::uart16550::MmioUart16550 as PlatformUart;
#[cfg(target_arch = "x86_64")]
use kdevice::serial::uart16550::PortUart16550 as PlatformUart;
use klog::{Level, Sink};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref UART: Mutex<PlatformUart> = Mutex::new(init_uart());
}

#[cfg(target_arch = "x86_64")]
fn init_uart() -> PlatformUart {
    let mut uart = PlatformUart::new_port();
    uart.init();
    uart
}

#[cfg(not(target_arch = "x86_64"))]
fn init_uart() -> PlatformUart {
    todo!("configure a platform-specific MMIO UART address and initialize it")
}

pub struct Serial;

impl Sink for Serial {
    fn write(&self, message: &klog::message::Message) {
        let level = message.level;
        let level_color = match message.level {
            Level::Error => "160",
            Level::Warn => "172",
            Level::Info => "47",
            Level::Debug => "25",
            Level::Trace => "103",
        };
        let module = message.module.unwrap_or_default();
        let line = message.line.unwrap_or_default();
        let mut serial = UART.lock();
        write!(
            serial,
            "\x1b[38;5;{level_color}m{level}\x1b[0m [{module}:{line}]: {}\r\n",
            message.text,
        )
        .expect("Failed to write to serial port");
    }
}
