use super::intelligence::CognitivePhase;
use super::sync::SpinLock;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityLifeState {
    Initializing = 0,
    Awake = 1,
    Observing = 2,
    Thinking = 3,
    Acting = 4,
    Verifying = 5,
    Communicating = 6,
    Sleeping = 7,
    Halted = 8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EntityResourceSnapshot {
    pub objects: u32,
    pub cells: u32,
    pub block_devices: u32,
    pub mounts: u32,
    pub heartbeat: u64,
    pub cognitive_cycles: u64,
}

impl EntityResourceSnapshot {
    pub const EMPTY: Self = Self {
        objects: 0,
        cells: 0,
        block_devices: 0,
        mounts: 0,
        heartbeat: 0,
        cognitive_cycles: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BennuEntity {
    pub identity: u64,
    pub generation: u64,
    pub life_state: EntityLifeState,
    pub cognitive_phase: CognitivePhase,
    pub heartbeat: u64,
    pub observations: u64,
    pub decisions: u64,
    pub actions: u64,
    pub verifications: u64,
    pub communications: u64,
    pub last_event_kind: u16,
    pub last_source: u64,
    pub last_value: u64,
    pub resources: EntityResourceSnapshot,
}

impl BennuEntity {
    pub const EMPTY: Self = Self {
        identity: 0x4245_4e4e_5500_0001,
        generation: 1,
        life_state: EntityLifeState::Initializing,
        cognitive_phase: CognitivePhase::Idle,
        heartbeat: 0,
        observations: 0,
        decisions: 0,
        actions: 0,
        verifications: 0,
        communications: 0,
        last_event_kind: 0,
        last_source: 0,
        last_value: 0,
        resources: EntityResourceSnapshot::EMPTY,
    };
}

static ENTITY: SpinLock<BennuEntity> = SpinLock::new(BennuEntity::EMPTY);

pub fn init() {
    *ENTITY.lock().get_mut() = BennuEntity::EMPTY;
}

pub fn state() -> BennuEntity {
    *ENTITY.lock().get()
}

pub fn heartbeat(phase: CognitivePhase) {
    let mut guard = ENTITY.lock();
    let entity = guard.get_mut();
    entity.heartbeat = entity.heartbeat.wrapping_add(1);
    entity.cognitive_phase = phase;
    entity.life_state = match phase {
        CognitivePhase::Idle => EntityLifeState::Awake,
        CognitivePhase::Observe => EntityLifeState::Observing,
        CognitivePhase::Plan => EntityLifeState::Thinking,
        CognitivePhase::Execute => EntityLifeState::Acting,
        CognitivePhase::Verify => EntityLifeState::Verifying,
        CognitivePhase::Communicate => EntityLifeState::Communicating,
    };
    entity.resources.heartbeat = entity.heartbeat;
}

pub fn observe(event: super::Event) {
    let mut guard = ENTITY.lock();
    let entity = guard.get_mut();
    entity.observations = entity.observations.wrapping_add(1);
    entity.last_event_kind = event.kind as u16;
    entity.last_source = event.source.0;
    entity.last_value = event.value;
}

pub fn decision() {
    let mut guard = ENTITY.lock();
    guard.get_mut().decisions = guard.get().decisions.wrapping_add(1);
}

pub fn action() {
    let mut guard = ENTITY.lock();
    guard.get_mut().actions = guard.get().actions.wrapping_add(1);
}

pub fn verification() {
    let mut guard = ENTITY.lock();
    guard.get_mut().verifications = guard.get().verifications.wrapping_add(1);
}

pub fn communication() {
    let mut guard = ENTITY.lock();
    guard.get_mut().communications = guard.get().communications.wrapping_add(1);
}

pub fn refresh_resources() {
    let (objects, cells, block_devices, mounts) = super::runtime::resource_counts();
    let cognitive = super::intelligence::runtime();
    let mut guard = ENTITY.lock();
    let entity = guard.get_mut();
    entity.resources.objects = objects;
    entity.resources.cells = cells;
    entity.resources.block_devices = block_devices;
    entity.resources.mounts = mounts;
    entity.resources.cognitive_cycles = cognitive.cycles;
}
