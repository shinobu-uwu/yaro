#[derive(Debug, Clone)]
pub struct GlobalDescriptorTable<const N: usize = 8> {
    entries: [Entry; N],
    len: usize,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub limit_low: u16,
    pub offset_low: u16,
    pub offset_mid: u8,
    pub access: u8,
    pub flags_limit_high: u8,
    pub offset_high: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum Descriptor {
    Segment(u64),
    SystemSegment(u64, u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Selector(u16);

impl<const N: usize> GlobalDescriptorTable<N> {
    #[inline]
    pub const fn new() -> Self {
        assert!(N > 0, "a GDT must have room for the null descriptor");
        assert!(N <= (1 << 13), "a GDT cannot exceed 8192 (2^13) entries");

        Self {
            entries: [const { Entry::null() }; N],
            len: 1, // first entry is the null descriptor
        }
    }

    #[inline]
    const fn push(&mut self, value: u64) -> usize {
        let index = self.len;
        self.entries[index] = Entry::from_u64(value);
        self.len += 1;
        index
    }

    pub const fn push_back(&mut self, descriptor: Descriptor) -> Selector {
        let selector_index = match descriptor {
            Descriptor::Segment(value) => {
                assert!(self.len <= self.entries.len().saturating_sub(1));
                self.push(value)
            }
            Descriptor::SystemSegment(low, high) => {
                assert!(self.len <= self.entries.len().saturating_sub(2));
                let index = self.push(low);
                self.push(high);
                index
            }
        };

        Selector::new(selector_index as u16)
    }
}

impl Entry {
    pub const fn from_u64(value: u64) -> Self {
        Self {
            limit_low: value as u16,
            offset_low: (value >> 16) as u16,
            offset_mid: (value >> 32) as u8,
            access: (value >> 40) as u8,
            flags_limit_high: (value >> 48) as u8,
            offset_high: (value >> 56) as u8,
        }
    }

    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            offset_low: 0,
            offset_mid: 0,
            access: 0,
            flags_limit_high: 0,
            offset_high: 0,
        }
    }
}

impl Selector {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}
