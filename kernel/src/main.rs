#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kmain(boot_info: *const bennu_kernel::boot_info::BootInfo) -> ! {
    bennu_kernel::init(boot_info);

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
