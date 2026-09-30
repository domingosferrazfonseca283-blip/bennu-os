use super::{AddressSpaceId,CellId,ObjectId};

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Domain {
 pub object:ObjectId,
 pub cell:CellId,
 pub address_space:AddressSpaceId,
 pub parent:ObjectId,
 pub flags:u32,
}
impl Domain { pub const EMPTY:Self=Self{object:ObjectId::NULL,cell:CellId(0),address_space:AddressSpaceId(0),parent:ObjectId::NULL,flags:0}; }
