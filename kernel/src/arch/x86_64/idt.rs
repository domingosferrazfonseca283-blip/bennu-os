//! Interrupt Descriptor Table and CPU exception boundary.

use core::arch::asm;
use core::mem::size_of;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    options: u16,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const EMPTY: Self = Self {
        offset_low: 0,
        selector: 0,
        options: 0,
        offset_mid: 0,
        offset_high: 0,
        reserved: 0,
    };

    fn set_address(&mut self, address: u64) {
        self.offset_low = address as u16;
        self.selector = super::gdt::KERNEL_CODE_SELECTOR;
        self.options = 0x8E00;
        self.offset_mid = (address >> 16) as u16;
        self.offset_high = (address >> 32) as u32;
        self.reserved = 0;
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

type Handler = extern "x86-interrupt" fn(InterruptStackFrame);
type HandlerWithError = extern "x86-interrupt" fn(InterruptStackFrame, u64);
type DivergingHandlerWithError =
    extern "x86-interrupt" fn(InterruptStackFrame, u64) -> !;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::EMPTY; 256];

extern "x86-interrupt" fn exception_handler(frame: InterruptStackFrame) {
    let _ = frame;
    super::diagnostics::write_line(1, b"BENNU EXCEPTION");
    halt_forever();
}

extern "x86-interrupt" fn breakpoint_handler(frame: InterruptStackFrame) {
    let _ = frame;
    super::diagnostics::write_line(1, b"BENNU BREAKPOINT");
}

extern "x86-interrupt" fn double_fault_handler(
    frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    let _ = frame;
    let _ = error_code;
    super::diagnostics::write_line(1, b"BENNU DOUBLE FAULT");
    halt_forever()
}

extern "x86-interrupt" fn page_fault_handler(
    frame: InterruptStackFrame,
    error_code: u64,
) {
    let _ = frame;
    let _ = error_code;
    super::diagnostics::write_line(1, b"BENNU PAGE FAULT");
    halt_forever();
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn init() {
    unsafe {
        for entry in &mut IDT {
            *entry = IdtEntry::EMPTY;
        }

        IDT[0].set_address(exception_handler as usize as u64);
        IDT[3].set_address(breakpoint_handler as usize as u64);
        IDT[8].set_address(
            double_fault_handler as DivergingHandlerWithError as usize as u64,
        );
        IDT[14].set_address(page_fault_handler as HandlerWithError as usize as u64);

        let idtr = Idtr {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: IDT.as_ptr() as u64,
        };

        asm!(
            "lidt [{idt}]",
            idt = in(reg) &idtr,
            options(readonly, nostack, preserves_flags),
        );
    }
}
