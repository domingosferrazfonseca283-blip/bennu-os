use super::{CapabilityRights, ObjectId, ObjectKind};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Unknown = 0,
    UsbController = 1,
    UsbStorage = 2,
    Storage = 3,
    Display = 4,
    Input = 5,
    Network = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DeviceDescriptor {
    pub object: ObjectId,
    pub class: DeviceClass,
    pub vendor: u16,
    pub product: u16,
    pub capabilities: CapabilityRights,
}

impl DeviceDescriptor {
    pub const EMPTY: Self = Self {
        object: ObjectId::NULL,
        class: DeviceClass::Unknown,
        vendor: 0,
        product: 0,
        capabilities: CapabilityRights::NONE,
    };

    pub fn is_valid(&self) -> bool {
        !self.object.is_null()
    }
}

pub const fn is_device(kind: ObjectKind) -> bool {
    matches!(kind, ObjectKind::Device)
}
