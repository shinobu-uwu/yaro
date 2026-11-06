use core::fmt::{self, Write};

use limine::{
    framebuffer::{Framebuffer as LimineFramebuffer, MemoryModel},
    request::FramebufferRequest,
};
use noto_sans_mono_bitmap::{FontWeight, RasterHeight, get_raster, get_raster_width};
use spin::Mutex;
use yaro_log::{Level, Record, Sink};

const FONT_WEIGHT: FontWeight = FontWeight::Regular;
const FONT_HEIGHT: RasterHeight = RasterHeight::Size16;
const LINE_HEIGHT: usize = 20;
const MARGIN: usize = 8;
const TEXT_COLOR: [u8; 3] = [220, 220, 220];

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

pub static FRAMEBUFFER: Framebuffer = Framebuffer {
    writer: Mutex::new(None),
};

pub struct Framebuffer {
    writer: Mutex<Option<Writer>>,
}

/// Takes ownership of the first boot framebuffer and clears it for logging.
///
/// # Safety
/// Call once during early boot, before registering the sink. Limine's response
/// and framebuffer mappings must remain valid for the kernel's lifetime. No
/// other code may access this framebuffer after initialization. As with the
/// other log sinks, callers must prevent interrupt reentry into the logger.
pub unsafe fn init() -> Result<(), &'static str> {
    let framebuffer = FRAMEBUFFER_REQUEST
        .get_response()
        .and_then(|response| response.framebuffers().next())
        .ok_or("No boot framebuffer available")?;
    // SAFETY: The caller grants exclusive access to the mapped framebuffer.
    let writer = unsafe { Writer::new(framebuffer) }?;
    *FRAMEBUFFER.writer.lock() = Some(writer);
    Ok(())
}

impl Sink for Framebuffer {
    fn write(&self, record: &Record<'_>) {
        let mut guard = self.writer.lock();
        let Some(writer) = guard.as_mut() else {
            return;
        };

        writer.color = match record.level() {
            Level::Error => [255, 95, 95],
            Level::Warn => [255, 195, 80],
            Level::Info => [100, 220, 140],
            Level::Debug => [100, 180, 255],
            Level::Trace => [190, 150, 255],
        };
        let _ = write!(writer, "{}", record.level());
        writer.color = TEXT_COLOR;
        let _ = writeln!(
            writer,
            " [{}:{}]: {}",
            record.module_path().unwrap_or_default(),
            record.line().unwrap_or_default(),
            record.args(),
        );
    }
}

struct Writer {
    buffer: &'static mut [u8],
    pitch: usize,
    bytes_per_pixel: usize,
    channels: [(u8, u8); 3], // (bit shift, bit count), in RGB order.
    columns: usize,
    rows: usize,
    column: usize,
    row: usize,
    color: [u8; 3],
}

impl Writer {
    /// The framebuffer mapping must remain valid and exclusively owned forever.
    unsafe fn new(framebuffer: LimineFramebuffer<'static>) -> Result<Self, &'static str> {
        if framebuffer.memory_model() != MemoryModel::RGB
            || !matches!(framebuffer.bpp(), 16 | 24 | 32)
        {
            return Err("Unsupported framebuffer pixel format");
        }

        let width =
            usize::try_from(framebuffer.width()).map_err(|_| "Invalid framebuffer width")?;
        let height =
            usize::try_from(framebuffer.height()).map_err(|_| "Invalid framebuffer height")?;
        let pitch =
            usize::try_from(framebuffer.pitch()).map_err(|_| "Invalid framebuffer pitch")?;
        let bytes_per_pixel = usize::from(framebuffer.bpp() / 8);
        let row_bytes = width
            .checked_mul(bytes_per_pixel)
            .ok_or("Framebuffer row size overflow")?;
        let len = pitch
            .checked_mul(height)
            .filter(|len| *len <= isize::MAX as usize)
            .ok_or("Framebuffer size overflow")?;
        if framebuffer.addr().is_null()
            || (framebuffer.addr() as usize).checked_add(len).is_none()
            || pitch < row_bytes
        {
            return Err("Invalid framebuffer memory layout");
        }

        let channels = [
            (framebuffer.red_mask_shift(), framebuffer.red_mask_size()),
            (
                framebuffer.green_mask_shift(),
                framebuffer.green_mask_size(),
            ),
            (framebuffer.blue_mask_shift(), framebuffer.blue_mask_size()),
        ];
        let mut used_bits = 0u32;
        for (shift, size) in channels {
            if !(1..=8).contains(&size) || u16::from(shift) + u16::from(size) > framebuffer.bpp() {
                return Err("Unsupported framebuffer color masks");
            }
            let mask = ((1u32 << size) - 1) << shift;
            if used_bits & mask != 0 {
                return Err("Overlapping framebuffer color masks");
            }
            used_bits |= mask;
        }

        let glyph_width = get_raster_width(FONT_WEIGHT, FONT_HEIGHT);
        if width < 2 * MARGIN + glyph_width || height < 2 * MARGIN + LINE_HEIGHT {
            return Err("Framebuffer too small for log text");
        }

        // SAFETY: The caller grants exclusive ownership of the boot mapping.
        // Initialize every byte before creating a reference or reading it while
        // scrolling; framebuffer memory may contain uninitialized bytes at boot.
        let buffer = unsafe {
            for offset in 0..len {
                framebuffer.addr().add(offset).write_volatile(0);
            }
            core::slice::from_raw_parts_mut(framebuffer.addr(), len)
        };

        Ok(Self {
            buffer,
            pitch,
            bytes_per_pixel,
            channels,
            columns: (width - 2 * MARGIN) / glyph_width,
            rows: (height - 2 * MARGIN) / LINE_HEIGHT,
            column: 0,
            row: 0,
            color: TEXT_COLOR,
        })
    }

    fn pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let mut pixel = 0u32;
        for (color, (shift, size)) in self.color.into_iter().zip(self.channels) {
            let shaded = u32::from(color) * u32::from(intensity) / 255;
            let channel = shaded * ((1u32 << size) - 1) / 255;
            pixel |= channel << shift;
        }

        let offset = y * self.pitch + x * self.bytes_per_pixel;
        // SAFETY: Cursor coordinates stay inside the validated text grid, whose
        // glyphs fit in the framebuffer. Byte stores avoid alignment assumptions
        // and support packed 16-, 24-, and 32-bit little-endian RGB layouts.
        unsafe {
            for (index, byte) in pixel.to_le_bytes()[..self.bytes_per_pixel]
                .iter()
                .enumerate()
            {
                self.buffer
                    .as_mut_ptr()
                    .add(offset + index)
                    .write_volatile(*byte);
            }
        }
    }

    /// The caller must ensure this range is within the exclusively owned mapping.
    unsafe fn clear_bytes(&mut self, start: usize, count: usize) {
        for offset in start..start + count {
            // SAFETY: The caller provides a valid range. Black is zero in each
            // supported RGB layout, including row padding and reserved bits.
            unsafe { self.buffer.as_mut_ptr().add(offset).write_volatile(0) };
        }
    }

    fn newline(&mut self) {
        self.column = 0;
        if self.row + 1 < self.rows {
            self.row += 1;
            return;
        }

        let start = MARGIN * self.pitch;
        let line_bytes = LINE_HEIGHT * self.pitch;
        let retained_bytes = (self.rows - 1) * line_bytes;
        // SAFETY: The validated grid fits in the framebuffer, and all bytes
        // were initialized on startup. Copy forward from a higher address to
        // handle overlap. Volatile accesses preserve framebuffer I/O semantics.
        unsafe {
            for offset in start..start + retained_bytes {
                let byte = self
                    .buffer
                    .as_ptr()
                    .add(offset + line_bytes)
                    .read_volatile();
                self.buffer.as_mut_ptr().add(offset).write_volatile(byte);
            }
            self.clear_bytes(start + retained_bytes, line_bytes);
        }
    }

    fn character(&mut self, character: char) {
        match character {
            '\n' => return self.newline(),
            '\r' => {
                self.column = 0;
                return;
            }
            '\t' => {
                for _ in 0..4 - self.column % 4 {
                    self.character(' ');
                }
                return;
            }
            character if character.is_control() => return,
            _ => {}
        }

        if self.column == self.columns {
            self.newline();
        }
        let Some(glyph) = get_raster(character, FONT_WEIGHT, FONT_HEIGHT)
            .or_else(|| get_raster('?', FONT_WEIGHT, FONT_HEIGHT))
        else {
            return;
        };
        let x = MARGIN + self.column * glyph.width();
        let y = MARGIN + self.row * LINE_HEIGHT;
        for (dy, row) in glyph.raster().iter().enumerate() {
            for (dx, intensity) in row.iter().enumerate() {
                self.pixel(x + dx, y + dy, *intensity);
            }
        }
        self.column += 1;
    }
}

impl Write for Writer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for character in text.chars() {
            self.character(character);
        }
        Ok(())
    }
}
