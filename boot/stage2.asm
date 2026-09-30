BITS 16
ORG 0x8000

start2:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax

    mov [boot_drive], dl

    mov si, dap
    mov dl, [boot_drive]
    mov ah, 0x42
    int 0x13
    jc error

    cli
    lgdt [gdt_descriptor]

    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:protected_mode

error:
    hlt
    jmp error

boot_drive db 0

dap:
    db 0x10
    db 0
    dw 64
    dw 0x0000
    dw 0x1000
    dq 5

BITS 32
protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x90000

    ; Copy the loaded kernel from 0x10000 to 1 MiB.
    mov esi, 0x10000
    mov edi, 0x100000
    mov ecx, 8192
    rep movsd

    ; PML4 -> PDPT -> PD, identity map first 2 MiB.
    mov dword [0x90000], 0x91003
    mov dword [0x90004], 0

    mov dword [0x91000], 0x92003
    mov dword [0x91004], 0

    mov dword [0x92000], 0x00000083
    mov dword [0x92004], 0

    mov eax, 0x90000
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
    mov rsp, 0x90000
    jmp 0x100000

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
