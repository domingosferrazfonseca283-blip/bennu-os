#![no_std]

pub mod arch;
pub mod panic;

pub fn init() {
    arch::init();
}
