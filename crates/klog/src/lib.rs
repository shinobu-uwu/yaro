#![no_std]

pub mod message;

use core::fmt::Display;

use circular_buffer::FixedCircularBuffer;
use heapless::Vec;
use log::LevelFilter;
use log::Record;
pub use log::{Level, debug, error, info, trace, warn};
use spin::Mutex;

use crate::message::Message;

static SYSLOG: Mutex<SysLog> = Mutex::new(SysLog {
    messages: FixedCircularBuffer::new(),
    sinks: Vec::new(),
});

struct Log;

pub fn init() -> Result<(), Error> {
    log::set_logger(&Log).map_err(|_| Error::Init)?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

pub fn register_sink(sink: &'static dyn Sink) -> Result<(), Error> {
    SYSLOG.lock().push_sink(sink)
}

struct SysLog {
    messages: FixedCircularBuffer<Message, 1024>,
    sinks: Vec<&'static dyn Sink, 128>,
}

impl SysLog {
    /// Registers a statically allocated sink without replacing existing sinks.
    pub fn push_sink(&mut self, sink: &'static dyn Sink) -> Result<(), Error> {
        self.sinks.push(sink).map_err(|_| Error::SinksFull)?;
        Ok(())
    }

    /// Pushes a message to the circular buffer
    pub fn push_message(&mut self, message: Message) {
        self.messages.push_back(message);
    }
}

impl log::Log for Log {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let message = record.into();
        let mut syslog = SYSLOG.lock();

        for sink in syslog.sinks.iter() {
            sink.write(&message);
        }

        syslog.push_message(message);
    }

    fn flush(&self) {
        // nop
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    SinksFull,
    Init,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let msg = match self {
            Error::SinksFull => "Sinks full",
            Error::Init => "Init",
        };

        write!(f, "{msg}")
    }
}

pub trait Sink: Sync {
    /// Hook for when a message is written to SysLog
    fn write(&self, message: &Message);
}
