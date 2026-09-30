use super::ObjectId;
use crate::arch::x86_64::io::{inb, outb};

const CONFIG_ADDRESS: u16 = 0x0cf8;
const CONFIG_DATA: u16 = 0x0cfc;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PciAddress { pub segment:u16,pub bus:u8,pub device:u8,pub function:u8 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PciDevice {
 pub object:ObjectId,pub address:PciAddress,pub vendor:u16,pub device_id:u16,
 pub class:u8,pub subclass:u8,pub prog_if:u8,pub bars:[u64;6],
}
impl PciDevice {
 pub const EMPTY:Self=Self{object:ObjectId::NULL,address:PciAddress{segment:0,bus:0,device:0,function:0},vendor:0xffff,device_id:0xffff,class:0,subclass:0,prog_if:0,bars:[0;6]};
 pub const fn is_present(&self)->bool{self.vendor!=0xffff}
}

unsafe fn config_address(address:PciAddress, offset:u8) {
    let value = 0x8000_0000u32
        | ((address.bus as u32) << 16)
        | ((address.device as u32) << 11)
        | ((address.function as u32) << 8)
        | ((offset as u32) & 0xfc);
    outb(CONFIG_ADDRESS, (value & 0xff) as u8);
    outb(CONFIG_ADDRESS + 1, ((value >> 8) & 0xff) as u8);
    outb(CONFIG_ADDRESS + 2, ((value >> 16) & 0xff) as u8);
    outb(CONFIG_ADDRESS + 3, ((value >> 24) & 0xff) as u8);
}

unsafe fn read32(address:PciAddress, offset:u8) -> u32 {
    config_address(address, offset);
    let mut value = 0u32;
    for shift in [0u32,8,16,24] {
        value |= (inb(CONFIG_DATA + (shift / 8) as u16) as u32) << shift;
    }
    value
}

unsafe fn read16(address:PciAddress, offset:u8) -> u16 {
    let aligned = offset & !3;
    let value = read32(address, aligned);
    let shift = ((offset & 2) * 8) as u32;
    (value >> shift) as u16
}

pub fn discover_legacy(bus_limit:u8, device_limit:u8, function_limit:u8) -> usize {
    let mut found = 0usize;
    for bus in 0..bus_limit {
        for device in 0..device_limit {
            let header = PciAddress { segment:0,bus,device,function:0 };
            let vendor = unsafe { read16(header,0x00) };
            if vendor == 0xffff { continue; }
            let header_type = unsafe { read32(header,0x0c) >> 16 } as u8;
            let functions = if header_type & 0x80 != 0 { function_limit } else { 1 };
            for function in 0..functions {
                let address = PciAddress { segment:0,bus,device,function };
                let id = unsafe { read32(address,0x00) };
                let vendor = id as u16;
                if vendor == 0xffff { continue; }
                let device_id = (id >> 16) as u16;
                let class_reg = unsafe { read32(address,0x08) };
                let class = (class_reg >> 24) as u8;
                let subclass = (class_reg >> 16) as u8;
                let prog_if = (class_reg >> 8) as u8;
                let mut bars = [0u64;6];
                for index in 0..6 {
                    let offset = 0x10 + (index as u8 * 4);
                    bars[index] = unsafe { read32(address, offset) as u64 };
                }
                let object = match super::runtime::create_object(super::ObjectKind::Device, 0) {
                    Ok(object) => object,
                    Err(_) => continue,
                };
                let device = PciDevice {
                    object, address, vendor, device_id, class, subclass, prog_if, bars,
                };
                let _ = super::graph::link(
                    object,
                    object,
                    super::RelationKind::Contains,
                );
                let _ = super::runtime::emit(super::Event::new(
                    super::EventKind::DeviceAttached,
                    object,
                    ObjectId::NULL,
                    ((class as u64) << 32) | ((subclass as u64) << 16) | prog_if as u64,
                ));
                let _ = device;
                found += 1;
            }
        }
    }
    found
}

pub const fn xhci_class() -> (u8,u8,u8) { (0x0c,0x03,0x30) }
