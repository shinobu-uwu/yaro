use core::fmt::{self, Write};

use heapless::String;
use log::Record;

pub struct Message {
    pub level: log::Level,
    pub text: String<256>,
    pub truncated: bool,
    pub module: Option<&'static str>,
    pub line: Option<usize>,
}

struct TruncatingWriter<'a, const N: usize> {
    text: &'a mut String<N>,
}

impl<const N: usize> Write for TruncatingWriter<'_, N> {
    // String guarantees len <= capacity, and index 0 is always a UTF-8 boundary,
    // so neither subtraction can underflow
    #[allow(clippy::arithmetic_side_effects)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining = self.text.capacity() - self.text.len();
        let mut end = remaining.min(s.len());

        // Preserve valid UTF-8 when a character crosses the capacity limit
        while !s.is_char_boundary(end) {
            end -= 1;
        }

        self.text.push_str(&s[..end]).map_err(|_| fmt::Error)?;

        if end < s.len() {
            // Stop formatting so later chunks cannot skip past omitted text
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl From<&Record<'_>> for Message {
    fn from(value: &Record<'_>) -> Self {
        let mut text = String::new();
        let incomplete = TruncatingWriter { text: &mut text }
            .write_fmt(*value.args())
            .is_err();

        Self {
            level: value.level(),
            text,
            truncated: incomplete,
            module: value.module_path_static(),
            line: value.line().map(|l| l as _),
        }
    }
}
