#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod boot_info;
pub mod memory;
pub mod model;

fn arch_user_install(root: u64) -> Result<(), &'static str> {
    arch::x86_64::userspace::install(model::CellId(2), root)
}

extern "C" fn bootstrap_cell() -> model::CellAction {
    model::CellAction::Stop
}

pub fn init(boot_info: *const boot_info::BootInfo) {
    arch::init();
    model::runtime::init();
    model::init_io_fabric(model::policy::AccessPolicy::USB_FIRST);

    let root = match model::runtime::create_object(model::ObjectKind::Cell, 0) {
        Ok(root) => root,
        Err(_) => {
            arch::diagnostics::write_line(1, b"BENNU MODEL: OBJECT SPACE FAILED");
            return;
        }
    };

    if model::runtime::create_cell(model::CellId(1), root).is_err() {
        arch::diagnostics::write_line(1, b"BENNU MODEL: ROOT CELL FAILED");
        return;
    }

    arch::diagnostics::write_line(1, b"BENNU MODEL: OBJECT + CELL FABRIC ONLINE");

    if let Ok(device) = model::runtime::create_object(model::ObjectKind::Device, 0) {
        let _ = model::graph::link(root, device, model::RelationKind::Contains);
        let _ = model::runtime::grant(
            model::CellId(1),
            device,
            model::CapabilityRights::OBSERVE,
        );
        arch::diagnostics::write_line(1, b"BENNU MODEL: CAPABILITY GRAPH ONLINE");
    }

    if boot_info.is_null() {
        arch::diagnostics::write_line(4, b"BENNU BOOT: NULL BOOT INFO");
        return;
    }

    let boot_info = unsafe { &*boot_info };

    if let Err(_) = memory::init(boot_info) {
        arch::diagnostics::write_line(4, b"BENNU MEMORY: INITIALIZATION FAILED");
        return;
    }
    arch::diagnostics::write_line(4, b"BENNU MEMORY: E820 + FRAME ALLOCATOR ONLINE");

    if let Err(_) = memory::paging::init() {
        arch::diagnostics::write_line(5, b"BENNU MEMORY: PAGING INITIALIZATION FAILED");
        return;
    }
    arch::diagnostics::write_line(5, b"BENNU MEMORY: KERNEL PAGING ONLINE");

    unsafe { memory::heap::init(); }
    arch::diagnostics::write_line(6, b"BENNU MEMORY: KERNEL HEAP ONLINE");

    let user_root_object = match model::runtime::create_object(model::ObjectKind::Cell, 0) {
        Ok(object) => object,
        Err(_) => {
            arch::diagnostics::write_line(1, b"BENNU USER: ROOT OBJECT FAILED");
            return;
        }
    };
    if model::runtime::create_cell(model::CellId(2), user_root_object).is_err() {
        arch::diagnostics::write_line(1, b"BENNU USER: CELL CREATE FAILED");
        return;
    }
    if model::runtime::prepare_cell_context(
        model::CellId(2),
        model::scheduler::cell_trampoline as usize as u64,
    ).is_err() {
        arch::diagnostics::write_line(1, b"BENNU USER: CELL CONTEXT FAILED");
        return;
    }
    let user_root = match model::runtime::context_ptr(model::CellId(2)) {
        Some(_) => model::runtime::cell_address_space_root(model::CellId(2)).unwrap_or(0),
        None => 0,
    };
    if user_root == 0 || arch_user_install(user_root).is_err() {
        arch::diagnostics::write_line(1, b"BENNU USER: ADDRESS SPACE INSTALL FAILED");
        return;
    }
    arch::diagnostics::write_line(6, b"BENNU USER: RING3 CELL ONLINE");

    if let Ok(memory_object) = model::runtime::create_object(model::ObjectKind::Memory, 2) {
        let _ = model::runtime::grant(
            model::CellId(2),
            memory_object,
            model::CapabilityRights::READ
                .union(model::CapabilityRights::WRITE)
                .union(model::CapabilityRights::MAP),
        );
        arch::diagnostics::write_line(6, b"BENNU USER: MEMORY OBJECT + MAP CAPABILITY ONLINE");
    }

    if model::runtime::bind_entry(model::CellId(1), bootstrap_cell).is_err() {
        arch::diagnostics::write_line(1, b"BENNU EXEC: CELL ENTRY BIND FAILED");
        return;
    }

    if model::runtime::prepare_cell_context(
        model::CellId(1),
        model::scheduler::cell_trampoline as usize as u64,
    ).is_err() {
        arch::diagnostics::write_line(1, b"BENNU EXEC: CELL STACK PREP FAILED");
        return;
    }

    arch::diagnostics::write_line(6, b"BENNU EXEC: NATIVE CELL CONTEXT ONLINE");

    if let Some(pci) = model::pci::find_xhci_legacy(32, 32, 8) {
        if let Some(mmio) = model::pci::first_mmio_bar(&pci) {
            match memory::paging::map_mmio(mmio, 64 * 1024) {
                Ok(mapped) => {
                    let cap = unsafe { model::xhci::probe_mmio(mapped) };
                    if model::runtime::register_device_fabric_pci(pci, model::DeviceClass::UsbController).is_ok() {
                        let mut controller = model::XhciController::EMPTY;
                        controller.object = pci.object;
                        if controller.configure(cap, mapped).is_ok() {
                            let reset = unsafe { model::xhci::reset_controller(mapped, cap) };
                            let dma = if reset.is_ok() { unsafe { model::xhci::setup_dma(&mut controller) } } else { Err("reset failed") };
                            let start = if dma.is_ok() { unsafe { model::xhci::start_controller(&mut controller, cap) } } else { Err("DMA setup failed") };
                            if start.is_ok() && model::runtime::register_xhci_controller(controller).is_ok() {
                                arch::diagnostics::write_line(6, b"BENNU USB: xHCI DMA + RINGS ONLINE");
                            } else {
                                arch::diagnostics::write_line(5, b"BENNU USB: xHCI DMA/START FAILED");
                            }
                        }
                    }
                }
                Err(_) => arch::diagnostics::write_line(5, b"BENNU USB: xHCI MMIO MAP FAILED"),
            }
        }
    } else {
        arch::diagnostics::write_line(6, b"BENNU USB: NO xHCI CONTROLLER");
    }

    arch::enable_interrupts();
    arch::diagnostics::write_line(6, b"BENNU KERNEL: INTERRUPTS ENABLED");

    let mut scheduler = model::scheduler::Scheduler::new();
    loop {
        model::runtime::service_device_io();
        model::runtime::service_device_events();
        if scheduler.step().is_none() {
            core::hint::spin_loop();
        }
    }
}
