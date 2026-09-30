//! Programmable Interval Timer (8253/8254) driver.

use core::sync::atomic::{AtomicU64, Ordering};

use super::io::outb;
use super::pic;

const CHANNEL0_DATA: u16 = 0x40;
const COMMAND: u16 = 0x43;
const BASE_FREQUENCY_HZ: u32 = 1_193_182;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// Configure PIT channel 0 for a periodic kernel tick.
pub fn init(frequency_hz: u32) {
    let frequency_hz = frequency_hz.clamp(19, BASE_FREQUENCY_HZ);
    let divisor = (BASE_FREQUENCY_HZ / frequency_hz).clamp(1, u16::MAX as u32) as u16;

    unsafe {
        // Channel 0, lobyte/hibyte access, mode 3 square wave, binary.
        outb(COMMAND, 0x36);
        outb(CHANNEL0_DATA, (divisor & 0xFF) as u8);
        outb(CHANNEL0_DATA, (divisor >> 8) as u8);
    }

    pic::unmask_irq(0);
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn on_interrupt() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    pic::end_of_interrupt(0);
}
