use super::object::ObjectId;

pub const MAX_BLOCK_DEVICES: usize = 16;
pub const MAX_BLOCK_REQUESTS: usize = 64;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockKind { Unknown=0, UsbMassStorage=1, Memory=2 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockGeometry { pub block_size:u32, pub block_count:u64 }
impl BlockGeometry {
    pub const EMPTY: Self = Self { block_size:0, block_count:0 };
    pub const fn byte_len(&self) -> u64 { self.block_size as u64 * self.block_count }
    pub const fn valid(&self) -> bool { self.block_size != 0 && self.block_count != 0 }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockDevice {
    pub object:ObjectId, pub kind:BlockKind, pub geometry:BlockGeometry,
    pub removable:bool, pub writable:bool,
}
impl BlockDevice {
    pub const EMPTY: Self = Self {
        object:ObjectId::NULL, kind:BlockKind::Unknown, geometry:BlockGeometry::EMPTY,
        removable:false, writable:false,
    };
    pub fn is_usb_storage(&self) -> bool { self.kind == BlockKind::UsbMassStorage && self.removable }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockOp { Read=1, Write=2, Flush=3 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockRequest {
    pub operation:BlockOp, pub device:ObjectId, pub lba:u64, pub blocks:u32,
    pub buffer:u64, pub token:u64,
}
impl BlockRequest {
    pub const EMPTY: Self = Self {
        operation:BlockOp::Read, device:ObjectId::NULL, lba:0, blocks:0, buffer:0, token:0, owner_cell:0,
    };
    pub const fn is_valid(&self) -> bool {
        !self.device.is_null() && self.blocks != 0 && self.token != 0
    }
}


pub fn geometry_from_read_capacity10(bytes:&[u8]) -> Result<BlockGeometry,&'static str> {
    if bytes.len()<8 { return Err("READ CAPACITY(10) response truncated"); }
    let last_lba=u32::from_be_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]) as u64;
    let block_size=u32::from_be_bytes([bytes[4],bytes[5],bytes[6],bytes[7]]);
    if block_size==0 { return Err("invalid SCSI block size"); }
    let block_count=last_lba.checked_add(1).ok_or("SCSI block count overflow")?;
    Ok(BlockGeometry{block_size,block_count})
}

pub fn lba_byte_range(geometry:BlockGeometry,lba:u64,blocks:u32)->Option<(u64,u64)> {
    if !geometry.valid() || blocks==0 || lba>=geometry.block_count { return None; }
    let end_lba=lba.checked_add(blocks as u64)?;
    if end_lba>geometry.block_count { return None; }
    let offset=lba.checked_mul(geometry.block_size as u64)?;
    let length=(blocks as u64).checked_mul(geometry.block_size as u64)?;
    Some((offset,length))
}


#[repr(u8)]
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum ScsiOpcode { TestUnitReady=0x00, Inquiry=0x12, ReadCapacity10=0x25, Read10=0x28, Write10=0x2a }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct ScsiBlockCommand {
 pub command: super::ScsiCommand,
 pub lba:u64,
 pub blocks:u32,
 pub direction_in:bool,
}
impl ScsiBlockCommand {
 pub const fn inquiry() -> Self {
  Self{command:super::ScsiCommand::inquiry(36),lba:0,blocks:0,direction_in:true}
 }
 pub const fn test_unit_ready() -> Self {
  Self{command:super::ScsiCommand::test_unit_ready(),lba:0,blocks:0,direction_in:false}
 }
 pub const fn read_capacity10() -> Self {
  Self{command:super::ScsiCommand::read_capacity10(),lba:0,blocks:0,direction_in:true}
 }
 pub const fn read10(lba:u64,blocks:u32) -> Option<Self> {
  if lba>0xffff_ffff || blocks==0 || blocks>0xffff { return None; }
  Some(Self{command:super::ScsiCommand::read10(lba as u32,blocks as u16),lba,blocks,direction_in:true})
 }
 pub const fn write10(lba:u64,blocks:u32) -> Option<Self> {
  if lba>0xffff_ffff || blocks==0 || blocks>0xffff { return None; }
  Some(Self{command:super::ScsiCommand::write10(lba as u32,blocks as u16),lba,blocks,direction_in:false})
 }
 pub fn transfer_bytes(&self,geometry:BlockGeometry)->Option<u64> {
  if self.blocks==0 { return Some(0); }
  let range=lba_byte_range(geometry,self.lba,self.blocks)?;
  Some(range.1)
 }
}
