use super::{CellId, ObjectId};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DriverState {
    Empty = 0,
    Discovered = 1,
    Bound = 2,
    Running = 3,
    Failed = 4,
    Stopped = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DriverBinding {
    pub device: ObjectId,
    pub driver_cell: CellId,
    pub state: DriverState,
    pub class: u8,
    pub vendor: u16,
    pub product: u16,
}

impl DriverBinding {
    pub const EMPTY: Self = Self {
        device: ObjectId::NULL,
        driver_cell: CellId(0),
        state: DriverState::Empty,
        class: 0,
        vendor: 0,
        product: 0,
    };
}
