use super::{DeviceClass,DeviceDescriptor,ObjectId,PciDevice,XhciController,UsbDeviceDescriptor,UsbAddress};

pub const MAX_DEVICES:usize=128;
pub const MAX_CONTROLLERS:usize=16;
pub const MAX_PENDING_COMMANDS:usize=64;
pub const MAX_PENDING_TRANSFERS:usize=64;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct PendingTransfer { pub valid:bool,pub device:ObjectId,pub token:u64,pub slot:u8,pub ring:u64,pub completion_trb:u64,pub owner_cell:u64,pub user_buffer:u64,pub dma_buffer:u64,pub length:u64,pub operation:u8,pub phase:u8,pub endpoint:u8,pub bot_tag:u32,pub bot_transfer_length:u32,pub bot_direction_in:bool,pub bot_command:u8,pub bot_data_dma:u64 }
impl PendingTransfer { pub const EMPTY:Self=Self{valid:false,device:ObjectId::NULL,token:0,slot:0,ring:0,completion_trb:0,owner_cell:0,user_buffer:0,dma_buffer:0,length:0,operation:0,phase:0,endpoint:0,bot_tag:0,bot_transfer_length:0,bot_direction_in:false,bot_command:0,bot_data_dma:0}; }

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
 pub mass_storage:[super::UsbMassStorageTransport;MAX_DEVICES],
}
impl DeviceFabric {
 pub const fn empty()->Self{Self{devices:[DeviceRecord::EMPTY;MAX_DEVICES],controllers:[XhciController::EMPTY;MAX_CONTROLLERS],device_count:0,controller_count:0,pending:[PendingCommand::EMPTY;MAX_PENDING_COMMANDS],transfers:[PendingTransfer::EMPTY;MAX_PENDING_TRANSFERS],usb_devices:[super::UsbDevice::EMPTY;MAX_DEVICES],mass_storage:[super::UsbMassStorageTransport::EMPTY;MAX_DEVICES]}}
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
 pub fn attach_usb_descriptor(&mut self,slot:u8,controller:ObjectId,object:ObjectId,descriptor:UsbDeviceDescriptor)->Result<(), &'static str> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].state==super::usb::UsbDeviceState::Detached && self.usb_devices[i].object.is_null() {
    self.usb_devices[i]=super::UsbDevice{
     object,controller,descriptor,configuration:super::UsbConfiguration::EMPTY,
     topology:super::UsbDeviceTopology::EMPTY,slot,port:0,
     state:super::usb::UsbDeviceState::Addressed,configured:false,
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
  unsafe { core::ptr::write_bytes(dma as *mut u8,0,crate::memory::PAGE_SIZE as usize); }
  let setup=super::xhci::UsbSetupPacket::get_descriptor(descriptor_type,0,length as u16);
  let ring=super::xhci::enqueue_control_transfer(&mut self.controllers[c],slot,setup,dma,length as u16)?;
  let p=(0..MAX_PENDING_TRANSFERS).find(|i| !self.transfers[*i].valid).ok_or("transfer tracking full")?;
  self.transfers[p]=PendingTransfer{valid:true,device:request.device,token:request.token,slot,ring,completion_trb:ring+32,owner_cell:request.owner_cell,user_buffer:request.buffer,dma_buffer:dma,length:length as u64,operation:request.opcode as u8,phase:0,endpoint:0,bot_tag:0,bot_transfer_length:0,bot_direction_in:false,bot_command:0,bot_data_dma:0};
  Ok(())
 }
 fn parse_configuration(&mut self,device:ObjectId,dma:u64,length:u64)->Result<(),&'static str> {
  if length<9 { return Err("USB configuration descriptor truncated"); }
  unsafe {
   let p=dma as *const u8;
   if core::ptr::read_volatile(p)!=9 || core::ptr::read_volatile(p.add(1))!=super::usb::USB_CONFIGURATION_DESCRIPTOR_TYPE { return Err("invalid USB configuration descriptor"); }
   let total=(core::ptr::read_volatile(p.add(2)) as u16) | ((core::ptr::read_volatile(p.add(3)) as u16)<<8);
   let configuration=core::ptr::read_volatile(p.add(5));
   let interfaces=core::ptr::read_volatile(p.add(4));
   let max_power=(core::ptr::read_volatile(p.add(8)) as u16)*2;
   for i in 0..MAX_DEVICES {
    if self.usb_devices[i].object!=device { continue; }
    self.usb_devices[i].configuration=super::UsbConfiguration{configuration,interfaces,max_power_ma:max_power};
    self.usb_devices[i].topology=super::UsbDeviceTopology::EMPTY;
    self.mass_storage[i]=super::UsbMassStorageTransport::EMPTY;
    let limit=(total as usize).min(length as usize).min(4096);
    let mut off=0usize;
    let mut current_interface:Option<super::UsbInterface>=None;
    while off+2<=limit {
     let len=core::ptr::read_volatile(p.add(off)) as usize;
     let typ=core::ptr::read_volatile(p.add(off+1));
     if len<2 || off+len>limit { break; }
     if typ==super::usb::USB_INTERFACE_DESCRIPTOR_TYPE && len>=9 {
      let v=super::UsbInterface{number:core::ptr::read_volatile(p.add(off+2)),alternate:core::ptr::read_volatile(p.add(off+3)),class_code:core::ptr::read_volatile(p.add(off+5)),subclass:core::ptr::read_volatile(p.add(off+6)),protocol:core::ptr::read_volatile(p.add(off+7)),endpoint_count:core::ptr::read_volatile(p.add(off+4))};
      let _=self.usb_devices[i].topology.add_interface(v); current_interface=Some(v);
     } else if typ==super::usb::USB_ENDPOINT_DESCRIPTOR_TYPE && len>=7 {
      let v=super::UsbEndpointDescriptor{address:core::ptr::read_volatile(p.add(off+2)),attributes:core::ptr::read_volatile(p.add(off+3)),max_packet:(core::ptr::read_volatile(p.add(off+4)) as u16)|((core::ptr::read_volatile(p.add(off+5)) as u16)<<8),interval:core::ptr::read_volatile(p.add(off+6)),interface_number:current_interface.map(|v|v.number).unwrap_or(0)};
      let _=self.usb_devices[i].topology.add_endpoint(v);
     }
     off+=len;
    }
    let _=current_interface;
    self.mass_storage[i]=self.usb_devices[i].topology.mass_storage_transport();
    return Ok(());
   }
  }
  Err("USB device object not found")
 }
 fn submit_bot_cbw(&mut self,device:ObjectId,token:u64,owner_cell:u64,command:super::ScsiCommand,user_buffer:u64,user_length:u64)->Result<(),&'static str> {
  let mut target=None;
  for i in 0..MAX_DEVICES { if self.usb_devices[i].object==device { target=Some(i); break; } }
  let i=target.ok_or("USB mass-storage device not found")?;
  let transport=self.mass_storage[i];
  if !transport.valid() { return Err("mass-storage transport not ready"); }
  let cbw=self.mass_storage[i].prepare_cbw(command);
  let dma=super::xhci::allocate_dma_page()?;
  unsafe { core::ptr::write_bytes(dma as *mut u8,0,super::xhci::XHCI_PAGE_SIZE as usize); }
  let mut bytes=[0u8;super::usb::USB_BOT_CBW_LENGTH];
  cbw.encode(&mut bytes);
  unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(),dma as *mut u8,bytes.len()); }
  let controller=self.usb_devices[i].controller;
  let slot=self.usb_devices[i].slot;
  for c in 0..self.controller_count {
   if self.controllers[c].object!=controller { continue; }
   let ring=super::xhci::enqueue_bulk_transfer(&mut self.controllers[c],slot,transport.bulk_out,dma,super::usb::USB_BOT_CBW_LENGTH as u32)?;
   let p=(0..MAX_PENDING_TRANSFERS).find(|x| !self.transfers[*x].valid).ok_or("BOT transfer tracking full")?;
   self.transfers[p]=PendingTransfer{
    valid:true,device,token,slot,ring,completion_trb:ring,owner_cell,
    user_buffer,dma_buffer:dma,length:super::usb::USB_BOT_CBW_LENGTH as u64,
    operation:9,phase:1,endpoint:transport.bulk_out,bot_tag:cbw.tag,
    bot_transfer_length:cbw.transfer_length.min(super::xhci::XHCI_PAGE_SIZE as u32),
    bot_direction_in:cbw.flags&0x80!=0,bot_command:command.bytes[0],bot_data_dma:0,
   };
   return Ok(());
  }
  Err("mass-storage controller unavailable")
 }

 fn queue_bot_csw(&mut self,t:PendingTransfer)->Result<(),&'static str> {
  let mut index=None;
  for i in 0..MAX_DEVICES { if self.usb_devices[i].object==t.device { index=Some(i); break; } }
  let i=index.ok_or("BOT device disappeared")?;
  let dma=super::xhci::allocate_dma_page()?;
  unsafe { core::ptr::write_bytes(dma as *mut u8,0,super::xhci::XHCI_PAGE_SIZE as usize); }
  let controller=self.usb_devices[i].controller;
  let slot=self.usb_devices[i].slot;
  let ep=self.mass_storage[i].bulk_in;
  for c in 0..self.controller_count {
   if self.controllers[c].object!=controller { continue; }
   let ring=super::xhci::enqueue_bulk_transfer(&mut self.controllers[c],slot,ep,dma,super::usb::USB_BOT_CSW_LENGTH as u32)?;
   let p=(0..MAX_PENDING_TRANSFERS).find(|x| !self.transfers[*x].valid).ok_or("BOT CSW tracking full")?;
   let mut next=t;
   next.ring=ring; next.completion_trb=ring; next.dma_buffer=dma;
   next.length=super::usb::USB_BOT_CSW_LENGTH as u64; next.operation=11; next.phase=3; next.endpoint=ep;
   self.transfers[p]=next;
   return Ok(());
  }
  Err("mass-storage controller unavailable")
 }

 fn queue_bot_data(&mut self,t:PendingTransfer)->Result<(),&'static str> {
  let mut index=None;
  for i in 0..MAX_DEVICES { if self.usb_devices[i].object==t.device { index=Some(i); break; } }
  let i=index.ok_or("BOT device disappeared")?;
  let dma=super::xhci::allocate_dma_page()?;
  unsafe { core::ptr::write_bytes(dma as *mut u8,0,super::xhci::XHCI_PAGE_SIZE as usize); }
  if !t.bot_direction_in {
   if t.user_buffer==0 { return Err("BOT OUT transfer requires user buffer"); }
   let root=crate::model::runtime::cell_address_space_root(crate::model::CellId(t.owner_cell as u32)).ok_or("owner Cell address space missing")?;
   crate::memory::user::copy_from_user(root,dma as *mut u8,t.user_buffer,t.bot_transfer_length as u64)?;
  }
  let controller=self.usb_devices[i].controller;
  let slot=self.usb_devices[i].slot;
  let ep=if t.bot_direction_in { self.mass_storage[i].bulk_in } else { self.mass_storage[i].bulk_out };
  for c in 0..self.controller_count {
   if self.controllers[c].object!=controller { continue; }
   let ring=super::xhci::enqueue_bulk_transfer(&mut self.controllers[c],slot,ep,dma,t.bot_transfer_length)?;
   let p=(0..MAX_PENDING_TRANSFERS).find(|x| !self.transfers[*x].valid).ok_or("BOT data tracking full")?;
   let mut next=t;
   next.ring=ring; next.completion_trb=ring; next.dma_buffer=dma; next.bot_data_dma=dma; next.length=t.bot_transfer_length as u64;
   next.operation=10; next.phase=2; next.endpoint=ep;
   self.transfers[p]=next;
   return Ok(());
  }
  Err("mass-storage controller unavailable")
 }

 fn start_mass_storage_probe(&mut self,device:ObjectId,token:u64,owner_cell:u64)->Result<(),&'static str> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].object==device {
    if !self.mass_storage[i].valid() { return Err("mass-storage endpoints unavailable"); }
    if self.mass_storage[i].stage!=super::UsbMassStorageStage::Idle { return Ok(()); }
    self.mass_storage[i].stage=super::UsbMassStorageStage::Command;
    return self.submit_bot_cbw(device,token,owner_cell,super::ScsiCommand::inquiry(36),0,36);
   }
  }
  Err("mass-storage device not found")
 }

 
 pub fn mass_storage_geometry(&self,device:ObjectId)->Option<(u32,u64,ObjectId)> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].object==device && self.mass_storage[i].block_size!=0 && self.mass_storage[i].block_count!=0 {
    return Some((self.mass_storage[i].block_size,self.mass_storage[i].block_count,self.mass_storage[i].block_object));
   }
  }
  None
 }
 pub fn bind_mass_storage_object(&mut self,device:ObjectId,object:ObjectId)->Result<(),&'static str> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].object==device {
    self.mass_storage[i].block_object=object;
    return Ok(());
   }
  }
  Err("mass-storage device not found")
 }

 pub fn mass_storage_endpoint_command_completed(&mut self,device:ObjectId,token:u64,owner_cell:u64)->Result<(),&'static str> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].object!=device { continue; }
   if self.mass_storage[i].configured_endpoints<2 { self.mass_storage[i].configured_endpoints+=1; }
   if self.mass_storage[i].configured_endpoints>=2 && self.mass_storage[i].stage==super::UsbMassStorageStage::Idle {
    return self.start_mass_storage_probe(device,token.wrapping_add(1),owner_cell);
   }
   return Ok(());
  }
  Err("mass-storage device not found")
 }

 pub fn configure_mass_storage_endpoints(&mut self,device:ObjectId,token:u64,owner_cell:u64)->Result<usize,&'static str> {
  for i in 0..MAX_DEVICES {
   if self.usb_devices[i].object!=device { continue; }
   let transport=self.usb_devices[i].topology.mass_storage_transport();
   if !transport.valid() { return Err("mass-storage bulk endpoints missing"); }
   let controller=self.usb_devices[i].controller;
   let slot=self.usb_devices[i].slot;
   for c in 0..self.controller_count {
    if self.controllers[c].object!=controller { continue; }
    let endpoints=[(transport.bulk_out,transport.max_packet_out),(transport.bulk_in,transport.max_packet_in)];
    let mut count=0usize;
    for (address,mps) in endpoints {
     let command_trb=super::xhci::enqueue_configure_endpoint(&mut self.controllers[c],slot,address,mps)?;
     let pending=(0..MAX_PENDING_COMMANDS).find(|p| !self.pending[*p].valid).ok_or("device command tracking full")?;
     self.pending[pending]=PendingCommand{valid:true,device,token:token.wrapping_add(count as u64),command_trb,owner_cell,operation:6};
     count+=1;
    }
    return Ok(count);
   }
   return Err("USB controller unavailable");
  }
  Err("USB device object not found")
 }
 fn submit_usb_set_configuration(&mut self,c:usize,request:&super::DeviceRequest)->Result<(),&'static str> {
  let slot=request.value as u8;
  if slot==0 { return Err("USB slot missing"); }
  let configuration=(request.flags & 0xff) as u8;
  if configuration==0 { return Err("USB configuration value missing"); }
  let ring=super::xhci::enqueue_control_transfer(&mut self.controllers[c],slot,super::xhci::UsbSetupPacket::set_configuration(configuration),0,0)?;
  let p=(0..MAX_PENDING_TRANSFERS).find(|i| !self.transfers[*i].valid).ok_or("transfer tracking full")?;
  self.transfers[p]=PendingTransfer{valid:true,device:request.device,token:request.token,slot,ring,completion_trb:ring+16,owner_cell:request.owner_cell,user_buffer:0,dma_buffer:0,length:0,operation:request.opcode as u8,phase:0,endpoint:0,bot_tag:0,bot_transfer_length:0,bot_direction_in:false,bot_command:0,bot_data_dma:0};
  Ok(())
 }
 fn submit_usb_bulk(&mut self,c:usize,request:&super::DeviceRequest,in_direction:bool)->Result<(),&'static str> {
  let slot=request.value as u8;
  let endpoint=request.flags as u8;
  if slot==0 || endpoint==0 || ((endpoint&0x80)!=0)!=in_direction { return Err("invalid USB bulk endpoint"); }
  if request.length==0 || request.length>super::xhci::XHCI_PAGE_SIZE { return Err("bulk transfer length exceeds DMA page"); }
  let dma=super::xhci::allocate_dma_page()?;
  unsafe { core::ptr::write_bytes(dma as *mut u8,0,super::xhci::XHCI_PAGE_SIZE as usize); }
  if !in_direction {
   let root=crate::model::runtime::cell_address_space_root(crate::model::CellId(request.owner_cell as u32)).ok_or("owner Cell address space missing")?;
   crate::memory::user::copy_from_user(root,dma as *mut u8,request.buffer,request.length)?;
  }
  let ring=super::xhci::enqueue_bulk_transfer(&mut self.controllers[c],slot,endpoint,dma,request.length as u32)?;
  let p=(0..MAX_PENDING_TRANSFERS).find(|i| !self.transfers[*i].valid).ok_or("transfer tracking full")?;
  self.transfers[p]=PendingTransfer{
   valid:true,device:request.device,token:request.token,slot,ring,completion_trb:ring,
   owner_cell:request.owner_cell,user_buffer:request.buffer,dma_buffer:dma,length:request.length,
   operation:request.opcode as u8,phase:0,endpoint,bot_tag:0,bot_transfer_length:0,bot_direction_in:in_direction,bot_command:0,bot_data_dma:0,
  };
  Ok(())
 }

 pub fn submit(&mut self, request:&super::DeviceRequest)->Result<(),&'static str> {

  for u in 0..MAX_DEVICES { if self.usb_devices[u].object==request.device && !request.device.is_null() { let controller=self.usb_devices[u].controller; for c in 0..self.controller_count { if self.controllers[c].object==controller { return match request.opcode { 4=>self.submit_usb_control(c,request,2), 5=>self.submit_usb_set_configuration(c,request), 7=>self.submit_usb_bulk(c,request,true), 8=>self.submit_usb_bulk(c,request,false), _=>Err("unsupported USB device operation") }; } } return Err("USB controller unavailable"); } }

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
        owner_cell:request.owner_cell,operation:request.opcode as u8
       };
       Ok(())
      },
      2 | 4 => { self.submit_usb_control(c,request,if request.opcode==2 {1} else {2})?; Ok(()) },
      3 => {
       let pending_slot=match (0..MAX_PENDING_COMMANDS).find(|p| !self.pending[*p].valid) { Some(p)=>p, None=>return Err("device command tracking full") };
       let slot=request.value as u8;
       let port=(request.flags & 0xff) as u8;
       let command_trb=super::xhci::enqueue_address_device_for_port(&mut self.controllers[c],slot,port)?;
       self.pending[pending_slot]=PendingCommand{valid:true,device:request.device,token:request.token,command_trb,owner_cell:request.owner_cell,operation:request.opcode as u8};
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
    super::xhci::TRB_TYPE_TRANSFER_EVENT => {
     let ptr=event.trb.parameter & !0xFu64;
     for p in 0..MAX_PENDING_TRANSFERS {
      if !self.transfers[p].valid || self.transfers[p].slot!=event.slot_id() || ptr!=self.transfers[p].completion_trb { continue; }
      let t=self.transfers[p];
      self.transfers[p]=PendingTransfer::EMPTY;
      if event.completion_code()!=1 { return Some((t.device,t.token,event.slot_id(),event.completion_code(),t.operation,t.owner_cell,None)); }

      if t.operation==9 {
       if t.bot_transfer_length!=0 {
        let _=self.queue_bot_data(t);
       } else {
        let _=self.queue_bot_csw(t);
       }
       return Some((t.device,t.token,event.slot_id(),event.completion_code(),t.operation,t.owner_cell,None));
      }

      if t.operation==10 {
       if t.bot_direction_in && t.user_buffer!=0 {
        if let Some(root)=crate::model::runtime::cell_address_space_root(crate::model::CellId(t.owner_cell as u32)) {
         let _=crate::memory::user::copy_to_user(root,t.user_buffer,t.dma_buffer as *const u8,t.bot_transfer_length as u64);
        }
       }
       let _=self.queue_bot_csw(t);
       return Some((t.device,t.token,event.slot_id(),event.completion_code(),t.operation,t.owner_cell,None));
      }

      if t.operation==11 {
       let mut csw_ok=false;
       unsafe {
        let pbytes=t.dma_buffer as *const u8;
        let bytes=[
         core::ptr::read_volatile(pbytes.add(0)),core::ptr::read_volatile(pbytes.add(1)),
         core::ptr::read_volatile(pbytes.add(2)),core::ptr::read_volatile(pbytes.add(3)),
         core::ptr::read_volatile(pbytes.add(4)),core::ptr::read_volatile(pbytes.add(5)),
         core::ptr::read_volatile(pbytes.add(6)),core::ptr::read_volatile(pbytes.add(7)),
         core::ptr::read_volatile(pbytes.add(8)),core::ptr::read_volatile(pbytes.add(9)),
         core::ptr::read_volatile(pbytes.add(10)),core::ptr::read_volatile(pbytes.add(11)),
         core::ptr::read_volatile(pbytes.add(12))
        ];
        let csw=super::UsbMassStorageBotCsw::decode(&bytes);
        csw_ok=csw.valid(t.bot_tag) && csw.residue<=t.bot_transfer_length;
       }
       if csw_ok {
        for i in 0..MAX_DEVICES {
         if self.usb_devices[i].object!=t.device { continue; }
         if t.bot_command==0x12 {
          self.mass_storage[i].stage=super::UsbMassStorageStage::Idle;
          let _=self.submit_bot_cbw(t.device,t.token.wrapping_add(1),t.owner_cell,super::ScsiCommand::read_capacity10(),0,8);
         } else if t.bot_command==0x25 {
          unsafe {
           let p=t.bot_data_dma as *const u8;
           let last=((core::ptr::read_volatile(p) as u32)<<24)|((core::ptr::read_volatile(p.add(1)) as u32)<<16)|((core::ptr::read_volatile(p.add(2)) as u32)<<8)|core::ptr::read_volatile(p.add(3)) as u32;
           let block=((core::ptr::read_volatile(p.add(4)) as u32)<<24)|((core::ptr::read_volatile(p.add(5)) as u32)<<16)|((core::ptr::read_volatile(p.add(6)) as u32)<<8)|core::ptr::read_volatile(p.add(7)) as u32;
           if block!=0 {
            self.mass_storage[i].block_size=block;
            self.mass_storage[i].block_count=(last as u64).saturating_add(1);
            self.mass_storage[i].stage=super::UsbMassStorageStage::Idle;
           } else { self.mass_storage[i].stage=super::UsbMassStorageStage::Failed; }
          }
         }
        }
       } else {
        for i in 0..MAX_DEVICES { if self.usb_devices[i].object==t.device { self.mass_storage[i].stage=super::UsbMassStorageStage::Failed; } }
       }
       return Some((t.device,t.token,event.slot_id(),event.completion_code(),t.operation,t.owner_cell,None));
      }

      if t.operation==4 {
       let _=self.parse_configuration(t.device,t.dma_buffer,t.length);
      }
      if t.operation==5 {
       for i in 0..MAX_DEVICES {
        if self.usb_devices[i].object==t.device {
         self.usb_devices[i].state=super::usb::UsbDeviceState::Configured;
         self.usb_devices[i].configured=true;
        }
       }
      }
      if (t.operation==4 || t.operation==7) && t.length!=0 && t.user_buffer!=0 {
       if let Some(root)=crate::model::runtime::cell_address_space_root(crate::model::CellId(t.owner_cell as u32)) {
        let _=crate::memory::user::copy_to_user(root,t.user_buffer,t.dma_buffer as *const u8,t.length);
       }
      }
      let descriptor=if t.operation==2 && event.slot_id()!=0 {
       unsafe {
        let p=t.dma_buffer as *const u8;
        if core::ptr::read_volatile(p.add(1))==super::usb::USB_DEVICE_DESCRIPTOR_TYPE && core::ptr::read_volatile(p)>=18 {
         Some(UsbDeviceDescriptor{
          address:UsbAddress{bus:0,address:0},
          vendor:(core::ptr::read_volatile(p.add(9)) as u16)<<8|core::ptr::read_volatile(p.add(8)) as u16,
          product:(core::ptr::read_volatile(p.add(11)) as u16)<<8|core::ptr::read_volatile(p.add(10)) as u16,
          class_code:core::ptr::read_volatile(p.add(4)),
          subclass:core::ptr::read_volatile(p.add(5)),
          protocol:core::ptr::read_volatile(p.add(6)),
         })
        } else { None }
       }
      } else { None };
      return Some((t.device,t.token,event.slot_id(),event.completion_code(),t.operation,t.owner_cell,descriptor));
     }
    }
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
