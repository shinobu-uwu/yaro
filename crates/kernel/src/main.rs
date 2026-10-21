#![no_std]
#![no_main]

mod allocator;
mod sink;

extern crate alloc;

use core::arch::asm;

use alloc::boxed::Box;
use limine::BaseRevision;
use limine::memory_map::EntryType;
use limine::request::{HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker};
use yaro_log::{error, info};
use yaro_memory::memory_map::{MemoryRegion, MemoryType};

/// Sets the base revision to the latest revision supported by the crate.
/// See specification for further info.
/// Be sure to mark all limine requests with #[used], otherwise they may be removed by the compiler.
#[used]
// The .requests section allows limine to find the requests faster and more safely.
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

/// Define the stand and end markers for Limine requests.
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());
    yaro_log::init().expect("Failed to init logging");
    yaro_log::register_sink(&sink::serial::Serial).expect("Sink buffer is full");
    // SAFETY: This is the boot CPU's only initialization call. Limine enters
    // with interrupts disabled; no secondary CPUs have been started.
    unsafe {
        yaro_arch::init();
    }

    let hhdm_offset = HHDM_REQUEST
        .get_response()
        .map(|r| r.offset() as usize)
        .expect("HHDM request failed");
    let memory_map = MEMORY_MAP_REQUEST
        .get_response()
        .map(|r| r.entries())
        .expect("Memory Map request failed");
    let regions = memory_map.iter().map(|entry| MemoryRegion {
        base: entry.base.into(),
        len: entry.length as usize,
        memory_type: convert_memory_type(entry.entry_type),
    });
    yaro_memory::init(hhdm_offset, regions);
    allocator::init();
    let boxed = Box::new(42);
    info!("Boxed: {:#?}", boxed);

    hcf();
}

fn convert_memory_type(entry_type: EntryType) -> MemoryType {
    match entry_type {
        EntryType::USABLE => MemoryType::Usable,
        EntryType::RESERVED => MemoryType::Reserved,
        EntryType::ACPI_RECLAIMABLE => MemoryType::AcpiReclaimable,
        EntryType::ACPI_NVS => MemoryType::AcpiNvs,
        EntryType::BAD_MEMORY => MemoryType::Bad,
        EntryType::BOOTLOADER_RECLAIMABLE => MemoryType::BootloaderReclaimable,
        EntryType::EXECUTABLE_AND_MODULES => MemoryType::Executable,
        EntryType::FRAMEBUFFER => MemoryType::FrameBuffer,
        // Unknown types must never be treated as usable memory.
        _ => MemoryType::Reserved,
    }
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    error!("Kernel panic\n{info}");
    hcf();
}
fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
