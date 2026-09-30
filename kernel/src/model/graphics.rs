use super::ObjectId;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat { Unknown=0, Rgba8888=1, Bgra8888=2 }

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
        object: ObjectId::NULL, address: 0, size: 0, stride: 0,
        width: 0, height: 0, format: PixelFormat::Unknown,
    };
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
