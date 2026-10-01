#![allow(dead_code)]

/// Magic identifying the Bennu BIOS boot protocol.
pub const BENNU_BOOT_MAGIC: u64 = 0x4245_4e4e_554f_5301;
pub const BENNU_BOOT_VERSION: u32 = 1;
pub const BENNU_E820_USABLE: u32 = 1;
pub const BENNU_E820_MAX_ENTRIES: usize = 128;
pub const BENNU_FRAMEBUFFER_BYTES_PER_PIXEL: u32 = 4;
pub const BENNU_BOOT_PROTOCOL_BIOS_VBE: u8 = 1;
pub const BENNU_BOOT_PROTOCOL_UEFI_GOP: u8 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct E820Entry {
    pub base: u64,
    pub length: u64,
    pub kind: u32,
    pub attributes: u32,
}

#[repr(C)]
pub struct BootInfo {
    pub magic: u64,
    pub version: u32,
    pub size: u32,
    pub boot_drive: u8,
    pub boot_protocol: u8,
    pub reserved: [u8; 6],
    pub kernel_base: u64,
    pub kernel_size: u64,
    pub memory_map_addr: u64,
    pub memory_map_entries: u32,
    pub memory_map_entry_size: u32,
    pub framebuffer_addr: u64,
    pub framebuffer_pitch: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp: u32,
    pub framebuffer_red_mask: u8,
    pub framebuffer_red_position: u8,
    pub framebuffer_green_mask: u8,
    pub framebuffer_green_position: u8,
    pub framebuffer_blue_mask: u8,
    pub framebuffer_blue_position: u8,
    pub framebuffer_reserved: [u8; 2],
}

impl BootInfo {
    pub fn is_valid(&self) -> bool {
        (self.boot_protocol == BENNU_BOOT_PROTOCOL_BIOS_VBE || self.boot_protocol == BENNU_BOOT_PROTOCOL_UEFI_GOP)
            && self.magic == BENNU_BOOT_MAGIC
            && self.version == BENNU_BOOT_VERSION
            && self.size as usize >= core::mem::size_of::<Self>()
            && (self.memory_map_entries == 0 || self.memory_map_addr != 0)
            && self.memory_map_entry_size as usize == core::mem::size_of::<E820Entry>()
            && self.memory_map_entries as usize <= BENNU_E820_MAX_ENTRIES
    }

    pub fn memory_map(&self) -> &[E820Entry] {
        if !self.is_valid() || self.memory_map_addr == 0 {
            return &[];
        }

        unsafe {
            core::slice::from_raw_parts(
                self.memory_map_addr as *const E820Entry,
                self.memory_map_entries as usize,
            )
        }
    }
}
