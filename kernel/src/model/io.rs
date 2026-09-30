use super::{BlockRequest, IoState, ObjectId};

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IoStatus { Ok=0, Invalid=1, Denied=2, NoDevice=3, QueueFull=4, Hardware=5 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IoCompletion {
    pub token:u64, pub device:ObjectId, pub state:IoState, pub status:u32, pub transferred:u64,
}
impl IoCompletion {
    pub const EMPTY: Self = Self { token:0, device:ObjectId::NULL, state:IoState::Empty, status:0, transferred:0 };
    pub const fn success(request:BlockRequest, transferred:u64) -> Self {
        Self { token:request.token, device:request.device, state:IoState::Completed, status:IoStatus::Ok as u32, transferred }
    }
    pub const fn failed(request:BlockRequest, status:IoStatus) -> Self {
        Self { token:request.token, device:request.device, state:IoState::Failed, status:status as u32, transferred:0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IoEnvelope { pub request:BlockRequest, pub completion:IoCompletion }
impl IoEnvelope {
    pub const EMPTY: Self = Self { request:BlockRequest::EMPTY, completion:IoCompletion::EMPTY };
}
