BITS 16
ORG 0x8000

%ifndef KERNEL_SECTORS
%define KERNEL_SECTORS 64
%endif

%if KERNEL_SECTORS < 1
%error "KERNEL_SECTORS must be positive"
%endif

%if KERNEL_SECTORS > 120
%error "kernel image exceeds early BIOS loader capacity"
%endif

%define BOOT_INFO        0x5000
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

    mov dword [BOOT_INFO + 0], 0x554E4E45
    mov dword [BOOT_INFO + 4], 0x014F534E
    mov dword [BOOT_INFO + 8], 1
    mov dword [BOOT_INFO + 12], 56
    mov byte  [BOOT_INFO + 16], dl

    mov dword [BOOT_INFO + 24], KERNEL_LOAD
    mov dword [BOOT_INFO + 28], 0
    mov dword [BOOT_INFO + 32], KERNEL_SECTORS * 512
    mov dword [BOOT_INFO + 36], 0

    mov dword [BOOT_INFO + 40], MEMORY_MAP
    mov dword [BOOT_INFO + 44], 0
    mov dword [BOOT_INFO + 48], E820_ENTRY_SIZE

    ; Clear the E820 destination area before BIOS writes variable-sized entries.
    mov di, MEMORY_MAP
    xor eax, eax
    mov cx, (E820_MAX * E820_ENTRY_SIZE) / 4
    rep stosd

    call detect_memory_map
    jc e820_error

    mov ax, [e820_count]
    mov [BOOT_INFO + 44], ax

    ; Load the kernel from LBA 5 into temporary low memory.
    mov si, dap
    mov dl, [boot_drive]
    mov ah, 0x42
    int 0x13
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

dap:
    db 0x10
    db 0
    dw KERNEL_SECTORS
    dw 0x0000
    dw 0x1000
    dq 5

BITS 32
protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, STACK_TOP

    ; Copy the loaded kernel from 0x10000 to 1 MiB.
    mov esi, 0x10000
    mov edi, KERNEL_LOAD
    mov ecx, (KERNEL_SECTORS * 512) / 4
    rep movsd

    ; Zero the page-table pages. The early stack is now safely below them.
    mov edi, PML4
    xor eax, eax
    mov ecx, (0x3000 / 4)
    rep stosd

    ; PML4 -> PDPT -> PD, identity-map the first 2 MiB.
    mov dword [PML4], PDPT | 0x003
    mov dword [PDPT], PD | 0x003
    mov dword [PD], 0x00000083

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

times 2048-($-$$) db 0
