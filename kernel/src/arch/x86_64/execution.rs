#[repr(C)]
#[derive(Clone, Copy)]
pub struct Context {
    pub stack_pointer: u64,
    pub instruction_pointer: u64,
    pub flags: u64,
}

impl Context {
    pub const EMPTY: Self = Self { stack_pointer: 0, instruction_pointer: 0, flags: 0x202 };
    pub const fn new(stack_pointer: u64, instruction_pointer: u64) -> Self {
        Self { stack_pointer, instruction_pointer, flags: 0x202 }
    }
    pub const fn is_initialized(&self) -> bool {
        self.stack_pointer != 0 && self.instruction_pointer != 0
    }
}

/// Diagnostic CPU snapshot. It is not yet a resumable Cell context.
#[inline]
pub fn capture_current() -> Context {
    let stack_pointer: u64;
    let instruction_pointer: u64;
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "mov {sp}, rsp",
            "lea {ip}, [rip]",
            "pushfq",
            "pop {flags}",
            sp = out(reg) stack_pointer,
            ip = out(reg) instruction_pointer,
            flags = out(reg) flags,
            options(preserves_flags),
        );
    }
    Context { stack_pointer, instruction_pointer, flags }
}

#[inline]
pub fn interrupts_enabled() -> bool {
    capture_current().flags & (1 << 9) != 0
}
