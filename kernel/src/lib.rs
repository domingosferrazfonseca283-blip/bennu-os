#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod boot_info;
pub mod memory;
pub mod model;
pub mod graphics;
pub mod drivers;
pub mod process;

const DEMO_BEXE: [u8; 33] = [
    b'B', b'E', b'X', b'E', 1, 0, 0x3e, 0,
    0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00,
    9, 0, 0, 0, 0, 0, 0, 0, 0xb8, 9, 0, 0, 0, 0xcd, 0x80, 0xeb, 0xfc,
];

fn arch_user_install(cell: model::CellId, root: u64) -> Result<(), &'static str> {
    let process = process::load_bexe(cell, root, &DEMO_BEXE)?;
    if process.entry != process::USER_IMAGE_BASE { return Err("demo BEXE entry was not installed at image base"); }
    Ok(())
}

extern "C" fn bootstrap_cell() -> model::CellAction { model::CellAction::Stop }

fn render_compositor(boot_info: &boot_info::BootInfo) {
    if !model::compositor::take_dirty() { return; }
    let mut commands = [model::compositor::CompositionCommand {
        window: model::WindowId::NULL, surface: model::ObjectId::NULL,
        source_x: 0, source_y: 0, destination_x: 0, destination_y: 0,
        width: 0, height: 0, buffer: 0, stride: 0,
        format: model::PixelFormat::Unknown,
    }; model::compositor::MAX_VISIBLE_RECTS];
    let count = model::compositor::build_visible_commands(
        boot_info.framebuffer_width, boot_info.framebuffer_height, &mut commands);
    let _ = graphics::present(boot_info, &commands[..count]);
}

fn init_graphics_surface(boot_info: &boot_info::BootInfo) {
    let memory = match model::runtime::create_object(model::ObjectKind::Memory, 0) { Ok(object) => object, Err(_) => return };
    let frame = match model::runtime::object_frame(memory) { Some(frame) => frame, None => return };
    let mapped = match memory::paging::map_mmio(frame, memory::PAGE_SIZE) { Ok(address) => address, Err(_) => return };
    let width = 32u32; let height = 32u32; let stride = width * 4;
    unsafe {
        let pixels = mapped as *mut u32;
        for y in 0..height { for x in 0..width {
            let border = x == 0 || y == 0 || x == width - 1 || y == height - 1;
            let value = if border { 0xff40a0ff } else if ((x / 4) + (y / 4)) & 1 == 0 { 0xff182840 } else { 0xff203048 };
            core::ptr::write_volatile(pixels.add((y * width + x) as usize), value);
        }}
    }
    if model::graphics::register(model::Buffer {
        object: memory, owner: model::CellId(0), address: mapped, size: memory::PAGE_SIZE,
        stride, width, height, format: model::PixelFormat::Rgba8888,
    }).is_err() { return; }
    let surface = match model::runtime::create_object(model::ObjectKind::Surface, 0) { Ok(object) => object, Err(_) => return };
    if model::surface::attach(surface, width, height, memory).is_err() { return; }
    if model::window::create(model::ObjectId::new(1, 1), surface, 40, 190, width, height).is_err() { return; }
    let mut commands = [model::compositor::CompositionCommand {
        window: model::WindowId::NULL, surface: model::ObjectId::NULL,
        source_x: 0, source_y: 0, destination_x: 0, destination_y: 0,
        width: 0, height: 0, buffer: 0, stride, format: model::PixelFormat::Rgba8888,
    }; model::compositor::MAX_VISIBLE_RECTS];
    let count = model::compositor::build_visible_commands(
        boot_info.framebuffer_width, boot_info.framebuffer_height, &mut commands);
    let _ = graphics::present(boot_info, &commands[..count]);
}

pub fn init(boot_info: *const boot_info::BootInfo) {
    arch::init(); drivers::keyboard::init(); drivers::console::init();
    model::runtime::init(); model::entity::init(); model::intelligence::init();
    model::graphics::init(); model::surface::init(); model::window::init();
    model::init_io_fabric(model::policy::AccessPolicy::USB_FIRST);
    let root = match model::runtime::create_object(model::ObjectKind::Cell, 0) { Ok(root) => root, Err(_) => { arch::diagnostics::write_line(1, b"BENNU MODEL: OBJECT SPACE FAILED"); return; }};
    if model::runtime::create_cell(model::CellId(1), root).is_err() { arch::diagnostics::write_line(1, b"BENNU MODEL: ROOT CELL FAILED"); return; }
    arch::diagnostics::write_line(1, b"BENNU MODEL: OBJECT + CELL FABRIC ONLINE");
    arch::diagnostics::write_line(6, b"BENNU ENTITY: IDENTITY + SELF MODEL ONLINE");
    if let Ok(device) = model::runtime::create_object(model::ObjectKind::Device, 0) {
        let _ = model::graph::link(root, device, model::RelationKind::Contains);
        let _ = model::runtime::grant(model::CellId(1), device, model::CapabilityRights::OBSERVE);
        arch::diagnostics::write_line(1, b"BENNU MODEL: CAPABILITY GRAPH ONLINE");
    }
    if boot_info.is_null() { arch::diagnostics::write_line(4, b"BENNU BOOT: NULL BOOT INFO"); return; }
    let boot_info = unsafe { &*boot_info };
    if let Err(_) = memory::init(boot_info) { arch::diagnostics::write_line(4, b"BENNU MEMORY: INITIALIZATION FAILED"); return; }
    arch::diagnostics::write_line(4, b"BENNU MEMORY: E820 + FRAME ALLOCATOR ONLINE");
    if let Err(_) = memory::paging::init() { arch::diagnostics::write_line(5, b"BENNU MEMORY: PAGING INITIALIZATION FAILED"); return; }
    arch::diagnostics::write_line(5, b"BENNU MEMORY: KERNEL PAGING ONLINE");
    if let Err(_) = graphics::init(boot_info) { arch::diagnostics::write_line(5, b"BENNU VIDEO: NO VBE FRAMEBUFFER"); } else { arch::diagnostics::write_line(6, b"BENNU VIDEO: FRAMEBUFFER ONLINE"); }
    init_graphics_surface(boot_info);
    unsafe { memory::heap::init(); } arch::diagnostics::write_line(6, b"BENNU MEMORY: KERNEL HEAP ONLINE");
    let user_root_object = match model::runtime::create_object(model::ObjectKind::Cell, 0) { Ok(object) => object, Err(_) => { arch::diagnostics::write_line(1, b"BENNU USER: ROOT OBJECT FAILED"); return; }};
    if model::runtime::create_cell(model::CellId(2), user_root_object).is_err() { arch::diagnostics::write_line(1, b"BENNU USER: CELL CREATE FAILED"); return; }
    if model::runtime::prepare_cell_context(model::CellId(2), model::scheduler::cell_trampoline as usize as u64).is_err() { arch::diagnostics::write_line(1, b"BENNU USER: CELL CONTEXT FAILED"); return; }
    let user_root = match model::runtime::context_ptr(model::CellId(2)) { Some(_) => model::runtime::cell_address_space_root(model::CellId(2)).unwrap_or(0), None => 0 };
    if user_root == 0 || arch_user_install(model::CellId(2), user_root).is_err() { arch::diagnostics::write_line(1, b"BENNU USER: ADDRESS SPACE INSTALL FAILED"); return; }
    arch::diagnostics::write_line(6, b"BENNU USER: RING3 CELL ONLINE");
    if let Ok(memory_object) = model::runtime::create_memory_object(2, 16) {
        let _ = model::runtime::grant(model::CellId(2), memory_object, model::CapabilityRights::READ.union(model::CapabilityRights::WRITE).union(model::CapabilityRights::MAP));
        arch::diagnostics::write_line(6, b"BENNU USER: MEMORY OBJECT + MAP CAPABILITY ONLINE");
    }
    if model::runtime::bind_entry(model::CellId(1), bootstrap_cell).is_err() { arch::diagnostics::write_line(1, b"BENNU EXEC: CELL ENTRY BIND FAILED"); return; }
    if model::runtime::prepare_cell_context(model::CellId(1), model::scheduler::cell_trampoline as usize as u64).is_err() { arch::diagnostics::write_line(1, b"BENNU EXEC: CELL STACK PREP FAILED"); return; }
    arch::diagnostics::write_line(6, b"BENNU EXEC: NATIVE CELL CONTEXT ONLINE");
    if let Some(pci) = model::pci::find_xhci_legacy(32, 32, 8) {
        if let Some(mmio) = model::pci::first_mmio_bar(&pci) {
            if let Ok(mapped) = memory::paging::map_mmio(mmio, 64 * 1024) {
                let cap = unsafe { model::xhci::probe_mmio(mapped) };
                if model::runtime::register_device_fabric_pci(pci, model::DeviceClass::UsbController).is_ok() {
                    let mut controller = model::XhciController::EMPTY; controller.object = pci.object;
                    if controller.configure(cap, mapped).is_ok() {
                        let reset = unsafe { model::xhci::reset_controller(mapped, cap) };
                        let dma = if reset.is_ok() { unsafe { model::xhci::setup_dma(&mut controller) } } else { Err("reset failed") };
                        let start = if dma.is_ok() { unsafe { model::xhci::start_controller(&mut controller, cap) } } else { Err("DMA setup failed") };
                        if start.is_ok() && model::runtime::register_xhci_controller(controller).is_ok() {
                            arch::diagnostics::write_line(6, b"BENNU USB: xHCI DMA + RINGS ONLINE");
                            let _ = model::runtime::start_usb_enumeration(1);
                        }
                    }
                }
            }
        }
    }
    arch::enable_interrupts(); arch::diagnostics::write_line(6, b"BENNU KERNEL: INTERRUPTS ENABLED");
    let mut scheduler = model::scheduler::Scheduler::new();
    loop {
        model::runtime::service_block_io();
        model::runtime::service_device_io();
        model::runtime::service_device_events();
        model::intelligence::tick();
        let cognitive = model::intelligence::runtime();
        model::entity::heartbeat(cognitive.phase);
        model::entity::refresh_resources();
        model::intelligence::execute_authorized();
        render_compositor(boot_info);
        if scheduler.step().is_none() { core::hint::spin_loop(); }
    }
}
