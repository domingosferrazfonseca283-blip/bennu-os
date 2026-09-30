use super::ObjectId;

pub const BENNUFS_MAGIC: u64 = 0x4245_4e4e_5546_5301;
pub const BENNUFS_VERSION: u32 = 1;
pub const BENNUFS_BLOCK_SIZE: u32 = 4096;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Superblock {
    pub magic: u64,
    pub version: u32,
    pub block_size: u32,
    pub total_blocks: u64,
    pub metadata_blocks: u64,
    pub free_blocks: u64,
    pub root_object: ObjectId,
    pub journal_start: u64,
    pub journal_blocks: u64,
    pub sequence: u64,
}

impl Superblock {
    pub const EMPTY: Self = Self {
        magic: BENNUFS_MAGIC,
        version: BENNUFS_VERSION,
        block_size: BENNUFS_BLOCK_SIZE,
        total_blocks: 0,
        metadata_blocks: 0,
        free_blocks: 0,
        root_object: ObjectId::NULL,
        journal_start: 0,
        journal_blocks: 0,
        sequence: 0,
    };

    pub const fn valid(&self) -> bool {
        self.magic == BENNUFS_MAGIC
            && self.version == BENNUFS_VERSION
            && self.block_size == BENNUFS_BLOCK_SIZE
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind { Free=0, Object=1, Directory=2, Stream=3, Device=4, Surface=5 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub id: ObjectId,
    pub parent: ObjectId,
    pub kind: NodeKind,
    pub flags: u32,
    pub size: u64,
    pub first_block: u64,
    pub block_count: u64,
}

impl Node {
    pub const EMPTY: Self = Self {
        id: ObjectId::NULL, parent: ObjectId::NULL, kind: NodeKind::Free,
        flags: 0, size: 0, first_block: 0, block_count: 0,
    };
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JournalOp { Begin=1, Map=2, Node=3, Commit=4, Checkpoint=5 }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JournalRecord {
    pub sequence: u64,
    pub operation: JournalOp,
    pub object: ObjectId,
    pub block: u64,
    pub value: u64,
}
