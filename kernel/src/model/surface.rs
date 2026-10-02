use super::graphics::{self, PixelFormat};
use super::object::ObjectId;
use super::sync::SpinLock;

pub const MAX_SURFACES: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Surface {
    pub object: ObjectId,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub buffer: ObjectId,
}

impl Surface {
    pub const EMPTY: Self = Self {
        object: ObjectId::NULL,
        width: 0,
        height: 0,
        stride: 0,
        format: PixelFormat::Unknown,
        buffer: ObjectId::NULL,
    };

    pub const fn bytes_per_pixel(&self) -> Option<u32> {
        match self.format {
            PixelFormat::Rgba8888 | PixelFormat::Bgra8888 => Some(4),
            PixelFormat::Unknown => None,
        }
    }

    pub const fn required_bytes(&self) -> Option<u64> {
        let bpp = match self.bytes_per_pixel() {
            Some(value) => value as u64,
            None => return None,
        };
        let minimum_stride = match (self.width as u64).checked_mul(bpp) {
            Some(value) => value,
            None => return None,
        };
        if self.width == 0 || self.height == 0 || (self.stride as u64) < minimum_stride {
            return None;
        }
        (self.stride as u64).checked_mul(self.height as u64)
    }

    pub const fn valid(&self) -> bool {
        if self.object.is_null() || self.buffer.is_null() {
            return false;
        }
        self.required_bytes().is_some()
    }
}

static SURFACES: SpinLock<[Surface; MAX_SURFACES]> =
    SpinLock::new([Surface::EMPTY; MAX_SURFACES]);

pub fn init() {
    *SURFACES.lock().get_mut() = [Surface::EMPTY; MAX_SURFACES];
}

pub fn attach(
    object: ObjectId,
    width: u32,
    height: u32,
    buffer: ObjectId,
) -> Result<usize, &'static str> {
    if object.is_null() || width == 0 || height == 0 || buffer.is_null() {
        return Err("invalid surface");
    }

    let backing = graphics::get(buffer).ok_or("graphics buffer is not registered")?;
    if backing.width < width || backing.height < height {
        return Err("graphics buffer is smaller than surface");
    }

    let stride = backing.stride;
    let format = backing.format;
    let value = Surface {
        object,
        width,
        height,
        stride,
        format,
        buffer,
    };

    if !value.valid() {
        return Err("invalid surface buffer");
    }

    let mut guard = SURFACES.lock();
    for (index, slot) in guard.get_mut().iter_mut().enumerate() {
        if slot.object.is_null() {
            *slot = value;
            return Ok(index);
        }
    }

    Err("surface table full")
}

pub fn get(index: usize) -> Option<Surface> {
    if index >= MAX_SURFACES {
        return None;
    }

    let guard = SURFACES.lock();
    let value = guard.get()[index];
    if value.valid() {
        Some(value)
    } else {
        None
    }
}
