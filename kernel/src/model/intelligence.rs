use super::sync::SpinLock;

pub const MAX_COGNITIVE_TASKS: usize = 16;

pub const CAPABILITY_OBSERVE: u32 = 1 << 0;
pub const CAPABILITY_RESEARCH: u32 = 1 << 1;
pub const CAPABILITY_EXECUTE: u32 = 1 << 2;
pub const CAPABILITY_COMMUNICATE: u32 = 1 << 3;
pub const CAPABILITY_STORAGE: u32 = 1 << 4;
pub const CAPABILITY_NETWORK: u32 = 1 << 5;
pub const CAPABILITY_PROCESS: u32 = 1 << 6;
pub const CAPABILITY_ALL: u32 = CAPABILITY_OBSERVE
    | CAPABILITY_RESEARCH
    | CAPABILITY_EXECUTE
    | CAPABILITY_COMMUNICATE
    | CAPABILITY_STORAGE
    | CAPABILITY_NETWORK
    | CAPABILITY_PROCESS;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Observe = 1,
    Research = 2,
    Execute = 3,
    Communicate = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Empty = 0,
    Queued = 1,
    Planning = 2,
    AwaitingAuthorization = 3,
    Running = 4,
    Verifying = 5,
    Completed = 6,
    Failed = 7,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CognitivePhase {
    Idle = 0,
    Observe = 1,
    Plan = 2,
    Execute = 3,
    Verify = 4,
    Communicate = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CognitiveTask {
    pub id: u64,
    pub kind: TaskKind,
    pub state: TaskState,
    pub phase: CognitivePhase,
    pub priority: u8,
    pub capability_mask: u32,
    pub input: u64,
    pub output: u64,
}

impl CognitiveTask {
    pub const EMPTY: Self = Self {
        id: 0,
        kind: TaskKind::Observe,
        state: TaskState::Empty,
        phase: CognitivePhase::Idle,
        priority: 0,
        capability_mask: 0,
        input: 0,
        output: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CognitiveRuntime {
    pub next_id: u64,
    pub active: u64,
    pub phase: CognitivePhase,
    pub cycles: u64,
    pub completed: u64,
    pub failed: u64,
    pub authorized_mask: u32,
}

impl CognitiveRuntime {
    pub const EMPTY: Self = Self {
        next_id: 1,
        active: 0,
        phase: CognitivePhase::Idle,
        cycles: 0,
        completed: 0,
        failed: 0,
        authorized_mask: 0,
    };
}

static TASKS: SpinLock<[CognitiveTask; MAX_COGNITIVE_TASKS]> =
    SpinLock::new([CognitiveTask::EMPTY; MAX_COGNITIVE_TASKS]);
static RUNTIME: SpinLock<CognitiveRuntime> = SpinLock::new(CognitiveRuntime::EMPTY);

pub fn init() {
    *RUNTIME.lock().get_mut() = CognitiveRuntime::EMPTY;
    *TASKS.lock().get_mut() = [CognitiveTask::EMPTY; MAX_COGNITIVE_TASKS];
}

pub fn authorize(capability_mask: u32) {
    let mut runtime = RUNTIME.lock();
    runtime.get_mut().authorized_mask |= capability_mask & CAPABILITY_ALL;
}

pub fn revoke(capability_mask: u32) {
    let mut runtime = RUNTIME.lock();
    runtime.get_mut().authorized_mask &= !(capability_mask & CAPABILITY_ALL);
}

pub fn revoke_all() {
    RUNTIME.lock().get_mut().authorized_mask = 0;
}

pub fn authorization_mask() -> u32 {
    RUNTIME.lock().get().authorized_mask
}

fn kind_capability(kind: TaskKind) -> u32 {
    match kind {
        TaskKind::Observe => CAPABILITY_OBSERVE,
        TaskKind::Research => CAPABILITY_RESEARCH,
        TaskKind::Execute => CAPABILITY_EXECUTE,
        TaskKind::Communicate => CAPABILITY_COMMUNICATE,
    }
}

pub fn submit(
    kind: TaskKind,
    priority: u8,
    capability_mask: u32,
    input: u64,
) -> Result<u64, &'static str> {
    let requested = capability_mask | kind_capability(kind);
    if requested & !CAPABILITY_ALL != 0 {
        return Err("unknown cognitive capability");
    }

    let mut runtime = RUNTIME.lock();
    let runtime_state = runtime.get_mut();
    let id = runtime_state.next_id;
    runtime_state.next_id = runtime_state.next_id.wrapping_add(1).max(1);
    drop(runtime);

    let mut tasks = TASKS.lock();
    for task in tasks.get_mut().iter_mut() {
        if task.state == TaskState::Empty
            || task.state == TaskState::Completed
            || task.state == TaskState::Failed
        {
            *task = CognitiveTask {
                id,
                kind,
                state: TaskState::Queued,
                phase: CognitivePhase::Idle,
                priority,
                capability_mask: requested,
                input,
                output: 0,
            };
            return Ok(id);
        }
    }
    Err("cognitive task queue full")
}

fn select_task(
    tasks: &[CognitiveTask; MAX_COGNITIVE_TASKS],
    authorized_mask: u32,
) -> Option<usize> {
    let mut selected = None;
    let mut priority = 0u8;
    for (index, task) in tasks.iter().enumerate() {
        let eligible = task.state == TaskState::Queued
            || (task.state == TaskState::AwaitingAuthorization
                && task.capability_mask & !authorized_mask == 0);
        if eligible && (selected.is_none() || task.priority > priority) {
            selected = Some(index);
            priority = task.priority;
        }
    }
    selected
}

pub fn tick() {
    let mut runtime = RUNTIME.lock();
    let runtime_state = runtime.get_mut();
    runtime_state.cycles = runtime_state.cycles.wrapping_add(1);
    let authorized_mask = runtime_state.authorized_mask;

    let mut tasks = TASKS.lock();
    let index = match select_task(tasks.get(), authorized_mask) {
        Some(value) => value,
        None => {
            runtime_state.active = 0;
            runtime_state.phase = CognitivePhase::Idle;
            return;
        }
    };

    let task = &mut tasks.get_mut()[index];
    runtime_state.active = task.id;

    match task.phase {
        CognitivePhase::Idle => {
            task.phase = CognitivePhase::Observe;
            task.state = TaskState::Planning;
            runtime_state.phase = CognitivePhase::Observe;
        }
        CognitivePhase::Observe => {
            task.phase = CognitivePhase::Plan;
            runtime_state.phase = CognitivePhase::Plan;
        }
        CognitivePhase::Plan => {
            if task.capability_mask & !runtime_state.authorized_mask != 0 {
                task.state = TaskState::AwaitingAuthorization;
                runtime_state.phase = CognitivePhase::Plan;
            } else {
                task.phase = CognitivePhase::Execute;
                task.state = TaskState::Running;
                runtime_state.phase = CognitivePhase::Execute;
            }
        }
        CognitivePhase::Execute => {
            if task.capability_mask & !runtime_state.authorized_mask != 0 {
                task.state = TaskState::AwaitingAuthorization;
                runtime_state.phase = CognitivePhase::Plan;
                task.phase = CognitivePhase::Plan;
            } else {
                task.output = task.input;
                task.phase = CognitivePhase::Verify;
                task.state = TaskState::Verifying;
                runtime_state.phase = CognitivePhase::Verify;
            }
        }
        CognitivePhase::Verify => {
            task.phase = CognitivePhase::Communicate;
            runtime_state.phase = CognitivePhase::Communicate;
        }
        CognitivePhase::Communicate => {
            task.state = TaskState::Completed;
            task.phase = CognitivePhase::Idle;
            runtime_state.completed = runtime_state.completed.wrapping_add(1);
            runtime_state.phase = CognitivePhase::Idle;
            runtime_state.active = 0;
        }
    }
}

pub fn runtime() -> CognitiveRuntime {
    *RUNTIME.lock().get()
}

pub fn task(id: u64) -> Option<CognitiveTask> {
    let tasks = TASKS.lock();
    tasks.get().iter().find(|task| task.id == id).copied()
}
