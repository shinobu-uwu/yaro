use core::{arch::asm, marker::PhantomData};

use kmemory::address::VirtualAddress;

use crate::x64::{TablePointer, gdt::SegmentSelector, registers::InterruptStackFrame};

const _: () = {
    const fn assert_entry_layout<T: InterruptHandler>() {
        assert!(core::mem::size_of::<Entry<T>>() == 16);
        assert!(core::mem::offset_of!(Entry<T>, offset_low) == 0);
        assert!(core::mem::offset_of!(Entry<T>, options) == 2);
        assert!(core::mem::offset_of!(Entry<T>, offset_middle) == 6);
        assert!(core::mem::offset_of!(Entry<T>, offset_high) == 8);
        assert!(core::mem::offset_of!(Entry<T>, reserved) == 12);
    }

    assert_entry_layout::<Handler>();
    assert_entry_layout::<HandlerWithErrCode>();
    assert_entry_layout::<DivergingHandler>();
    assert_entry_layout::<DivergingHandlerWithErrCode>();

    assert!(core::mem::size_of::<EntryOptions>() == 4);
    assert!(core::mem::offset_of!(EntryOptions, cs) == 0);
    assert!(core::mem::offset_of!(EntryOptions, bits) == 2);
    assert!(core::mem::size_of::<InterruptDescriptorTable>() == 256 * 16);
    assert!(core::mem::offset_of!(InterruptDescriptorTable, interrupts) == 32 * 16);
};

#[repr(C, packed)]
pub struct InterruptDescriptorTable {
    pub divide_error: Entry<Handler>,
    pub debug: Entry<Handler>,
    pub nmi: Entry<Handler>,
    pub breakpoint: Entry<Handler>,
    pub overflow: Entry<Handler>,
    pub bound_range_exceeded: Entry<Handler>,
    pub invalid_opcode: Entry<Handler>,
    pub device_not_available: Entry<Handler>,
    pub double_fault: Entry<DivergingHandlerWithErrCode>,
    coprocessor_segment_overrun: Entry<Handler>,
    pub invalid_tss: Entry<HandlerWithErrCode>,
    pub segment_not_present: Entry<HandlerWithErrCode>,
    pub stack_segment_fault: Entry<HandlerWithErrCode>,
    pub general_protection_fault: Entry<HandlerWithErrCode>,
    pub page_fault: Entry<HandlerWithErrCode>,
    reserved_1: Entry<Handler>,
    pub x87_floating_point: Entry<Handler>,
    pub alignment_check: Entry<HandlerWithErrCode>,
    pub machine_check: Entry<DivergingHandler>,
    pub simd_floating_point: Entry<Handler>,
    pub virtualization: Entry<Handler>,
    pub cp_protection_exception: Entry<HandlerWithErrCode>,
    reserved_2: [Entry<Handler>; 6],
    pub hv_injection_exception: Entry<Handler>,
    pub vmm_communication_exception: Entry<HandlerWithErrCode>,
    pub security_exception: Entry<HandlerWithErrCode>,
    reserved_3: Entry<Handler>,
    interrupts: [Entry<Handler>; 256 - 32],
}

impl InterruptDescriptorTable {
    pub const fn empty() -> Self {
        Self {
            divide_error: Entry::missing(),
            debug: Entry::missing(),
            nmi: Entry::missing(),
            breakpoint: Entry::missing(),
            overflow: Entry::missing(),
            bound_range_exceeded: Entry::missing(),
            invalid_opcode: Entry::missing(),
            device_not_available: Entry::missing(),
            double_fault: Entry::missing(),
            coprocessor_segment_overrun: Entry::missing(),
            invalid_tss: Entry::missing(),
            segment_not_present: Entry::missing(),
            stack_segment_fault: Entry::missing(),
            general_protection_fault: Entry::missing(),
            page_fault: Entry::missing(),
            reserved_1: Entry::missing(),
            x87_floating_point: Entry::missing(),
            alignment_check: Entry::missing(),
            machine_check: Entry::missing(),
            simd_floating_point: Entry::missing(),
            virtualization: Entry::missing(),
            cp_protection_exception: Entry::missing(),
            reserved_2: [Entry::missing(); 6],
            hv_injection_exception: Entry::missing(),
            vmm_communication_exception: Entry::missing(),
            security_exception: Entry::missing(),
            reserved_3: Entry::missing(),
            interrupts: [Entry::missing(); 256 - 32],
        }
    }

    pub unsafe fn load(&self) {
        unsafe { asm!("lidt [{}]", in(reg) &self.pointer(), options(readonly, nostack)) }
    }

    pub fn pointer(&self) -> TablePointer {
        let limit = (size_of::<Self>() - 1) as _;
        let base = VirtualAddress::new(self as *const Self as _);

        TablePointer { limit, base }
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Entry<T: InterruptHandler> {
    offset_low: u16,
    options: EntryOptions,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
    _marker: PhantomData<T>,
}

#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq)]
pub struct EntryOptions {
    cs: SegmentSelector,
    bits: u16,
}

pub trait InterruptHandler: sealed::Sealed {
    fn address(&self) -> VirtualAddress;
}

pub type Handler = extern "x86-interrupt" fn(InterruptStackFrame);
pub type HandlerWithErrCode = extern "x86-interrupt" fn(InterruptStackFrame, u64);
pub type DivergingHandler = extern "x86-interrupt" fn(InterruptStackFrame) -> !;
pub type DivergingHandlerWithErrCode = extern "x86-interrupt" fn(InterruptStackFrame, u64) -> !;

impl<T: InterruptHandler> Entry<T> {
    #[inline]
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            options: EntryOptions::minimal(),
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
            _marker: PhantomData,
        }
    }

    pub fn set_handler(&mut self, handler: T) -> &mut Self {
        let addr = handler.address().as_u64();
        self.offset_low = addr as u16;
        self.offset_middle = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.options = EntryOptions::minimal();
        let cs: u16;
        unsafe {
            asm!("mov {:x}, cs", out(reg) cs);
        }
        self.options.cs = SegmentSelector::from_u16(cs);
        self.options.bits |= 1 << 15; // present bit

        self
    }

    /// Selects a zero-based index into the TSS's seven IST stacks.
    /// `None` uses normal CPU stack selection. Call after `set_handler`,
    /// which resets the entry's options.
    ///
    /// # Panics
    /// Panics if the index is outside `0..7`.
    #[inline]
    pub fn set_stack_index(&mut self, index: Option<usize>) {
        let encoded = match index {
            None => 0,
            Some(index) => {
                assert!(index < 7, "TSS stack index must be in 0..7");
                (index + 1) as u16
            }
        };

        self.options.bits = (self.options.bits & !0b111) | encoded;
    }
}

impl EntryOptions {
    #[inline]
    const fn minimal() -> Self {
        EntryOptions {
            cs: SegmentSelector::from_u16(0),
            bits: 0b1110_0000_0000, // Default to a 64-bit Interrupt Gate
        }
    }
}

impl sealed::Sealed for Handler {}
impl sealed::Sealed for HandlerWithErrCode {}
impl sealed::Sealed for DivergingHandler {}
impl sealed::Sealed for DivergingHandlerWithErrCode {}
impl InterruptHandler for Handler {
    fn address(&self) -> VirtualAddress {
        VirtualAddress::new(*self as _)
    }
}
impl InterruptHandler for HandlerWithErrCode {
    fn address(&self) -> VirtualAddress {
        VirtualAddress::new(*self as _)
    }
}
impl InterruptHandler for DivergingHandler {
    fn address(&self) -> VirtualAddress {
        VirtualAddress::new(*self as _)
    }
}
impl InterruptHandler for DivergingHandlerWithErrCode {
    fn address(&self) -> VirtualAddress {
        VirtualAddress::new(*self as _)
    }
}

mod sealed {
    pub trait Sealed {}
}
