#![no_std]
#![no_main]

use core::ffi::c_void;

#[repr(C)]
pub struct EfiGuid { pub data1:u32, pub data2:u16, pub data3:u16, pub data4:[u8;8] }

#[repr(C)]
pub struct EfiTableHeader { pub signature:u64, pub revision:u32, pub header_size:u32, pub crc32:u32, pub reserved:u32 }

#[repr(C)]
pub struct EfiSystemTable {
    pub hdr:EfiTableHeader,
    pub firmware_vendor:*mut u16,
    pub firmware_revision:u32,
    pub console_in:*mut c_void,
    pub console_out:*mut c_void,
    pub stderr:*mut c_void,
    pub runtime:*mut c_void,
    pub boot_services:*mut EfiBootServices,
}

#[repr(C)]
pub struct EfiBootServices { pub hdr:EfiTableHeader, pub reserved:[usize;8] }

/// UEFI/GOP entry contract. The concrete firmware boot service implementation
/// is deliberately kept behind this ABI boundary so the kernel consumes only
/// the stable Bennu BootInfo framebuffer contract.
#[repr(C)]
pub struct GopModeInfo {
    pub version:u32,
    pub horizontal_resolution:u32,
    pub vertical_resolution:u32,
    pub pixel_format:u32,
    pub pixel_information:[u32;4],
    pub pixels_per_scan_line:u32,
}

pub const GOP_PIXEL_RGB_RESERVED_8BIT_PER_COLOR:u32 = 1;
pub const GOP_PIXEL_BGR_RESERVED_8BIT_PER_COLOR:u32 = 2;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop { core::hint::spin_loop(); } }

/// Placeholder entry used while the PE/COFF image and firmware protocol glue
/// are assembled. It intentionally does not claim successful UEFI boot.
#[no_mangle]
pub extern "efiapi" fn efi_main(_image:usize, _system_table:*mut EfiSystemTable) -> usize { 1 }
