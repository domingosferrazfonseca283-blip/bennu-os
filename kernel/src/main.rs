#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    unsafe {
        let vga = 0xb8000 as *mut u8;
        let msg = b"BENNU OS KERNEL OK";
        for (i, byte) in msg.iter().enumerate() {
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
