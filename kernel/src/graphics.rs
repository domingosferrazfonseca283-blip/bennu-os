use super::boot_info::{BootInfo, BENNU_FRAMEBUFFER_BYTES_PER_PIXEL};

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
