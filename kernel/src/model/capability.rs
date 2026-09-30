use super::object::ObjectId;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapabilityId(pub u64);

impl CapabilityId {
    pub const NULL: Self = Self(0);

    pub const fn new(_object: ObjectId, slot: u32) -> Self {
        Self((1u64 << 32) | slot as u64)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRights(pub u32);

impl CapabilityRights {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const OBSERVE: Self = Self(1 << 3);
    pub const DRAW: Self = Self(1 << 4);
    pub const SHARE: Self = Self(1 << 5);

    pub const fn contains(self, required: Self) -> bool {
        (self.0 & required.0) == required.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capability {
    pub id: CapabilityId,
    pub object: ObjectId,
    pub rights: CapabilityRights,
    pub owner_cell: u32,
    pub generation: u32,
}

impl Capability {
    pub const EMPTY: Self = Self {
        id: CapabilityId::NULL,
        object: ObjectId::NULL,
        rights: CapabilityRights::NONE,
        owner_cell: 0,
        generation: 0,
    };

    pub const fn permits(&self, cell: u32, object: ObjectId, rights: CapabilityRights) -> bool {
        self.owner_cell == cell
            && self.object.0 == object.0
            && self.rights.contains(rights)
            && !object.is_null()
    }
}
