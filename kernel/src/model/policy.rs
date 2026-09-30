use super::ObjectId;

#[repr(u8)]
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum StoragePolicy { DenyInternal=0, ExplicitOnly=1, Trusted=2 }

#[repr(C)]
#[derive(Clone,Copy)]
pub struct AccessPolicy {
 pub boot_device:ObjectId,
 pub storage:StoragePolicy,
 pub allow_internal:bool,
}
impl AccessPolicy {
 pub const USB_FIRST:Self=Self{boot_device:ObjectId::NULL,storage:StoragePolicy::ExplicitOnly,allow_internal:false};
 pub const fn permits_internal(&self)->bool{self.allow_internal}
}
