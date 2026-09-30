use super::object::ObjectId;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapabilityId(pub u64);

impl CapabilityId {
    pub const NULL: Self = Self(0);
    pub const fn new(_object: ObjectId, slot: u32, generation: u32) -> Self {
        Self(((generation as u64) << 32) | slot as u64)
    }
    pub const fn slot(self)->usize { (self.0 & 0xffff_ffff) as usize }
    pub const fn generation(self)->u32 { (self.0 >> 32) as u32 }
}

#[repr(transparent)]
#[derive(Clone,Copy,PartialEq,Eq)]
pub struct CapabilityRights(pub u32);
impl CapabilityRights {
    pub const NONE:Self=Self(0);
    pub const READ:Self=Self(1<<0);
    pub const WRITE:Self=Self(1<<1);
    pub const EXECUTE:Self=Self(1<<2);
    pub const OBSERVE:Self=Self(1<<3);
    pub const DRAW:Self=Self(1<<4);
    pub const SHARE:Self=Self(1<<5);
    pub const MAP:Self=Self(1<<6);
    pub const DEVICE:Self=Self(1<<7);
    pub const ADMIN:Self=Self(1<<31);
    pub const fn contains(self,required:Self)->bool{(self.0&required.0)==required.0}
    pub const fn union(self,other:Self)->Self{Self(self.0|other.0)}
}
#[repr(C)]#[derive(Clone,Copy)]
pub struct Capability {
    pub id:CapabilityId,pub object:ObjectId,pub rights:CapabilityRights,
    pub owner_cell:u32,pub generation:u32,
}
impl Capability {
    pub const EMPTY:Self=Self{id:CapabilityId::NULL,object:ObjectId::NULL,rights:CapabilityRights::NONE,owner_cell:0,generation:0};
    pub const fn permits(&self,cell:u32,capability:CapabilityId,object:ObjectId,rights:CapabilityRights)->bool{
        self.id.0==capability.0 && self.owner_cell==cell && self.object.0==object.0 &&
        self.generation==capability.generation() && self.rights.contains(rights) && !object.is_null()
    }
}
