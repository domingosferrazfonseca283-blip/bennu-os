//! Minimal pre-userspace diagnostics.
//!
//! This deliberately writes directly to the legacy VGA text buffer. It is a
//! temporary hardware diagnostic channel, not the future Bennu console API.

const VGA: *mut u8 = 0xb8000 as *mut u8;

pub fn write_line(row: usize, message: &[u8]) {
    let offset = row * 80 * 2;

    unsafe {
        for column in 0..80 {
            VGA.add(offset + column * 2).write_volatile(b' ');
            VGA.add(offset + column * 2 + 1).write_volatile(0x07);
        }

        for (column, byte) in message.iter().copied().take(80).enumerate() {
            VGA.add(offset + column * 2).write_volatile(byte);
            VGA.add(offset + column * 2 + 1).write_volatile(0x0f);
        }
    }
}
