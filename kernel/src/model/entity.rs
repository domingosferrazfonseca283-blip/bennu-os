use super::intelligence::CognitivePhase;
use super::sync::SpinLock;

pub const MAX_ENTITY_RELATIONS: usize = 64;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityLifeState {
    Initializing = 0, Awake = 1, Observing = 2, Thinking = 3, Acting = 4,
    Verifying = 5, Communicating = 6, Sleeping = 7, Halted = 8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EntityRelation { pub from: u64, pub to: u64, pub kind: u8 }
impl EntityRelation { pub const EMPTY: Self = Self { from: 0, to: 0, kind: 0 }; }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EntityResourceSnapshot {
    pub objects: u32, pub cells: u32, pub block_devices: u32, pub mounts: u32,
    pub relations: u32, pub heartbeat: u64, pub cognitive_cycles: u64,
}
impl EntityResourceSnapshot {
    pub const EMPTY: Self = Self { objects: 0, cells: 0, block_devices: 0, mounts: 0, relations: 0, heartbeat: 0, cognitive_cycles: 0 };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BennuEntity {
    pub identity: u64, pub generation: u64, pub life_state: EntityLifeState,
    pub cognitive_phase: CognitivePhase, pub heartbeat: u64, pub observations: u64,
    pub decisions: u64, pub actions: u64, pub verifications: u64, pub communications: u64,
    pub last_event_kind: u16, pub last_source: u64, pub last_value: u64,
    pub resources: EntityResourceSnapshot,
}
impl BennuEntity {
    pub const EMPTY: Self = Self {
        identity: 0x4245_4e4e_5500_0001, generation: 1,
        life_state: EntityLifeState::Initializing, cognitive_phase: CognitivePhase::Idle,
        heartbeat: 0, observations: 0, decisions: 0, actions: 0, verifications: 0,
        communications: 0, last_event_kind: 0, last_source: 0, last_value: 0,
        resources: EntityResourceSnapshot::EMPTY,
    };
}

static ENTITY: SpinLock<BennuEntity> = SpinLock::new(BennuEntity::EMPTY);
static RELATION_SNAPSHOT: SpinLock<[EntityRelation; MAX_ENTITY_RELATIONS]> =
    SpinLock::new([EntityRelation::EMPTY; MAX_ENTITY_RELATIONS]);

pub fn init() {
    *ENTITY.lock().get_mut() = BennuEntity::EMPTY;
    *RELATION_SNAPSHOT.lock().get_mut() = [EntityRelation::EMPTY; MAX_ENTITY_RELATIONS];
}
pub fn state() -> BennuEntity { *ENTITY.lock().get() }
pub fn relations(out: &mut [EntityRelation]) -> usize {
    let snapshot = RELATION_SNAPSHOT.lock();
    let count = core::cmp::min(out.len(), snapshot.get().len());
    out[..count].copy_from_slice(&snapshot.get()[..count]);
    count
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
pub fn decision() { let mut g=ENTITY.lock(); let e=g.get_mut(); e.decisions=e.decisions.wrapping_add(1); }
pub fn action() { let mut g=ENTITY.lock(); let e=g.get_mut(); e.actions=e.actions.wrapping_add(1); }
pub fn verification() { let mut g=ENTITY.lock(); let e=g.get_mut(); e.verifications=e.verifications.wrapping_add(1); }
pub fn communication() { let mut g=ENTITY.lock(); let e=g.get_mut(); e.communications=e.communications.wrapping_add(1); }

pub fn refresh_resources() {
    let (objects, cells, block_devices, mounts) = super::runtime::resource_counts();
    let cognitive = super::intelligence::runtime();
    let mut source = [super::graph::Relation {
        from: super::ObjectId::NULL, to: super::ObjectId::NULL,
        kind: super::graph::RelationKind::Contains,
    }; MAX_ENTITY_RELATIONS];
    let relation_count = super::graph::snapshot(&mut source);
    let mut snapshot = RELATION_SNAPSHOT.lock();
    let target = snapshot.get_mut();
    *target = [EntityRelation::EMPTY; MAX_ENTITY_RELATIONS];
    for index in 0..relation_count {
        let relation = source[index];
        target[index] = EntityRelation { from: relation.from.0, to: relation.to.0, kind: relation.kind as u8 };
    }
    drop(snapshot);
    let mut guard = ENTITY.lock();
    let entity = guard.get_mut();
    entity.resources.objects = objects;
    entity.resources.cells = cells;
    entity.resources.block_devices = block_devices;
    entity.resources.mounts = mounts;
    entity.resources.relations = relation_count as u32;
    entity.resources.cognitive_cycles = cognitive.cycles;
}
