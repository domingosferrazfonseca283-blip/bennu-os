use super::{
    CapabilityId, CapabilityRights, Cell, CellId, CellState, Event, ObjectId, ObjectKind,
    ResourceObject, EventKind, MAX_CELLS, MAX_OBJECTS,
};
use super::sync::SpinLock;

const EVENT_QUEUE_SIZE: usize = 128;
const MAX_MEMORY_PAGES: usize = 256;
const MAX_BENNUFS_MOUNTS: usize = 16;
const BENNUFS_PROBE_ADDRESS: u64 = 0x0000_0070_0000_0000;
const BENNUFS_PROBE_TOKEN_BASE: u64 = 0x4245_4e4e_5546_5300;

struct RuntimeState {
    objects: [ResourceObject; MAX_OBJECTS],
    generations: [u32; MAX_OBJECTS],
    cells: [Cell; MAX_CELLS],
    events: [Event; EVENT_QUEUE_SIZE],
    event_head: usize,
    event_tail: usize,
    next_object: usize,
    memory_frames: [[u64; MAX_MEMORY_PAGES]; MAX_OBJECTS],
    block_devices: [super::BlockDevice; super::storage::MAX_BLOCK_DEVICES],
    bennufs_mounts: [super::filesystem::BennuFsMount; MAX_BENNUFS_MOUNTS],
    bennufs_probe_device: ObjectId,
    bennufs_probe_token: u64,
    bennufs_probe_buffer: ObjectId,
    bennufs_probe_phase: u8,
    bennufs_probe_superblock: super::filesystem::Superblock,
    bennufs_probe_next_block: u64,
    bennufs_probe_journal_end: u64,
    bennufs_probe_transaction: u64,
    bennufs_probe_highest: u64,
    bennufs_probe_root_node: super::filesystem::Node,
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
        memory_frames: [[0; MAX_MEMORY_PAGES]; MAX_OBJECTS],
        block_devices: [super::BlockDevice::EMPTY; super::storage::MAX_BLOCK_DEVICES],
        bennufs_mounts: [super::filesystem::BennuFsMount::EMPTY; MAX_BENNUFS_MOUNTS],
        bennufs_probe_device: ObjectId::NULL,
        bennufs_probe_token: 0,
        bennufs_probe_buffer: ObjectId::NULL,
        bennufs_probe_phase: 0,
        bennufs_probe_superblock: super::filesystem::Superblock::EMPTY,
        bennufs_probe_next_block: 0,
        bennufs_probe_journal_end: 0,
        bennufs_probe_transaction: 0,
        bennufs_probe_highest: 0,
        bennufs_probe_root_node: super::filesystem::Node::EMPTY,
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

pub fn service_block_io() {
    let envelope = match super::io_fabric::begin_next() { Some(v) => v, None => return };
    let target = cell_root_object(CellId(envelope.request.owner_cell as u32));
    let mut device = super::BlockDevice::EMPTY;
    {
        let guard = RUNTIME.lock();
        let state = guard.get();
        for i in 0..super::storage::MAX_BLOCK_DEVICES {
            if state.block_devices[i].object == envelope.request.device {
                device = state.block_devices[i];
                break;
            }
        }
    }
    if device.object.is_null() {
        let _ = emit(super::Event::new(
            super::EventKind::DeviceTransferCompleted,
            envelope.request.device,
            target,
            ((super::IoStatus::NoDevice as u64) << 56) | (envelope.request.token & 0x00ff_ffff_ffff_ffff),
        ));
        return;
    }
    if !device.is_usb_storage() {
        let _ = emit(super::Event::new(
            super::EventKind::DeviceTransferCompleted,
            envelope.request.device,
            target,
            ((super::IoStatus::Invalid as u64) << 56) | (envelope.request.token & 0x00ff_ffff_ffff_ffff),
        ));
        return;
    }
    let result = DEVICE_FABRIC.lock().get_mut().submit_block_request(&envelope.request);
    if result.is_err() {
        let _ = emit(super::Event::new(
            super::EventKind::DeviceTransferCompleted,
            envelope.request.device,
            target,
            ((super::IoStatus::Hardware as u64) << 56) | (envelope.request.token & 0x00ff_ffff_ffff_ffff),
        ));
    } else {
        let _ = emit(super::Event::new(
            super::EventKind::DeviceQueued,
            envelope.request.device,
            target,
            envelope.request.token,
        ));
    }
}

pub fn start_usb_enumeration(owner_cell:u64) -> Result<usize,&'static str> {
    DEVICE_FABRIC.lock().get_mut().start_usb_enumeration(owner_cell)
}

pub fn service_device_io() {
    let request = match super::io_fabric::begin_device() { Some(r) => r, None => return };
    let target = cell_root_object(CellId(request.owner_cell as u32));
    let result = DEVICE_FABRIC.lock().get_mut().submit(&request);
    let kind = if result.is_ok() { super::EventKind::DeviceQueued } else { super::EventKind::ResourceChanged };
    let _ = emit(super::Event::new(kind, request.device, target, request.token));
}

pub fn service_device_events() {
    let completion = DEVICE_FABRIC.lock().get_mut().service_events();
    if let Some((device, token, slot, completion_code, operation, owner_cell, descriptor)) = completion {
        let mut published = super::ObjectId::NULL;
        if operation == 11 {
            complete_bennufs_probe(token, completion_code);
        }
        if operation == 1 && completion_code == 1 {
            let _ = DEVICE_FABRIC.lock().get_mut().continue_usb_enumeration(
                device, slot, 1, owner_cell, token
            );
        } else if operation == 3 && completion_code == 1 {
            let _ = DEVICE_FABRIC.lock().get_mut().fetch_usb_device_descriptor(
                device, slot, owner_cell, token
            );
        } else if operation == 2 {
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
                        let _ = prepare_bennufs_mount(object);
                        published = object;
                    }
                }
            }
        }
        let target = cell_root_object(CellId(owner_cell as u32));
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
        let _ = emit(super::Event::new(kind, source, target, value));
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

fn submit_bennufs_probe_read(
    device: ObjectId,
    buffer: ObjectId,
    owner: CellId,
    lba: u64,
    token: u64,
) -> Result<(), &'static str> {
    let request = super::BlockRequest {
        owner_cell: owner.0 as u64,
        operation: super::BlockOp::Read,
        device,
        lba,
        blocks: 1,
        buffer: BENNUFS_PROBE_ADDRESS,
        token,
    };
    let block = {
        let guard = RUNTIME.lock();
        let state = guard.get();
        let mut found = super::BlockDevice::EMPTY;
        for i in 0..super::storage::MAX_BLOCK_DEVICES {
            if state.block_devices[i].object == device {
                found = state.block_devices[i];
                break;
            }
        }
        found
    };
    super::io_fabric::submit(request, block, false)
}

pub fn prepare_bennufs_mount(device: ObjectId) -> Result<(), &'static str> {
    let (geometry, owner) = {
        let guard = RUNTIME.lock();
        let state = guard.get();
        let mut geometry = None;
        for i in 0..super::storage::MAX_BLOCK_DEVICES {
            let block = state.block_devices[i];
            if block.object == device {
                geometry = Some(block.geometry);
                break;
            }
        }
        (geometry.ok_or("block device not found")?, CellId(2))
    };

    if !super::filesystem::validate_block_geometry(
        geometry.block_size,
        geometry.block_count,
    ) {
        return Err("block device geometry is incompatible with BennuFS");
    }

    {
        let guard = RUNTIME.lock();
        let state = guard.get();
        for i in 0..MAX_BENNUFS_MOUNTS {
            if state.bennufs_mounts[i].mounted && state.bennufs_mounts[i].device == device {
                return Ok(());
            }
        }
        if !state.bennufs_probe_device.is_null() {
            return Err("another BennuFS superblock probe is pending");
        }
    }

    let buffer = create_memory_object(owner.0, 1)?;
    let capability = grant(
        owner,
        buffer,
        super::CapabilityRights::READ
            .union(super::CapabilityRights::WRITE)
            .union(super::CapabilityRights::MAP),
    )?;
    let _ = memory_map(owner, capability, buffer, BENNUFS_PROBE_ADDRESS, true)?;

    let token = BENNUFS_PROBE_TOKEN_BASE
        .wrapping_add(device.0 as u64)
        .wrapping_add(1);
    let request = super::BlockRequest {
        owner_cell: owner.0 as u64,
        operation: super::BlockOp::Read,
        device,
        lba: super::filesystem::BENNUFS_SUPERBLOCK_BLOCK,
        blocks: 1,
        buffer: BENNUFS_PROBE_ADDRESS,
        token,
    };
    super::io_fabric::submit(request, {
        let guard = RUNTIME.lock();
        let state = guard.get();
        let mut found = super::BlockDevice::EMPTY;
        for i in 0..super::storage::MAX_BLOCK_DEVICES {
            if state.block_devices[i].object == device {
                found = state.block_devices[i];
                break;
            }
        }
        found
    }, false)?;

    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    state.bennufs_probe_device = device;
    state.bennufs_probe_token = token;
    state.bennufs_probe_buffer = buffer;
    Ok(())
}

pub fn complete_bennufs_probe(token: u64, completion_code: u8) {
    let (device, buffer, phase) = {
        let guard = RUNTIME.lock();
        let state = guard.get();
        if state.bennufs_probe_token != token || state.bennufs_probe_device.is_null() {
            return;
        }
        (state.bennufs_probe_device, state.bennufs_probe_buffer, state.bennufs_probe_phase)
    };

    let clear_probe = || {
        let mut guard = RUNTIME.lock();
        let state = guard.get_mut();
        state.bennufs_probe_device = ObjectId::NULL;
        state.bennufs_probe_token = 0;
        state.bennufs_probe_buffer = ObjectId::NULL;
        state.bennufs_probe_phase = 0;
        state.bennufs_probe_superblock = super::filesystem::Superblock::EMPTY;
        state.bennufs_probe_next_block = 0;
        state.bennufs_probe_journal_end = 0;
        state.bennufs_probe_transaction = 0;
        state.bennufs_probe_highest = 0;
        state.bennufs_probe_root_node = super::filesystem::Node::EMPTY;
    };

    if completion_code != 1 {
        clear_probe();
        return;
    }

    let mut bytes = [0u8; super::filesystem::BENNUFS_BLOCK_SIZE as usize];
    let root = match cell_address_space_root(CellId(2)) {
        Some(value) if value != 0 => value,
        _ => {
            clear_probe();
            return;
        }
    };
    if crate::memory::user::copy_from_user(
        root,
        bytes.as_mut_ptr(),
        BENNUFS_PROBE_ADDRESS,
        bytes.len() as u64,
    ).is_err() {
        clear_probe();
        return;
    }

    if phase == 1 {
        let superblock = match super::filesystem::deserialize_superblock(&bytes) {
            Some(value) => value,
            None => {
                clear_probe();
                return;
            }
        };

        let geometry_ok = {
            let guard = RUNTIME.lock();
            let state = guard.get();
            let mut geometry = None;
            for i in 0..super::storage::MAX_BLOCK_DEVICES {
                if state.block_devices[i].object == device {
                    geometry = Some(state.block_devices[i].geometry);
                    break;
                }
            }
            geometry.map(|g| {
                g.block_size == super::filesystem::BENNUFS_BLOCK_SIZE
                    && g.block_count == superblock.total_blocks
            }).unwrap_or(false)
        };
        if !geometry_ok {
            clear_probe();
            return;
        }

        let journal_end = match superblock.journal_start.checked_add(superblock.journal_blocks) {
            Some(value) if value <= superblock.total_blocks => value,
            _ => {
                clear_probe();
                return;
            }
        };

        if superblock.journal_blocks == 0 {
            let mut mount = superblock;
            mount.sequence = superblock.sequence;
            let mut guard = RUNTIME.lock();
            let state = guard.get_mut();
            for i in 0..MAX_BENNUFS_MOUNTS {
                if !state.bennufs_mounts[i].mounted {
                    state.bennufs_mounts[i] = super::filesystem::BennuFsMount {
                        device, superblock: mount, mounted: true
                    };
                    let target = state.cells[2].root;
                    let _ = emit_unlocked(
                        state,
                        Event::new(EventKind::ResourceChanged, device, state.cells[2].root, mount.sequence),
                    );
                    drop(guard);
                    clear_probe();
                    return;
                }
            }
            drop(guard);
            clear_probe();
            return;
        }

        let next_block = superblock.journal_start;
        let next_token = BENNUFS_PROBE_TOKEN_BASE
            .wrapping_add(device.0 as u64)
            .wrapping_add(next_block);
        if submit_bennufs_probe_read(
            device, buffer, CellId(2), next_block, next_token
        ).is_err() {
            clear_probe();
            return;
        }

        let mut guard = RUNTIME.lock();
        let state = guard.get_mut();
        state.bennufs_probe_token = next_token;
        state.bennufs_probe_phase = 2;
        state.bennufs_probe_superblock = superblock;
        state.bennufs_probe_next_block = next_block;
        state.bennufs_probe_journal_end = journal_end;
        state.bennufs_probe_transaction = 0;
        state.bennufs_probe_highest = superblock.sequence;
        return;
    }

    if phase == 3 {
        let superblock = {
            let guard = RUNTIME.lock();
            guard.get().bennufs_probe_superblock
        };
        let node = match super::filesystem::deserialize_node(&bytes) {
            Some(value) => value,
            None => {
                clear_probe();
                return;
            }
        };
        if node.id != superblock.root_object
            || !node.valid_extent(superblock.total_blocks)
        {
            clear_probe();
            return;
        }

        let mut guard = RUNTIME.lock();
        let state = guard.get_mut();
        state.bennufs_probe_root_node = node;
        let mut mounted_superblock = superblock;
        mounted_superblock.sequence = state.bennufs_probe_highest;
        for i in 0..MAX_BENNUFS_MOUNTS {
            if !state.bennufs_mounts[i].mounted {
                state.bennufs_mounts[i] = super::filesystem::BennuFsMount {
                    device,
                    superblock: mounted_superblock,
                    mounted: true,
                };
                let target = state.cells[2].root;
                let _ = emit_unlocked(
                    state,
                    Event::new(
                        EventKind::ResourceChanged,
                        device,
                        target,
                        mounted_superblock.sequence,
                    ),
                );
                drop(guard);
                clear_probe();
                return;
            }
        }
        drop(guard);
        clear_probe();
        return;
    }

    if phase != 2 {
        clear_probe();
        return;
    }

    let (superblock, block, journal_end, mut transaction, mut highest) = {
        let guard = RUNTIME.lock();
        let state = guard.get();
        (
            state.bennufs_probe_superblock,
            state.bennufs_probe_next_block,
            state.bennufs_probe_journal_end,
            state.bennufs_probe_transaction,
            state.bennufs_probe_highest,
        )
    };

    let record_size = core::mem::size_of::<super::filesystem::JournalRecord>();
    if record_size == 0 || record_size > bytes.len() {
        clear_probe();
        return;
    }

    let records = bytes.len() / record_size;
    for index in 0..records {
        let offset = index * record_size;
        let record = unsafe {
            core::ptr::read_unaligned(
                bytes.as_ptr().add(offset)
                    as *const super::filesystem::JournalRecord
            )
        };
        if !record.valid_for(&superblock) || record.sequence < superblock.sequence {
            continue;
        }

        match record.operation {
            super::filesystem::JournalOp::Begin => {
                transaction = record.sequence;
            }
            super::filesystem::JournalOp::Commit
                if transaction == record.sequence =>
            {
                if record.sequence > highest {
                    highest = record.sequence;
                }
                transaction = 0;
            }
            super::filesystem::JournalOp::Checkpoint => {
                if record.sequence > highest {
                    highest = record.sequence;
                }
            }
            _ => {}
        }
    }

    let next_block = match block.checked_add(1) {
        Some(value) => value,
        None => {
            clear_probe();
            return;
        }
    };

    if next_block < journal_end {
        let next_token = BENNUFS_PROBE_TOKEN_BASE
            .wrapping_add(device.0 as u64)
            .wrapping_add(next_block);
        if submit_bennufs_probe_read(
            device, buffer, CellId(2), next_block, next_token
        ).is_err() {
            clear_probe();
            return;
        }

        let mut guard = RUNTIME.lock();
        let state = guard.get_mut();
        state.bennufs_probe_token = next_token;
        state.bennufs_probe_next_block = next_block;
        state.bennufs_probe_transaction = transaction;
        state.bennufs_probe_highest = highest;
        return;
    }

    let root_block = match super::filesystem::root_node_block(&superblock) {
        Some(value) => value,
        None => {
            clear_probe();
            return;
        }
    };
    let next_token = BENNUFS_PROBE_TOKEN_BASE
        .wrapping_add(device.0 as u64)
        .wrapping_add(root_block);
    if submit_bennufs_probe_read(device, buffer, CellId(2), root_block, next_token).is_err() {
        clear_probe();
        return;
    }

    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    state.bennufs_probe_token = next_token;
    state.bennufs_probe_phase = 3;
    state.bennufs_probe_transaction = transaction;
    state.bennufs_probe_highest = highest;
    return;

}

pub fn bennufs_mount(device: ObjectId) -> Option<super::filesystem::BennuFsMount> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    for i in 0..MAX_BENNUFS_MOUNTS {
        if state.bennufs_mounts[i].mounted && state.bennufs_mounts[i].device == device {
            return Some(state.bennufs_mounts[i]);
        }
    }
    None
}

pub fn create_object(kind: ObjectKind, owner: u32) -> Result<ObjectId, &'static str> {
    create_object_with_pages(kind, owner, 1)
}

pub fn create_memory_object(owner: u32, pages: usize) -> Result<ObjectId, &'static str> {
    create_object_with_pages(ObjectKind::Memory, owner, pages)
}

fn create_object_with_pages(
    kind: ObjectKind,
    owner: u32,
    pages: usize,
) -> Result<ObjectId, &'static str> {
    if matches!(kind, ObjectKind::Memory | ObjectKind::Data)
        && (pages == 0 || pages > MAX_MEMORY_PAGES)
    {
        return Err("invalid memory object size");
    }
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();

    // The frame allocator is currently monotonic, so a partial multi-page
    // allocation cannot be rolled back. Check capacity before consuming any
    // frame, then publish the object only after every backing page exists.
    if matches!(kind, ObjectKind::Memory | ObjectKind::Data)
        && crate::memory::available_frames() < pages as u64
    {
        return Err("insufficient physical memory for object");
    }

    for offset in 0..MAX_OBJECTS {
        let index = (state.next_object + offset) % MAX_OBJECTS;
        if index == 0 || state.objects[index].kind != ObjectKind::Empty { continue; }

        let generation = state.generations[index].wrapping_add(1).max(1);
        let id = ObjectId::new(index as u32, generation);
        let mut frames = [0u64; MAX_MEMORY_PAGES];

        if matches!(kind, ObjectKind::Memory | ObjectKind::Data) {
            for page in 0..pages {
                frames[page] = crate::memory::allocate_frame_below(64 * 1024 * 1024)
                    .ok_or("physical memory changed during allocation")?;
            }
        }

        state.generations[index] = generation;
        state.objects[index] = ResourceObject { id, kind, owner, flags: 0 };
        state.memory_frames[index] = frames;
        state.next_object = (index + 1) % MAX_OBJECTS;
        let _ = emit_unlocked(state, Event::new(EventKind::ResourceCreated, id, ObjectId::NULL, 0));
        return Ok(id);
    }
    Err("object space exhausted")
}

pub fn object_frame(id: ObjectId) -> Option<u64> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    if !object_exists_unlocked(state, id) {
        return None;
    }
    let index = id.index();
    let frame = state.memory_frames[index][0];
    if frame == 0 { None } else { Some(frame) }
}

pub fn validate_graphics_buffer_mapping(
    cell: CellId,
    buffer: ObjectId,
) -> bool {
    let graphics = match super::graphics::get_for_owner(buffer, cell) {
        Some(value) => value,
        None => return false,
    };
    let (frames, page_count) = match object_frames(buffer) {
        Some(value) => value,
        None => return false,
    };
    if page_count == 0 || graphics.address & (crate::memory::PAGE_SIZE - 1) != 0 {
        return false;
    }

    let required = match graphics.required_bytes() {
        Some(value) => value,
        None => return false,
    };
    if required > graphics.size {
        return false;
    }
    let pages_needed = match (required
        .saturating_add(crate::memory::PAGE_SIZE - 1))
        .checked_div(crate::memory::PAGE_SIZE)
    {
        Some(value) => value as usize,
        None => return false,
    };
    if pages_needed == 0 || pages_needed > page_count {
        return false;
    }

    let root = match cell_address_space_root(cell) {
        Some(value) if value != 0 => value,
        _ => return false,
    };

    for page in 0..pages_needed {
        let address = match graphics.address.checked_add(
            (page as u64) * crate::memory::PAGE_SIZE
        ) {
            Some(value) => value,
            None => return false,
        };
        match crate::memory::paging::translate_user_address(root, address, false) {
            Some(physical) if physical == frames[page] => {}
            _ => return false,
        }
    }
    true
}

pub fn object_frames(id: ObjectId) -> Option<([u64; MAX_MEMORY_PAGES], usize)> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    if !object_exists_unlocked(state, id) {
        return None;
    }
    let kind = state.objects[id.index()].kind;
    if !matches!(kind, ObjectKind::Memory | ObjectKind::Data) {
        return None;
    }
    let mut frames = [0u64; MAX_MEMORY_PAGES];
    let mut count = 0usize;
    while count < MAX_MEMORY_PAGES && state.memory_frames[id.index()][count] != 0 {
        frames[count] = state.memory_frames[id.index()][count];
        count += 1;
    }
    Some((frames, count))
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
    // The bootstrap allocator returns individual 4 KiB frames. Keep the
    // execution stack exactly as large as the frame we actually own and map.
    let stack = crate::memory::allocate_frame_below(64 * 1024 * 1024)
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

pub fn start_cell(cell: CellId) -> Result<(), &'static str> {
    let mut guard = RUNTIME.lock();
    let state = guard.get_mut();
    let index = cell.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        return Err("cell does not exist");
    }
    if state.cells[index].state != CellState::Ready {
        return Err("cell is not ready");
    }
    state.cells[index].state = CellState::Running;
    Ok(())
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

    let frames = state.memory_frames[object_index];
    if frames[0] == 0 {
        return Err("memory object has no backing frame");
    }
    if writable && !state.cells[cell_index].permits(capability, object, CapabilityRights::WRITE) {
        return Err("writable mapping requires WRITE capability");
    }

    let root = state.cells[cell_index].address_space_root;
    let base = virtual_address & !(crate::memory::PAGE_SIZE - 1);
    for page in 0..MAX_MEMORY_PAGES {
        let frame = frames[page];
        if frame == 0 {
            break;
        }
        let offset = (page as u64)
            .checked_mul(crate::memory::PAGE_SIZE)
            .ok_or("memory mapping offset overflow")?;
        let va = base.checked_add(offset).ok_or("memory mapping address overflow")?;
        crate::memory::paging::map_user_page_in_root(
            root,
            va,
            frame,
            writable,
            false,
        )?;
    }
    Ok(base)
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

pub fn object_owner(id: ObjectId) -> Option<CellId> {
    let guard = RUNTIME.lock();
    let state = guard.get();
    if !object_exists_unlocked(state, id) {
        return None;
    }
    Some(CellId(state.objects[id.index()].owner))
}

pub fn present_surface(
    cell: CellId,
    capability: CapabilityId,
    present: &super::graphics::Present,
) -> Result<(), &'static str> {
    if present.surface.is_null() || present.buffer.is_null()
        || present.width == 0 || present.height == 0
        || present.x < 0 || present.y < 0
    {
        return Err("invalid present request");
    }

    if !permits(cell, capability, present.surface, CapabilityRights::DRAW) {
        return Err("surface draw capability denied");
    }

    let surface = super::surface::find_by_object(present.surface)
        .ok_or("surface does not exist")?;

    if object_owner(present.surface) != Some(cell) {
        return Err("surface is not owned by Cell");
    }
    if surface.buffer != present.buffer {
        return Err("present buffer does not match surface");
    }
    if super::graphics::get_for_owner(present.buffer, cell).is_none() {
        return Err("present buffer is not owned by Cell");
    }

    let right = present.x as u64 + present.width as u64;
    let bottom = present.y as u64 + present.height as u64;
    if right > surface.width as u64 || bottom > surface.height as u64 {
        return Err("present rectangle exceeds surface");
    }

    let target = cell_root_object(cell);
    emit(super::Event::new(
        EventKind::ResourceChanged,
        present.surface,
        target,
        present.buffer.0,
    ))?;
    super::compositor::mark_dirty();
    Ok(())
}

pub fn object_exists(id: ObjectId) -> bool {
    let guard = RUNTIME.lock();
    object_exists_unlocked(guard.get(), id)
}

pub fn cell_root_object(id: CellId) -> ObjectId {
    let guard = RUNTIME.lock();
    let state = guard.get();
    let index = id.0 as usize;
    if index >= MAX_CELLS || state.cells[index].state == CellState::Empty {
        ObjectId::NULL
    } else {
        state.cells[index].root
    }
}

pub fn cell_state(id: CellId) -> Option<CellState> {
    let guard = RUNTIME.lock();
    let index = id.0 as usize;
    if index >= MAX_CELLS { return None; }
    let state = guard.get();
    if state.cells[index].state == CellState::Empty { None } else { Some(state.cells[index].state) }
}
