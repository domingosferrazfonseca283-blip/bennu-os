#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

use core::alloc::Layout;
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

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    bennu_kernel::arch::diagnostics::write_line(3, b"BENNU HEAP OUT OF MEMORY");

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
