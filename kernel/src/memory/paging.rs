//! Kernel-owned x86_64 paging bootstrap.
//!
//! Stage 2 supplies a temporary identity map so Rust can start. This module
//! replaces that map with page tables owned by Bennu itself. The first policy
//! maps the low 64 MiB with 2 MiB pages; later virtual-memory code will replace
//! this bootstrap mapping with per-address-space page tables.

use core::arch::asm;

use super::allocate_frame_below;

const PAGE_TABLE_ENTRIES: usize = 512;
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const HUGE_PAGE: u64 = 1 << 7;
const BOOTSTRAP_LIMIT: u64 = 0x0020_0000;
const IDENTITY_LIMIT: u64 = 64 * 1024 * 1024;

#[repr(C, align(4096))]
struct PageTable {
    entries: [u64; PAGE_TABLE_ENTRIES],
}

unsafe fn table_at(physical: u64) -> &'static mut PageTable {
    &mut *(physical as *mut PageTable)
}

unsafe fn zero_table(physical: u64) -> &'static mut PageTable {
    core::ptr::write_bytes(physical as *mut u8, 0, 4096);
    table_at(physical)
}

unsafe fn load_cr3(physical: u64) {
    asm!(
        "mov cr3, {value}",
        value = in(reg) physical,
        options(nostack, preserves_flags),
    );
}

/// Build and activate the first page tables owned by Bennu.
pub fn init() -> Result<(), &'static str> {
    let pml4_frame = allocate_frame_below(BOOTSTRAP_LIMIT)
        .ok_or("cannot allocate bootstrap PML4")?;
    let pdpt_frame = allocate_frame_below(BOOTSTRAP_LIMIT)
        .ok_or("cannot allocate bootstrap PDPT")?;
    let pd_frame = allocate_frame_below(BOOTSTRAP_LIMIT)
        .ok_or("cannot allocate bootstrap PD")?;

    unsafe {
        let pml4 = zero_table(pml4_frame);
        let pdpt = zero_table(pdpt_frame);
        let pd = zero_table(pd_frame);

        pml4.entries[0] = pdpt_frame | PRESENT | WRITABLE;
        pdpt.entries[0] = pd_frame | PRESENT | WRITABLE;

        let mut address = 0u64;
        let mut index = 0usize;

        while address < IDENTITY_LIMIT {
            pd.entries[index] = address | PRESENT | WRITABLE | HUGE_PAGE;
            address += 2 * 1024 * 1024;
            index += 1;
        }

        load_cr3(pml4_frame);
    }

    Ok(())
}
