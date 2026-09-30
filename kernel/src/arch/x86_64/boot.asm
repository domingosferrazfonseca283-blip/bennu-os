; Bennu OS — ponto de entrada inicial
section .text
global bennu_kernel_entry

bennu_kernel_entry:
    cli
    hlt
