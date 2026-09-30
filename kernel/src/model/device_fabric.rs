use super::{DeviceClass,DeviceDescriptor,ObjectId,PciDevice,XhciController};

pub const MAX_DEVICES:usize=128;
pub const MAX_CONTROLLERS:usize=16;
pub const MAX_PENDING_COMMANDS:usize=64;
pub const MAX_PENDING_TRANSFERS:usize=64;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PendingTransfer { pub valid:bool,pub device:ObjectId,pub token:u64,pub slot:u8,pub ring:u64 }
impl PendingTransfer { pub const EMPTY:Self=Self{valid:false,device:ObjectId::NULL,token:0,slot:0,ring:0}; }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PendingCommand {
 pub valid:bool,
 pub device:ObjectId,
 pub token:u64,
 pub command_trb:u64,
}
impl PendingCommand {
 pub const EMPTY:Self=Self{valid:false,device:ObjectId::NULL,token:0,command_trb:0};
}

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
 pub pending:[PendingCommand;MAX_PENDING_COMMANDS],
 pub transfers:[PendingTransfer;MAX_PENDING_TRANSFERS],
}
impl DeviceFabric {
 pub const fn empty()->Self{Self{devices:[DeviceRecord::EMPTY;MAX_DEVICES],controllers:[XhciController::EMPTY;MAX_CONTROLLERS],device_count:0,controller_count:0,pending:[PendingCommand::EMPTY;MAX_PENDING_COMMANDS],transfers:[PendingTransfer::EMPTY;MAX_PENDING_TRANSFERS]}}
 pub fn register_pci(&mut self,pci:PciDevice,class:DeviceClass)->Result<ObjectId,&'static str>{
  if self.device_count>=MAX_DEVICES{return Err("device fabric full");}
  let object=pci.object;
  self.devices[self.device_count]=DeviceRecord{descriptor:DeviceDescriptor{object,class,vendor:pci.vendor,product:pci.device_id,capabilities:super::CapabilityRights::OBSERVE.union(super::CapabilityRights::DEVICE)},pci};
  self.device_count+=1; Ok(object)
 }
 pub fn discover_xhci(&mut self,pci:PciDevice,cap:super::xhci::CapabilityRegisters)->Result<ObjectId,&'static str>{
  if !super::pci::is_xhci(&pci) { return Err("PCI device is not xHCI"); }
  let mmio=super::pci::first_mmio_bar(&pci).ok_or("xHCI MMIO BAR missing")?;
  let mut controller=XhciController::EMPTY;
  controller.object=pci.object;
  controller.configure(cap,mmio)?;
  self.register_xhci(controller)?;
  Ok(pci.object)
 }
 pub fn register_xhci(&mut self,controller:XhciController)->Result<(),&'static str>{
  if self.controller_count>=MAX_CONTROLLERS{return Err("controller table full");}
  self.controllers[self.controller_count]=controller;self.controller_count+=1;Ok(())
 }
}


impl DeviceFabric {
 pub fn contains(&self, object:ObjectId)->bool {
  for i in 0..self.device_count { if self.devices[i].descriptor.object==object { return true; } }
  false
 }
 pub fn submit(&mut self, request:&super::DeviceRequest)->Result<(),&'static str> {
  for i in 0..self.device_count {
   if self.devices[i].descriptor.object != request.device { continue; }
   if self.devices[i].descriptor.class == DeviceClass::UsbController {
    for c in 0..self.controller_count {
     if self.controllers[c].object != request.device { continue; }
     return match request.opcode {
      1 => {
       let pending_slot=match (0..MAX_PENDING_COMMANDS).find(|p| !self.pending[*p].valid) {
        Some(p)=>p,
        None=>return Err("device command tracking full"),
       };
       let command_trb=super::xhci::enqueue_enable_slot(&mut self.controllers[c])?;
       self.pending[pending_slot]=PendingCommand{
        valid:true,device:request.device,token:request.token,command_trb
       };
       Ok(())
      },
      2 => { let slot=request.value as u8; let ring=super::xhci::enqueue_control_transfer(&mut self.controllers[c],slot,super::xhci::UsbSetupPacket::get_descriptor(1,0,18),request.buffer,18)?; let p=(0..MAX_PENDING_TRANSFERS).find(|i| !self.transfers[*i].valid).ok_or("transfer tracking full")?; self.transfers[p]=PendingTransfer{valid:true,device:request.device,token:request.token,slot,ring}; Ok(()) },
      _ => Err("unsupported xHCI device operation"),
     };
    }
    return Err("xHCI controller not registered");
   }
   return Err("device class has no native executor");
  }
  Err("device not registered in fabric")
 }
 pub fn service_events(&mut self)->Option<(ObjectId,u64,u8,u8)> {
  for c in 0..self.controller_count {
   let event=unsafe { super::xhci::poll_event(&mut self.controllers[c]) };
   if event.is_none() { continue; }
   let event=event.unwrap();
   match event.event_type() {
    super::xhci::TRB_TYPE_TRANSFER_EVENT => { let ptr=event.trb.parameter & !0xFu64; for p in 0..MAX_PENDING_TRANSFERS { if self.transfers[p].valid && self.transfers[p].slot==event.slot_id() && ptr>=self.transfers[p].ring && ptr<self.transfers[p].ring+48 { let t=self.transfers[p]; self.transfers[p]=PendingTransfer::EMPTY; return Some((t.device,t.token,event.slot_id(),event.completion_code())); } } }
    super::xhci::TRB_TYPE_CMD_COMPLETION => {
     let command_trb=event.trb.parameter & !0xFu64;
     for p in 0..MAX_PENDING_COMMANDS {
      if self.pending[p].valid && self.pending[p].command_trb==command_trb {
       let pending=self.pending[p];
       self.pending[p]=PendingCommand::EMPTY;
       return Some((pending.device,pending.token,event.slot_id(),event.completion_code()));
      }
     }
    }
    _ => {}
   }
  }
  None
 }
}
