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
    /// Default Bennu policy: the removable USB boot medium is the only
    /// storage authority until another device is explicitly granted.
    pub const USB_FIRST:Self=Self{
        boot_device:ObjectId::NULL,
        storage:StoragePolicy::ExplicitOnly,
        allow_internal:false,
    };

    pub const fn permits_internal(&self)->bool {
        self.allow_internal && self.storage == StoragePolicy::Trusted
    }

    pub const fn permits_storage(&self, object:ObjectId, removable:bool, is_boot_device:bool)->bool {
        if removable || is_boot_device {
            return !object.is_null();
        }
        self.permits_internal()
    }
}
