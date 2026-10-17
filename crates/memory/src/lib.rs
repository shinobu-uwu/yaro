#![no_std]

use spin::Once;
use yaro_log::info;

use crate::{
    address::PhysicalAddress,
    allocator::physical::bitmap::BitmapAllocator,
    paging::{arch::x64::X86Arch, mapper::PageMapper},
};

pub mod address;
pub mod allocator;
pub mod memory_map;
pub mod paging;

static HHDM_OFFSET: Once<usize> = Once::new();
static ALLOCATOR: BitmapAllocator = BitmapAllocator::new();

pub fn init(hhdm_offset: usize) {
    HHDM_OFFSET.call_once(|| hhdm_offset);
    let mapper: PageMapper<X86Arch> = PageMapper::new();
    let phys_addr = PhysicalAddress::new(0x40001);
    let virt_addr = phys_addr.to_hhdm();

    let translated = unsafe { mapper.translate(virt_addr) };

    info!("Phys: {:#?}", phys_addr);
    info!("Virt: {:#?}", virt_addr);
    info!("Translated: {:#?}", translated);

    info!("{:#?}", ALLOCATOR);
}

pub fn hhdm_offset() -> usize {
    *HHDM_OFFSET.get().expect("Memory subsystem not initialized")
}
