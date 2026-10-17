use crate::address::PhysicalAddress;

pub mod bitmap;

pub trait FrameAllocator {
    fn allocate(&mut self) -> Option<PhysicalAddress>;
    fn deallocate(&mut self, frame: PhysicalAddress);
}
