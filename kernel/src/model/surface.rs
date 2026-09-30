use super::object::ObjectId;

pub const MAX_SURFACES: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Surface {
    pub object: ObjectId,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: u32,
    pub buffer: u64,
}

impl Surface {
    pub const EMPTY: Self = Self {
        object: ObjectId::NULL,
        width: 0,
        height: 0,
        stride: 0,
        format: 0,
        buffer: 0,
    };
}

static mut SURFACES: [Surface; MAX_SURFACES] = [Surface::EMPTY; MAX_SURFACES];

pub fn init() {
    unsafe { SURFACES = [Surface::EMPTY; MAX_SURFACES]; }
}

pub fn attach(object: ObjectId, width: u32, height: u32, buffer: u64) -> Result<usize, &'static str> {
    if object.is_null() || width == 0 || height == 0 {
        return Err("invalid surface");
    }
    unsafe {
        for (index, surface) in SURFACES.iter_mut().enumerate() {
            if surface.object.is_null() {
                *surface = Surface {
                    object,
                    width,
                    height,
                    stride: width,
                    format: 1,
                    buffer,
                };
                return Ok(index);
            }
        }
    }
    Err("surface table full")
}

pub fn get(index: usize) -> Option<Surface> {
    if index >= MAX_SURFACES { return None; }
    unsafe {
        let surface = SURFACES[index];
        if surface.object.is_null() { None } else { Some(surface) }
    }
}
