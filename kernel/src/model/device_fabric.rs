use super::{DeviceClass,DeviceDescriptor,ObjectId,PciDevice,XhciController,UsbDeviceDescriptor,UsbAddress};

pub const MAX_DEVICES:usize=128;
pub const MAX_CONTROLLERS:usize=16;
pub const MAX_PENDING_COMMANDS:usize=64;
pub const MAX_PENDING_TRANSFERS:usize=64;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PendingTransfer { pub valid:bool,pub device:ObjectId,pub token:u64,pub slot:u8,pub ring:u64,pub completion_trb:u64,pub owner_cell:u64,pub user_buffer:u64,pub dma_buffer:u64,pub length:u64,pub operation:u8 }
impl PendingTransfer { pub const EMPTY:Self=Self{valid:false,device:ObjectId::NULL,token:0,slot:0,ring:0,completion_trb:0,owner_cell:0,user_buffer:0,dma_buffer:0,length:0,operation:0}; }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PendingCommand {
 pub valid:bool,
 pub device:ObjectId,
 pub token:u64,
 pub command_trb:u64,
 pub owner_cell:u64,
 pub operation:u8,
}
impl PendingCommand {
 pub const EMPTY:Self=Self{valid:false,device:ObjectId::NULL,token:0,command_trb:0,owner_cell:0,operation:0};
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
 pub usb_devices:[super::UsbDevice;MAX_DEVICES],
}
impl DeviceFabric {
 pub const fn empty()->Self{Self{devices:[DeviceRecord::EMPTY;MAX_DEVICES],controllers:[XhciController::EMPTY;MAX_CONTROLLERS],device_count:0,controller_count:0,pending:[PendingCommand::EMPTY;MAX_PENDING_COMMANDS],transfers:[PendingTransfer::EMPTY;MAX_PENDING_TRANSFERS],usb_devices:[super::UsbDevice::EMPTY;MAX_DEVICES]}}
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
 pub fn attach_usb_descriptor(&mut self,slot:u8,object:ObjectId,descriptor:UsbDeviceDescriptor)->Result<(), &'static str> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].state==super::usb::UsbDeviceState::Detached && self.usb_devices[i].object.is_null() {
    self.usb_devices[i]=super::UsbDevice{
     object,controller,descriptor,configuration:super::UsbConfiguration::EMPTY,
     topology:super::UsbDeviceTopology::EMPTY,slot,port:0,
     state:super::usb::UsbDeviceState::Default,configured:false,
    };
    return Ok(());
   }
  }
  Err("USB device table full")
 }
 pub fn contains(&self, object:ObjectId)->bool {
  for i in 0..self.device_count { if self.devices[i].descriptor.object==object { return true; } }
  false
 }
 fn submit_usb_control(&mut self,c:usize,request:&super::DeviceRequest,descriptor_type:u8)->Result<(),&'static str> {
  let slot=request.value as u8;
  let length=(if descriptor_type==1 {18} else {request.length as usize}).min(4096);
  if length==0 { return Err("USB transfer length is zero"); }
  let dma=crate::memory::allocate_frame_below(super::xhci::XHCI_DMA_LIMIT).ok_or("USB transfer DMA buffer allocation failed")?;
  unsafe { core::ptr::write_bytes(dma as *mut u8,0,crate::memory::PAGE_SIZE); }
  let setup=super::xhci::UsbSetupPacket::get_descriptor(descriptor_type,0,length as u16);
  let ring=super::xhci::enqueue_control_transfer(&mut self.controllers[c],slot,setup,dma,length as u16)?;
  let p=(0..MAX_PENDING_TRANSFERS).find(|i| !self.transfers[*i].valid).ok_or("transfer tracking full")?;
  self.transfers[p]=PendingTransfer{valid:true,device:request.device,token:request.token,slot,ring,completion_trb:ring+32,owner_cell:request.owner_cell,user_buffer:request.buffer,dma_buffer:dma,length:length as u64,operation:request.opcode};
  Ok(())
 }
 pub fn submit(&mut self, request:&super::DeviceRequest)->Result<(),&'static str> {
  for u in 0..MAX_DEVICES { if self.usb_devices[u].object==request.device && !request.device.is_null() { let controller=self.usb_devices[u].controller; for c in 0..self.controller_count { if self.controllers[c].object==controller { return match request.opcode { 4=>self.submit_usb_control(c,request,2), _=>Err("unsupported USB device operation") }; } } return Err("USB controller unavailable"); } }

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
        valid:true,device:request.device,token:request.token,command_trb,
        owner_cell:request.owner_cell,operation:request.opcode
       };
       Ok(())
      },
      2 | 4 => { self.submit_usb_control(c,request,if request.opcode==2 {1} else {2})?; Ok(()) },
      3 => {
       let pending_slot=match (0..MAX_PENDING_COMMANDS).find(|p| !self.pending[*p].valid) { Some(p)=>p, None=>return Err("device command tracking full") };
       let slot=request.value as u8;
       let port=(request.flags & 0xff) as u8;
       let command_trb=super::xhci::enqueue_address_device_for_port(&mut self.controllers[c],slot,port)?;
       self.pending[pending_slot]=PendingCommand{valid:true,device:request.device,token:request.token,command_trb,owner_cell:request.owner_cell,operation:request.opcode};
       Ok(())
      },
      _ => Err("unsupported xHCI device operation"),
     };
    }
    return Err("xHCI controller not registered");
   }
   return Err("device class has no native executor");
  }
  Err("device not registered in fabric")
 }
 pub fn service_events(&mut self)->Option<(ObjectId,u64,u8,u8,u8,u64,Option<UsbDeviceDescriptor>)> {
  for c in 0..self.controller_count {
   let event=unsafe { super::xhci::poll_event(&mut self.controllers[c]) };
   if event.is_none() { continue; }
   let event=event.unwrap();
   match event.event_type() {
    super::xhci::TRB_TYPE_TRANSFER_EVENT => { let ptr=event.trb.parameter & !0xFu64; for p in 0..MAX_PENDING_TRANSFERS { if self.transfers[p].valid && self.transfers[p].slot==event.slot_id() && ptr==self.transfers[p].completion_trb { let t=self.transfers[p]; self.transfers[p]=PendingTransfer::EMPTY; if event.completion_code()==1 { if let Some(root)=crate::model::runtime::cell_address_space_root(crate::model::CellId(t.owner_cell as u64)) { let _=crate::memory::user::copy_to_user(root,t.user_buffer,t.dma_buffer as *const u8,t.length as usize); } } let descriptor = if event.slot_id() != 0 && event.completion_code() == 1 {
          unsafe {
           let p=t.dma_buffer as *const u8;
           if core::ptr::read_volatile(p.add(1)) == super::usb::USB_DEVICE_DESCRIPTOR_TYPE && core::ptr::read_volatile(p) >= 18 {
            Some(UsbDeviceDescriptor{
             address:UsbAddress{bus:0,address:0},
             vendor:(core::ptr::read_volatile(p.add(9)) as u16) << 8 | core::ptr::read_volatile(p.add(8)) as u16,
             product:(core::ptr::read_volatile(p.add(11)) as u16) << 8 | core::ptr::read_volatile(p.add(10)) as u16,
             class_code:core::ptr::read_volatile(p.add(4)),
             subclass:core::ptr::read_volatile(p.add(5)),
             protocol:core::ptr::read_volatile(p.add(6)),
            })
           } else { None }
          }
         } else { None };
         return Some((t.device,t.token,event.slot_id(),event.completion_code(),t.operation,t.owner_cell,descriptor)); } } }
    super::xhci::TRB_TYPE_CMD_COMPLETION => {
     let command_trb=event.trb.parameter & !0xFu64;
     for p in 0..MAX_PENDING_COMMANDS {
      if self.pending[p].valid && self.pending[p].command_trb==command_trb {
       let pending=self.pending[p];
       self.pending[p]=PendingCommand::EMPTY;
       return Some((pending.device,pending.token,event.slot_id(),event.completion_code(),pending.operation,pending.owner_cell,None));
      }
     }
    }
    _ => {}
   }
  }
  None
 }
}
