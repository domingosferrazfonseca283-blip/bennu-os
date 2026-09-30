//! Global Descriptor Table (GDT) for the Bennu x86_64 kernel.
//!
//! The bootloader establishes a temporary GDT. The kernel immediately replaces it
//! with its own table so that all long-lived CPU state belongs to Bennu.

use core::arch::asm;

const NULL: u64 = 0;
const KERNEL_DATA: u64 = 0x00CF92000000FFFF;
const KERNEL_CODE: u64 = 0x00AF9A000000FFFF;

/// Selector used by the current boot-time code segment.
///
/// Keeping the first kernel GDT compatible with the bootloader lets us reload
/// the descriptor table without changing CS in the first bring-up stage.
pub const KERNEL_CODE_SELECTOR: u16 = 0x18;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

/// Bennu's first permanent GDT.
///
/// Layout:
///   0x00 null
///   0x08 reserved for a future kernel code layout
///   0x10 ring-0 data
///   0x18 ring-0 64-bit code
///
/// The unused 0x08 slot is intentional: it leaves room to migrate the ABI
/// later without changing the current boot contract.
static GDT: [u64; 4] = [
    NULL,
    0,
    KERNEL_DATA,
    KERNEL_CODE,
];

pub fn init() {
    let pointer = DescriptorTablePointer {
        limit: (core::mem::size_of_val(&GDT) - 1) as u16,
        base: GDT.as_ptr() as u64,
    };

    unsafe {
        asm!(
            "lgdt [{gdt}]",
            gdt = in(reg) &pointer,
            options(readonly, nostack, preserves_flags),
        );

        // CS remains 0x18, which is the same selector used by Bennu Stage 2.
        // Reload the data segments against the new GDT.
        asm!(
            "mov ds, {sel:x}",
            "mov es, {sel:x}",
            "mov ss, {sel:x}",
            sel = in(reg) KERNEL_DATA_SELECTOR as u64,
            options(nostack, preserves_flags),
        );
    }
}
