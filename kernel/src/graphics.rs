use super::boot_info::{BootInfo, BENNU_FRAMEBUFFER_BYTES_PER_PIXEL};

pub struct Framebuffer {
    base: *mut u32,
    width: usize,
    height: usize,
    pitch_pixels: usize,
    red_mask: u8,
    red_shift: u8,
    green_mask: u8,
    green_shift: u8,
    blue_mask: u8,
    blue_shift: u8,
}

impl Framebuffer {
    fn validate(boot_info: &BootInfo) -> Result<(), &'static str> {
        if boot_info.framebuffer_addr == 0
            || boot_info.framebuffer_width == 0
            || boot_info.framebuffer_height == 0
            || boot_info.framebuffer_pitch == 0
            || boot_info.framebuffer_bpp != 32
        {
            return Err("no supported framebuffer");
        }

        let min_pitch = (boot_info.framebuffer_width as u64)
            .checked_mul(BENNU_FRAMEBUFFER_BYTES_PER_PIXEL as u64)
            .ok_or("framebuffer pitch overflow")?;
        if boot_info.framebuffer_pitch as u64 < min_pitch {
            return Err("framebuffer pitch is too small");
        }

        if boot_info.framebuffer_red_position >= 32
            || boot_info.framebuffer_green_position >= 32
            || boot_info.framebuffer_blue_position >= 32
            || (boot_info.framebuffer_red_position as u16 + boot_info.framebuffer_red_mask as u16) > 32
            || (boot_info.framebuffer_green_position as u16 + boot_info.framebuffer_green_mask as u16) > 32
            || (boot_info.framebuffer_blue_position as u16 + boot_info.framebuffer_blue_mask as u16) > 32
        {
            return Err("invalid framebuffer channel layout");
        }

        Ok(())
    }

    pub unsafe fn from_boot_info(boot_info: &BootInfo) -> Result<Self, &'static str> {
        Self::validate(boot_info)?;
        let bytes = (boot_info.framebuffer_pitch as u64)
            .checked_mul(boot_info.framebuffer_height as u64)
            .ok_or("framebuffer size overflow")?;
        let base = crate::memory::paging::map_mmio(boot_info.framebuffer_addr, bytes)? as *mut u32;

        Ok(Self {
            base,
            width: boot_info.framebuffer_width as usize,
            height: boot_info.framebuffer_height as usize,
            pitch_pixels: boot_info.framebuffer_pitch as usize / BENNU_FRAMEBUFFER_BYTES_PER_PIXEL as usize,
            red_mask: boot_info.framebuffer_red_mask,
            red_shift: boot_info.framebuffer_red_position,
            green_mask: boot_info.framebuffer_green_mask,
            green_shift: boot_info.framebuffer_green_position,
            blue_mask: boot_info.framebuffer_blue_mask,
            blue_shift: boot_info.framebuffer_blue_position,
        })
    }

    fn pixel(&self, red: u8, green: u8, blue: u8) -> u32 {
        let r = ((red as u32 * self.red_mask as u32 + 127) / 255) << self.red_shift;
        let g = ((green as u32 * self.green_mask as u32 + 127) / 255) << self.green_shift;
        let b = ((blue as u32 * self.blue_mask as u32 + 127) / 255) << self.blue_shift;
        r | g | b
    }

    pub unsafe fn clear(&self, pixel: u32) {
        for y in 0..self.height {
            let row = self.base.add(y * self.pitch_pixels);
            for x in 0..self.width {
                core::ptr::write_volatile(row.add(x), pixel);
            }
        }
    }

    pub unsafe fn fill_rect(&self, x: usize, y: usize, width: usize, height: usize, pixel: u32) {
        let x2 = core::cmp::min(x.saturating_add(width), self.width);
        let y2 = core::cmp::min(y.saturating_add(height), self.height);
        for yy in y..y2 {
            let row = self.base.add(yy * self.pitch_pixels);
            for xx in x..x2 {
                core::ptr::write_volatile(row.add(xx), pixel);
            }
        }
    }
}

fn glyph(byte: u8, row: usize) -> u8 {
    const FONT: [[u8; 8]; 16] = [
        [0x3c,0x66,0x6e,0x76,0x66,0x66,0x3c,0x00],[0x18,0x38,0x18,0x18,0x18,0x18,0x7e,0x00],
        [0x3c,0x66,0x06,0x0c,0x30,0x60,0x7e,0x00],[0x3c,0x66,0x06,0x1c,0x06,0x66,0x3c,0x00],
        [0x0c,0x1c,0x3c,0x6c,0x7e,0x0c,0x0c,0x00],[0x7e,0x60,0x7c,0x06,0x06,0x66,0x3c,0x00],
        [0x1c,0x30,0x60,0x7c,0x66,0x66,0x3c,0x00],[0x7e,0x06,0x0c,0x18,0x30,0x30,0x30,0x00],
        [0x3c,0x66,0x66,0x3c,0x66,0x66,0x3c,0x00],[0x3c,0x66,0x66,0x3e,0x06,0x0c,0x38,0x00],
        [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00],[0x18,0x3c,0x66,0x66,0x7e,0x66,0x66,0x00],
        [0x7c,0x66,0x66,0x7c,0x66,0x66,0x7c,0x00],[0x3c,0x66,0x60,0x60,0x60,0x66,0x3c,0x00],
        [0x78,0x6c,0x66,0x66,0x66,0x6c,0x78,0x00],[0x7e,0x60,0x60,0x7c,0x60,0x60,0x7e,0x00],
    ];
    let index = match byte { b'A'..=b'F' => (byte-b'A') as usize + 10, b'0'..=b'9' => (byte-b'0') as usize, b' ' => 10, _ => 10 };
    FONT[index][row]
}

impl Framebuffer {
    pub unsafe fn text(&self, x:usize, y:usize, text:&[u8], scale:usize, fg:u32) {
        let scale = core::cmp::max(scale, 1);
        let mut cursor = x;
        let mut baseline = y;
        for &byte in text {
            if byte == b'\n' {
                cursor = x;
                baseline = baseline.saturating_add(9 * scale);
                continue;
            }
            if cursor.saturating_add(8 * scale) > self.width {
                cursor = x;
                baseline = baseline.saturating_add(9 * scale);
            }
            if baseline.saturating_add(8 * scale) > self.height { break; }
            for row in 0..8 {
                let bits = glyph(byte, row);
                for col in 0..8 {
                    if bits & (0x80 >> col) != 0 {
                        self.fill_rect(cursor + col * scale, baseline + row * scale, scale, scale, fg);
                    }
                }
            }
            cursor = cursor.saturating_add(8 * scale);
        }
    }
}

pub fn init(boot_info: &BootInfo) -> Result<(), &'static str> {
    let framebuffer = unsafe { Framebuffer::from_boot_info(boot_info)? };
    let dark_a = framebuffer.pixel(16, 24, 40);
    let dark_b = framebuffer.pixel(32, 48, 72);
    let black = framebuffer.pixel(0, 0, 0);

    unsafe {
        for y in 0..framebuffer.height {
            let row = framebuffer.base.add(y * framebuffer.pitch_pixels);
            for x in 0..framebuffer.width {
                let band = ((x / 64) + (y / 64)) & 1;
                core::ptr::write_volatile(row.add(x), if band == 0 { dark_a } else { dark_b });
            }
        }

        let bar_height = core::cmp::min(72, framebuffer.height);
        framebuffer.fill_rect(0, 0, framebuffer.width, bar_height, black);

        let white = framebuffer.pixel(255, 255, 255);
        let accent = framebuffer.pixel(64, 160, 255);
        let text = framebuffer.pixel(160, 216, 255);
        framebuffer.text(24, 20, b"BENNU OS", 2, white);
        framebuffer.fill_rect(24, 112, core::cmp::min(360, framebuffer.width.saturating_sub(48)), 2, accent);
        framebuffer.text(24, 136, b"VIDEO ONLINE", 2, text);
    }

    Ok(())
}
