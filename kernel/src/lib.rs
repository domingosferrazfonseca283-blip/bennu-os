#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod boot_info;
pub mod memory;
pub mod model;


extern "C" fn bootstrap_cell() -> model::CellAction {
    model::CellAction::Stop
}

pub fn init(boot_info: *const boot_info::BootInfo) {
    arch::init();
    model::runtime::init();

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

    arch::diagnostics::write_line(1, b"BENNU MODEL: OBJECT + CELL ENGINE ONLINE");

    if let Ok(device) = model::runtime::create_object(model::ObjectKind::Device, 0) {
        let _ = model::graph::link(root, device, model::RelationKind::Contains);
        let _ = model::runtime::grant(model::CellId(1), device, model::CapabilityRights::OBSERVE);
        arch::diagnostics::write_line(1, b"BENNU MODEL: ROOT CAPABILITY ONLINE");

    if model::runtime::bind_entry(model::CellId(1), bootstrap_cell).is_ok() {
        match model::runtime::run_once(model::CellId(1)) {
            Ok(model::CellAction::Stop) => arch::diagnostics::write_line(1, b"BENNU EXEC: CELL COMPLETED"),
            Ok(_) => arch::diagnostics::write_line(1, b"BENNU EXEC: CELL YIELDED"),
            Err(_) => arch::diagnostics::write_line(1, b"BENNU EXEC: CELL FAILED"),
        }
    }
    }

    if boot_info.is_null() {
        arch::diagnostics::write_line(4, b"BENNU BOOT: NULL BOOT INFO");
        return;
    }

    let boot_info = unsafe { &*boot_info };

    if let Err(message) = memory::init(boot_info) {
        let _ = message;
        arch::diagnostics::write_line(4, b"BENNU MEMORY: INITIALIZATION FAILED");
        return;
    }

    arch::diagnostics::write_line(4, b"BENNU MEMORY: E820 + FRAME ALLOCATOR ONLINE");

    if let Err(message) = memory::paging::init() {
        let _ = message;
        arch::diagnostics::write_line(5, b"BENNU MEMORY: PAGING INITIALIZATION FAILED");
        return;
    }

    arch::diagnostics::write_line(5, b"BENNU MEMORY: KERNEL PAGING ONLINE");

    unsafe {
        memory::heap::init();
    }

    arch::diagnostics::write_line(6, b"BENNU MEMORY: KERNEL HEAP ONLINE");
}
