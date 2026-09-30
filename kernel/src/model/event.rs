use super::object::ObjectId;

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    None = 0,
    ResourceCreated = 1,
    ResourceChanged = 2,
    ResourceRemoved = 3,
    Input = 4,
    DeviceAttached = 5,
    DeviceDetached = 6,
    Wake = 7,
    DeviceQueued = 8,
    DeviceCompleted = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Event {
    pub kind: EventKind,
    pub source: ObjectId,
    pub target: ObjectId,
    pub value: u64,
}

impl Event {
    pub const EMPTY: Self = Self {
        kind: EventKind::None,
        source: ObjectId::NULL,
        target: ObjectId::NULL,
        value: 0,
    };

    pub const fn new(kind: EventKind, source: ObjectId, target: ObjectId, value: u64) -> Self {
        Self { kind, source, target, value }
    }
}
