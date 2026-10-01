use super::ObjectId;

pub const BENNUFS_MAGIC: u64 = 0x4245_4e4e_5546_5301;
pub const BENNUFS_VERSION: u32 = 1;
pub const BENNUFS_BLOCK_SIZE: u32 = 4096;
pub const BENNUFS_SUPERBLOCK_BLOCK: u64 = 0;
pub const BENNUFS_METADATA_START: u64 = 1;
pub const BENNUFS_DEFAULT_JOURNAL_BLOCKS: u64 = 256;

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

    /// Build the initial on-disk layout for a block device.
    ///
    /// The first block is the superblock; the journal follows the metadata
    /// reservation. The journal size is capped by the available medium so a
    /// small USB device can still be formatted safely.
    pub const fn format(total_blocks: u64, journal_blocks: u64, root_object: ObjectId) -> Option<Self> {
        if total_blocks < 8 || journal_blocks == 0 {
            return None;
        }

        let journal_blocks = if journal_blocks > total_blocks / 4 {
            total_blocks / 4
        } else {
            journal_blocks
        };
        if journal_blocks == 0 {
            return None;
        }

        let journal_start = BENNUFS_METADATA_START;
        let metadata_blocks = journal_start
            .checked_add(journal_blocks)?;
        if metadata_blocks >= total_blocks {
            return None;
        }

        Some(Self {
            magic: BENNUFS_MAGIC,
            version: BENNUFS_VERSION,
            block_size: BENNUFS_BLOCK_SIZE,
            total_blocks,
            metadata_blocks,
            free_blocks: total_blocks - metadata_blocks,
            root_object,
            journal_start,
            journal_blocks,
            sequence: 1,
        })
    }

    pub const fn valid(&self) -> bool {
        if self.magic != BENNUFS_MAGIC
            || self.version != BENNUFS_VERSION
            || self.block_size != BENNUFS_BLOCK_SIZE
            || self.total_blocks < 8
            || self.journal_start < BENNUFS_METADATA_START
            || self.journal_blocks == 0
            || self.metadata_blocks < self.journal_start
        {
            return false;
        }

        let journal_end = match self.journal_start.checked_add(self.journal_blocks) {
            Some(end) => end,
            None => return false,
        };
        if journal_end > self.total_blocks || self.metadata_blocks > journal_end {
            return false;
        }

        self.free_blocks <= self.total_blocks
            && self.free_blocks == self.total_blocks - self.metadata_blocks
    }

    pub const fn journal_block(&self, index: u64) -> Option<u64> {
        if index >= self.journal_blocks {
            return None;
        }
        self.journal_start.checked_add(index)
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

    pub const fn valid_extent(&self, total_blocks: u64) -> bool {
        if self.id.is_null() {
            return self.kind == NodeKind::Free
                && self.size == 0
                && self.block_count == 0;
        }

        if self.block_count == 0 {
            return self.size == 0;
        }

        let end = match self.first_block.checked_add(self.block_count) {
            Some(end) => end,
            None => return false,
        };
        self.first_block < total_blocks && end <= total_blocks
    }
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

impl JournalRecord {
    pub const EMPTY: Self = Self {
        sequence: 0,
        operation: JournalOp::Begin,
        object: ObjectId::NULL,
        block: 0,
        value: 0,
    };

    pub const fn valid_for(&self, superblock: &Superblock) -> bool {
        if self.sequence == 0 || !superblock.valid() {
            return false;
        }

        match self.operation {
            JournalOp::Begin | JournalOp::Commit | JournalOp::Checkpoint => true,
            JournalOp::Map | JournalOp::Node => self.block < superblock.total_blocks,
        }
    }
}
