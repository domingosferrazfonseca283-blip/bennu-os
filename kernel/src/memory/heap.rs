//! Small page-backed kernel heap.
//!
//! This is intentionally a bootstrap allocator, not yet a general-purpose
//! slab allocator. It provides aligned allocations from virtual memory owned
//! by Bennu and obtains each backing page from the physical frame allocator.
//! Deallocation will be added when the process/address-space layer exists.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use super::paging::{map_page, HEAP_BASE, HEAP_SIZE, PAGE_SIZE};
use super::allocate_frame;

const HEAP_PAGES: usize = (HEAP_SIZE / PAGE_SIZE) as usize;

pub struct BennuHeap {
    next: usize,
    limit: usize,
}

impl BennuHeap {
    pub const fn new() -> Self {
        Self { next: 0, limit: 0 }
    }

    pub unsafe fn init(&mut self) {
        self.next = HEAP_BASE as usize;
        self.limit = self.next + HEAP_SIZE as usize;
    }

    unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        if self.next == 0 {
            return null_mut();
        }

        let align = layout.align().max(1);
        let size = layout.size().max(1);
        let aligned = (self.next + align - 1) & !(align - 1);
        let end = match aligned.checked_add(size) {
            Some(value) => value,
            None => return null_mut(),
        };

        if end > self.limit {
            return null_mut();
        }

        let first_page = aligned & !(PAGE_SIZE as usize - 1);
        let last_page = (end - 1) & !(PAGE_SIZE as usize - 1);

        let mut page = first_page;
        while page <= last_page {
            if map_page(page as u64, allocate_frame().unwrap_or(0)).is_err() {
                return null_mut();
            }
            page += PAGE_SIZE as usize;
        }

        self.next = end;
        aligned as *mut u8
    }
}

unsafe impl GlobalAlloc for BennuHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // The kernel is single-threaded during bootstrap. A lock-free heap
        // comes later with the scheduler/CPU-local infrastructure.
        let heap = self as *const Self as *mut Self;
        (*heap).allocate(layout)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static mut KERNEL_HEAP: BennuHeap = BennuHeap::new();

pub unsafe fn init() {
    KERNEL_HEAP.init();
}

pub fn capacity_pages() -> usize {
    HEAP_PAGES
}
