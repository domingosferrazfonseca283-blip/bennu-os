use super::{BlockDevice, BlockRequest, IoCompletion, IoEnvelope, IoState, IoStatus, ObjectId};
use super::policy::AccessPolicy;
use super::sync::SpinLock;

pub const IO_QUEUE_CAPACITY: usize = 64;

struct IoStateTable {
    queue: [IoEnvelope; IO_QUEUE_CAPACITY],
    head: usize,
    tail: usize,
    policy: AccessPolicy,
}

impl IoStateTable {
    const EMPTY: Self = Self {
        queue: [IoEnvelope::EMPTY; IO_QUEUE_CAPACITY],
        head: 0,
        tail: 0,
        policy: AccessPolicy::USB_FIRST,
    };

    fn push(&mut self, envelope: IoEnvelope) -> Result<(), &'static str> {
        let next = (self.tail + 1) % IO_QUEUE_CAPACITY;
        if next == self.head { return Err("I/O queue full"); }
        self.queue[self.tail] = envelope;
        self.tail = next;
        Ok(())
    }

    fn pop(&mut self) -> Option<IoEnvelope> {
        if self.head == self.tail { return None; }
        let value = self.queue[self.head];
        self.head = (self.head + 1) % IO_QUEUE_CAPACITY;
        Some(value)
    }
}

static IO: SpinLock<IoStateTable> = SpinLock::new(IoStateTable::EMPTY);

pub fn init(policy: AccessPolicy) {
    *IO.lock().get_mut() = IoStateTable { policy, ..IoStateTable::EMPTY };
}

pub fn submit(request: BlockRequest, device: BlockDevice, is_boot_device: bool) -> Result<(), &'static str> {
    if !request.is_valid() || request.device != device.object {
        return Err("invalid block request");
    }
    if !device.geometry.valid() {
        return Err("invalid block geometry");
    }
    let guard = IO.lock();
    if !guard.get().policy.permits_storage(device.object, device.removable, is_boot_device) {
        return Err("storage access denied");
    }
    drop(guard);

    let envelope = IoEnvelope {
        request,
        completion: IoCompletion {
            token: request.token,
            device: request.device,
            state: IoState::Queued,
            status: IoStatus::Ok as u32,
            transferred: 0,
        },
    };
    IO.lock().get_mut().push(envelope)
}

pub fn begin_next() -> Option<IoEnvelope> {
    let mut guard = IO.lock();
    let mut envelope = guard.get_mut().pop()?;
    envelope.completion.state = IoState::Running;
    Some(envelope)
}

pub fn complete(request: BlockRequest, status: IoStatus, transferred: u64) -> IoCompletion {
    let state = if status == IoStatus::Ok { IoState::Completed } else { IoState::Failed };
    IoCompletion {
        token: request.token,
        device: request.device,
        state,
        status: status as u32,
        transferred,
    }
}

pub fn queued() -> usize {
    let guard = IO.lock();
    let state = guard.get();
    if state.tail >= state.head { state.tail - state.head } else { IO_QUEUE_CAPACITY - state.head + state.tail }
}

pub const fn queue_capacity() -> usize { IO_QUEUE_CAPACITY }
