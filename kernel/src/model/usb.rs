#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum HostControllerKind{Unknown=0,Uhci=1,Ohci=2,Ehci=3,Xhci=4}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum TransferKind{Control=0,Bulk=1,Interrupt=2,Isochronous=3}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbAddress{pub bus:u8,pub address:u8}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbEndpoint{pub address:u8,pub transfer:TransferKind,pub max_packet:u16,pub interval:u8}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbDeviceDescriptor{pub address:UsbAddress,pub vendor:u16,pub product:u16,pub class_code:u8,pub subclass:u8,pub protocol:u8}
impl UsbDeviceDescriptor{pub const EMPTY:Self=Self{address:UsbAddress{bus:0,address:0},vendor:0,product:0,class_code:0,subclass:0,protocol:0};}


pub const USB_DEVICE_DESCRIPTOR_TYPE:u8=1;
pub const USB_CONFIGURATION_DESCRIPTOR_TYPE:u8=2;
pub const USB_MAX_ADDRESS:u8=127;
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum UsbDeviceState{Detached=0,Default=1,Addressed=2,Configured=3}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbConfiguration {
 pub configuration:u8,
 pub interfaces:u8,
 pub max_power_ma:u16,
}
impl UsbConfiguration { pub const EMPTY:Self=Self{configuration:0,interfaces:0,max_power_ma:0}; }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbDevice {
 pub object:super::ObjectId,
 pub controller:super::ObjectId,
 pub descriptor:UsbDeviceDescriptor,
 pub configuration:UsbConfiguration,
 pub topology:UsbDeviceTopology,
 pub slot:u8,
 pub port:u8,
 pub state:UsbDeviceState,
 pub configured:bool,
}
impl UsbDevice {
 pub const EMPTY:Self=Self{object:super::ObjectId::NULL,controller:super::ObjectId::NULL,descriptor:UsbDeviceDescriptor::EMPTY,configuration:UsbConfiguration::EMPTY,topology:UsbDeviceTopology::EMPTY,slot:0,port:0,state:UsbDeviceState::Detached,configured:false};
 pub const fn is_mass_storage(&self)->bool {
  self.descriptor.class_code==8
 }
}


pub const USB_INTERFACE_DESCRIPTOR_TYPE:u8=4;
pub const USB_ENDPOINT_DESCRIPTOR_TYPE:u8=5;
pub const USB_CLASS_MASS_STORAGE:u8=8;
pub const USB_SUBCLASS_SCSI_TRANSPARENT:u8=6;
pub const USB_PROTOCOL_BULK_ONLY:u8=0x50;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbInterface {
 pub number:u8,
 pub alternate:u8,
 pub class_code:u8,
 pub subclass:u8,
 pub protocol:u8,
 pub endpoint_count:u8,
}
impl UsbInterface {
 pub const EMPTY:Self=Self{number:0,alternate:0,class_code:0,subclass:0,protocol:0,endpoint_count:0};
 pub const fn is_mass_storage(&self)->bool {
  self.class_code==USB_CLASS_MASS_STORAGE &&
  self.subclass==USB_SUBCLASS_SCSI_TRANSPARENT &&
  self.protocol==USB_PROTOCOL_BULK_ONLY
 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbEndpointDescriptor {
 pub address:u8,
 pub attributes:u8,
 pub max_packet:u16,
 pub interval:u8,
}
impl UsbEndpointDescriptor {
 pub const EMPTY:Self=Self{address:0,attributes:0,max_packet:0,interval:0,interface_number:0};
 pub const fn transfer_kind(&self)->TransferKind {
  match self.attributes & 0x03 {
   1=>TransferKind::Isochronous,
   2=>TransferKind::Bulk,
   3=>TransferKind::Interrupt,
   _=>TransferKind::Control,
  }
 }
 pub const fn is_in(&self)->bool { self.address & 0x80 != 0 }
}

#[repr(u8)]
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum UsbMassStorageStage { Idle=0, Command=1, Data=2, Status=3, Failed=4 }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbMassStorageTransport {
 pub interface_number:u8,
 pub bulk_in:u8,
 pub bulk_out:u8,
 pub max_packet_in:u16,
 pub max_packet_out:u16,
 pub tag:u32,
 pub stage:UsbMassStorageStage,
 pub block_size:u32,
 pub block_count:u64,
}
impl UsbMassStorageTransport {
 pub const EMPTY:Self=Self{
  interface_number:0,bulk_in:0,bulk_out:0,max_packet_in:0,max_packet_out:0,
  tag:0,stage:UsbMassStorageStage::Idle,block_size:0,block_count:0,
 };
 pub const fn valid(&self)->bool { self.bulk_in!=0 && self.bulk_out!=0 && self.max_packet_in!=0 && self.max_packet_out!=0 }
 pub fn next_tag(&mut self)->u32 { self.tag=self.tag.wrapping_add(1).max(1); self.tag }
}

pub const USB_MAX_INTERFACES:usize=32;
pub const USB_MAX_ENDPOINTS:usize=64;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbDeviceTopology {
 pub interfaces:[UsbInterface;USB_MAX_INTERFACES],
 pub interface_count:u8,
 pub endpoints:[UsbEndpointDescriptor;USB_MAX_ENDPOINTS],
 pub endpoint_count:u8,
}
impl UsbDeviceTopology {
 pub const EMPTY:Self=Self{
  interfaces:[UsbInterface::EMPTY;USB_MAX_INTERFACES],interface_count:0,
  endpoints:[UsbEndpointDescriptor::EMPTY;USB_MAX_ENDPOINTS],endpoint_count:0,
 };
 pub fn add_interface(&mut self,v:UsbInterface)->Result<(),&'static str>{
  if self.interface_count as usize>=USB_MAX_INTERFACES{return Err("USB interface table full");}
  self.interfaces[self.interface_count as usize]=v; self.interface_count+=1; Ok(())
 }
 pub fn add_endpoint(&mut self,v:UsbEndpointDescriptor)->Result<(),&'static str>{
  if self.endpoint_count as usize>=USB_MAX_ENDPOINTS{return Err("USB endpoint table full");}
  self.endpoints[self.endpoint_count as usize]=v; self.endpoint_count+=1; Ok(())
 }
 pub fn mass_storage_interface(&self)->Option<UsbInterface>{
  for i in 0..self.interface_count as usize { if self.interfaces[i].is_mass_storage(){return Some(self.interfaces[i]);} }
  None
 }
 pub fn mass_storage_transport(&self)->UsbMassStorageTransport {
  let interface=match self.mass_storage_interface(){Some(v)=>v,None=>return UsbMassStorageTransport::EMPTY};
  let mut out=UsbMassStorageTransport::EMPTY;
  out.interface_number=interface.number;
  for i in 0..self.endpoint_count as usize {
   let ep=self.endpoints[i];
   if ep.interface_number!=interface.number || ep.transfer_kind()!=TransferKind::Bulk { continue; }
   if ep.is_in() && out.bulk_in==0 { out.bulk_in=ep.address; out.max_packet_in=ep.max_packet; }
   if !ep.is_in() && out.bulk_out==0 { out.bulk_out=ep.address; out.max_packet_out=ep.max_packet; }
  }
  out
 }
}

pub const USB_BOT_CBW_LENGTH:usize=31;
pub const USB_BOT_CSW_LENGTH:usize=13;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbMassStorageBotCbw {
 pub signature:u32,
 pub tag:u32,
 pub transfer_length:u32,
 pub flags:u8,
 pub lun:u8,
 pub command_length:u8,
 pub command:[u8;16],
}
impl UsbMassStorageBotCbw {
 pub fn encode(&self,out:&mut [u8;USB_BOT_CBW_LENGTH]) {
  *out=[0;USB_BOT_CBW_LENGTH];
  out[0..4].copy_from_slice(&self.signature.to_le_bytes());
  out[4..8].copy_from_slice(&self.tag.to_le_bytes());
  out[8..12].copy_from_slice(&self.transfer_length.to_le_bytes());
  out[12]=self.flags; out[13]=self.lun; out[14]=self.command_length;
  out[15..31].copy_from_slice(&self.command);
 }
 pub const SIGNATURE:u32=0x43425355;
 pub const EMPTY:Self=Self{signature:Self::SIGNATURE,tag:0,transfer_length:0,flags:0,lun:0,command_length:0,command:[0;16]};
 pub const fn new(tag:u32,transfer_length:u32,in_direction:bool,lun:u8,command_length:u8,command:[u8;16])->Self{
  Self{signature:Self::SIGNATURE,tag,transfer_length,flags:if in_direction{0x80}else{0},lun,command_length,command}
 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct UsbMassStorageBotCsw {
 pub signature:u32,
 pub tag:u32,
 pub residue:u32,
 pub status:u8,
}
impl UsbMassStorageBotCsw {
 pub fn decode(bytes:&[u8;USB_BOT_CSW_LENGTH])->Self {
  Self{
   signature:u32::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]),
   tag:u32::from_le_bytes([bytes[4],bytes[5],bytes[6],bytes[7]]),
   residue:u32::from_le_bytes([bytes[8],bytes[9],bytes[10],bytes[11]]),
   status:bytes[12],
  }
 }
 pub const SIGNATURE:u32=0x53425355;
 pub const STATUS_PASSED:u8=0;
 pub const STATUS_FAILED:u8=1;
 pub const STATUS_PHASE_ERROR:u8=2;
 pub const fn valid(&self,expected_tag:u32)->bool {
  self.signature==Self::SIGNATURE && self.tag==expected_tag && self.status<=Self::STATUS_PHASE_ERROR
 }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct ScsiCommand {
 pub bytes:[u8;16],
 pub length:u8,
}
impl ScsiCommand {
 pub const fn test_unit_ready()->Self { Self{bytes:[0;16],length:6} }
 pub const fn inquiry(allocation:u8)->Self { Self{bytes:[0x12,0,0,0,allocation,0,0,0,0,0,0,0,0,0,0,0],length:6} }
 pub const fn read_capacity10()->Self { Self{bytes:[0x25,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],length:10} }
 pub const fn read10(lba:u32,blocks:u16)->Self { Self{bytes:[0x28,0,(lba>>24) as u8,(lba>>16) as u8,(lba>>8) as u8,lba as u8,0,(blocks>>8) as u8,blocks as u8,0,0,0,0,0,0,0],length:10} }
 pub const fn write10(lba:u32,blocks:u16)->Self { Self{bytes:[0x2A,0,(lba>>24) as u8,(lba>>16) as u8,(lba>>8) as u8,lba as u8,0,(blocks>>8) as u8,blocks as u8,0,0,0,0,0,0,0],length:10} }
}
