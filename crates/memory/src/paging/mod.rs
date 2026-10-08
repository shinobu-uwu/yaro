use crate::address::PhysicalAddress;

pub mod arch;

pub trait Arch: Clone + Copy {
    /// The hardware entry representation used by this architecture.
    type Entry: Entry;
    fn table() -> PhysicalAddress;
}

pub trait Entry: Clone {}
