use super::{
    CapabilityId, CapabilityRights, Cell, CellId, CellState, Event, ObjectId, ObjectKind,
    ResourceObject, EventKind, MAX_CELLS, MAX_OBJECTS,
};

const EVENT_QUEUE_SIZE: usize = 128;

static mut OBJECTS: [ResourceObject; MAX_OBJECTS] = [ResourceObject::EMPTY; MAX_OBJECTS];
static mut OBJECT_GENERATIONS: [u32; MAX_OBJECTS] = [0; MAX_OBJECTS];
static mut CELLS: [Cell; MAX_CELLS] = [const { Cell::empty() }; MAX_CELLS];
static mut EVENTS: [Event; EVENT_QUEUE_SIZE] = [Event::EMPTY; EVENT_QUEUE_SIZE];
static mut EVENT_HEAD: usize = 0;
static mut EVENT_TAIL: usize = 0;
static mut NEXT_OBJECT: usize = 1;

pub fn init() {
    unsafe {
        OBJECTS = [ResourceObject::EMPTY; MAX_OBJECTS];
        OBJECT_GENERATIONS = [0; MAX_OBJECTS];
        CELLS = [const { Cell::empty() }; MAX_CELLS];
        EVENTS = [Event::EMPTY; EVENT_QUEUE_SIZE];
        EVENT_HEAD = 0;
        EVENT_TAIL = 0;
        NEXT_OBJECT = 1;
    }
}

pub fn create_object(kind: ObjectKind, owner: u32) -> Result<ObjectId, &'static str> {
    unsafe {
        for offset in 0..MAX_OBJECTS {
            let index = (NEXT_OBJECT + offset) % MAX_OBJECTS;
            if index == 0 || OBJECTS[index].kind != ObjectKind::Empty {
                continue;
            }

            let generation = OBJECT_GENERATIONS[index].wrapping_add(1).max(1);
            OBJECT_GENERATIONS[index] = generation;
            let id = ObjectId::new(index as u32, generation);
            OBJECTS[index] = ResourceObject {
                id,
                kind,
                owner,
                flags: 0,
            };
            NEXT_OBJECT = (index + 1) % MAX_OBJECTS;
            let _ = emit(Event::new(EventKind::ResourceCreated, id, ObjectId::NULL, 0));
            return Ok(id);
        }
    }

    Err("object space exhausted")
}

pub fn create_cell(id: CellId, root: ObjectId) -> Result<(), &'static str> {
    if root.is_null() {
        return Err("cell requires a root object");
    }

    unsafe {
        let index = id.0 as usize;
        if index >= MAX_CELLS || CELLS[index].state != CellState::Empty {
            return Err("cell slot unavailable");
        }
        if !object_exists(root) {
            return Err("cell root does not exist");
        }
        CELLS[index] = Cell::create(id, root);
    }

    Ok(())
}

pub fn bind_entry(cell: CellId, entry: super::CellEntry) -> Result<(), &'static str> {
    unsafe {
        let index = cell.0 as usize;
        if index >= MAX_CELLS || CELLS[index].state == CellState::Empty {
            return Err("cell does not exist");
        }
        CELLS[index].bind_entry(entry)
    }
}

pub fn run_once(cell: CellId) -> Result<super::CellAction, &'static str> {
    unsafe {
        let index = cell.0 as usize;
        if index >= MAX_CELLS || CELLS[index].state == CellState::Empty {
            return Err("cell does not exist");
        }
        CELLS[index].run_once()
    }
}

pub fn grant(
    cell: CellId,
    object: ObjectId,
    rights: CapabilityRights,
) -> Result<CapabilityId, &'static str> {
    unsafe {
        let index = cell.0 as usize;
        if index >= MAX_CELLS || CELLS[index].state == CellState::Empty {
            return Err("cell does not exist");
        }
        if !object_exists(object) {
            return Err("object does not exist");
        }
        CELLS[index].grant(object, rights)
    }
}

pub fn permits(
    cell: CellId,
    capability: CapabilityId,
    object: ObjectId,
    rights: CapabilityRights,
) -> bool {
    unsafe {
        let index = cell.0 as usize;
        if index >= MAX_CELLS || CELLS[index].state == CellState::Empty {
            return false;
        }
        CELLS[index].permits(capability, object, rights)
    }
}

pub fn emit(event: Event) -> Result<(), &'static str> {
    unsafe {
        let next = (EVENT_TAIL + 1) % EVENT_QUEUE_SIZE;
        if next == EVENT_HEAD {
            return Err("event fabric full");
        }
        EVENTS[EVENT_TAIL] = event;
        EVENT_TAIL = next;
    }
    Ok(())
}

pub fn poll() -> Option<Event> {
    unsafe {
        if EVENT_HEAD == EVENT_TAIL {
            return None;
        }
        let event = EVENTS[EVENT_HEAD];
        EVENT_HEAD = (EVENT_HEAD + 1) % EVENT_QUEUE_SIZE;
        Some(event)
    }
}

pub fn object_exists(id: ObjectId) -> bool {
    if id.is_null() {
        return false;
    }

    unsafe {
        let index = id.index();
        index < MAX_OBJECTS
            && OBJECTS[index].id == id
            && OBJECTS[index].kind != ObjectKind::Empty
    }
}
