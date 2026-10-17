use bit_field::BitField;

use crate::{
    address::PhysicalAddress,
    allocator::physical::FrameAllocator,
    memory_map::{MemoryRegion, MemoryType},
};

const STORAGE_SIZE: usize = 4 * 1024 * 1024; // 4MiB
const STORAGE_LEN: usize = STORAGE_SIZE / size_of::<u64>();
const FRAME_SIZE: usize = 4096;

#[derive(Debug)]
pub struct BitmapAllocator {
    words: [u64; STORAGE_LEN],
}

impl BitmapAllocator {
    /// Create an allocator with all bytes busy
    pub const fn new() -> Self {
        Self {
            words: [u64::MAX; STORAGE_LEN],
        }
    }

    pub fn init(&mut self, memory_map: &[MemoryRegion]) {
        for region in memory_map {
            if region.memory_type != MemoryType::Usable {
                continue;
            }

            let start = region.base.as_usize().div_ceil(FRAME_SIZE);
            let region_end = region
                .base
                .as_usize()
                .checked_add(region.len)
                .expect("memory region address overflow");
            let end = region_end / FRAME_SIZE;

            assert!(
                end <= self.words.len() * u64::BITS as usize,
                "usable memory exceeds bitmap capacity"
            );

            for frame in start..end {
                let word_index = frame / u64::BITS as usize;
                let bit_index = frame % u64::BITS as usize;
                self.words[word_index].set_bit(bit_index, false);
            }
        }
    }
}

impl FrameAllocator for BitmapAllocator {
    fn allocate(&mut self) -> Option<PhysicalAddress> {
        let (word_index, word) = self
            .words
            .iter_mut()
            .enumerate()
            .find(|(_, w)| **w != u64::MAX)?;
        let bit_index = 63 - (!*word).leading_zeros() as usize;

        word.set_bit(bit_index, true);
        let addr = (word_index * u64::BITS as usize + bit_index) * FRAME_SIZE;

        Some(PhysicalAddress::new(addr))
    }

    fn deallocate(&mut self, frame: PhysicalAddress) {
        assert!(frame.is_aligned(FRAME_SIZE));

        let addr = frame.as_usize();
        let frame_index = addr / FRAME_SIZE;
        let word_index = frame_index / u64::BITS as usize;
        let bit_index = frame_index % u64::BITS as usize;

        assert!(word_index < self.words.len(), "frame outside bitmap");
        assert!(
            self.words[word_index].get_bit(bit_index),
            "frame already free"
        );

        self.words[word_index].set_bit(bit_index, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deallocate_reuses_only_the_freed_frame() {
        // Keep the 4 MiB bitmap off the test thread's stack.
        static ALLOCATOR: spin::Mutex<BitmapAllocator> = spin::Mutex::new(BitmapAllocator::new());
        let mut allocator = ALLOCATOR.lock();
        allocator.init(&[MemoryRegion {
            base: PhysicalAddress::new(0),
            len: 66 * FRAME_SIZE,
            memory_type: MemoryType::Usable,
        }]);

        let mut allocated = [false; 66];
        for _ in 0..allocated.len() {
            let address = allocator.allocate().expect("frame should be available");
            assert!(address.is_aligned(FRAME_SIZE));
            let frame_index = address.as_usize() / FRAME_SIZE;
            assert!(!allocated[frame_index], "frame allocated twice");
            allocated[frame_index] = true;
        }
        assert_eq!(allocator.allocate(), None);

        // Exercise nonzero bits and both sides of a bitmap word boundary.
        for frame_index in [1, 63, 64, 65] {
            let address = PhysicalAddress::new(frame_index * FRAME_SIZE);
            allocator.deallocate(address);
            assert_eq!(allocator.allocate(), Some(address));
            assert_eq!(allocator.allocate(), None);
        }
    }
}
