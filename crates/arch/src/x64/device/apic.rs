use core::arch::x86_64::__cpuid;

use yaro_memory::{
    FRAME_ALLOCATOR,
    address::{PhysicalAddress, VirtualAddress},
    paging::{
        MappingFlags,
        arch::x64::{PageFlags, X86Arch},
        mapper::PageMapper,
    },
};

use crate::x64::registers::msr::{IA32_APIC_BASE, read_msr, write_msr};

pub(crate) const SPURIOUS_VECTOR_INDEX: u8 = 0xFF;
pub(crate) const TIMER_VECTOR: u8 = 0x40;

#[derive(Debug)]
pub struct LocalApic {
    mode: LocalApicMode,
}

#[derive(Debug, PartialEq, Eq)]
enum LocalApicMode {
    X(VirtualAddress),
    X2,
}

impl LocalApic {
    pub fn is_supported() -> bool {
        let max_leaf = __cpuid(0x0).eax;

        if max_leaf < 1 {
            return false;
        }

        let features = __cpuid(1);
        features.edx & (1 << 9) != 0
    }

    pub fn new() -> Self {
        let features = __cpuid(1);
        let mode = if features.ecx & (1 << 21) != 0 {
            LocalApicMode::X2
        } else {
            LocalApicMode::X(VirtualAddress::new(0))
        };

        Self { mode }
    }

    pub fn init(&mut self, mapper: &mut PageMapper<X86Arch>) {
        let mut apic_base = unsafe { read_msr(IA32_APIC_BASE) };
        if apic_base & (1 << 11) == 0 {
            apic_base |= 1 << 11;
            // SAFETY: Early boot runs at ring 0 on this CPU. Preserve the base
            // and other control bits, enabling xAPIC before entering x2APIC.
            unsafe { write_msr(IA32_APIC_BASE, apic_base) };
        }

        match self.mode {
            LocalApicMode::X(_) => {
                let xapic_base = unsafe { read_msr(IA32_APIC_BASE) & 0x000F_FFFF_FFFF_F000 };
                let xapic_base = PhysicalAddress::from(xapic_base as usize);
                unsafe {
                    mapper
                        .map_to(
                            xapic_base,
                            xapic_base.to_hhdm(),
                            PageFlags::new().write(true) | PageFlags::NO_CACHE,
                            &mut *FRAME_ALLOCATOR.lock(),
                        )
                        .expect("Failed to map xAPIC page")
                        .flush();
                    self.mode = LocalApicMode::X(xapic_base.to_hhdm());
                }
            }
            LocalApicMode::X2 => unsafe {
                write_msr(IA32_APIC_BASE, apic_base | (1 << 10));
            },
        }

        // Software-enable LAPIC and select spurious vector 255.
        unsafe {
            // Mask local sources without handlers, including legacy PIC delivery
            // through LINT0. The timer is unmasked separately when started.
            self.write(0x320, 1 << 16);
            self.write(0x350, 1 << 16);
            self.write(0x360, 1 << 16);
            self.write(0x370, 1 << 16);
            self.write(0x80, 0); // Accept all interrupt priority classes.
            self.write(0xF0, 0x100 | SPURIOUS_VECTOR_INDEX as u32);
        }
    }

    /// Starts a periodic countdown in uncalibrated APIC timer ticks.
    ///
    /// # Safety
    /// This CPU's LAPIC must be initialized, local interrupts disabled, and
    /// `vector` installed in the IDT with a handler that sends EOI.
    pub unsafe fn start_timer(&mut self, vector: u8, initial_count: u32) {
        assert!(vector >= 32 && vector != SPURIOUS_VECTOR_INDEX);
        assert!(initial_count > 0);

        // SAFETY: The caller guarantees initialized hardware and exclusive
        // access. Configure while masked and start by writing the count last.
        unsafe {
            self.write(0x320, (1 << 16) | u32::from(vector));
            self.write(0x380, 0);
            self.write(0x3E0, 0b1011); // Divide by 1.
            self.write(0x320, (1 << 17) | u32::from(vector)); // Periodic.
            self.write(0x380, initial_count);
        }
    }

    /// Acknowledges the current interrupt on this CPU.
    ///
    /// # Safety
    /// The APIC must be initialized, and an interrupt requiring EOI
    /// must have been serviced on this CPU.
    pub unsafe fn eoi(&mut self) {
        unsafe {
            self.write(0xB0, 0);
        }
    }

    unsafe fn write(&mut self, reg: u32, value: u32) {
        match self.mode {
            LocalApicMode::X(base) => unsafe {
                core::ptr::write_volatile((base + reg).as_mut_ptr(), value);
            },
            LocalApicMode::X2 => {
                let msr = 0x800 + (reg >> 4);
                unsafe {
                    write_msr(msr, value as _);
                }
            }
        }
    }

    unsafe fn read(&mut self, reg: u32) -> u32 {
        match self.mode {
            LocalApicMode::X(base) => unsafe {
                core::ptr::read_volatile((base + reg).as_ptr::<u32>())
            },
            LocalApicMode::X2 => {
                let msr = 0x800 + (reg >> 4);
                unsafe { read_msr(msr) as _ }
            }
        }
    }
}
