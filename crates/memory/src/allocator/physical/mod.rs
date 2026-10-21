use crate::address::PhysicalAddress;

pub mod bitmap;

/// Allocates and releases individual physical memory frames.
pub trait FrameAllocator {
    /// Returns the base address of a free frame, or `None` if none are available.
    /// The frame must be aligned to the allocator's frame size and exclusively
    /// allocated until released. Its contents are not guaranteed to be zeroed.
    fn allocate(&mut self) -> Option<PhysicalAddress>;

    /// Releases a frame previously allocated by this allocator for reuse.
    /// The frame must still be allocated, and all uses that require it to remain
    /// allocated must have ended before calling this method.
    fn deallocate(&mut self, frame: PhysicalAddress);
}
