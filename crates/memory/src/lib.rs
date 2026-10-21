#![no_std]

use spin::{Mutex, Once};

use crate::{
    allocator::physical::bitmap::BitmapAllocator,
    memory_map::MemoryRegion,
    paging::{Arch, CurrentArch},
};

pub mod address;
pub mod allocator;
pub mod memory_map;
pub mod paging;

pub const PAGE_SIZE: usize = CurrentArch::PAGE_SIZE;

static HHDM_OFFSET: Once<usize> = Once::new();
pub static FRAME_ALLOCATOR: Mutex<BitmapAllocator> = Mutex::new(BitmapAllocator::new());

pub fn init(hhdm_offset: usize, memory_map: impl IntoIterator<Item = MemoryRegion>) {
    HHDM_OFFSET.call_once(|| hhdm_offset);
    FRAME_ALLOCATOR.lock().init(memory_map);
}

pub fn hhdm_offset() -> usize {
    *HHDM_OFFSET.get().expect("Memory subsystem not initialized")
}
