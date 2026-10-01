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
        let metadata_blocks = match journal_start.checked_add(journal_blocks) {
            Some(value) => value,
            None => return None,
        };
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
            return self.kind as u8 == NodeKind::Free as u8
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


pub const fn blocks_for_bytes(bytes: u64) -> Option<u64> {
    if bytes == 0 { return Some(0); }
    bytes.checked_add(BENNUFS_BLOCK_SIZE as u64 - 1).map(|v| v / BENNUFS_BLOCK_SIZE as u64)
}

pub const fn data_block_start(superblock: &Superblock) -> Option<u64> {
    if !superblock.valid() { return None; }
    Some(superblock.metadata_blocks)
}

pub const fn data_block_end(superblock: &Superblock) -> Option<u64> {
    if !superblock.valid() { return None; }
    Some(superblock.total_blocks)
}

pub const fn data_block_count(superblock: &Superblock) -> Option<u64> {
    if !superblock.valid() { return None; }
    superblock.total_blocks.checked_sub(superblock.metadata_blocks)
}

pub const fn serialize_superblock(superblock: &Superblock, out: &mut [u8]) -> bool {
    if !superblock.valid() || out.len() < core::mem::size_of::<Superblock>() { return false; }
    let src = superblock as *const Superblock as *const u8;
    let mut i = 0;
    while i < core::mem::size_of::<Superblock>() {
        out[i] = unsafe { *src.add(i) };
        i += 1;
    }
    true
}

pub const fn deserialize_superblock(bytes: &[u8]) -> Option<Superblock> {
    if bytes.len() < core::mem::size_of::<Superblock>() { return None; }
    let mut value = Superblock::EMPTY;
    let dst = &mut value as *mut Superblock as *mut u8;
    let mut i = 0;
    while i < core::mem::size_of::<Superblock>() {
        unsafe { *dst.add(i) = bytes[i]; }
        i += 1;
    }
    if value.valid() { Some(value) } else { None }
}

pub const fn block_range(superblock: &Superblock, first: u64, count: u64) -> Option<(u64,u64)> {
    if !superblock.valid() || count == 0 { return None; }
    let end = match first.checked_add(count) { Some(v) => v, None => return None };
    if first < superblock.metadata_blocks || end > superblock.total_blocks { return None; }
    Some((first, end))
}


#[repr(C)]
#[derive(Clone, Copy)]
pub struct FreeMap {
    pub first: u64,
    pub blocks: u64,
}
impl FreeMap {
    pub const EMPTY: Self = Self { first: 0, blocks: 0 };
    pub const fn new(superblock: &Superblock) -> Option<Self> {
        if !superblock.valid() || superblock.free_blocks == 0 { return None; }
        Some(Self { first: superblock.metadata_blocks, blocks: superblock.free_blocks })
    }
    pub const fn contains(&self, block: u64) -> bool {
        block >= self.first && block < self.first.saturating_add(self.blocks)
    }
    pub const fn allocate(&mut self, count: u64) -> Option<u64> {
        if count == 0 || count > self.blocks { return None; }
        let start = self.first;
        self.first = self.first.checked_add(count)?;
        self.blocks -= count;
        Some(start)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JournalTransaction {
    pub sequence: u64,
    pub start: u64,
    pub records: u32,
    pub committed: bool,
}
impl JournalTransaction {
    pub const EMPTY: Self = Self { sequence: 0, start: 0, records: 0, committed: false };
    pub const fn begin(superblock: &Superblock) -> Option<Self> {
        if !superblock.valid() { return None; }
        Some(Self { sequence: superblock.sequence + 1, start: 0, records: 0, committed: false })
    }
    pub fn append(&mut self, record: &JournalRecord) -> Result<(), &'static str> {
        if self.sequence == 0 || record.sequence != self.sequence { return Err("journal sequence mismatch"); }
        if self.records == u32::MAX { return Err("journal transaction full"); }
        self.records += 1;
        Ok(())
    }
    pub fn commit(&mut self) -> Result<(), &'static str> {
        if self.sequence == 0 || self.records == 0 { return Err("empty journal transaction"); }
        self.committed = true;
        Ok(())
    }
}

pub fn format_block_device(total_blocks: u64, root_object: ObjectId, out: &mut [u8]) -> Option<Superblock> {
    let sb = Superblock::format(total_blocks, BENNUFS_DEFAULT_JOURNAL_BLOCKS, root_object)?;
    if out.len() < BENNUFS_BLOCK_SIZE as usize { return None; }
    let mut i = 0;
    while i < BENNUFS_BLOCK_SIZE as usize { out[i] = 0; i += 1; }
    if !serialize_superblock(&sb, out) { return None; }
    Some(sb)
}

pub fn recover_journal(superblock: &Superblock, records: &[JournalRecord]) -> u64 {
    if !superblock.valid() { return 0; }
    let mut highest = superblock.sequence;
    for record in records {
        if record.valid_for(superblock) && record.sequence > highest {
            highest = record.sequence;
        }
    }
    highest
}
