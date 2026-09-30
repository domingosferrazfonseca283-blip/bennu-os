use super::object::ObjectId;

pub const MAX_ADDRESS_SPACES: usize = 32;
pub const MAX_MAPPINGS_PER_SPACE: usize = 128;
pub const PAGE_SIZE: u64 = 4096;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AddressSpaceId(pub u32);

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MappingKind {
    Unmapped = 0,
    Anonymous = 1,
    Object = 2,
    Device = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mapping {
    pub virtual_base: u64,
    pub physical_base: u64,
    pub pages: u64,
    pub flags: u64,
    pub kind: MappingKind,
    pub object: ObjectId,
}

impl Mapping {
    pub const EMPTY: Self = Self {
        virtual_base: 0,
        physical_base: 0,
        pages: 0,
        flags: 0,
        kind: MappingKind::Unmapped,
        object: ObjectId::NULL,
    };

    pub const fn end(&self) -> u64 {
        self.virtual_base.saturating_add(self.pages.saturating_mul(PAGE_SIZE))
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AddressSpace {
    pub id: AddressSpaceId,
    pub owner: ObjectId,
    pub root_table: u64,
    pub mappings: [Mapping; MAX_MAPPINGS_PER_SPACE],
    pub mapping_count: usize,
}

impl AddressSpace {
    pub const EMPTY: Self = Self {
        id: AddressSpaceId(0),
        owner: ObjectId::NULL,
        root_table: 0,
        mappings: [Mapping::EMPTY; MAX_MAPPINGS_PER_SPACE],
        mapping_count: 0,
    };

    pub fn attach_root(&mut self, id:AddressSpaceId, owner:ObjectId, root:u64) -> Result<(), &'static str> {
        if root==0 || root % PAGE_SIZE != 0 { return Err("invalid address-space root"); }
        self.id=id;
        self.owner=owner;
        self.root_table=root;
        Ok(())
    }

    pub fn map(&mut self, mapping: Mapping) -> Result<(), &'static str> {
        if mapping.pages == 0 || mapping.virtual_base % PAGE_SIZE != 0 || mapping.physical_base % PAGE_SIZE != 0 {
            return Err("unaligned mapping");
        }
        if self.mapping_count >= MAX_MAPPINGS_PER_SPACE {
            return Err("address space mapping table full");
        }
        let end = mapping.end();
        for existing in self.mappings.iter().take(self.mapping_count) {
            if mapping.virtual_base < existing.end() && existing.virtual_base < end {
                return Err("mapping overlaps existing range");
            }
        }
        self.mappings[self.mapping_count] = mapping;
        self.mapping_count += 1;
        Ok(())
    }

    pub fn find(&self, virtual_address: u64) -> Option<Mapping> {
        self.mappings.iter().take(self.mapping_count)
            .copied()
            .find(|m| virtual_address >= m.virtual_base && virtual_address < m.end())
    }
}
