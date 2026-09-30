#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    bennu_kernel::init();

    bennu_kernel::arch::diagnostics::write_line(
        2,
        b"BENNU OS KERNEL OK",
    );

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    bennu_kernel::arch::diagnostics::write_line(3, b"BENNU KERNEL PANIC");

    loop {
        core::hint::spin_loop();
    }
}
