use crate::model::abi::{self, Call, Operation, ResultCode};
use crate::model::{CapabilityId, ObjectId};

extern "C" { fn bennu_syscall_entry(); }

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
    mov r15, cr3
    push r15
    mov r15, [rip + bennu_kernel_root]
    mov cr3, r15

    // Stack layout: CR3, r15..r11. RegisterFrame starts at the saved r11.
    lea rdi, [rsp + 120]\n    call bennu_syscall_dispatch
    cmp rax, 5
    jne 1f
    call bennu_syscall_yield
1:
    mov r15, [rsp]
    mov cr3, r15
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