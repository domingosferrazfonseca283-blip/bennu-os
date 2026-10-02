use super::sync::SpinLock;
use super::{CellId, ObjectId};

pub const MAX_BUFFERS: usize = 32;

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
    pub owner: CellId,
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
        owner: CellId(0),
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
        if self.width == 0 || self.height == 0 || (self.stride as u64) < min_stride {
            return None;
        }
        let last_row = match (self.height as u64 - 1).checked_mul(self.stride as u64) {
            Some(value) => value,
            None => return None,
        };
        let last_pixel = match (self.width as u64).checked_mul(bpp) {
            Some(value) => value,
            None => return None,
        };
        last_row.checked_add(last_pixel)
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

static BUFFERS: SpinLock<[Buffer; MAX_BUFFERS]> =
    SpinLock::new([Buffer::EMPTY; MAX_BUFFERS]);

pub fn init() {
    *BUFFERS.lock().get_mut() = [Buffer::EMPTY; MAX_BUFFERS];
}

pub fn register(buffer: Buffer) -> Result<(), &'static str> {
    if !buffer.valid() {
        return Err("invalid graphics buffer");
    }

    let mut guard = BUFFERS.lock();
    for slot in guard.get_mut().iter_mut() {
        if slot.object == buffer.object {
            *slot = buffer;
            return Ok(());
        }
    }
    for slot in guard.get_mut().iter_mut() {
        if slot.object.is_null() {
            *slot = buffer;
            return Ok(());
        }
    }
    Err("graphics buffer table full")
}

pub fn get(object: ObjectId) -> Option<Buffer> {
    if object.is_null() {
        return None;
    }
    let guard = BUFFERS.lock();
    for slot in guard.get().iter() {
        if slot.object == object && slot.valid() {
            return Some(*slot);
        }
    }
    None
}

pub fn get_for_owner(object: ObjectId, owner: CellId) -> Option<Buffer> {
    let value = get(object)?;
    if value.owner == owner {
        Some(value)
    } else {
        None
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
