//! Physical memory foundation.
//!
//! The first allocator is deliberately conservative: it consumes the firmware
//! E820 map supplied by Bennu Boot, ignores all non-RAM regions, and never
//! allocates from the first MiB. Paging and the kernel heap will build on this
//! stable physical-frame interface.

use crate::boot_info::{BootInfo, E820Entry, BENNU_E820_USABLE};

pub mod paging;

pub const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy)]
struct Region {
    start: u64,
    end: u64,
    next: u64,
}

impl Region {
    const EMPTY: Self = Self {
        start: 0,
        end: 0,
        next: 0,
    };
}

const MAX_REGIONS: usize = 128;
static mut REGIONS: [Region; MAX_REGIONS] = [Region::EMPTY; MAX_REGIONS];
static mut REGION_COUNT: usize = 0;

fn align_up(value: u64, alignment: u64) -> u64 {
    (value + alignment - 1) & !(alignment - 1)
}

fn add_region(entry: E820Entry, kernel_start: u64, kernel_end: u64) {
    if entry.kind != BENNU_E820_USABLE || entry.length == 0 {
        return;
    }

    let mut start = entry.base.max(0x0010_0000);
    let end = entry.base.saturating_add(entry.length);

    if end <= start {
        return;
    }

    start = align_up(start, PAGE_SIZE);

    // Never hand the kernel its own image back as free memory.
    if start < kernel_end && end > kernel_start {
        if start < kernel_start {
            unsafe {
                if REGION_COUNT < MAX_REGIONS {
                    REGIONS[REGION_COUNT] = Region {
                        start,
                        end: kernel_start,
                        next: start,
                    };
                    REGION_COUNT += 1;
                }
            }
        }

        start = align_up(kernel_end, PAGE_SIZE);
    }

    if start < end {
        unsafe {
            if REGION_COUNT < MAX_REGIONS {
                REGIONS[REGION_COUNT] = Region {
                    start,
                    end,
                    next: start,
                };
                REGION_COUNT += 1;
            }
        }
    }
}

/// Initialize the physical frame allocator from the Bennu boot contract.
pub fn init(boot_info: &BootInfo) -> Result<(), &'static str> {
    if !boot_info.is_valid() {
        return Err("invalid boot information");
    }

    let kernel_start = boot_info.kernel_base;
    let kernel_end = kernel_start.saturating_add(boot_info.kernel_size);

    unsafe {
        REGION_COUNT = 0;
        REGIONS = [Region::EMPTY; MAX_REGIONS];
    }

    for entry in boot_info.memory_map().iter().copied() {
        add_region(entry, kernel_start, kernel_end);
    }

    if region_count() == 0 {
        return Err("no usable physical memory");
    }

    Ok(())
}

pub fn region_count() -> usize {
    unsafe { REGION_COUNT }
}

/// Allocate one 4 KiB physical frame.
///
/// Frames are returned in ascending physical order. The first implementation
/// intentionally has no freeing operation; reclamation arrives with the
/// virtual-memory and process subsystems.
pub fn allocate_frame() -> Option<u64> {
    allocate_frame_below(u64::MAX)
}

/// Allocate one frame below a physical-address limit.
///
/// This is used during paging bootstrap because the bootloader initially
/// identity-maps only the first 2 MiB.
pub fn allocate_frame_below(limit: u64) -> Option<u64> {
    unsafe {
        for index in 0..REGION_COUNT {
            let region = &mut REGIONS[index];
            if region.next < region.end {
                let frame = region.next;
                let next = region.next.saturating_add(PAGE_SIZE);
                if next <= limit {
                    region.next = next;
                    return Some(frame);
                }
            }
        }
    }

    None
}

/// Total number of currently available frames.
pub fn available_frames() -> u64 {
    unsafe {
        let mut total = 0;
        for index in 0..REGION_COUNT {
            total += (REGIONS[index].end - REGIONS[index].next) / PAGE_SIZE;
        }
        total
    }
}
