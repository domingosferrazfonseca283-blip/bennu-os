#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    const MESSAGE: &[u8] = b"BENNU OS KERNEL OK";

    unsafe {
        let vga = 0xb8000 as *mut u8;

        for (i, byte) in MESSAGE.iter().enumerate() {
            vga.add(i * 2).write_volatile(*byte);
            vga.add(i * 2 + 1).write_volatile(0x0f);
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
