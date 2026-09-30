use core::sync::atomic::{AtomicU64, Ordering};
use super::{io::outb, pic};

const CHANNEL0: u16 = 0x40;
const COMMAND: u16 = 0x43;
const INPUT_HZ: u32 = 1_193_182;
const FREQUENCY_HZ: u32 = 100;
static TICKS: AtomicU64 = AtomicU64::new(0);
static PENDING: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    let divisor = (INPUT_HZ / FREQUENCY_HZ) as u16;
    unsafe {
        outb(COMMAND, 0x36);
        outb(CHANNEL0, divisor as u8);
        outb(CHANNEL0, (divisor >> 8) as u8);
        pic::init();
    }
}

pub fn on_interrupt() {
    TICKS.fetch_add(1, Ordering::Relaxed);
    PENDING.fetch_add(1, Ordering::Release);
    unsafe { pic::end_of_interrupt(0); }
}

pub fn ticks() -> u64 { TICKS.load(Ordering::Acquire) }

pub fn take_tick() -> bool {
    PENDING.try_update(Ordering::Acquire, Ordering::Relaxed, |v| {
        if v == 0 { None } else { Some(v - 1) }
    }).is_ok()
}
