use super::ObjectId;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Unknown = 0,
    Rgba8888 = 1,
    Bgra8888 = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Buffer {
    pub object: ObjectId,
    pub address: u64,
    pub size: u64,
    pub stride: u32,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
}

impl Buffer {
    pub const EMPTY: Self = Self {
        object: ObjectId::NULL,
        address: 0,
        size: 0,
        stride: 0,
        width: 0,
        height: 0,
        format: PixelFormat::Unknown,
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
        let min_stride = match (self.width as u64).checked_mul(bpp) {
            Some(value) => value,
            None => return None,
        };
        if self.width == 0
            || self.height == 0
            || (self.stride as u64) < min_stride
        {
            return None;
        }
        let row = match (self.stride as u64).checked_mul(self.height as u64) {
            Some(value) => value,
            None => return None,
        };
        row.checked_add(bpp)
    }

    pub const fn valid(&self) -> bool {
        if self.object.is_null() || self.address == 0 || self.width == 0 || self.height == 0 {
            return false;
        }
        match self.required_bytes() {
            Some(bytes) => bytes <= self.size,
            None => false,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Present {
    pub surface: ObjectId,
    pub buffer: ObjectId,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
