//! Kernel-owned x86_64 paging and virtual-memory primitives.

use core::arch::asm;

use super::allocate_frame_below;

pub const PAGE_SIZE: u64 = 4096;
pub const HEAP_BASE: u64 = 64 * 1024 * 1024;
pub const HEAP_SIZE: u64 = 8 * 1024 * 1024;

const PAGE_TABLE_ENTRIES: usize = 512;
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const USER: u64 = 1 << 2;
const NO_EXECUTE: u64 = 1u64 << 63;
const HUGE_PAGE: u64 = 1 << 7;
const BOOTSTRAP_LIMIT: u64 = 64 * 1024 * 1024;
const IDENTITY_LIMIT: u64 = 64 * 1024 * 1024;

#[no_mangle]
pub static mut bennu_kernel_root: u64 = 0;

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

unsafe fn read_cr3() -> u64 {
    let value: u64;
    asm!("mov {value}, cr3", value = out(reg) value, options(nostack, preserves_flags));
    value
}

unsafe fn load_cr3(physical: u64) {
    asm!("mov cr3, {value}", value = in(reg) physical, options(nostack, preserves_flags));
}

unsafe fn invalidate_page(virtual_address: u64) {
    asm!("invlpg [{address}]", address = in(reg) virtual_address, options(nostack, preserves_flags));
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
        bennu_kernel_root = pml4_frame;
    }

    Ok(())
}

/// Map one 4 KiB virtual page to a physical frame.
///
/// The page-table pages themselves are kept below 64 MiB so the bootstrap
/// identity map can access them while the virtual-memory manager is still
/// being brought up.
pub fn current_root() -> u64 { unsafe { read_cr3() } }

pub fn kernel_root() -> u64 { unsafe { bennu_kernel_root } }

pub fn create_address_space_root() -> Result<u64, &'static str> {
    let root = allocate_frame_below(BOOTSTRAP_LIMIT).ok_or("cannot allocate address-space root")?;
    unsafe { zero_table(root); }
    Ok(root)
}

pub unsafe fn switch_address_space(root:u64) -> Result<(), &'static str> {
    if root==0 || root & (PAGE_SIZE-1)!=0 { return Err("invalid address-space root"); }
    load_cr3(root);
    Ok(())
}

pub fn map_page_in_root(root: u64, virtual_address: u64, physical_frame: u64) -> Result<(), &'static str> {
    map_page_in_root_with_flags(root, virtual_address, physical_frame, false, true, true)
}

pub fn map_supervisor_page_in_root(
    root: u64,
    virtual_address: u64,
    physical_frame: u64,
    writable: bool,
    executable: bool,
) -> Result<(), &'static str> {
    map_page_in_root_with_flags(root, virtual_address, physical_frame, false, writable, executable)
}

pub fn map_user_page_in_root(
    root: u64,
    virtual_address: u64,
    physical_frame: u64,
    writable: bool,
    executable: bool,
) -> Result<(), &'static str> {
    map_page_in_root_with_flags(root, virtual_address, physical_frame, true, writable, executable)
}

fn map_page_in_root_with_flags(
    root: u64,
    virtual_address: u64,
    physical_frame: u64,
    user: bool,
    writable: bool,
    executable: bool,
) -> Result<(), &'static str> {
    if root == 0 || root & (PAGE_SIZE - 1) != 0 {
        return Err("invalid address-space root");
    }
    if virtual_address & (PAGE_SIZE - 1) != 0 || physical_frame & (PAGE_SIZE - 1) != 0 {
        return Err("unaligned page mapping");
    }

    let pml4_index = ((virtual_address >> 39) & 0x1ff) as usize;
    let pdpt_index = ((virtual_address >> 30) & 0x1ff) as usize;
    let pd_index = ((virtual_address >> 21) & 0x1ff) as usize;
    let pt_index = ((virtual_address >> 12) & 0x1ff) as usize;
    let intermediate = PRESENT | WRITABLE | if user { USER } else { 0 };
    let leaf = PRESENT | if writable { WRITABLE } else { 0 } | if user { USER } else { 0 } | if executable { 0 } else { NO_EXECUTE };

    unsafe {
        let pml4 = table_at(root);
        let pdpt_frame = pml4.entries[pml4_index] & 0x000f_ffff_ffff_f000;
        let pdpt_frame = if pdpt_frame == 0 {
            let f = allocate_frame_below(BOOTSTRAP_LIMIT).ok_or("cannot allocate address-space PDPT")?;
            zero_table(f);
            pml4.entries[pml4_index] = f | intermediate;
            f
        } else {
            if user { pml4.entries[pml4_index] |= USER; }
            pdpt_frame
        };

        let pdpt = table_at(pdpt_frame);
        if pdpt.entries[pdpt_index] & HUGE_PAGE != 0 {
            return Err("address-space mapping hits huge page");
        }
        let pd_frame = pdpt.entries[pdpt_index] & 0x000f_ffff_ffff_f000;
        let pd_frame = if pd_frame == 0 {
            let f = allocate_frame_below(BOOTSTRAP_LIMIT).ok_or("cannot allocate address-space PD")?;
            zero_table(f);
            pdpt.entries[pdpt_index] = f | intermediate;
            f
        } else {
            if user { pdpt.entries[pdpt_index] |= USER; }
            pd_frame
        };

        let pd = table_at(pd_frame);
        if pd.entries[pd_index] & HUGE_PAGE != 0 {
            return Err("address-space mapping hits huge page");
        }
        let pt_frame = pd.entries[pd_index] & 0x000f_ffff_ffff_f000;
        let pt_frame = if pt_frame == 0 {
            let f = allocate_frame_below(BOOTSTRAP_LIMIT).ok_or("cannot allocate address-space PT")?;
            zero_table(f);
            pd.entries[pd_index] = f | intermediate;
            f
        } else {
            if user { pd.entries[pd_index] |= USER; }
            pt_frame
        };

        let pt = table_at(pt_frame);
        pt.entries[pt_index] = physical_frame | leaf;
    }
    Ok(())
}


pub fn translate_user_address(root: u64, virtual_address: u64, write: bool) -> Option<u64> {
    if root == 0 || root & (PAGE_SIZE - 1) != 0 {
        return None;
    }

    let pml4_index = ((virtual_address >> 39) & 0x1ff) as usize;
    let pdpt_index = ((virtual_address >> 30) & 0x1ff) as usize;
    let pd_index = ((virtual_address >> 21) & 0x1ff) as usize;
    let pt_index = ((virtual_address >> 12) & 0x1ff) as usize;

    unsafe {
        let pml4 = table_at(root);
        let pml4e = pml4.entries[pml4_index];
        if pml4e & PRESENT == 0 || pml4e & USER == 0 {
            return None;
        }
        let pdpt = table_at(pml4e & 0x000f_ffff_ffff_f000);
        let pdpte = pdpt.entries[pdpt_index];
        if pdpte & PRESENT == 0 || pdpte & USER == 0 {
            return None;
        }
        if pdpte & HUGE_PAGE != 0 {
            let physical = (pdpte & 0x000f_ffff_c000_0000)
                + (virtual_address & 0x3fff_ffff);
            return Some(physical);
        }

        let pd = table_at(pdpte & 0x000f_ffff_ffff_f000);
        let pde = pd.entries[pd_index];
        if pde & PRESENT == 0 || pde & USER == 0 {
            return None;
        }
        if pde & HUGE_PAGE != 0 {
            let physical = (pde & 0x000f_ffff_ffe0_0000)
                + (virtual_address & 0x1f_ffff);
            return Some(physical);
        }

        let pt = table_at(pde & 0x000f_ffff_ffff_f000);
        let pte = pt.entries[pt_index];
        if pte & PRESENT == 0 || pte & USER == 0 || (write && pte & WRITABLE == 0) {
            return None;
        }

        Some((pte & 0x000f_ffff_ffff_f000) + (virtual_address & 0xfff))
    }
}

pub fn map_page(virtual_address: u64, physical_frame: u64) -> Result<(), &'static str> {
    if virtual_address & (PAGE_SIZE - 1) != 0
        || physical_frame & (PAGE_SIZE - 1) != 0
    {
        return Err("unaligned page mapping");
    }

    let pml4_index = ((virtual_address >> 39) & 0x1ff) as usize;
    let pdpt_index = ((virtual_address >> 30) & 0x1ff) as usize;
    let pd_index = ((virtual_address >> 21) & 0x1ff) as usize;
    let pt_index = ((virtual_address >> 12) & 0x1ff) as usize;

    unsafe {
        let pml4 = table_at(read_cr3());
        let pdpt_frame = pml4.entries[pml4_index] & 0x000f_ffff_ffff_f000;

        let pdpt_frame = if pdpt_frame == 0 {
            let frame = allocate_frame_below(BOOTSTRAP_LIMIT)
                .ok_or("cannot allocate PDPT")?;
            zero_table(frame);
            pml4.entries[pml4_index] = frame | PRESENT | WRITABLE;
            frame
        } else {
            pdpt_frame
        };

        let pdpt = table_at(pdpt_frame);
        let pd_frame = pdpt.entries[pdpt_index] & 0x000f_ffff_ffff_f000;

        let pd_frame = if pd_frame == 0 {
            let frame = allocate_frame_below(BOOTSTRAP_LIMIT)
                .ok_or("cannot allocate page directory")?;
            zero_table(frame);
            pdpt.entries[pdpt_index] = frame | PRESENT | WRITABLE;
            frame
        } else {
            if pdpt.entries[pdpt_index] & HUGE_PAGE != 0 {
                return Err("cannot replace huge-page mapping");
            }
            pd_frame
        };

        let pd = table_at(pd_frame);
        let pt_frame = pd.entries[pd_index] & 0x000f_ffff_ffff_f000;

        let pt_frame = if pt_frame == 0 {
            let frame = allocate_frame_below(BOOTSTRAP_LIMIT)
                .ok_or("cannot allocate page table")?;
            zero_table(frame);
            pd.entries[pd_index] = frame | PRESENT | WRITABLE;
            frame
        } else {
            if pd.entries[pd_index] & HUGE_PAGE != 0 {
                return Err("cannot replace huge-page mapping");
            }
            pt_frame
        };

        let pt = table_at(pt_frame);
        pt.entries[pt_index] = physical_frame | PRESENT | WRITABLE;
        invalidate_page(virtual_address);
    }

    Ok(())
}


pub const MMIO_BASE: u64 = 0xffff_8000_0000_0000;

pub fn map_mmio(physical_base:u64, length:u64) -> Result<u64,&'static str> {
    if length == 0 { return Err("empty MMIO range"); }
    let physical = physical_base & !(PAGE_SIZE - 1);
    let end = physical_base.checked_add(length).ok_or("MMIO range overflow")?;
    let pages = (end - physical + PAGE_SIZE - 1) / PAGE_SIZE;
    for page in 0..pages {
        let va = MMIO_BASE.checked_add(page * PAGE_SIZE).ok_or("MMIO virtual range overflow")?;
        let pa = physical.checked_add(page * PAGE_SIZE).ok_or("MMIO physical range overflow")?;
        map_page(va, pa)?;
    }
    Ok(MMIO_BASE + (physical_base - physical))
}
