#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum HostControllerKind{Unknown=0,Uhci=1,Ohci=2,Ehci=3,Xhci=4}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum TransferKind{Control=0,Bulk=1,Interrupt=2,Isochronous=3}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbAddress{pub bus:u8,pub address:u8}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbEndpoint{pub address:u8,pub transfer:TransferKind,pub max_packet:u16,pub interval:u8}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbDeviceDescriptor{pub address:UsbAddress,pub vendor:u16,pub product:u16,pub class_code:u8,pub subclass:u8,pub protocol:u8}
impl UsbDeviceDescriptor{pub const EMPTY:Self=Self{address:UsbAddress{bus:0,address:0},vendor:0,product:0,class_code:0,subclass:0,protocol:0};}


pub const USB_DEVICE_DESCRIPTOR_TYPE:u8=1;
pub const USB_CONFIGURATION_DESCRIPTOR_TYPE:u8=2;
pub const USB_MAX_ADDRESS:u8=127;

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
 pub descriptor:UsbDeviceDescriptor,
 pub configuration:UsbConfiguration,
 pub configured:bool,
}
impl UsbDevice {
 pub const EMPTY:Self=Self{object:super::ObjectId::NULL,descriptor:UsbDeviceDescriptor::EMPTY,configuration:UsbConfiguration::EMPTY,configured:false};
 pub const fn is_mass_storage(&self)->bool {
  self.descriptor.class_code==8
 }
}
