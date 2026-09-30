pub mod diagnostics;
pub mod gdt;
pub mod idt;

pub fn init() {
    gdt::init();
    idt::init();

    diagnostics::write_line(0, b"BENNU KERNEL: GDT + IDT ONLINE");
}
