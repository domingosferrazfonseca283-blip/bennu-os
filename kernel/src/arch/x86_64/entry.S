.section .text.boot
.global bennu_kernel_entry
.type bennu_kernel_entry, @function

bennu_kernel_entry:
    cli
    mov $0x90000, %rsp
    call kmain

1:
    hlt
    jmp 1b
