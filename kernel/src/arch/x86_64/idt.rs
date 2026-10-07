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

    fn set_user_address(&mut self, address: u64) {
        self.offset_low = address as u16;
        self.selector = super::gdt::KERNEL_CODE_SELECTOR;
        self.options = 0xEE00;
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

/// Complete register/return frame built by the timer entry stub.
///
/// The register order exactly matches bennu_timer_entry: the first field
/// is the value at RSP when Rust receives the pointer, followed by the CPU
/// interrupt-return frame. Keeping this layout explicit is the foundation
/// for safe preemption and later iretq-based context restoration.
#[repr(C)]
pub struct TimerInterruptFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

core::arch::global_asm!(r#"
.global bennu_timer_entry
.type bennu_timer_entry,@function
bennu_timer_entry:
    cld
    push rax
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    mov rdi, rsp
    call bennu_timer_dispatch
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rax
    iretq
"#);

extern "C" {
    fn bennu_timer_entry();
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

extern "x86-interrupt" fn keyboard_handler(frame: InterruptStackFrame) {
    let _ = frame;
    let scancode = unsafe { super::io::inb(0x60) };
    crate::drivers::keyboard::feed_scancode(scancode);
    unsafe { super::pic::end_of_interrupt(1); }
}

#[no_mangle]
extern "C" fn bennu_timer_dispatch(frame: *mut TimerInterruptFrame) {
    if frame.is_null() {
        halt_forever();
    }

    let from_user = unsafe { (*frame).code_segment & 0x3 == 0x3 };
    unsafe {
        if crate::memory::paging::switch_address_space(
            crate::memory::paging::kernel_root(),
        ).is_err() {
            halt_forever();
        }
    }

    super::pit::on_interrupt();

    if from_user {
        if let Some(root) = crate::model::scheduler::current_cell()
            .and_then(crate::model::runtime::cell_address_space_root)
        {
            unsafe {
                if crate::memory::paging::switch_address_space(root).is_err() {
                    halt_forever();
                }
            }
        } else {
            halt_forever();
        }
    }
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

extern "x86-interrupt" fn error_code_handler(
    frame: InterruptStackFrame,
    error_code: u64,
) {
    let _ = frame;
    let _ = error_code;
    super::diagnostics::write_line(1, b"BENNU CPU EXCEPTION (ERR)");
    halt_forever();
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
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

pub fn timer_handler_address() -> u64 {
    bennu_timer_entry as usize as u64
}

pub fn init() {
    unsafe {
        for entry in &mut IDT {
            *entry = IdtEntry::EMPTY;
        }

        // Install a handler for the architecturally defined exception range.
        // Error-code exceptions use a different ABI and are installed separately.
        const ERROR_CODE_VECTORS: [usize; 7] = [8, 10, 11, 12, 13, 14, 17];

        for vector in 0..32 {
            if !ERROR_CODE_VECTORS.contains(&vector) {
                IDT[vector].set_address(exception_handler as usize as u64);
            }
        }

        IDT[3].set_address(breakpoint_handler as usize as u64);
        IDT[33].set_address(keyboard_handler as usize as u64);
        IDT[32].set_address(bennu_timer_entry as usize as u64);
        extern "C" {
            fn bennu_syscall_entry();
        }
        IDT[0x80].set_user_address(bennu_syscall_entry as usize as u64);
        IDT[8].set_address(
            double_fault_handler as DivergingHandlerWithError as usize as u64,
        );

        for vector in [10usize, 11, 12, 13, 17] {
            IDT[vector].set_address(error_code_handler as HandlerWithError as usize as u64);
        }

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
