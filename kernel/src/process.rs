#![allow(dead_code)]

use crate::model::{CellId, ObjectId};

pub const BEXE_HEADER_SIZE: usize = core::mem::size_of::<ExecutableHeader>();
pub const USER_IMAGE_BASE: u64 = 0x0000_0080_0000_0000;
pub const USER_STACK_BASE: u64 = USER_IMAGE_BASE + 0x0000_0000_0100_0000;
pub const USER_STACK_TOP: u64 = USER_STACK_BASE + crate::memory::PAGE_SIZE;
pub const MAX_BEXE_IMAGE: usize = 4 * 1024 * 1024;

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
    pub const MAGIC: [u8; 4] = *b"BEXE";

    pub fn valid(&self, bytes: usize) -> bool {
        if self.magic != Self::MAGIC
            || self.version != 1
            || self.machine != 0x3e
            || self.image_size == 0
            || self.image_size as usize > MAX_BEXE_IMAGE
        {
            return false;
        }

        let image_size = self.image_size as usize;
        if bytes < BEXE_HEADER_SIZE || image_size > bytes - BEXE_HEADER_SIZE {
            return false;
        }

        let end = match USER_IMAGE_BASE.checked_add(self.image_size) {
            Some(value) => value,
            None => return false,
        };

        self.entry >= USER_IMAGE_BASE
            && self.entry < end
            && end <= USER_STACK_BASE
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
    pub const EMPTY: Self = Self {
        id: CellId(0),
        address_space: 0,
        root_object: ObjectId::NULL,
        entry: 0,
    };
}

/// Parse the fixed BEXE header without dereferencing an unaligned input pointer.
pub fn parse_header(bytes: &[u8]) -> Result<ExecutableHeader, &'static str> {
    if bytes.len() < BEXE_HEADER_SIZE {
        return Err("BEXE header is truncated");
    }

    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);

    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    let machine = u16::from_le_bytes([bytes[6], bytes[7]]);

    let mut entry_bytes = [0u8; 8];
    entry_bytes.copy_from_slice(&bytes[8..16]);

    let mut size_bytes = [0u8; 8];
    size_bytes.copy_from_slice(&bytes[16..24]);

    Ok(ExecutableHeader {
        magic,
        version,
        machine,
        entry: u64::from_le_bytes(entry_bytes),
        image_size: u64::from_le_bytes(size_bytes),
    })
}

/// Validate the image and return the payload slice after the fixed header.
pub fn image_payload(bytes: &[u8]) -> Result<(ExecutableHeader, &[u8]), &'static str> {
    let header = parse_header(bytes)?;
    if !header.valid(bytes.len()) {
        return Err("invalid BEXE image");
    }

    let size = header.image_size as usize;
    Ok((header, &bytes[BEXE_HEADER_SIZE..BEXE_HEADER_SIZE + size]))
}

/// Load a BEXE payload into a Cell's private user address space.
///
/// The image is mapped read-only/executable and the initial stack is mapped
/// writable/non-executable. Physical backing is owned by the Cell's memory
/// objects, so the user mapping cannot alias an unrelated physical page.
pub fn load_bexe(cell: CellId, root: u64, bytes: &[u8]) -> Result<Process, &'static str> {
    if root == 0 || root & (crate::memory::PAGE_SIZE - 1) != 0 {
        return Err("invalid BEXE address-space root");
    }

    let (header, payload) = image_payload(bytes)?;
    let image_pages = (payload.len() as u64 + crate::memory::PAGE_SIZE - 1)
        / crate::memory::PAGE_SIZE;
    let image_pages = image_pages as usize;
    if image_pages == 0 {
        return Err("empty BEXE image");
    }

    let image_object = crate::model::runtime::create_memory_object(cell.0, image_pages)?;
    let (frames, frame_count) = crate::model::runtime::object_frames(image_object)
        .ok_or("BEXE image backing object unavailable")?;
    if frame_count != image_pages {
        return Err("BEXE image backing page count mismatch");
    }

    for page in 0..image_pages {
        let address = USER_IMAGE_BASE
            .checked_add((page as u64) * crate::memory::PAGE_SIZE)
            .ok_or("BEXE image address overflow")?;
        crate::memory::paging::map_user_page_in_root(
            root,
            address,
            frames[page],
            false,
            true,
        )?;

        let source_start = page * crate::memory::PAGE_SIZE as usize;
        if source_start >= payload.len() {
            break;
        }
        let copy_len = core::cmp::min(
            crate::memory::PAGE_SIZE as usize,
            payload.len() - source_start,
        );

        unsafe {
            core::ptr::copy_nonoverlapping(
                payload.as_ptr().add(source_start),
                frames[page] as *mut u8,
                copy_len,
            );
            if copy_len < crate::memory::PAGE_SIZE as usize {
                core::ptr::write_bytes(
                    (frames[page] + copy_len as u64) as *mut u8,
                    0,
                    crate::memory::PAGE_SIZE as usize - copy_len,
                );
            }
        }
    }

    let stack_object = crate::model::runtime::create_memory_object(cell.0, 1)?;
    let (stack_frames, stack_count) = crate::model::runtime::object_frames(stack_object)
        .ok_or("BEXE stack backing object unavailable")?;
    if stack_count != 1 {
        return Err("BEXE stack backing page count mismatch");
    }

    crate::memory::paging::map_user_page_in_root(
        root,
        USER_STACK_BASE,
        stack_frames[0],
        true,
        false,
    )?;

    unsafe {
        core::ptr::write_bytes(
            stack_frames[0] as *mut u8,
            0,
            crate::memory::PAGE_SIZE as usize,
        );
    }

    let rsp = USER_STACK_TOP - 16;
    crate::model::runtime::configure_user_entry(cell, header.entry, rsp)?;

    Ok(Process {
        id: cell,
        address_space: root,
        root_object: image_object,
        entry: header.entry,
    })
}
