use super::capability::{Capability, CapabilityId, CapabilityRights};
use super::object::ObjectId;
use super::MAX_CAPABILITIES_PER_CELL;
use crate::arch::x86_64::execution::Context;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CellId(pub u32);

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Empty = 0,
    Ready = 1,
    Running = 2,
    Waiting = 3,
    Stopped = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CellAction {
    Yield = 0,
    Wait = 1,
    Stop = 2,
}

pub type CellEntry = extern "C" fn() -> CellAction;

#[repr(C)]
pub struct Cell {
    pub id: CellId,
    pub state: CellState,
    pub root: ObjectId,
    pub entry: Option<CellEntry>,
    pub context: Context,
    capabilities: [Capability; MAX_CAPABILITIES_PER_CELL],
    capability_generations: [u32; MAX_CAPABILITIES_PER_CELL],
    capability_count: usize,
}

impl Cell {
    pub const fn empty() -> Self {
        Self {
            id: CellId(0),
            state: CellState::Empty,
            root: ObjectId::NULL,
            entry: None,
            context: Context::EMPTY,
            capabilities: [Capability::EMPTY; MAX_CAPABILITIES_PER_CELL],
            capability_generations: [0; MAX_CAPABILITIES_PER_CELL],
            capability_count: 0,
        }
    }

    pub fn create(id: CellId, root: ObjectId) -> Self {
        Self {
            id,
            state: CellState::Ready,
            root,
            entry: None,
            context: Context::EMPTY,
            capabilities: [Capability::EMPTY; MAX_CAPABILITIES_PER_CELL],
            capability_generations: [0; MAX_CAPABILITIES_PER_CELL],
            capability_count: 0,
        }
    }

    pub fn grant(
        &mut self,
        object: ObjectId,
        rights: CapabilityRights,
    ) -> Result<CapabilityId, &'static str> {
        if object.is_null() {
            return Err("null object");
        }
        let slot = (0..MAX_CAPABILITIES_PER_CELL)
            .find(|&index| self.capabilities[index].id == CapabilityId::NULL)
            .ok_or("cell capability table full")?;
        let generation = self.capability_generations[slot].wrapping_add(1).max(1);
        self.capability_generations[slot] = generation;
        let id = CapabilityId::new(object, slot as u32, generation);
        self.capabilities[slot] = Capability {
            id,
            object,
            rights,
            owner_cell: self.id.0,
            generation,
        };
        self.capability_count += 1;
        Ok(id)
    }

    pub fn permits(
        &self,
        capability: CapabilityId,
        object: ObjectId,
        rights: CapabilityRights,
    ) -> bool {
        for index in 0..MAX_CAPABILITIES_PER_CELL {
            let entry = &self.capabilities[index];
            if entry.id == capability && entry.permits(self.id.0, object, rights) {
                return true;
            }
        }
        false
    }

    pub fn capability_count(&self) -> usize {
        self.capability_count
    }

    pub fn revoke(&mut self, capability: CapabilityId) -> Result<(), &'static str> {
        for index in 0..MAX_CAPABILITIES_PER_CELL {
            if self.capabilities[index].id == capability {
                self.capabilities[index] = Capability::EMPTY;
                self.capability_count -= 1;
                return Ok(());
            }
        }
        Err("capability not found")
    }

    pub fn set_context(&mut self, context: Context) -> Result<(), &'static str> {
        if !context.is_initialized() { return Err("cell context is not initialized"); }
        self.context = context;
        Ok(())
    }

    pub fn context(&self) -> Context { self.context }

    pub fn bind_entry(&mut self, entry: CellEntry) -> Result<(), &'static str> {
        if self.state == CellState::Empty || self.state == CellState::Stopped {
            return Err("cell is not executable");
        }
        self.entry = Some(entry);
        self.state = CellState::Ready;
        Ok(())
    }

    pub fn run_once(&mut self) -> Result<CellAction, &'static str> {
        let entry = self.entry.ok_or("cell has no entry")?;
        self.state = CellState::Running;
        let action = entry();
        self.state = match action {
            CellAction::Yield => CellState::Ready,
            CellAction::Wait => CellState::Waiting,
            CellAction::Stop => CellState::Stopped,
        };
        Ok(action)
    }
}
