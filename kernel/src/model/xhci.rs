use super::ObjectId;

pub const XHCI_MAX_SLOTS:usize=256;
pub const XHCI_RING_TRBS:usize=256;
pub const XHCI_MAX_PORTS:usize=256;

pub const USBCMD_RUN_STOP:u32=1<<0;
pub const USBCMD_HCRST:u32=1<<1;
pub const USBSTS_HCH:u32=1<<0;
pub const USBSTS_CNR:u32=1<<11;

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
 pub fn push(&mut self,trb:Trb)->Result<(),&'static str>{
  let next=(self.enqueue+1)%XHCI_RING_TRBS;
  if next==self.dequeue{return Err("xHCI ring full");}
  self.trbs[self.enqueue]=trb.with_cycle(self.cycle);
  self.enqueue=next; Ok(())
 }
 pub fn pop(&mut self)->Option<Trb>{
  if self.dequeue==self.enqueue{return None;}
  let trb=self.trbs[self.dequeue]; self.dequeue=(self.dequeue+1)%XHCI_RING_TRBS; Some(trb)
 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct XhciPort { pub index:u8,pub status:u32,pub connected:bool,pub enabled:bool }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct XhciController {
 pub object:ObjectId,pub mmio_base:u64,pub slots:u16,pub ports:u8,pub initialized:bool,
 pub command_ring:XhciRing,pub event_ring:XhciRing,pub ports_state:[XhciPort;XHCI_MAX_PORTS],
}
impl XhciController {
 pub const EMPTY:Self=Self{
  object:ObjectId::NULL,mmio_base:0,slots:0,ports:0,initialized:false,
  command_ring:XhciRing::EMPTY,event_ring:XhciRing::EMPTY,
  ports_state:[XhciPort{index:0,status:0,connected:false,enabled:false};XHCI_MAX_PORTS],
 };
 pub fn configure(&mut self,cap:CapabilityRegisters,mmio_base:u64)->Result<(),&'static str>{
  if mmio_base==0{return Err("xHCI MMIO base missing");}
  self.mmio_base=mmio_base;
  self.slots=cap.max_slots().min(XHCI_MAX_SLOTS as u16);
  self.ports=cap.max_ports().min(XHCI_MAX_PORTS as u8);
  self.command_ring.reset(); self.event_ring.reset();
  for i in 0..self.ports as usize { self.ports_state[i].index=(i+1) as u8; }
  Ok(())
 }
}
