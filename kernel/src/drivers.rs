#![allow(dead_code)]

pub mod keyboard {
    use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

    static LAST_SCANCODE: AtomicU8 = AtomicU8::new(0);
    static EVENTS: AtomicU32 = AtomicU32::new(0);

    pub fn init() { unsafe { crate::arch::x86_64::io::outb(0x64, 0xAE); } }
    pub fn feed_scancode(scancode: u8) { LAST_SCANCODE.store(scancode, Ordering::Release); EVENTS.fetch_add(1, Ordering::AcqRel); }
    pub fn last_scancode() -> u8 { LAST_SCANCODE.load(Ordering::Acquire) }
    pub fn pending() -> u32 { EVENTS.load(Ordering::Acquire) }
}

pub mod console {
    use core::sync::atomic::{AtomicU32, Ordering};
    static CURSOR: AtomicU32 = AtomicU32::new(0);
    pub fn init() { CURSOR.store(0, Ordering::Release); }
    pub fn cursor() -> u32 { CURSOR.load(Ordering::Acquire) }
    pub fn write(bytes: &[u8]) { for &b in bytes { if b == b'\n' { CURSOR.fetch_add(80 - (CURSOR.load(Ordering::Relaxed) % 80), Ordering::Relaxed); } else { CURSOR.fetch_add(1, Ordering::Relaxed); } } }
}
