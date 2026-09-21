use core::fmt::Write;

use klog::{Level, Sink};
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    pub static ref SERIAL_PORT: Mutex<SerialPort> = unsafe {
        let mut s = SerialPort::new(0x3F8);
        s.init();
        Mutex::new(s)
    };
}

pub struct SerialSink;

impl Sink for SerialSink {
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
        let mut serial = SERIAL_PORT.lock();
        write!(
            serial,
            "\x1b[38;5;{level_color}m{level}\x1b[0m [{module}:{line}]: {}\r\n",
            message.text,
        )
        .expect("Failed to write to serial port");
    }
}
