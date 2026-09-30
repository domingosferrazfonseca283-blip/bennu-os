use super::ObjectId;

pub const XHCI_MAX_SLOTS:usize=256;
pub const XHCI_RING_TRBS:usize=256;
pub const XHCI_MAX_PORTS:usize=256;
pub const XHCI_PAGE_SIZE:u64=4096;
pub const XHCI_DMA_LIMIT:u64=64*1024*1024;
pub const TRB_TYPE_LINK:u32=6<<10;
pub const TRB_TYPE_NOOP_CMD:u32=23<<10;
pub const TRB_TYPE_TRANSFER_EVENT:u32=32<<10;
pub const TRB_TYPE_CMD_COMPLETION:u32=33<<10;
pub const TRB_TYPE_PORT_STATUS:u32=34<<10;
pub const TRB_TYPE_ENABLE_SLOT:u32=9<<10;
pub const TRB_TYPE_ADDRESS_DEVICE:u32=11<<10;
pub const TRB_TYPE_SETUP_STAGE:u32=2<<10;
pub const TRB_TYPE_DATA_STAGE:u32=3<<10;
pub const TRB_TYPE_STATUS_STAGE:u32=4<<10;
pub const TRB_COMPLETION_CODE_SHIFT:u32=24;

pub const USBCMD_RUN_STOP:u32=1<<0;
pub const USBCMD_HCRST:u32=1<<1;
pub const USBSTS_HCH:u32=1<<0;
pub const USBSTS_CNR:u32=1<<11;
pub const USBCMD_INTE:u32=1<<2;
pub const IMAN_INTERRUPT_PENDING:u32=1<<0;
pub const IMAN_INTERRUPT_ENABLE:u32=1<<1;
pub const PORTSC_CCS:u32=1<<0;
pub const PORTSC_PED:u32=1<<1;
pub const PORTSC_OCA:u32=1<<3;
pub const PORTSC_PR:u32=1<<4;
pub const PORTSC_PP:u32=1<<9;
pub const PORTSC_CSC:u32=1<<17;
pub const PORTSC_PRC:u32=1<<21;
pub const PORTSC_PLC:u32=1<<18;
pub const PORTSC_CEC:u32=1<<23;
pub const PORTSC_SPEED_SHIFT:u32=10;
pub const PORTSC_SPEED_MASK:u32=0xF<<PORTSC_SPEED_SHIFT;
pub const XHCI_PORT_REG_BASE:u64=0x400;
pub const XHCI_PORT_REG_STRIDE:u64=0x10;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct CapabilityRegisters {
 pub cap_length:u8,pub version:u16,pub hcs_params1:u32,pub hcs_params2:u32,pub hcs_params3:u32,
 pub hcc_params1:u32,pub dboff:u32,pub rtsoff:u32,pub hcc_params2:u32,
}
impl CapabilityRegisters {
 pub const fn max_slots(&self)->u16 { (self.hcs_params1 & 0xff) as u16 }
 pub const fn max_ports(&self)->u8 { ((self.hcs_params1 >> 24) & 0xff) as u8 }
 pub const fn supports_64bit(&self)->bool { self.hcc_params1 & 1 != 0 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct OperationalRegisters {
 pub usbcmd:u32,pub usbsts:u32,pub pagesize:u32,pub reserved0:[u32;2],
 pub dnctrl:u32,pub crcr:u64,pub reserved1:[u32;4],pub dcbaap:u64,pub config:u32,
}
impl OperationalRegisters {
 pub const fn halted(&self)->bool { self.usbsts & USBSTS_HCH != 0 }
 pub const fn controller_not_ready(&self)->bool { self.usbsts & USBSTS_CNR != 0 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Trb { pub parameter:u64,pub status:u32,pub control:u32 }
impl Trb {
 pub const EMPTY:Self=Self{parameter:0,status:0,control:0};
 pub const fn with_cycle(mut self,cycle:bool)->Self {
  if cycle { self.control|=1; } else { self.control&=!1; } self
 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct XhciRing {
 pub trbs:[Trb;XHCI_RING_TRBS],
 pub enqueue:usize,
 pub dequeue:usize,
 pub cycle:bool,
}
impl XhciRing {
 pub const EMPTY:Self=Self{trbs:[Trb::EMPTY;XHCI_RING_TRBS],enqueue:0,dequeue:0,cycle:true};
 pub fn reset(&mut self){*self=Self::EMPTY;}
 pub fn push_with_index(&mut self,trb:Trb)->Result<usize,&'static str>{
  if self.enqueue==XHCI_RING_TRBS-1 { self.enqueue=0; self.cycle=!self.cycle; }
  let next=(self.enqueue+1)%(XHCI_RING_TRBS-1);
  if next==self.dequeue{return Err("xHCI ring full");}
  let index=self.enqueue;
  self.trbs[index]=trb.with_cycle(self.cycle);
  self.enqueue=next;
  Ok(index)
 }
 pub fn push(&mut self,trb:Trb)->Result<(),&'static str>{
  self.push_with_index(trb).map(|_|())
 }
 pub fn pop(&mut self)->Option<Trb>{
  if self.dequeue==self.enqueue{return None;}
  let trb=self.trbs[self.dequeue];
  self.dequeue=(self.dequeue+1)%(XHCI_RING_TRBS-1);
  Some(trb)
 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct XhciPort {
 pub index:u8,pub status:u32,pub connected:bool,pub enabled:bool,pub slot:u8,pub speed:u8,
}
impl XhciPort {
 pub const fn empty(index:u8)->Self { Self{index,status:0,connected:false,enabled:false,slot:0,speed:0} }
 pub const fn speed(&self)->u8 { self.speed }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct XhciController {
 pub object:ObjectId,pub mmio_base:u64,pub slots:u16,pub ports:u8,pub initialized:bool,
 pub dcbaa_phys:u64,pub command_ring_phys:u64,pub event_ring_phys:u64,pub erst_phys:u64,
 pub command_ring:XhciRing,pub event_ring:XhciRing,pub ports_state:[XhciPort;XHCI_MAX_PORTS],
 pub capability:CapabilityRegisters,
 pub next_slot:u8,
 pub event_dequeue:usize,
 pub event_cycle:bool,
}
impl XhciController {
 pub const EMPTY:Self=Self{
  object:ObjectId::NULL,mmio_base:0,slots:0,ports:0,initialized:false,
  dcbaa_phys:0,command_ring_phys:0,event_ring_phys:0,erst_phys:0,
  command_ring:XhciRing::EMPTY,event_ring:XhciRing::EMPTY,
  next_slot:1,
  event_dequeue:0,
  event_cycle:true,
  ports_state:[XhciPort::empty(0);XHCI_MAX_PORTS],
  capability:CapabilityRegisters{cap_length:0,version:0,hcs_params1:0,hcs_params2:0,hcs_params3:0,hcc_params1:0,dboff:0,rtsoff:0,hcc_params2:0},
 };
 pub fn configure(&mut self,cap:CapabilityRegisters,mmio_base:u64)->Result<(),&'static str>{
  if mmio_base==0{return Err("xHCI MMIO base missing");}
  self.mmio_base=mmio_base;
  self.capability=cap;
  self.slots=cap.max_slots().min(XHCI_MAX_SLOTS as u16);
  self.ports=cap.max_ports().min(XHCI_MAX_PORTS as u8);
  self.command_ring.reset(); self.event_ring.reset();
  for i in 0..self.ports as usize { self.ports_state[i]=XhciPort::empty((i+1) as u8); }
  Ok(())
 }
}


unsafe fn read32(base:u64,offset:u64)->u32 {
    core::ptr::read_volatile((base + offset) as *const u32)
}
unsafe fn write32(base:u64,offset:u64,value:u32) {
    core::ptr::write_volatile((base + offset) as *mut u32,value);
}

pub unsafe fn probe_mmio(mmio:u64)->CapabilityRegisters {
    CapabilityRegisters {
        cap_length:read32(mmio,0) as u8,
        version:core::ptr::read_volatile((mmio + 2) as *const u16),
        hcs_params1:read32(mmio,4),
        hcs_params2:read32(mmio,8),
        hcs_params3:read32(mmio,12),
        hcc_params1:read32(mmio,16),
        dboff:read32(mmio,20),
        rtsoff:read32(mmio,24),
        hcc_params2:read32(mmio,28),
    }
}

pub unsafe fn reset_controller(mmio:u64,cap:CapabilityRegisters)->Result<(),&'static str> {
    let op=mmio + cap.cap_length as u64;
    let mut cmd=read32(op,0);
    cmd &= !USBCMD_RUN_STOP;
    write32(op,0,cmd);
    for _ in 0..1_000_000 {
        if read32(op,4) & USBSTS_HCH != 0 { break; }
        core::hint::spin_loop();
    }
    if read32(op,4) & USBSTS_HCH == 0 { return Err("xHCI did not halt"); }
    write32(op,0,read32(op,0) | USBCMD_HCRST);
    for _ in 0..1_000_000 {
        let cmd_now=read32(op,0);
        if cmd_now & USBCMD_HCRST == 0 { break; }
        core::hint::spin_loop();
    }
    if read32(op,0) & USBCMD_HCRST != 0 { return Err("xHCI reset timeout"); }
    if read32(op,4) & USBSTS_CNR != 0 { return Err("xHCI controller not ready"); }
    Ok(())
}



pub const fn operational_base(mmio:u64,cap:CapabilityRegisters)->u64 { mmio + cap.cap_length as u64 }
pub const fn portsc_address(mmio:u64,cap:CapabilityRegisters,port:u8)->u64 {
 operational_base(mmio,cap) + XHCI_PORT_REG_BASE + XHCI_PORT_REG_STRIDE * (port.saturating_sub(1) as u64)
}
pub unsafe fn read_port_status(mmio:u64,cap:CapabilityRegisters,port:u8)->Result<u32,&'static str> {
 if port==0 || port>cap.max_ports() { return Err("invalid xHCI port"); }
 Ok(read32(mmio,cap.cap_length as u64 + XHCI_PORT_REG_BASE + XHCI_PORT_REG_STRIDE*((port-1) as u64)))
}
pub unsafe fn reset_port(mmio:u64,cap:CapabilityRegisters,port:u8)->Result<(),&'static str> {
 if port==0 || port>cap.max_ports() { return Err("invalid xHCI port"); }
 let addr=portsc_address(mmio,cap,port);
 let mut v=read32(mmio,cap.cap_length as u64 + XHCI_PORT_REG_BASE + XHCI_PORT_REG_STRIDE*((port-1) as u64));
 if v & PORTSC_CCS == 0 { return Err("USB device not connected"); }
 v &= !(PORTSC_CSC|PORTSC_PRC|PORTSC_PLC|PORTSC_CEC);
 v |= PORTSC_PR;
 core::ptr::write_volatile(addr as *mut u32,v);
 for _ in 0..1_000_000 {
  let now=core::ptr::read_volatile(addr as *const u32);
  if now & PORTSC_PR == 0 {
   if now & PORTSC_PED != 0 { return Ok(()); }
   return Err("USB port reset completed without enable");
  }
  core::hint::spin_loop();
 }
 Err("USB port reset timeout")
}
pub unsafe fn acknowledge_port_changes(mmio:u64,cap:CapabilityRegisters,port:u8) {
 if port==0 || port>cap.max_ports() { return; }
 let addr=portsc_address(mmio,cap,port);
 let v=core::ptr::read_volatile(addr as *const u32);
 let clear=v & (PORTSC_CSC|PORTSC_PRC|PORTSC_PLC|PORTSC_CEC);
 if clear!=0 { core::ptr::write_volatile(addr as *mut u32,clear); }
}
pub unsafe fn ring_doorbell(mmio:u64,cap:CapabilityRegisters,target:u8) {
 let addr=mmio + cap.dboff as u64 + (target as u64)*4;
 core::ptr::write_volatile(addr as *mut u32,0);
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct EventRingSegment { pub base:u64,pub size:u16,pub reserved:u16 }
pub unsafe fn setup_dma(controller:&mut XhciController)->Result<(),&'static str>{
    let dcbaa=crate::memory::allocate_frame_below(XHCI_DMA_LIMIT).ok_or("xHCI DCBAA allocation failed")?;
    let command=crate::memory::allocate_frame_below(XHCI_DMA_LIMIT).ok_or("xHCI command ring allocation failed")?;
    let event=crate::memory::allocate_frame_below(XHCI_DMA_LIMIT).ok_or("xHCI event ring allocation failed")?;
    let erst=crate::memory::allocate_frame_below(XHCI_DMA_LIMIT).ok_or("xHCI ERST allocation failed")?;
    core::ptr::write_bytes(dcbaa as *mut u8,0,4096);
    core::ptr::write_bytes(command as *mut u8,0,4096);
    core::ptr::write_bytes(event as *mut u8,0,4096);
    core::ptr::write_bytes(erst as *mut u8,0,4096);
    let link=Trb{parameter:command,status:0,control:TRB_TYPE_LINK|0x3};
    core::ptr::write_volatile((command as *mut Trb).add(XHCI_RING_TRBS-1),link);
    let segment=EventRingSegment{base:event,size:(XHCI_RING_TRBS-1) as u16,reserved:0};
    core::ptr::write_volatile(erst as *mut EventRingSegment,segment);
    controller.dcbaa_phys=dcbaa;
    controller.command_ring_phys=command;
    controller.event_ring_phys=event;
    controller.erst_phys=erst;
    Ok(())
}

pub unsafe fn start_controller(controller:&mut XhciController,cap:CapabilityRegisters)->Result<(),&'static str>{
    if controller.mmio_base==0 || controller.dcbaa_phys==0 { return Err("xHCI DMA not configured"); }
    let op=controller.mmio_base+cap.cap_length as u64;
    let max_slots=controller.slots.max(1) as u32;
    write32(op,0x38,max_slots);
    core::ptr::write_volatile((op+0x18) as *mut u64,controller.command_ring_phys|1);
    core::ptr::write_volatile((op+0x30) as *mut u64,controller.dcbaa_phys);
    let rt=controller.mmio_base+cap.rtsoff as u64;
    let ir0=rt+0x20;
    core::ptr::write_volatile((ir0+0x00) as *mut u32,IMAN_INTERRUPT_ENABLE);
    core::ptr::write_volatile((ir0+0x04) as *mut u32,0);
    core::ptr::write_volatile((ir0+0x08) as *mut u32,1);
    core::ptr::write_volatile((ir0+0x10) as *mut u64,controller.erst_phys);
    core::ptr::write_volatile((ir0+0x18) as *mut u64,controller.event_ring_phys);
    write32(op,0,read32(op,0)|USBCMD_INTE|USBCMD_RUN_STOP);
    for _ in 0..1_000_000 { if read32(op,4)&USBSTS_HCH==0 { controller.initialized=true; return Ok(()); } core::hint::spin_loop(); }
    Err("xHCI failed to start")
}


#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbSetupPacket {
 pub request_type:u8,pub request:u8,pub value:u16,pub index:u16,pub length:u16,
}
impl UsbSetupPacket {
 pub const fn get_descriptor(kind:u8,index:u8,length:u16)->Self {
  Self{request_type:0x80,request:6,value:((kind as u16)<<8)|index as u16,index:0,length}
 }
}

#[derive(Clone,Copy)]
pub struct XhciEvent {
 pub trb:Trb,
}
impl XhciEvent {
 pub const fn event_type(&self)->u32 { self.trb.control & 0x3f00 }
 pub const fn completion_code(&self)->u8 { ((self.trb.status >> TRB_COMPLETION_CODE_SHIFT)&0xff) as u8 }
 pub const fn slot_id(&self)->u8 { ((self.trb.control>>24)&0xff) as u8 }
 pub const fn port_id(&self)->u8 { ((self.trb.parameter>>24)&0xff) as u8 }
}

pub unsafe fn poll_event(controller:&mut XhciController)->Option<XhciEvent> {
 if controller.event_ring_phys==0 || controller.mmio_base==0 { return controller.event_ring.pop().map(|trb|XhciEvent{trb}); }
 let trb_ptr=(controller.event_ring_phys as *const Trb).add(controller.event_dequeue);
 let trb=core::ptr::read_volatile(trb_ptr);
 let cycle=trb.control & 1 != 0;
 if cycle != controller.event_cycle { return None; }
 controller.event_dequeue += 1;
 if controller.event_dequeue >= XHCI_RING_TRBS-1 {
  controller.event_dequeue=0;
  controller.event_cycle=!controller.event_cycle;
 }
 let rt=controller.mmio_base + controller.capability.rtsoff as u64;
 let erdp=controller.event_ring_phys + (controller.event_dequeue as u64)*core::mem::size_of::<Trb>() as u64;
 core::ptr::write_volatile((rt+0x20+0x18) as *mut u64,erdp | (1u64<<3));
 Some(XhciEvent{trb})
}

pub fn consume_event(controller:&mut XhciController)->Option<XhciEvent> {
 unsafe { poll_event(controller) }
}

pub fn handle_event(controller:&mut XhciController,event:XhciEvent)->Option<u8> {
 match event.event_type() {
  TRB_TYPE_PORT_STATUS => {
   let port=event.port_id();
   if port==0 || port as usize>XHCI_MAX_PORTS { return None; }
   let state=&mut controller.ports_state[(port-1) as usize];
   state.status=event.trb.status;
   state.connected=true;
   Some(port)
  }
  TRB_TYPE_CMD_COMPLETION => {
   let slot=event.slot_id();
   if slot!=0 { Some(slot) } else { None }
  }
  _=>None,
 }
}

pub fn enable_slot_trb()->Trb { Trb{parameter:0,status:0,control:TRB_TYPE_ENABLE_SLOT} }
pub fn enqueue_enable_slot(controller:&mut XhciController)->Result<u64,&'static str>{
 let index=controller.command_ring.push_with_index(enable_slot_trb())?;
 unsafe {
  let physical=(controller.command_ring_phys as *mut Trb).add(index);
  core::ptr::write_volatile(physical,controller.command_ring.trbs[index]);
  ring_doorbell(controller.mmio_base,controller.capability,0);
 }
 Ok(controller.command_ring_phys + (index as u64)*core::mem::size_of::<Trb>() as u64)
}
pub fn enqueue_address_device(controller:&mut XhciController,input_context:u64,slot:u8)->Result<u64,&'static str>{
 let index=controller.command_ring.push_with_index(address_device_trb(input_context,slot))?;
 unsafe {
  let physical=(controller.command_ring_phys as *mut Trb).add(index);
  core::ptr::write_volatile(physical,controller.command_ring.trbs[index]);
  ring_doorbell(controller.mmio_base,controller.capability,0);
 }
 Ok(controller.command_ring_phys + (index as u64)*core::mem::size_of::<Trb>() as u64)
}

pub fn address_device_trb(input_context:u64,slot:u8)->Trb {
 Trb{parameter:input_context,status:0,control:TRB_TYPE_ADDRESS_DEVICE|((slot as u32)<<24)}
}

pub fn setup_stage_trb(setup:UsbSetupPacket,transfer_type:u32)->Trb {
 let packed=(setup.request_type as u64)
  |((setup.request as u64)<<8)
  |((setup.value as u64)<<16)
  |((setup.index as u64)<<32)
  |((setup.length as u64)<<48);
 Trb{parameter:packed,status:0,control:TRB_TYPE_SETUP_STAGE|transfer_type}
}
