#![allow(dead_code)]

use crate::model::{CellId, ObjectId};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExecutableHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub machine: u16,
    pub entry: u64,
    pub image_size: u64,
}
impl ExecutableHeader {
    pub const MAGIC: [u8;4] = *b"BEXE";
    pub fn valid(&self, bytes: usize) -> bool {
        self.magic == Self::MAGIC && self.version == 1 && self.machine == 0x3e && self.entry != 0 && self.image_size as usize <= bytes
    }
}

#[derive(Clone, Copy)]
pub struct Process {
    pub id: CellId,
    pub address_space: u64,
    pub root_object: ObjectId,
    pub entry: u64,
}
impl Process {
    pub const EMPTY: Self = Self { id: CellId(0), address_space: 0, root_object: ObjectId::NULL, entry: 0 };
}
