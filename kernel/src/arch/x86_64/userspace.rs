//! First real Ring-3 execution boundary for Bennu Cells.

use crate::memory::{allocate_frame_below, paging};

pub const USER_CODE: u64 = 0x0000_0000_4000_0000;
pub const USER_STACK: u64 = 0x0000_0000_8000_0000;
pub const USER_STACK_TOP: u64 = USER_STACK + paging::PAGE_SIZE;

extern "C" {
    fn bennu_enter_user(rip: u64, rsp: u64) -> !;
}

core::arch::global_asm!(r#"
.global bennu_enter_user
.type bennu_enter_user,@function
bennu_enter_user:
    mov ax, 0x23
    mov ds, ax
    mov es, ax
    push 0x23
    push rsi
    pushfq
    push 0x2b
    push rdi
    iretq
"#);

pub fn install(cell: crate::model::CellId, root: u64) -> Result<(), &'static str> {
    let code_frame = allocate_frame_below(64 * 1024 * 1024).ok_or("no frame for user code")?;
    let stack_frame = allocate_frame_below(64 * 1024 * 1024).ok_or("no frame for user stack")?;

    unsafe {
        let code = code_frame as *mut u8;
        core::ptr::write_bytes(code, 0x90, paging::PAGE_SIZE as usize);
        core::ptr::write(code.add(0), 0xB8);
        core::ptr::write(code.add(1) as *mut u32, 9u32);
        core::ptr::write(code.add(5), 0xCD);
        core::ptr::write(code.add(6), 0x80);
        core::ptr::write(code.add(7), 0xEB);
        core::ptr::write(code.add(8), 0xF7u8);
        core::ptr::write_bytes(stack_frame as *mut u8, 0, paging::PAGE_SIZE as usize);
    }

    paging::map_user_page_in_root(root, USER_CODE, code_frame, false)?;
    paging::map_user_page_in_root(root, USER_STACK, stack_frame, true)?;
    crate::model::runtime::configure_user_entry(cell, USER_CODE, USER_STACK_TOP)
}

pub unsafe fn enter(rip: u64, rsp: u64) -> ! {
    bennu_enter_user(rip, rsp)
}