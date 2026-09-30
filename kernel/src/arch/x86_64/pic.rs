//! 8259A Programmable Interrupt Controller support.
//!
//! The legacy PIC is intentionally kept as the first interrupt-controller
//! implementation. It gives Bennu deterministic IRQ delivery before APIC/IOAPIC
//! support is introduced.

use super::io::{io_wait, inb, outb};

const MASTER_COMMAND: u16 = 0x20;
const MASTER_DATA: u16 = 0x21;
const SLAVE_COMMAND: u16 = 0xA0;
const SLAVE_DATA: u16 = 0xA1;

const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;
const EOI: u8 = 0x20;

pub const MASTER_VECTOR_OFFSET: u8 = 32;
pub const SLAVE_VECTOR_OFFSET: u8 = 40;

pub fn remap() {
    unsafe {
        let master_mask = inb(MASTER_DATA);
        let slave_mask = inb(SLAVE_DATA);

        outb(MASTER_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        outb(SLAVE_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();

        outb(MASTER_DATA, MASTER_VECTOR_OFFSET);
        io_wait();
        outb(SLAVE_DATA, SLAVE_VECTOR_OFFSET);
        io_wait();

        outb(MASTER_DATA, 4);
        io_wait();
        outb(SLAVE_DATA, 2);
        io_wait();

        outb(MASTER_DATA, ICW4_8086);
        io_wait();
        outb(SLAVE_DATA, ICW4_8086);
        io_wait();

        // Restore masks. PIT IRQ0 is enabled by unmask_irq(0).
        outb(MASTER_DATA, master_mask);
        outb(SLAVE_DATA, slave_mask);
    }
}

pub fn mask_all() {
    unsafe {
        outb(MASTER_DATA, 0xFF);
        outb(SLAVE_DATA, 0xFF);
    }
}

pub fn unmask_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            let mask = inb(MASTER_DATA) & !(1 << irq);
            outb(MASTER_DATA, mask);
        } else {
            let mask = inb(SLAVE_DATA) & !(1 << (irq - 8));
            outb(SLAVE_DATA, mask);

            // The slave is connected to IRQ2 on the master.
            let master_mask = inb(MASTER_DATA) & !(1 << 2);
            outb(MASTER_DATA, master_mask);
        }
    }
}

pub fn end_of_interrupt(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(SLAVE_COMMAND, EOI);
        }
        outb(MASTER_COMMAND, EOI);
    }
}
