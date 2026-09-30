use super::ObjectId;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PciAddress { pub segment:u16,pub bus:u8,pub device:u8,pub function:u8 }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PciDevice {
 pub object:ObjectId,pub address:PciAddress,pub vendor:u16,pub device_id:u16,
 pub class:u8,pub subclass:u8,pub prog_if:u8,pub bars:[u64;6],
}
impl PciDevice {
 pub const EMPTY:Self=Self{object:ObjectId::NULL,address:PciAddress{segment:0,bus:0,device:0,function:0},vendor:0xffff,device_id:0xffff,class:0,subclass:0,prog_if:0,bars:[0;6]};
 pub const fn is_present(&self)->bool{self.vendor!=0xffff}
}
