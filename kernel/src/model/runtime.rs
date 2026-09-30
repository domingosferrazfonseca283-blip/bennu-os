use super::{
    CapabilityId, CapabilityRights, Cell, CellId, CellState, Event, ObjectId, ObjectKind,
    ResourceObject, EventKind, MAX_CELLS, MAX_OBJECTS,
};
use super::sync::SpinLock;

const EVENT_QUEUE_SIZE: usize = 128;

struct RuntimeState {
    objects: [ResourceObject; MAX_OBJECTS],
    generations: [u32; MAX_OBJECTS],
    cells: [Cell; MAX_CELLS],
    events: [Event; EVENT_QUEUE_SIZE],
    event_head: usize,
    event_tail: usize,
    next_object: usize,
    memory_frames: [u64; MAX_OBJECTS],
}

impl RuntimeState {
    const EMPTY: Self = Self {
        objects: [ResourceObject::EMPTY; MAX_OBJECTS],
        generations: [0; MAX_OBJECTS],
        cells: [const { Cell::empty() }; MAX_CELLS],
        events: [Event::EMPTY; EVENT_QUEUE_SIZE],
        event_head: 0,
        event_tail: 0,
        next_object: 1,
        memory_frames: [0; MAX_OBJECTS],
    };
}

static RUNTIME: SpinLock<RuntimeState> = SpinLock::new(RuntimeState::EMPTY);

pub fn init() {
    *RUNTIME.lock().get_mut() = RuntimeState::EMPTY;
    super::graph::init();
    super::surface::init();
}

fn object_exists_unlocked(state: &RuntimeState, id: ObjectId) -> bool {
    if id.is_null() { return false; }
    let index = id.index();
    index < MAX_OBJECTS
        && state.objects[index].id == id
        && state.objects[index].kind != ObjectKind::Empty
}

fn emit_unlocked(state: &mut RuntimeState, event: Event) -> Result<(), &'static str> {
    let next = (state.event_tail + 1) % EVENT_QUEUE_SIZE;
    if next == state.event_head { return Err("event fabric full"); }
    state.events[state.event_tail] = event;
    state.event_tail = next;
    Ok(())
}

pub fn create_object(kind: ObjectKind, owner: u32) -> Result<ObjectId, &'static str> {
    let backing = match kind {
        ObjectKind::Memory | ObjectKind::Data => {
            crate::memory::allocate_frame_below(64 * 1024 * 1024)
                .ok_or("no physical frame for memory object")?
        }
        _ => 0,
    };
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    for offset in 0..MAX_OBJECTS {
        let index = (state.next_object + offset) % MAX_OBJECTS;
        if index == 0 || state.objects[index].kind != ObjectKind::Empty { continue; }
        let generation = state.generations[index].wrapping_add(1).max(1);
        state.generations[index] = generation;
        let id = ObjectId::new(index as u32, generation);
        state.objects[index] = ResourceObject { id, kind, owner, flags: 0 };
        state.memory_frames[index] = backing;
        state.next_object = (index + 1) % MAX_OBJECTS;
        let _ = emit_unlocked(state, Event::new(EventKind::ResourceCreated, id, ObjectId::NULL, 0));
        return Ok(id);
    }
    Err("object space exhausted")
}

pub fn create_cell(id: CellId, root: ObjectId) -> Result<(), &'static str> {
    if root.is_null() { return Err("cell requires a root object"); }
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = id.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state != CellState::Empty {
        return Err("cell slot unavailable");
    }
    if !object_exists_unlocked(state, root) { return Err("cell root does not exist"); }
    state.cells[index] = Cell::create(id, root);
    let _ = state.cells[index].grant(root, CapabilityRights::ADMIN.union(CapabilityRights::READ).union(CapabilityRights::MAP));
    Ok(())
}

pub fn entry(cell: CellId) -> Option<super::CellEntry> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return None;
    }
    state.cells[index].entry
}

pub fn context_ptr(cell: CellId) -> Option<(*mut crate::arch::x86_64::execution::Context, u64)> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return None;
    }
    Some((&mut state.cells[index].context as *mut _, state.cells[index].address_space_root()))
}

pub fn prepare_cell_context(cell: CellId, trampoline: u64) -> Result<(), &'static str> {
    let stack = crate::memory::allocate_frame_below(crate::memory::PAGE_SIZE * 16384)
        .ok_or("no physical frame for cell stack")?;

    let root = crate::memory::paging::create_address_space_root()?;

    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }

    state.cells[index].attach_address_space_root(root)?;
    state.cells[index].kernel_stack_top = stack.checked_add(crate::memory::PAGE_SIZE).ok_or("cell kernel stack overflow")?;

    crate::memory::paging::map_supervisor_page_in_root(
        root,
        stack,
        stack,
        true,
        false,
    )?;

    let trampoline = trampoline & !(crate::memory::PAGE_SIZE - 1);
    if trampoline >= 64 * 1024 * 1024 {
        return Err("cell trampoline is outside bootstrap kernel mapping");
    }
    crate::memory::paging::map_supervisor_page_in_root(
        root,
        trampoline,
        trampoline,
        false,
        true,
    )?;

    let timer = crate::arch::x86_64::idt::timer_handler_address();
    let timer_page = timer & !(crate::memory::PAGE_SIZE - 1);
    if timer_page >= 64 * 1024 * 1024 {
        return Err("timer handler is outside bootstrap kernel mapping");
    }
    crate::memory::paging::map_supervisor_page_in_root(
        root,
        timer_page,
        timer_page,
        false,
        true,
    )?;

    unsafe {
        crate::arch::x86_64::execution::prepare_context(
            &mut state.cells[index].context,
            stack,
            trampoline,
        )?;
    }
    Ok(())
}

pub fn configure_user_entry(cell: CellId, rip: u64, rsp: u64) -> Result<(), &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty { return Err("cell does not exist"); }
    state.cells[index].configure_user_entry(rip, rsp)
}

pub fn user_entry(cell: CellId) -> Option<(u64, u64)> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty { return None; }
    state.cells[index].user_entry()
}

pub fn cell_address_space_root(cell: CellId) -> Option<u64> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        None
    } else {
        Some(state.cells[index].address_space_root)
    }
}

pub fn kernel_stack_top(cell: CellId) -> Option<u64> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty { None } else { Some(state.cells[index].kernel_stack_top) }
}

pub fn finish_cell(cell: CellId, action: super::CellAction) -> Result<(), &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    state.cells[index].state = match action {
        super::CellAction::Yield => CellState::Ready,
        super::CellAction::Wait => CellState::Waiting,
        super::CellAction::Stop => CellState::Stopped,
    };
    Ok(())
}

pub fn bind_entry(cell: CellId, entry: super::CellEntry) -> Result<(), &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    state.cells[index].bind_entry(entry)
}

pub fn run_once(cell: CellId) -> Result<super::CellAction, &'static str> {
    let entry = {
        let mut guard = RUNTIME.lock();
        let state = guard.get_mut();
        let index = cell.0 as usize;
        if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
            return Err("cell does not exist");
        }
        let entry = state.cells[index].entry.ok_or("cell has no entry")?;
        state.cells[index].state = CellState::Running;
        entry
    };

    let action = entry();

    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS { return Err("cell disappeared"); }
    state.cells[index].state = match action {
        super::CellAction::Yield => CellState::Ready,
        super::CellAction::Wait => CellState::Waiting,
        super::CellAction::Stop => CellState::Stopped,
    };
    Ok(action)
}

pub fn memory_map(
    cell: CellId,
    capability: CapabilityId,
    object: ObjectId,
    virtual_address: u64,
    writable: bool,
) -> Result<u64, &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let cell_index = cell.0 as usize;
    if cell_index >= MAX_CELLS || state.cells[cell_index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    if !state.cells[cell_index].permits(capability, object, CapabilityRights::MAP) {
        return Err("memory map capability denied");
    }

    let object_index = object.index();
    if object_index >= MAX_OBJECTS
        || state.objects[object_index].id != object
        || !matches!(state.objects[object_index].kind, ObjectKind::Memory | ObjectKind::Data)
    {
        return Err("object is not mappable memory");
    }

    let frame = state.memory_frames[object_index];
    if frame == 0 {
        return Err("memory object has no backing frame");
    }
    if writable && !state.cells[cell_index].permits(capability, object, CapabilityRights::WRITE) {
        return Err("writable mapping requires WRITE capability");
    }

    let root = state.cells[cell_index].address_space_root;
    crate::memory::paging::map_user_page_in_root(
        root,
        virtual_address & !(crate::memory::PAGE_SIZE - 1),
        frame,
        writable,
        false,
    )?;
    Ok(virtual_address & !(crate::memory::PAGE_SIZE - 1))
}

pub fn grant(cell: CellId, object: ObjectId, rights: CapabilityRights) -> Result<CapabilityId, &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    if !object_exists_unlocked(state, object) { return Err("object does not exist"); }
    state.cells[index].grant(object, rights)
}

pub fn revoke(cell: CellId, capability: CapabilityId) -> Result<(), &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    state.cells[index].revoke(capability)
}

pub fn permits(cell: CellId, capability: CapabilityId, object: ObjectId, rights: CapabilityRights) -> bool {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    index < MAX_CELLS
        && state.cells[index].state != CellState::Empty
        && state.cells[index].permits(capability, object, rights)
}

pub fn emit(event: Event) -> Result<(), &'static str> {
    let mut guard = RUNTIME.lock();
    emit_unlocked(guard.get_mut(), event)
}

pub fn poll() -> Option<Event> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    if state.event_head == state.event_tail { return None; }
    let event = state.events[state.event_head];
    state.event_head = (state.event_head + 1) % EVENT_QUEUE_SIZE;
    Some(event)
}

pub fn object_exists(id: ObjectId) -> bool {
    let guard = RUNTIME.lock();
    object_exists_unlocked(guard.get(), id)
}

pub fn cell_state(id: CellId) -> Option<CellState> {
    let guard = RUNTIME.lock();
    let index = id.0 as usize;
    if index >= MAX_CELLS { return None; }
    let state = guard.get();
    if state.cells[index].state == CellState::Empty { None } else { Some(state.cells[index].state) }
}
