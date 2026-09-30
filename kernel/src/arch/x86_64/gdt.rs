//! Permanent x86_64 descriptor state, including the first Ring-3 boundary.

use core::arch::asm;

const NULL: u64 = 0;
const KERNEL_DATA: u64 = 0x00CF92000000FFFF;
const KERNEL_CODE: u64 = 0x00AF9A000000FFFF;
const USER_DATA: u64 = 0x00CFF2000000FFFF;
const USER_CODE: u64 = 0x00AFFA000000FFFF;

pub const KERNEL_CODE_SELECTOR: u16 = 0x18;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x23;
pub const USER_CODE_SELECTOR: u16 = 0x2B;
pub const TSS_SELECTOR: u16 = 0x30;

#[repr(C, packed)]
struct DescriptorTablePointer { limit: u16, base: u64 }

#[repr(C)]
pub struct TaskStateSegment {
    _reserved0: u32,
    pub rsp0: u64,
    _rsp1: u64,
    _rsp2: u64,
    _reserved1: u64,
    _ist: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn empty() -> Self {
        Self {
            _reserved0: 0, rsp0: 0, _rsp1: 0, _rsp2: 0, _reserved1: 0,
            _ist: [0; 7], _reserved2: 0, _reserved3: 0,
            iomap_base: core::mem::size_of::<Self>() as u16,
        }
    }
}

static mut TSS: TaskStateSegment = TaskStateSegment::empty();

fn tss_descriptor(base: u64) -> (u64, u64) {
    let limit = (core::mem::size_of::<TaskStateSegment>() - 1) as u64;
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89u64 << 40)
        | (((limit >> 16) & 0xf) << 48)
        | (((base >> 24) & 0xff) << 56);
    (low, base >> 32)
}

static mut GDT: [u64; 8] = [0; 8];

pub fn init() {
    unsafe {
        GDT[0] = NULL;
        GDT[1] = 0;
        GDT[2] = KERNEL_DATA;
        GDT[3] = KERNEL_CODE;
        GDT[4] = USER_DATA;
        GDT[5] = USER_CODE;
        let (tss_low, tss_high) = tss_descriptor((&raw const TSS) as u64);
        GDT[6] = tss_low;
        GDT[7] = tss_high;

        let pointer = DescriptorTablePointer {
            limit: (core::mem::size_of_val(&GDT) - 1) as u16,
            base: (&raw const GDT) as u64,
        };

        asm!("lgdt [{gdt}]", gdt = in(reg) &pointer, options(readonly, nostack, preserves_flags));
        asm!(
            "mov ds, {sel:x}", "mov es, {sel:x}", "mov ss, {sel:x}",
            sel = in(reg) KERNEL_DATA_SELECTOR as u64,
            options(nostack, preserves_flags),
        );
        asm!("ltr {sel:x}", sel = in(reg) TSS_SELECTOR, options(nostack, preserves_flags));
    }
}

pub fn set_kernel_stack(stack_top: u64) -> Result<(), &'static str> {
    if stack_top == 0 || stack_top & 0xf != 0 {
        return Err("invalid TSS kernel stack");
    }
    unsafe { TSS.rsp0 = stack_top; }
    Ok(())
}
