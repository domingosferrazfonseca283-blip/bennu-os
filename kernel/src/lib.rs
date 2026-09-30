#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;

pub fn init() {
    arch::init();
}
