pub mod diagnostics;
pub mod gdt;
pub mod idt;
pub mod pit;
pub mod pic;
pub mod io;

pub fn init() {
    gdt::init();
    idt::init();
    pit::init();

    unsafe { core::arch::asm!("sti", options(nostack, preserves_flags)); }

    diagnostics::write_line(0, b"BENNU KERNEL: GDT + IDT + TIMER ONLINE");
}
