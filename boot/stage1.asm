BITS 16
ORG 0x7C00

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    mov [boot_drive], dl

    mov si, dap
    mov dl, [boot_drive]
    mov ah, 0x42
    int 0x13
    jc disk_error

    jmp 0x0000:0x8000

disk_error:
    cli
    hlt
    jmp disk_error

boot_drive db 0

dap:
    db 0x10
    db 0
    dw 8
    dw 0x8000
    dw 0x0000
    dq 1

times 510-($-$$) db 0
dw 0xAA55
