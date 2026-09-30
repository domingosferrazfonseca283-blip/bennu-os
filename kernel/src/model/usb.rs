#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum HostControllerKind{Unknown=0,Uhci=1,Ohci=2,Ehci=3,Xhci=4}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum TransferKind{Control=0,Bulk=1,Interrupt=2,Isochronous=3}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbAddress{pub bus:u8,pub address:u8}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbEndpoint{pub address:u8,pub transfer:TransferKind,pub max_packet:u16,pub interval:u8}
#[repr(C)]#[derive(Clone,Copy)]pub struct UsbDeviceDescriptor{pub address:UsbAddress,pub vendor:u16,pub product:u16,pub class_code:u8,pub subclass:u8,pub protocol:u8}
impl UsbDeviceDescriptor{pub const EMPTY:Self=Self{address:UsbAddress{bus:0,address:0},vendor:0,product:0,class_code:0,subclass:0,protocol:0};}
