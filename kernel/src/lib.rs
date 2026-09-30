#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod boot_info;
pub mod memory;

pub fn init(boot_info: *const boot_info::BootInfo) {
    arch::init();

    if boot_info.is_null() {
        arch::diagnostics::write_line(4, b"BENNU BOOT: NULL BOOT INFO");
        return;
    }

    let boot_info = unsafe { &*boot_info };

    if let Err(message) = memory::init(boot_info) {
        let _ = message;
        arch::diagnostics::write_line(4, b"BENNU MEMORY: INITIALIZATION FAILED");
        return;
    }

    arch::diagnostics::write_line(4, b"BENNU MEMORY: E820 + FRAME ALLOCATOR ONLINE");

    if let Err(message) = memory::paging::init() {
        let _ = message;
        arch::diagnostics::write_line(5, b"BENNU MEMORY: PAGING INITIALIZATION FAILED");
        return;
    }

    arch::diagnostics::write_line(5, b"BENNU MEMORY: KERNEL PAGING ONLINE");

    unsafe {
        memory::heap::init();
    }

    arch::diagnostics::write_line(6, b"BENNU MEMORY: KERNEL HEAP ONLINE");
}
