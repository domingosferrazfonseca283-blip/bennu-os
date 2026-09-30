pub mod capability;
pub mod cell;
pub mod event;
pub mod surface;
pub mod graph;
pub mod object;
pub mod runtime;

pub use capability::{Capability, CapabilityId, CapabilityRights};
pub use cell::{Cell, CellAction, CellEntry, CellId, CellState};
pub use event::{Event, EventKind};
pub use object::{ObjectId, ObjectKind, ResourceObject};
pub use graph::{Relation, RelationKind};
pub use surface::Surface;

pub const MAX_CELLS: usize = 64;
pub const MAX_OBJECTS: usize = 256;
pub const MAX_CAPABILITIES_PER_CELL: usize = 32;
