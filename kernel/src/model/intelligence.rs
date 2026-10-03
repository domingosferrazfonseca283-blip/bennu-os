use super::sync::SpinLock;

pub const MAX_COGNITIVE_TASKS: usize = 16;
pub const MAX_COGNITIVE_OBSERVATIONS: usize = 32;

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
pub struct CognitiveObservation {
    pub event_kind: u16,
    pub source: u64,
    pub target: u64,
    pub value: u64,
}

impl CognitiveObservation {
    pub const EMPTY: Self = Self {
        event_kind: 0,
        source: 0,
        target: 0,
        value: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AuthorizationRequest {
    pub id: u64,
    pub task_id: u64,
    pub kind: TaskKind,
    pub capability_mask: u32,
    pub input: u64,
}

impl AuthorizationRequest {
    pub const EMPTY: Self = Self {
        id: 0,
        task_id: 0,
        kind: TaskKind::Observe,
        capability_mask: 0,
        input: 0,
    };
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
    pub authorized: bool,
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
        authorized: false,
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
}

impl CognitiveRuntime {
    pub const EMPTY: Self = Self {
        next_id: 1,
        active: 0,
        phase: CognitivePhase::Idle,
        cycles: 0,
        completed: 0,
        failed: 0,
    };
}

static TASKS: SpinLock<[CognitiveTask; MAX_COGNITIVE_TASKS]> =
    SpinLock::new([CognitiveTask::EMPTY; MAX_COGNITIVE_TASKS]);
static OBSERVATIONS: SpinLock<[CognitiveObservation; MAX_COGNITIVE_OBSERVATIONS]> =
    SpinLock::new([CognitiveObservation::EMPTY; MAX_COGNITIVE_OBSERVATIONS]);
static RUNTIME: SpinLock<CognitiveRuntime> = SpinLock::new(CognitiveRuntime::EMPTY);

pub fn init() {
    *RUNTIME.lock().get_mut() = CognitiveRuntime::EMPTY;
    *TASKS.lock().get_mut() = [CognitiveTask::EMPTY; MAX_COGNITIVE_TASKS];
    *OBSERVATIONS.lock().get_mut() = [CognitiveObservation::EMPTY; MAX_COGNITIVE_OBSERVATIONS];
}

pub fn observe(event: super::Event) -> Result<(), &'static str> {
    let mut observations = OBSERVATIONS.lock();
    for slot in observations.get_mut().iter_mut() {
        if slot.event_kind == 0 {
            *slot = CognitiveObservation {
                event_kind: event.kind as u16,
                source: event.source.0,
                target: event.target.0,
                value: event.value,
            };
            return Ok(());
        }
    }
    Err("cognitive observation queue full")
}

fn consume_observation() -> Option<CognitiveObservation> {
    let mut observations = OBSERVATIONS.lock();
    let slot = observations.get_mut().iter_mut().find(|slot| slot.event_kind != 0)?;
    let value = *slot;
    *slot = CognitiveObservation::EMPTY;
    Some(value)
}

fn observation_task_kind(observation: CognitiveObservation) -> TaskKind {
    match observation.event_kind {
        5 | 6 | 8 | 9 | 10 | 11 => TaskKind::Execute,
        _ => TaskKind::Observe,
    }
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
                authorized: false,
            };
            return Ok(id);
        }
    }
    Err("cognitive task queue full")
}

pub fn authorization_request(id: u64) -> Option<AuthorizationRequest> {
    let tasks = TASKS.lock();
    let task = tasks.get().iter().find(|task| {
        task.id == id && task.state == TaskState::AwaitingAuthorization
    })?;
    Some(AuthorizationRequest {
        id: task.id,
        task_id: task.id,
        kind: task.kind,
        capability_mask: task.capability_mask,
        input: task.input,
    })
}

pub fn authorize_task(id: u64) -> Result<(), &'static str> {
    let mut tasks = TASKS.lock();
    let task = tasks
        .get_mut()
        .iter_mut()
        .find(|task| task.id == id)
        .ok_or("cognitive task not found")?;

    if task.state != TaskState::AwaitingAuthorization {
        return Err("cognitive task is not awaiting authorization");
    }

    task.authorized = true;
    task.state = TaskState::Queued;
    Ok(())
}

pub fn deny_task(id: u64) -> Result<(), &'static str> {
    let mut tasks = TASKS.lock();
    let task = tasks
        .get_mut()
        .iter_mut()
        .find(|task| task.id == id)
        .ok_or("cognitive task not found")?;

    if task.state != TaskState::AwaitingAuthorization {
        return Err("cognitive task is not awaiting authorization");
    }

    task.state = TaskState::Failed;
    task.authorized = false;
    Ok(())
}

fn select_task(tasks: &[CognitiveTask; MAX_COGNITIVE_TASKS]) -> Option<usize> {
    let mut selected = None;
    let mut priority = 0u8;
    for (index, task) in tasks.iter().enumerate() {
        if task.state == TaskState::Queued
            && (selected.is_none() || task.priority > priority)
        {
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

    let mut tasks = TASKS.lock();
    let index = match select_task(tasks.get()) {
        Some(value) => value,
        None => {
            drop(tasks);
            if let Some(observation) = consume_observation() {
                let kind = observation_task_kind(observation);
                let input = ((observation.event_kind as u64) << 48)
                    | (observation.value & 0x0000_ffff_ffff_ffff);
                drop(runtime);
                let _ = submit(kind, 0, 0, input).map(|id| {
                    if kind == TaskKind::Observe {
                        let _ = authorize_task(id);
                    }
                });
                return;
            }
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
            if !task.authorized {
                task.state = TaskState::AwaitingAuthorization;
                runtime_state.phase = CognitivePhase::Plan;
            } else {
                task.phase = CognitivePhase::Execute;
                task.state = TaskState::Running;
                runtime_state.phase = CognitivePhase::Execute;
            }
        }
        CognitivePhase::Execute => {
            if !task.authorized {
                task.state = TaskState::AwaitingAuthorization;
                runtime_state.phase = CognitivePhase::Plan;
                task.phase = CognitivePhase::Plan;
            } else {
                task.output = ((task.capability_mask as u64) << 32) | (task.input & 0xffff_ffff);
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
