use super::object::ObjectId;
pub const MAX_BLOCK_DEVICES:usize=16;
pub const MAX_BLOCK_REQUESTS:usize=64;
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum BlockKind{Unknown=0,UsbMassStorage=1,Memory=2}
#[repr(C)]#[derive(Clone,Copy)]pub struct BlockGeometry{pub block_size:u32,pub block_count:u64}
impl BlockGeometry{pub const EMPTY:Self=Self{block_size:0,block_count:0};pub const fn byte_len(&self)->u64{self.block_size as u64*self.block_count}}
#[repr(C)]#[derive(Clone,Copy)]pub struct BlockDevice{pub object:ObjectId,pub kind:BlockKind,pub geometry:BlockGeometry,pub removable:bool,pub writable:bool}
impl BlockDevice{pub const EMPTY:Self=Self{object:ObjectId::NULL,kind:BlockKind::Unknown,geometry:BlockGeometry::EMPTY,removable:false,writable:false};}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum BlockOp{Read=1,Write=2,Flush=3}
#[repr(C)]#[derive(Clone,Copy)]pub struct BlockRequest{pub operation:BlockOp,pub device:ObjectId,pub lba:u64,pub blocks:u32,pub buffer:u64,pub token:u64}
impl BlockRequest{pub const EMPTY:Self=Self{operation:BlockOp::Read,device:ObjectId::NULL,lba:0,blocks:0,buffer:0,token:0};}
