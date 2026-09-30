use crate::model::abi::{self, Call, Operation, ResultCode};
use crate::model::{CapabilityId, ObjectId};

core::arch::global_asm!(r#"
.global bennu_syscall_entry
.type bennu_syscall_entry,@function
bennu_syscall_entry:
    push r11
    push r10
    push r9
    push r8
    push rdx
    push rsi
    push rdi
    push rax
    push rcx
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    sub rsp, 8

    lea rdi, [rsp + 8]
    call bennu_syscall_dispatch

    add rsp, 8
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    pop rcx
    pop rax
    pop rdi
    pop rsi
    pop rdx
    pop r8
    pop r9
    pop r10
    pop r11
    iretq
"#);

#[repr(C)]
struct RegisterFrame {
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rax: u64,
    rcx: u64,
    rbx: u64,
    rbp: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

#[no_mangle]
extern "C" fn bennu_syscall_dispatch(frame: *mut RegisterFrame) -> u64 {
    if frame.is_null() {
        return abi::ABI_STATUS_INVALID;
    }

    let regs = unsafe { &mut *frame };
    let cell = match crate::model::scheduler::current_cell() {
        Some(cell) => cell,
        None => return abi::ABI_STATUS_INVALID,
    };

    let operation = match regs.rax as u16 {
        0 => Operation::None,
        1 => Operation::ObjectCreate,
        2 => Operation::ObjectQuery,
        3 => Operation::CapabilityGrant,
        4 => Operation::EventWait,
        5 => Operation::EventEmit,
        6 => Operation::MemoryMap,
        7 => Operation::SurfaceCreate,
        8 => Operation::DeviceSubmit,
        _ => return abi::ABI_STATUS_UNSUPPORTED,
    };

    let call = Call {
        operation,
        flags: 0,
        capability: CapabilityId(regs.rdi),
        object: ObjectId(regs.rsi),
        argument: regs.rdx,
        value: regs.r10,
    };

    let result: ResultCode = abi::dispatch(cell, &call);
    regs.rax = result.status;
    regs.rdx = result.value;
    result.status
}
