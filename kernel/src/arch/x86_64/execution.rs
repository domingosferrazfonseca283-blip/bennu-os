use core::ptr;

core::arch::global_asm!(r#"
.global bennu_context_switch
.type bennu_context_switch,@function
bennu_context_switch:
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    mov [rdi], rsp
    mov rsp, [rsi]
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
"#);

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

    pub const fn with_stack(stack_pointer: u64, instruction_pointer: u64) -> Self {
        Self { stack_pointer, instruction_pointer, flags: 0x202 }
    }
}

static mut SCHEDULER_CONTEXT: Context = Context::EMPTY;
static mut SCHEDULER_ROOT: u64 = 0;
static mut CURRENT_CELL_CONTEXT: *mut Context = ptr::null_mut();

/// Prepare a fresh kernel stack for the Bennu Cell context-switch ABI.
///
/// The assembly switcher restores six SysV callee-saved registers and then
/// executes ret. The prepared stack therefore contains those six slots plus
/// the first return address (the Cell trampoline).
pub unsafe fn prepare_context(
    context: &mut Context,
    stack_frame: u64,
    trampoline: u64,
) -> Result<(), &'static str> {
    if stack_frame == 0 || stack_frame & 0xfff != 0 {
        return Err("cell stack is not page aligned");
    }
    if trampoline == 0 {
        return Err("cell trampoline is null");
    }

    let stack_top = stack_frame.checked_add(4096).ok_or("cell stack overflow")?;
    let sp = (stack_top.saturating_sub(7 * core::mem::size_of::<u64>() as u64)) & !0xf;
    let slots = sp as *mut u64;

    for index in 0..6 {
        ptr::write_volatile(slots.add(index), 0);
    }
    ptr::write_volatile(slots.add(6), trampoline);

    *context = Context::new(sp, trampoline);
    Ok(())
}

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

extern "C" {
    fn bennu_context_switch(old_stack: *mut u64, new_stack: *const u64);
}

/// Switches the kernel execution stack between two prepared Cell contexts.
#[inline]
pub unsafe fn switch_stack(old: &mut Context, new: &Context) {
    bennu_context_switch(&mut old.stack_pointer, &new.stack_pointer);
}

/// Enter a Cell and return here when the Cell yields.
pub unsafe fn switch_to_cell(context: *mut Context, address_space_root: u64) -> Result<(), &'static str> {
    if context.is_null() || !(*context).is_initialized() {
        return Err("cell context is not initialized");
    }
    if address_space_root == 0 || address_space_root & 0xfff != 0 {
        return Err("cell address-space root is invalid");
    }

    if SCHEDULER_ROOT == 0 {
        SCHEDULER_ROOT = crate::memory::paging::current_root();
    }

    CURRENT_CELL_CONTEXT = context;
    crate::memory::paging::switch_address_space(address_space_root)?;
    switch_stack(&mut SCHEDULER_CONTEXT, &*context);
    Ok(())
}

/// Return from a Cell to the scheduler stack.
pub unsafe fn switch_back_to_scheduler() -> Result<(), &'static str> {
    if CURRENT_CELL_CONTEXT.is_null() {
        return Err("no current cell context");
    }
    if SCHEDULER_ROOT == 0 {
        return Err("scheduler address-space root is not initialized");
    }
    crate::memory::paging::switch_address_space(SCHEDULER_ROOT)?;
    switch_stack(&mut *CURRENT_CELL_CONTEXT, &SCHEDULER_CONTEXT);
    Ok(())
}
