#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectId(pub u64);

impl ObjectId {
    pub const NULL: Self = Self(0);

    pub const fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << 32) | index as u64)
    }

    pub const fn index(self) -> usize {
        (self.0 & 0xffff_ffff) as usize
    }

    pub const fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Empty = 0,
    Memory = 1,
    Device = 2,
    Data = 3,
    Surface = 4,
    EventPort = 5,
    Cell = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResourceObject {
    pub id: ObjectId,
    pub kind: ObjectKind,
    pub owner: u32,
    pub flags: u32,
}

impl ResourceObject {
    pub const EMPTY: Self = Self {
        id: ObjectId::NULL,
        kind: ObjectKind::Empty,
        owner: 0,
        flags: 0,
    };
}
