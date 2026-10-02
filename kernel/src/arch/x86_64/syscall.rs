use crate::model::abi::{self, Call, Operation, ResultCode};
use crate::model::{CapabilityId, ObjectId};
use crate::model::graphics::Present;

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
    lea rdi, [rsp + 120]
    call bennu_syscall_dispatch
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
extern "C" fn bennu_syscall_yield() {
    if let Some(cell) = crate::model::scheduler::current_cell() {
        let _ = crate::model::runtime::finish_cell(cell, crate::model::CellAction::Yield);
    }
    unsafe {
        if crate::arch::x86_64::execution::switch_back_to_scheduler().is_err() {
            loop { core::hint::spin_loop(); }
        }
    }
}

#[inline]
fn is_canonical_user_address(address: u64) -> bool {
    address < 0x0000_8000_0000_0000
}

#[inline]
fn valid_user_range(base: u64, length: u64) -> bool {
    if length == 0 || !is_canonical_user_address(base) { return false; }
    match base.checked_add(length - 1) {
        Some(end) => is_canonical_user_address(end),
        None => false,
    }
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
        9 => Operation::Yield,
        10 => Operation::Present,
        _ => return abi::ABI_STATUS_UNSUPPORTED,
    };

    if !valid_user_range(regs.rdx, if regs.r10 == 0 { 1 } else { regs.r10 }) && matches!(operation, Operation::MemoryMap | Operation::SurfaceCreate | Operation::DeviceSubmit) {
        regs.rax = abi::ABI_STATUS_INVALID;
        return abi::ABI_STATUS_INVALID;
    }

    if matches!(operation, Operation::Present) {
        let root = match crate::model::runtime::cell_address_space_root(cell) {
            Some(root) if root != 0 => root,
            _ => {
                regs.rax = abi::ABI_STATUS_INVALID;
                return abi::ABI_STATUS_INVALID;
            }
        };
        let length = core::mem::size_of::<Present>() as u64;
        if !crate::memory::paging::validate_user_buffer(root, regs.rdx, length, false) {
            regs.rax = abi::ABI_STATUS_INVALID;
            return abi::ABI_STATUS_INVALID;
        }

        let mut bytes = [0u8; core::mem::size_of::<Present>()];
        let mut offset = 0usize;
        while offset < bytes.len() {
            let address = match regs.rdx.checked_add(offset as u64) {
                Some(value) => value,
                None => {
                    regs.rax = abi::ABI_STATUS_INVALID;
                    return abi::ABI_STATUS_INVALID;
                }
            };
            let physical = match crate::memory::paging::translate_user_address(root, address, false) {
                Some(value) => value,
                None => {
                    regs.rax = abi::ABI_STATUS_INVALID;
                    return abi::ABI_STATUS_INVALID;
                }
            };
            bytes[offset] = unsafe { core::ptr::read_volatile(physical as *const u8) };
            offset += 1;
        }

        let present = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Present) };
        let result = abi::dispatch_present(cell, CapabilityId(regs.rdi), &present);
        regs.rax = result.status;
        regs.rdx = result.value;
        return result.status;
    }

    if matches!(operation, Operation::DeviceSubmit) {
        let root = match crate::model::runtime::cell_address_space_root(cell) {
            Some(root) if root != 0 => root,
            _ => {
                regs.rax = abi::ABI_STATUS_INVALID;
                return abi::ABI_STATUS_INVALID;
            }
        };
        if !crate::memory::paging::validate_user_buffer(root, regs.rdx, regs.r10, true) {
            regs.rax = abi::ABI_STATUS_INVALID;
            return abi::ABI_STATUS_INVALID;
        }
    }

    let call = Call {
        operation,
        flags: if matches!(operation, Operation::DeviceSubmit) { regs.r11 as u16 } else { 0 },
        capability: CapabilityId(regs.rdi),
        object: ObjectId(regs.rsi),
        argument: regs.rdx,
        value: if matches!(operation, Operation::DeviceSubmit) { regs.r8 } else { regs.r10 },
        length: if matches!(operation, Operation::DeviceSubmit) { regs.r10 } else { 0 },
    };

    let result: ResultCode = abi::dispatch(cell, &call);
    regs.rax = result.status;
    regs.rdx = result.value;
    result.status
}


pub fn entry_address() -> u64 {
    bennu_syscall_entry as usize as u64
}