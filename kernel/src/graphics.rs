use super::boot_info::{BootInfo, BENNU_FRAMEBUFFER_BYTES_PER_PIXEL};

pub struct Framebuffer {
    base: *mut u32,
    width: usize,
    height: usize,
    pitch_pixels: usize,
}

impl Framebuffer {
    pub unsafe fn from_boot_info(boot_info: &BootInfo) -> Result<Self, &'static str> {
        if boot_info.framebuffer_addr == 0 || boot_info.framebuffer_width == 0 || boot_info.framebuffer_height == 0 || boot_info.framebuffer_pitch == 0 || boot_info.framebuffer_bpp != 32 { return Err("no supported framebuffer"); }
        let bytes = (boot_info.framebuffer_pitch as u64).checked_mul(boot_info.framebuffer_height as u64).ok_or("framebuffer size overflow")?;
        let base = crate::memory::paging::map_mmio(boot_info.framebuffer_addr, bytes)? as *mut u32;
        Ok(Self { base, width: boot_info.framebuffer_width as usize, height: boot_info.framebuffer_height as usize, pitch_pixels: boot_info.framebuffer_pitch as usize / 4 })
    }

    pub unsafe fn clear(&self, pixel: u32) { for y in 0..self.height { let row=self.base.add(y*self.pitch_pixels); for x in 0..self.width { core::ptr::write_volatile(row.add(x),pixel); } } }
    pub unsafe fn fill_rect(&self, x:usize, y:usize, width:usize, height:usize, pixel:u32) { let x2=core::cmp::min(x.saturating_add(width),self.width); let y2=core::cmp::min(y.saturating_add(height),self.height); for yy in y..y2 { let row=self.base.add(yy*self.pitch_pixels); for xx in x..x2 { core::ptr::write_volatile(row.add(xx),pixel); } } }
}

pub fn init(boot_info: &BootInfo) -> Result<(), &'static str> {
    if boot_info.framebuffer_addr == 0
        || boot_info.framebuffer_width == 0
        || boot_info.framebuffer_height == 0
        || boot_info.framebuffer_pitch == 0
        || boot_info.framebuffer_bpp != 32
    {
        return Err("no supported framebuffer");
    }

    let bytes = (boot_info.framebuffer_pitch as u64)
        .checked_mul(boot_info.framebuffer_height as u64)
        .ok_or("framebuffer size overflow")?;
    let framebuffer = crate::memory::paging::map_mmio(boot_info.framebuffer_addr, bytes)? as *mut u32;
    let pitch_pixels = boot_info.framebuffer_pitch as usize / BENNU_FRAMEBUFFER_BYTES_PER_PIXEL as usize;
    let width = boot_info.framebuffer_width as usize;
    let height = boot_info.framebuffer_height as usize;

    unsafe {
        for y in 0..height {
            let row = framebuffer.add(y * pitch_pixels);
            for x in 0..width {
                let band = ((x / 64) + (y / 64)) & 1;
                let pixel = if band == 0 { 0x00101828 } else { 0x00203048 };
                core::ptr::write_volatile(row.add(x), pixel);
            }
        }

        let bar_height = core::cmp::min(72, height);
        for y in 0..bar_height {
            let row = framebuffer.add(y * pitch_pixels);
            for x in 0..width {
                core::ptr::write_volatile(row.add(x), 0x00000000);
            }
        }
    }

    Ok(())
}
