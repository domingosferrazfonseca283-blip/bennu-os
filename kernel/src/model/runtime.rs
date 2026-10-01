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
    block_devices: [super::BlockDevice; super::storage::MAX_BLOCK_DEVICES],
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
        block_devices: [super::BlockDevice::EMPTY; super::storage::MAX_BLOCK_DEVICES],
    };
}

static RUNTIME: SpinLock<RuntimeState> = SpinLock::new(RuntimeState::EMPTY);

static DEVICE_FABRIC: SpinLock<super::DeviceFabric> = SpinLock::new(super::DeviceFabric::empty());

pub fn register_device_fabric_pci(pci: super::PciDevice, class: super::DeviceClass) -> Result<ObjectId, &'static str> {
    DEVICE_FABRIC.lock().get_mut().register_pci(pci, class)
}

pub fn register_xhci_controller(controller: super::XhciController) -> Result<(), &'static str> {
    DEVICE_FABRIC.lock().get_mut().register_xhci(controller)
}

pub fn service_device_io() {
    let request = match super::io_fabric::begin_device() { Some(r) => r, None => return };
    let result = DEVICE_FABRIC.lock().get_mut().submit(&request);
    let kind = if result.is_ok() { super::EventKind::DeviceQueued } else { super::EventKind::ResourceChanged };
    let _ = emit(super::Event::new(kind, request.device, super::ObjectId::NULL, request.token));
}

pub fn service_device_events() {
    let completion = DEVICE_FABRIC.lock().get_mut().service_events();
    if let Some((device, token, slot, completion_code, operation, owner_cell, descriptor)) = completion {
        let mut published = super::ObjectId::NULL;
        if operation == 2 {
            if let Some(desc) = descriptor {
                if let Ok(object) = create_object(super::ObjectKind::Device, owner_cell as u32) {
                    if DEVICE_FABRIC.lock().get_mut().attach_usb_descriptor(slot, device, object, desc).is_ok() {
                        let _ = grant(CellId(owner_cell as u32), object, super::CapabilityRights::READ.union(super::CapabilityRights::WRITE).union(super::CapabilityRights::OBSERVE).union(super::CapabilityRights::DEVICE));
                        published = object;
                    }
                }
            }
        } else if operation == 5 && completion_code == 1 {
            let _ = DEVICE_FABRIC.lock().get_mut().configure_mass_storage_endpoints(
                device, token, owner_cell
            );
        } else if operation == 6 && completion_code == 1 {
            let _ = DEVICE_FABRIC.lock().get_mut().mass_storage_endpoint_command_completed(
                device, token, owner_cell
            );
        } else if operation == 11 && completion_code == 1 {
            let geometry = DEVICE_FABRIC.lock().get_mut().mass_storage_geometry(device);
            if let Some((block_size, block_count, existing)) = geometry {
                if existing.is_null() {
                    if let Ok(object) = create_object(super::ObjectKind::Device, owner_cell as u32) {
                        let block = super::BlockDevice {
                            object,
                            kind: super::BlockKind::UsbMassStorage,
                            geometry: super::BlockGeometry { block_size, block_count },
                            removable: true,
                            writable: true,
                        };
                        let mut guard = RUNTIME.lock();
                        let state = guard.get_mut();
                        for i in 0..super::storage::MAX_BLOCK_DEVICES {
                            if state.block_devices[i].object.is_null() {
                                state.block_devices[i]=block;
                                break;
                            }
                        }
                        drop(guard);
                        let _ = DEVICE_FABRIC.lock().get_mut().bind_mass_storage_object(device, object);
                        let _ = grant(CellId(owner_cell as u32), object, super::CapabilityRights::READ.union(super::CapabilityRights::WRITE).union(super::CapabilityRights::DEVICE).union(super::CapabilityRights::OBSERVE));
                        published = object;
                    }
                }
            }
        }
        let kind = if operation == 2 {
            super::EventKind::DeviceTransferCompleted
        } else {
            super::EventKind::DeviceCommandCompleted
        };
        let value = ((completion_code as u64) << 56)
            | ((slot as u64) << 48)
            | ((owner_cell & 0xffff) << 32)
            | (token & 0xffff_ffff);
        let source = if published.is_null() { device } else { published };
        let _ = emit(super::Event::new(kind, source, super::ObjectId::NULL, value));
    }
}


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

pub fn delegate(
    source_cell: CellId,
    source_capability: CapabilityId,
    target_cell: CellId,
    object: ObjectId,
    rights: CapabilityRights,
) -> Result<CapabilityId, &'static str> {
    if !rights.is_valid() {
        return Err("invalid capability rights");
    }

    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();

    let source_index = source_cell.0 as usize;
    let target_index = target_cell.0 as usize;
    if source_index >= MAX_CELLS || state.cells[source_index].state == CellState::Empty {
        return Err("source cell does not exist");
    }
    if target_index >= MAX_CELLS || state.cells[target_index].state == CellState::Empty {
        return Err("target cell does not exist");
    }
    if !object_exists_unlocked(state, object) {
        return Err("object does not exist");
    }

    let source_rights = state.cells[source_index]
        .capability_rights(source_capability, object)
        .ok_or("source capability not found")?;

    let administrative = source_rights.contains(CapabilityRights::ADMIN);
    if !administrative {
        if !source_rights.contains(CapabilityRights::SHARE) {
            return Err("capability is not delegable");
        }
        if !rights.is_subset_of(source_rights) {
            return Err("delegation exceeds source authority");
        }
    }

    state.cells[target_index].grant(object, rights)
}

pub fn grant(cell: CellId, object: ObjectId, rights: CapabilityRights) -> Result<CapabilityId, &'static str> {
    if !rights.is_valid() {
        return Err("invalid capability rights");
    }
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    if !object_exists_unlocked(state, object) { return Err("object does not exist"); }
    state.cells[index].grant(object, rights)
}

pub fn device_submit(
    cell: CellId,
    capability: CapabilityId,
    object: ObjectId,
    opcode: u32,
    flags: u32,
    argument: u64,
    value: u64,
    buffer: u64,
    length: u64,
    token: u64,
) -> Result<(), &'static str> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    if !state.cells[index].permits(capability, object, CapabilityRights::DEVICE.union(CapabilityRights::WRITE)) {
        return Err("device capability denied");
    }
    if !object_exists_unlocked(state, object) || state.objects[object.index()].kind != ObjectKind::Device {
        return Err("object is not a device");
    }
    drop(guard);
    super::io_fabric::submit_device(super::DeviceRequest {
        owner_cell: cell.0 as u64,
        device: object, opcode, flags, argument, value, buffer, length, token,
    })
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

pub fn permits_any(cell: CellId, capability: CapabilityId, object: ObjectId, rights: &[CapabilityRights]) -> bool {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty { return false; }
    for required in rights { if state.cells[index].permits(capability, object, *required) { return true; } }
    false
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

pub fn poll_for_object(object: ObjectId) -> Option<Event> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let mut cursor = state.event_head;
    while cursor != state.event_tail {
        let event = state.events[cursor];
        if event.source == object || event.target == object {
            let mut current = cursor;
            loop {
                let next = (current + 1) % EVENT_QUEUE_SIZE;
                if next == state.event_tail { break; }
                state.events[current] = state.events[next];
                current = next;
            }
            state.event_tail = if state.event_tail == 0 {
                EVENT_QUEUE_SIZE - 1
            } else {
                state.event_tail - 1
            };
            return Some(event);
        }
        cursor = (cursor + 1) % EVENT_QUEUE_SIZE;
    }
    None
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
