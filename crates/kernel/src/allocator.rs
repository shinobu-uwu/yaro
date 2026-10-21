use linked_list_allocator::LockedHeap;
use yaro_memory::{
    FRAME_ALLOCATOR, PAGE_SIZE,
    address::VirtualAddress,
    allocator::physical::FrameAllocator,
    paging::{CurrentArch, MappingFlags, arch::x64::PageFlags, mapper::PageMapper},
};

pub const HEAP_START: VirtualAddress = VirtualAddress::new(0xffff_ff7f_8000_0000);
pub const HEAP_SIZE: usize = 10 * 1024 * 1024; // 100 KiB

#[global_allocator]
// TODO: implement slab allocator ourselves
static HEAP: LockedHeap = LockedHeap::empty();

pub fn init() {
    let mut mapper: PageMapper<CurrentArch> = PageMapper::new();
    let pages = HEAP_SIZE / PAGE_SIZE;
    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    for i in 0..pages {
        let base = HEAP_START + (i * PAGE_SIZE);
        let frame = frame_allocator
            .allocate()
            .expect("Failed to allocate frame");
        unsafe {
            mapper
                .map_to(
                    frame,
                    base,
                    PageFlags::new().write(true).user(true).execute(false),
                    &mut *frame_allocator,
                )
                .expect("Failed to map heap addresses")
                // SAFETY: This boot CPU is using the mapper's active root,
                // runs at kernel privilege, and no secondary CPUs are active.
                .flush();
        }
    }

    unsafe {
        HEAP.lock().init(HEAP_START.as_mut_ptr(), HEAP_SIZE);
    }
}
