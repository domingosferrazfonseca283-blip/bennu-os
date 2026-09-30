use super::ObjectId;

pub const XHCI_MAX_SLOTS:usize=256;
pub const XHCI_RING_TRBS:usize=256;
pub const XHCI_MAX_PORTS:usize=256;
pub const XHCI_PAGE_SIZE:u64=4096;
pub const XHCI_DMA_LIMIT:u64=64*1024*1024;
pub const TRB_TYPE_LINK:u32=6<<10;
pub const TRB_TYPE_ENABLE_SLOT:u32=9<<10;
pub const TRB_TYPE_NOOP_CMD:u32=23<<10;

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
 pub dcbaa_phys:u64,pub command_ring_phys:u64,pub event_ring_phys:u64,pub erst_phys:u64,
 pub command_ring:XhciRing,pub event_ring:XhciRing,pub ports_state:[XhciPort;XHCI_MAX_PORTS],
}
impl XhciController {
 pub const EMPTY:Self=Self{
  object:ObjectId::NULL,mmio_base:0,slots:0,ports:0,initialized:false,
  dcbaa_phys:0,command_ring_phys:0,event_ring_phys:0,erst_phys:0,
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
    let link=Trb{parameter:command,status:0,control:TRB_TYPE_LINK|1};
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
    write32(op,0x30,max_slots);
    core::ptr::write_volatile((op+0x18) as *mut u64,controller.command_ring_phys|1);
    core::ptr::write_volatile((op+0x30) as *mut u64,controller.dcbaa_phys);
    let rt=controller.mmio_base+cap.rtsoff as u64;
    let ir0=rt+0x20;
    core::ptr::write_volatile((ir0+0x08) as *mut u64,controller.erst_phys);
    core::ptr::write_volatile((ir0+0x10) as *mut u64,controller.event_ring_phys);
    core::ptr::write_volatile((ir0+0x08-8) as *mut u32,1);
    write32(op,0,read32(op,0)|USBCMD_RUN_STOP);
    for _ in 0..1_000_000 { if read32(op,4)&USBSTS_HCH==0 { controller.initialized=true; return Ok(()); } core::hint::spin_loop(); }
    Err("xHCI failed to start")
}
