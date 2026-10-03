use super::sync::SpinLock;

pub const MAX_COGNITIVE_TASKS: usize = 16;
pub const MAX_COGNITIVE_OBSERVATIONS: usize = 32;
pub const MAX_COGNITIVE_MEMORY: usize = 64;
pub const MAX_COGNITIVE_PLAN_STEPS: usize = 8;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Observation = 1,
    Decision = 2,
    Result = 3,
    Completion = 4,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CognitiveMemoryEntry {
    pub id: u64,
    pub kind: MemoryKind,
    pub task_id: u64,
    pub event_kind: u16,
    pub source: u64,
    pub value: u64,
    pub outcome: u64,
}

impl CognitiveMemoryEntry {
    pub const EMPTY: Self = Self {
        id: 0,
        kind: MemoryKind::Observation,
        task_id: 0,
        event_kind: 0,
        source: 0,
        value: 0,
        outcome: 0,
    };
}

static MEMORY: SpinLock<[CognitiveMemoryEntry; MAX_COGNITIVE_MEMORY]> =
    SpinLock::new([CognitiveMemoryEntry::EMPTY; MAX_COGNITIVE_MEMORY]);
static MEMORY_NEXT_ID: SpinLock<u64> = SpinLock::new(1);

pub fn remember(entry: CognitiveMemoryEntry) -> u64 {
    let id = {
        let mut next = MEMORY_NEXT_ID.lock();
        let value = *next.get();
        *next.get_mut() = value.wrapping_add(1).max(1);
        value
    };

    let mut memory = MEMORY.lock();
    let index = ((id - 1) as usize) % MAX_COGNITIVE_MEMORY;
    let mut value = entry;
    value.id = id;
    memory.get_mut()[index] = value;
    id
}

pub fn recall(id: u64) -> Option<CognitiveMemoryEntry> {
    if id == 0 {
        return None;
    }
    let memory = MEMORY.lock();
    memory.get().iter().find(|entry| entry.id == id).copied()
}

pub fn memory() -> [CognitiveMemoryEntry; MAX_COGNITIVE_MEMORY] {
    *MEMORY.lock().get()
}


#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlanStepState {
    Empty = 0,
    Pending = 1,
    Running = 2,
    Completed = 3,
    Failed = 4,
    AwaitingAuthorization = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CognitivePlanStep {
    pub id: u64,
    pub task_id: u64,
    pub action: CognitiveDecision,
    pub capability_mask: u32,
    pub input: u64,
    pub output: u64,
    pub state: PlanStepState,
}

impl CognitivePlanStep {
    pub const EMPTY: Self = Self {
        id: 0,
        task_id: 0,
        action: CognitiveDecision::None,
        capability_mask: 0,
        input: 0,
        output: 0,
        state: PlanStepState::Empty,
    };
}

static PLAN: SpinLock<[CognitivePlanStep; MAX_COGNITIVE_PLAN_STEPS]> =
    SpinLock::new([CognitivePlanStep::EMPTY; MAX_COGNITIVE_PLAN_STEPS]);
static PLAN_NEXT_ID: SpinLock<u64> = SpinLock::new(1);

pub fn plan_for_task(task_id: u64, kind: TaskKind, input: u64, capability_mask: u32) -> Result<u64, &'static str> {
    let action = match kind {
        TaskKind::Observe => CognitiveDecision::Observe,
        TaskKind::Execute => CognitiveDecision::Execute,
        TaskKind::Research => CognitiveDecision::RequestAuthorization,
        TaskKind::Communicate => CognitiveDecision::RequestAuthorization,
    };
    let mut next = PLAN_NEXT_ID.lock();
    let id = *next.get();
    *next.get_mut() = id.wrapping_add(1).max(1);
    drop(next);
    let mut plan = PLAN.lock();
    for step in plan.get_mut().iter_mut() {
        if step.state == PlanStepState::Empty || step.state == PlanStepState::Completed || step.state == PlanStepState::Failed {
            *step = CognitivePlanStep {
                id,
                task_id,
                action,
                capability_mask,
                input,
                output: 0,
                state: if action == CognitiveDecision::RequestAuthorization { PlanStepState::AwaitingAuthorization } else { PlanStepState::Pending },
            };
            return Ok(id);
        }
    }
    Err("cognitive plan full")
}

pub fn plan_step(id: u64) -> Option<CognitivePlanStep> {
    let plan = PLAN.lock();
    plan.get().iter().find(|step| step.id == id).copied()
}

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
    pub source: u64,
}

impl AuthorizationRequest {
    pub const EMPTY: Self = Self {
        id: 0,
        task_id: 0,
        kind: TaskKind::Observe,
        capability_mask: 0,
        input: 0,
        source: 0,
    };
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CognitiveDecision {
    None = 0,
    Observe = 1,
    RequestAuthorization = 2,
    Execute = 3,
    Verify = 4,
    Remember = 5,
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
    pub source: u64,
    pub output: u64,
    pub decision: CognitiveDecision,
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
        source: 0,
        output: 0,
        decision: CognitiveDecision::None,
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
    *MEMORY.lock().get_mut() = [CognitiveMemoryEntry::EMPTY; MAX_COGNITIVE_MEMORY];
    *MEMORY_NEXT_ID.lock().get_mut() = 1;
    *PLAN.lock().get_mut() = [CognitivePlanStep::EMPTY; MAX_COGNITIVE_PLAN_STEPS];
    *PLAN_NEXT_ID.lock().get_mut() = 1;
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
            remember(CognitiveMemoryEntry {
                id: 0,
                kind: MemoryKind::Observation,
                task_id: 0,
                event_kind: event.kind as u16,
                source: event.source.0,
                value: event.value,
                outcome: event.target.0,
            });
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
                source: 0,
                output: 0,
                decision: CognitiveDecision::None,
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
        source: task.source,
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
    let _ = plan_for_task(task.id, task.kind, task.input, task.capability_mask);
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
    remember(CognitiveMemoryEntry {
        id: 0,
        kind: MemoryKind::Decision,
        task_id: task.id,
        event_kind: (task.input >> 48) as u16,
        source: task.source,
        value: task.input,
        outcome: 0,
    });
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
                    if let Some(task) = TASKS.lock().get_mut().iter_mut().find(|task| task.id == id) {
                        task.source = observation.source;
                    }
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
                task.decision = CognitiveDecision::RequestAuthorization;
                task.state = TaskState::AwaitingAuthorization;
                let _ = plan_for_task(task.id, task.kind, task.input, task.capability_mask);
                remember(CognitiveMemoryEntry {
                    id: 0,
                    kind: MemoryKind::Decision,
                    task_id: task.id,
                    event_kind: (task.input >> 48) as u16,
                    source: task.source,
                    value: task.input,
                    outcome: CognitiveDecision::RequestAuthorization as u64,
                });
                runtime_state.phase = CognitivePhase::Plan;
            } else {
                task.decision = CognitiveDecision::Execute;
                let _ = plan_for_task(task.id, task.kind, task.input, task.capability_mask);
                task.phase = CognitivePhase::Execute;
                task.state = TaskState::Running;
                remember(CognitiveMemoryEntry {
                    id: 0,
                    kind: MemoryKind::Decision,
                    task_id: task.id,
                    event_kind: (task.input >> 48) as u16,
                    source: task.source,
                    value: task.input,
                    outcome: CognitiveDecision::Execute as u64,
                });
                runtime_state.phase = CognitivePhase::Execute;
            }
        }
        CognitivePhase::Execute => {
            if !task.authorized {
                task.decision = CognitiveDecision::RequestAuthorization;
                task.state = TaskState::AwaitingAuthorization;
                runtime_state.phase = CognitivePhase::Plan;
                task.phase = CognitivePhase::Plan;
            } else {
                task.decision = CognitiveDecision::Verify;
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
            remember(CognitiveMemoryEntry {
                id: 0,
                kind: MemoryKind::Completion,
                task_id: task.id,
                event_kind: (task.input >> 48) as u16,
                source: task.source,
                value: task.output,
                outcome: 1,
            });
            runtime_state.completed = runtime_state.completed.wrapping_add(1);
            runtime_state.phase = CognitivePhase::Idle;
            runtime_state.active = 0;
        }
    }
}

pub fn execute_authorized() {
    let pending = {
        let tasks = TASKS.lock();
        tasks.get().iter().find(|task| task.state == TaskState::Running && task.authorized).copied()
    };
    let task = match pending { Some(value) => value, None => return };
    let result = match task.kind {
        TaskKind::Execute
            if task.input >> 48 == super::EventKind::DeviceTransferCompleted as u64 =>
        {
            super::runtime::prepare_bennufs_mount(super::ObjectId(task.source))
        }
        TaskKind::Observe => Ok(()),
        _ => Err("cognitive action has no kernel executor"),
    };
    let mut tasks = TASKS.lock();
    if let Some(current) = tasks.get_mut().iter_mut().find(|current| current.id == task.id) {
        match result {
            Ok(()) => {
                current.output = 1;
                current.decision = CognitiveDecision::Verify;
                current.phase = CognitivePhase::Verify;
                current.state = TaskState::Verifying;
                remember(CognitiveMemoryEntry {
                    id: 0,
                    kind: MemoryKind::Result,
                    task_id: current.id,
                    event_kind: (current.input >> 48) as u16,
                    source: current.source,
                    value: current.input,
                    outcome: 1,
                });
            }
            Err(_) => {
                current.output = 0;
                current.state = TaskState::Failed;
                current.phase = CognitivePhase::Idle;
                remember(CognitiveMemoryEntry {
                    id: 0,
                    kind: MemoryKind::Result,
                    task_id: current.id,
                    event_kind: (current.input >> 48) as u16,
                    source: current.source,
                    value: current.input,
                    outcome: 0,
                });
            }
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
