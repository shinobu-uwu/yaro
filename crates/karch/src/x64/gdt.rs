#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable<const N: usize = 8> {
    entries: [Entry; N],
    len: usize,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub limitl: u16,
    pub offsetl: u16,
    pub offsetm: u8,
    pub access: u8,
    pub flags_limith: u8,
    pub offseth: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum Descriptor {
    UserSegment(u64),
    SystemSegment(u64, u64),
}
