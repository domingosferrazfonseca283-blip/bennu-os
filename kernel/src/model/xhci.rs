use super::ObjectId;

pub const XHCI_MAX_SLOTS:usize=256;
pub const XHCI_RING_TRBS:usize=256;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct CapabilityRegisters {
 pub cap_length:u8,pub version:u16,pub hcs_params1:u32,pub hcs_params2:u32,pub hcs_params3:u32,
 pub hcc_params1:u32,pub dboff:u32,pub rtsoff:u32,pub hcc_params2:u32,
}
#[repr(C)]
#[derive(Clone,Copy)]
pub struct OperationalRegisters {
 pub usbcmd:u32,pub usbsts:u32,pub pagesize:u32,pub reserved0:[u32;2],
 pub dnctrl:u32,pub crcr:u64,pub reserved1:[u32;4],pub dcbaap:u64,
 pub config:u32,
}
#[repr(C)]
#[derive(Clone,Copy)]
pub struct Trb {
 pub parameter:u64,pub status:u32,pub control:u32,
}
impl Trb { pub const EMPTY:Self=Self{parameter:0,status:0,control:0}; }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct XhciController {
 pub object:ObjectId,pub mmio_base:u64,pub slots:u16,pub ports:u8,pub initialized:bool,
}
impl XhciController {
 pub const EMPTY:Self=Self{object:ObjectId::NULL,mmio_base:0,slots:0,ports:0,initialized:false};
}
