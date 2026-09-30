//! Checked copies across the Ring-3 memory boundary.

use super::paging;

const MAX_COPY: u64 = 1024 * 1024;

fn checked_range(base: u64, length: u64) -> Result<u64, &'static str> {
    if length == 0 || length > MAX_COPY {
        return Err("invalid user copy length");
    }
    let end = base.checked_add(length - 1).ok_or("user range overflow")?;
    if end >= 0x0000_8000_0000_0000 {
        return Err("user range is not canonical");
    }
    Ok(end)
}

pub fn copy_from_user(root: u64, dst: *mut u8, src: u64, length: u64) -> Result<(), &'static str> {
    checked_range(src, length)?;
    if dst.is_null() {
        return Err("null kernel destination");
    }

    let mut offset = 0u64;
    while offset < length {
        let va = src + offset;
        let physical = paging::translate_user_address(root, va, false)
            .ok_or("unmapped user source")?;
        let page_remaining = paging::PAGE_SIZE - (physical & (paging::PAGE_SIZE - 1));
        let chunk = core::cmp::min(page_remaining, length - offset);

        unsafe {
            core::ptr::copy_nonoverlapping(
                physical as *const u8,
                dst.add(offset as usize),
                chunk as usize,
            );
        }
        offset += chunk;
    }

    Ok(())
}

pub fn copy_to_user(root: u64, dst: u64, src: *const u8, length: u64) -> Result<(), &'static str> {
    checked_range(dst, length)?;
    if src.is_null() {
        return Err("null kernel source");
    }

    let mut offset = 0u64;
    while offset < length {
        let va = dst + offset;
        let physical = paging::translate_user_address(root, va, true)
            .ok_or("unmapped or read-only user destination")?;
        let page_remaining = paging::PAGE_SIZE - (physical & (paging::PAGE_SIZE - 1));
        let chunk = core::cmp::min(page_remaining, length - offset);

        unsafe {
            core::ptr::copy_nonoverlapping(
                src.add(offset as usize),
                physical as *mut u8,
                chunk as usize,
            );
        }
        offset += chunk;
    }

    Ok(())
}
