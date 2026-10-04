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
    Present = 10,
    AuthorizationQuery = 11,
    AuthorizationResolve = 12,
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
    pub length: u64,
}

impl Call {
    pub const EMPTY: Self = Self {
        operation: Operation::None,
        flags: 0,
        capability: CapabilityId::NULL,
        object: ObjectId::NULL,
        argument: 0,
        value: 0,
        length: 0,
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
    use super::CapabilityRights as R;

    match call.operation {
        Operation::Yield => ResultCode::error(ABI_STATUS_YIELD),
        Operation::Present => ResultCode::error(ABI_STATUS_INVALID),
        Operation::AuthorizationQuery => {
            match super::intelligence::authorization_request(call.argument) {
                Some(request) => ResultCode { status: ABI_STATUS_OK, value: request.task_id },
                None => ResultCode::error(ABI_STATUS_NOT_FOUND),
            }
        }
        Operation::AuthorizationResolve => {
            if cell.0 != 1 { return ResultCode::error(ABI_STATUS_DENIED); }
            let granted = (call.value & 1) != 0;
            let result = if granted {
                super::intelligence::authorize_task(call.argument)
            } else {
                super::intelligence::deny_task(call.argument)
            };
            match result { Ok(()) => ResultCode::OK, Err(_) => ResultCode::error(ABI_STATUS_DENIED) }
        }
        Operation::None => ResultCode::error(ABI_STATUS_INVALID),

        Operation::ObjectQuery => {
            if !super::runtime::permits(cell, call.capability, call.object, R::READ) {
                return ResultCode::error(ABI_STATUS_DENIED);
            }
            if super::runtime::object_exists(call.object) {
                ResultCode::OK
            } else {
                ResultCode::error(ABI_STATUS_NOT_FOUND)
            }
        }

        Operation::ObjectCreate => {
            if !super::runtime::permits(cell, call.capability, call.object, R::ADMIN) {
                return ResultCode::error(ABI_STATUS_DENIED);
            }
            let kind = match call.value as u8 {
                1 => super::ObjectKind::Memory,
                2 => super::ObjectKind::Device,
                3 => super::ObjectKind::Data,
                4 => super::ObjectKind::Surface,
                5 => super::ObjectKind::EventPort,
                6 => super::ObjectKind::Cell,
                _ => return ResultCode::error(ABI_STATUS_INVALID),
            };
            match super::runtime::create_object(kind, cell.0) {
                Ok(id) => ResultCode { status: ABI_STATUS_OK, value: id.0 },
                Err(_) => ResultCode::error(ABI_STATUS_INVALID),
            }
        }

        Operation::CapabilityGrant => {
            let target = super::CellId(call.argument as u32);
            let rights = R(call.value as u32);
            match super::runtime::delegate(cell, call.capability, target, call.object, rights) {
                Ok(id) => ResultCode { status: ABI_STATUS_OK, value: id.0 },
                Err(_) => ResultCode::error(ABI_STATUS_DENIED),
            }
        }

        Operation::EventWait => {
            if !super::runtime::permits(cell, call.capability, call.object, R::OBSERVE) {
                return ResultCode::error(ABI_STATUS_DENIED);
            }
            match super::runtime::poll_for_object(call.object) {
                Some(event) => ResultCode { status: ABI_STATUS_OK, value: event.value },
                None => ResultCode::error(ABI_STATUS_NOT_FOUND),
            }
        }

        Operation::MemoryMap => {
            match super::runtime::memory_map(
                cell, call.capability, call.object, call.argument, (call.value & 1) != 0,
            ) {
                Ok(address) => ResultCode { status: ABI_STATUS_OK, value: address },
                Err(_) => ResultCode::error(ABI_STATUS_DENIED),
            }
        }

        Operation::SurfaceCreate => {
            if !super::runtime::permits(cell, call.capability, call.object, R::DRAW) {
                return ResultCode::error(ABI_STATUS_DENIED);
            }
            match super::runtime::create_object(super::ObjectKind::Surface, cell.0) {
                Ok(id) => ResultCode { status: ABI_STATUS_OK, value: id.0 },
                Err(_) => ResultCode::error(ABI_STATUS_INVALID),
            }
        }

        Operation::DeviceSubmit => {
            let opcode = (call.flags as u32) & 0x00ff;
            let flags = ((call.flags as u32) >> 8) & 0x00ff;
            let packed = call.value;
            let token = packed & 0x00ff_ffff_ffff_ffff;
            let value = (packed >> 56) & 0xff;
            match super::runtime::device_submit(
                cell, call.capability, call.object, opcode, flags,
                0, value, call.argument, call.length, token,
            ) {
                Ok(()) => ResultCode::OK,
                Err(_) => ResultCode::error(ABI_STATUS_DENIED),
            }
        }

        Operation::EventEmit => {
            if !super::runtime::permits(cell, call.capability, call.object, R::WRITE) {
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
    }
}


pub fn dispatch_present(
    cell: super::CellId,
    capability: CapabilityId,
    present: &super::graphics::Present,
) -> ResultCode {
    match super::runtime::present_surface(cell, capability, present) {
        Ok(()) => ResultCode::OK,
        Err(_) => ResultCode::error(ABI_STATUS_DENIED),
    }
}
