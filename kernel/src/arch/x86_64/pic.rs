use super::io::{inb, outb};

const MASTER_COMMAND: u16 = 0x20;
const MASTER_DATA: u16 = 0x21;
const SLAVE_COMMAND: u16 = 0xA0;
const SLAVE_DATA: u16 = 0xA1;
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;
const EOI: u8 = 0x20;

pub unsafe fn init() {
    let master_mask = inb(MASTER_DATA);
    let slave_mask = inb(SLAVE_DATA);
    outb(MASTER_COMMAND, ICW1_INIT | ICW1_ICW4);
    outb(SLAVE_COMMAND, ICW1_INIT | ICW1_ICW4);
    outb(MASTER_DATA, 0x20);
    outb(SLAVE_DATA, 0x28);
    outb(MASTER_DATA, 4);
    outb(SLAVE_DATA, 2);
    outb(MASTER_DATA, ICW4_8086);
    outb(SLAVE_DATA, ICW4_8086);
    // Keep timer (IRQ0) and keyboard (IRQ1) enabled; mask the remaining legacy IRQs.
    outb(MASTER_DATA, master_mask & !0x03);
    outb(SLAVE_DATA, slave_mask | 0xFF);
}

pub unsafe fn end_of_interrupt(irq: u8) {
    if irq >= 8 { outb(SLAVE_COMMAND, EOI); }
    outb(MASTER_COMMAND, EOI);
}
