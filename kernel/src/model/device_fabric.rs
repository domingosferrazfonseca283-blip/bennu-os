use super::{DeviceClass,DeviceDescriptor,ObjectId,PciDevice,XhciController};

pub const MAX_DEVICES:usize=128;
pub const MAX_CONTROLLERS:usize=16;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct DeviceRecord { pub descriptor:DeviceDescriptor,pub pci:PciDevice }
impl DeviceRecord { pub const EMPTY:Self=Self{descriptor:DeviceDescriptor::EMPTY,pci:PciDevice::EMPTY}; }

#[repr(C)]
pub struct DeviceFabric {
 pub devices:[DeviceRecord;MAX_DEVICES],
 pub controllers:[XhciController;MAX_CONTROLLERS],
 pub device_count:usize,
 pub controller_count:usize,
}
impl DeviceFabric {
 pub const fn empty()->Self{Self{devices:[DeviceRecord::EMPTY;MAX_DEVICES],controllers:[XhciController::EMPTY;MAX_CONTROLLERS],device_count:0,controller_count:0}}
 pub fn register_pci(&mut self,pci:PciDevice,class:DeviceClass)->Result<ObjectId,&'static str>{
  if self.device_count>=MAX_DEVICES{return Err("device fabric full");}
  let object=pci.object;
  self.devices[self.device_count]=DeviceRecord{descriptor:DeviceDescriptor{object,class,vendor:pci.vendor,product:pci.device_id,capabilities:super::CapabilityRights::OBSERVE.union(super::CapabilityRights::DEVICE)},pci};
  self.device_count+=1; Ok(object)
 }
 pub fn register_xhci(&mut self,controller:XhciController)->Result<(),&'static str>{
  if self.controller_count>=MAX_CONTROLLERS{return Err("controller table full");}
  self.controllers[self.controller_count]=controller;self.controller_count+=1;Ok(())
 }
}
