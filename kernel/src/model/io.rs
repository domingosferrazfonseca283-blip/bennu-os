use super::{BlockRequest, ObjectId};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IoState { Empty=0, Queued=1, Running=2, Completed=3, Failed=4 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IoCompletion {
    pub token: u64,
    pub device: ObjectId,
    pub state: IoState,
    pub status: u32,
    pub transferred: u64,
}

impl IoCompletion {
    pub const EMPTY: Self = Self {
        token: 0,
        device: ObjectId::NULL,
        state: IoState::Empty,
        status: 0,
        transferred: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IoEnvelope {
    pub request: BlockRequest,
    pub completion: IoCompletion,
}

impl IoEnvelope {
    pub const EMPTY: Self = Self {
        request: BlockRequest::EMPTY,
        completion: IoCompletion::EMPTY,
    };
}
