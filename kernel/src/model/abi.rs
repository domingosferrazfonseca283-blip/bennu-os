use super::{CapabilityId, ObjectId};

pub const BENNU_ABI_MAJOR: u16 = 1;
pub const BENNU_ABI_MINOR: u16 = 0;

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    None = 0,
    ObjectCreate = 1,
    ObjectQuery = 2,
    CapabilityGrant = 3,
    EventWait = 4,
    EventEmit = 5,
    MemoryMap = 6,
    SurfaceCreate = 7,
    DeviceSubmit = 8,
    Yield = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Call {
    pub operation: Operation,
    pub flags: u16,
    pub capability: CapabilityId,
    pub object: ObjectId,
    pub argument: u64,
    pub value: u64,
}

impl Call {
    pub const EMPTY: Self = Self {
        operation: Operation::None,
        flags: 0,
        capability: CapabilityId::NULL,
        object: ObjectId::NULL,
        argument: 0,
        value: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResultCode {
    pub status: u64,
    pub value: u64,
}

impl ResultCode {
    pub const OK: Self = Self { status: 0, value: 0 };
    pub const fn error(code: u64) -> Self { Self { status: code, value: 0 } }
}


pub const ABI_STATUS_OK: u64 = 0;
pub const ABI_STATUS_INVALID: u64 = 1;
pub const ABI_STATUS_DENIED: u64 = 2;
pub const ABI_STATUS_NOT_FOUND: u64 = 3;
pub const ABI_STATUS_UNSUPPORTED: u64 = 4;
pub const ABI_STATUS_YIELD: u64 = 5;

pub fn dispatch(cell: super::CellId, call: &Call) -> ResultCode {
    match call.operation {
        Operation::Yield => ResultCode::error(ABI_STATUS_YIELD),
        Operation::None => ResultCode::error(ABI_STATUS_INVALID),
        Operation::ObjectQuery => {
            if !super::runtime::permits(cell, call.capability, call.object, super::CapabilityRights::READ) {
                return ResultCode::error(ABI_STATUS_DENIED);
            }
            if super::runtime::object_exists(call.object) {
                ResultCode::OK
            } else {
                ResultCode::error(ABI_STATUS_NOT_FOUND)
            }
        }
        Operation::MemoryMap => {
            match super::runtime::memory_map(
                cell,
                call.capability,
                call.object,
                call.argument,
                (call.value & 1) != 0,
            ) {
                Ok(address) => ResultCode { status: ABI_STATUS_OK, value: address },
                Err(_) => ResultCode::error(ABI_STATUS_DENIED),
            }
        }
        Operation::EventEmit => {
            if !super::runtime::permits(
                cell,
                call.capability,
                call.object,
                super::CapabilityRights::OBSERVE,
            ) {
                return ResultCode::error(ABI_STATUS_DENIED);
            }
            match super::runtime::emit(super::Event::new(
                super::EventKind::ResourceChanged,
                call.object,
                super::ObjectId::NULL,
                call.argument,
            )) {
                Ok(()) => ResultCode::OK,
                Err(_) => ResultCode::error(ABI_STATUS_INVALID),
            }
        }
        _ => ResultCode::error(ABI_STATUS_UNSUPPORTED),
    }
}
