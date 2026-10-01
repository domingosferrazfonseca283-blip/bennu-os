BITS 16
ORG 0x8000

%ifndef KERNEL_SECTORS
%define KERNEL_SECTORS 64
%endif

%if KERNEL_SECTORS < 1
%error "KERNEL_SECTORS must be positive"
%endif

%if KERNEL_SECTORS > 65535
%error "kernel image exceeds BIOS sector-count field"
%endif

%define BIOS_CHUNK_SECTORS 64

%define BOOT_INFO        0x5000
%define VBE_MODE_INFO   0x5400
%define VBE_MODE        0x118
%define MEMORY_MAP       0x5100
%define STACK_TOP        0x80000
%define PML4             0x90000
%define PDPT             0x91000
%define PD              0x92000
%define KERNEL_LOAD      0x100000
%define E820_MAX         128
%define E820_ENTRY_SIZE  24

start2:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax

    mov [boot_drive], dl

    ; Clear and initialize the Bennu boot contract.
    mov di, BOOT_INFO
    xor eax, eax
    mov cx, 32
    rep stosd

    mov dword [BOOT_INFO + 0], 0x554F5301
    mov dword [BOOT_INFO + 4], 0x42454E4E
    mov dword [BOOT_INFO + 8], 1
    mov dword [BOOT_INFO + 12], 88
    mov byte  [BOOT_INFO + 16], dl

    mov dword [BOOT_INFO + 24], KERNEL_LOAD
    mov dword [BOOT_INFO + 28], 0
    mov dword [BOOT_INFO + 32], KERNEL_SECTORS * 512
    mov dword [BOOT_INFO + 36], 0

    mov dword [BOOT_INFO + 40], MEMORY_MAP
    mov dword [BOOT_INFO + 48], 0
    mov dword [BOOT_INFO + 52], E820_ENTRY_SIZE
    mov dword [BOOT_INFO + 56], 0
    mov dword [BOOT_INFO + 60], 0
    mov dword [BOOT_INFO + 64], 0
    mov dword [BOOT_INFO + 68], 0
    mov dword [BOOT_INFO + 72], 0
    mov dword [BOOT_INFO + 76], 0

    ; Clear the E820 destination area before BIOS writes variable-sized entries.
    mov di, MEMORY_MAP
    xor eax, eax
    mov cx, (E820_MAX * E820_ENTRY_SIZE) / 4
    rep stosd

    call detect_memory_map
    jc e820_error

    ; Enable A20 so protected-mode copies can reach the full kernel image.
    in al,0x92
    or al,0x02
    out 0x92,al

    mov ax, [e820_count]
    mov [BOOT_INFO + 48], ax

    ; Ask the BIOS for a linear 32-bit framebuffer before leaving real mode.
    ; Mode 118h is the standard 1024x768 32-bpp VBE mode when exposed.
    call setup_vbe

    ; Stream the kernel through a bounded BIOS transfer buffer and copy
    ; each chunk to its final address in extended memory.
    call load_kernel
    jc disk_error

    cli
    lgdt [gdt_descriptor]

    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:protected_mode

; ---------------------------------------------------------------------------
; BIOS E820 memory discovery.
; Returns CF=0 on success, CF=1 when the firmware does not provide E820.
; ---------------------------------------------------------------------------
detect_memory_map:
    xor ebx, ebx
    mov word [e820_count], 0
    mov word [e820_cursor], MEMORY_MAP

.next:
    mov ax, [e820_count]
    cmp ax, E820_MAX
    jae .done

    mov di, [e820_cursor]
    mov eax, 0xE820
    mov edx, 0x534D4150
    mov ecx, E820_ENTRY_SIZE
    int 0x15
    jc .failure

    cmp eax, 0x534D4150
    jne .failure

    ; A 20-byte legacy entry has no attributes field. Treat it as valid.
    cmp ecx, 24
    jb .accept

    test dword [di + 20], 1
    jz .skip

.accept:
    inc word [e820_count]
    add word [e820_cursor], E820_ENTRY_SIZE

.skip:
    test ebx, ebx
    jz .done
    jmp .next

.done:
    cmp word [e820_count], 0
    je .failure
    clc
    ret

.failure:
    stc
    ret

load_kernel:
    mov word [kernel_remaining],KERNEL_SECTORS
    mov dword [kernel_lba],9
    mov dword [kernel_dest],KERNEL_LOAD
.next:
    cmp word [kernel_remaining],0
    je .done
    mov ax,[kernel_remaining]
    cmp ax,BIOS_CHUNK_SECTORS
    jbe .count_ready
    mov ax,BIOS_CHUNK_SECTORS
.count_ready:
    mov [chunk_sectors],ax
    mov [dap_count],ax
    mov eax,[kernel_lba]
    mov dword [dap_lba],eax
    mov dword [dap_lba+4],0

    mov si,dap
    mov dl,[boot_drive]
    mov ah,0x42
    int 0x13
    jc .failure

    cli
    lgdt [gdt_descriptor]
    mov eax,cr0
    or eax,1
    mov cr0,eax
    jmp 0x08:copy_chunk

.done:
    clc
    ret
.failure:
    stc
    ret

BITS 32
copy_chunk:
    mov ax,0x10
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov esi,0x10000
    mov edi,[kernel_dest]
    movzx ecx,word [chunk_sectors]
    shl ecx,7
    rep movsd

    movzx eax,word [chunk_sectors]
    add [kernel_lba],eax
    sub [kernel_remaining],ax
    shl eax,9
    add [kernel_dest],eax

    mov eax,cr0
    and eax,0xFFFFFFFE
    mov cr0,eax
    jmp 0x0000:real_mode_after_copy

BITS 16
real_mode_after_copy:
    xor ax,ax
    mov ds,ax
    mov es,ax
    mov ss,ax
    mov sp,STACK_TOP & 0xFFFF
    sti
    jmp load_kernel

e820_error:
    cli
    hlt
    jmp e820_error

disk_error:
    cli
    hlt
    jmp disk_error

boot_drive db 0
e820_count dw 0
e820_cursor dw MEMORY_MAP
kernel_remaining dw KERNEL_SECTORS
chunk_sectors dw 0
kernel_lba dd 5
kernel_dest dd KERNEL_LOAD

dap:
    db 0x10
    db 0
dap_count:
    dw 0
    dw 0x0000
    dw 0x1000
dap_lba:
    dq 9

setup_vbe:
    push ds
    push es
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov di, VBE_MODE_INFO
    xor ax, ax
    mov cx, 128
    rep stosd

    mov ax, 0x4F01
    mov cx, VBE_MODE
    mov di, VBE_MODE_INFO
    int 0x10
    cmp ax, 0x004F
    jne .done

    mov ax, [VBE_MODE_INFO + 0]
    test ax, 0x0081
    jz .done
    cmp byte [VBE_MODE_INFO + 25], 32
    jne .done

    mov ax, 0x4F02
    mov bx, VBE_MODE | 0x4000
    mov di, VBE_MODE_INFO
    int 0x10
    cmp ax, 0x004F
    jne .done

    mov eax, [VBE_MODE_INFO + 40]
    mov [BOOT_INFO + 56], eax
    mov dword [BOOT_INFO + 60], 0
    movzx eax, word [VBE_MODE_INFO + 16]
    mov [BOOT_INFO + 64], eax
    movzx eax, word [VBE_MODE_INFO + 18]
    mov [BOOT_INFO + 68], eax
    movzx eax, word [VBE_MODE_INFO + 20]
    mov [BOOT_INFO + 72], eax
    movzx eax, byte [VBE_MODE_INFO + 25]
    mov [BOOT_INFO + 76], eax
    mov al, [VBE_MODE_INFO + 31]
    mov [BOOT_INFO + 80], al
    mov al, [VBE_MODE_INFO + 32]
    mov [BOOT_INFO + 81], al
    mov al, [VBE_MODE_INFO + 29]
    mov [BOOT_INFO + 82], al
    mov [BOOT_INFO + 83], byte 8
    mov al, [VBE_MODE_INFO + 28]
    mov [BOOT_INFO + 84], al
    mov [BOOT_INFO + 85], byte 8
    mov al, [VBE_MODE_INFO + 27]
    mov [BOOT_INFO + 86], al
    mov [BOOT_INFO + 87], byte 8
.done:
    pop es
    pop ds
    ret

BITS 32
protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, STACK_TOP

    ; Zero the page-table pages. The early stack is now safely below them.
    mov edi, PML4
    xor eax, eax
    mov ecx, (0x3000 / 4)
    rep stosd

    ; PML4 -> PDPT -> PD, identity-map the first 4 MiB.
    mov dword [PML4], PDPT | 0x003
    mov dword [PDPT], PD | 0x003
    mov dword [PD], 0x00000083
    mov dword [PD + 8], 0x00200083

    mov eax, PML4
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    jmp 0x18:long_mode

BITS 64
long_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, STACK_TOP

    ; SysV-style first argument: RDI = Bennu BootInfo physical address.
    mov rdi, BOOT_INFO
    jmp KERNEL_LOAD

align 8
gdt:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
    dq 0x00AF9A000000FFFF

gdt_descriptor:
    dw gdt_descriptor - gdt - 1
    dd gdt

times 4096-($-$) db 0
