# Bennu Boot

A camada de boot do Bennu OS é própria e independente de Linux.

## Cadeia atual

firmware BIOS → Bennu Stage 1 → Bennu Stage 2 → Bennu Kernel

- **Stage 1:** setor de boot de 512 bytes, carrega o Stage 2.
- **Stage 2:** usa BIOS INT 13h Extensions para carregar o kernel, entra em protected mode e depois long mode, cria uma paginação inicial de 2 MiB e salta para 0x100000.
- **Kernel:** executa em x86_64 sem a biblioteca padrão e escreve um diagnóstico inicial na memória VGA.

O bootloader não depende de GRUB, Linux, systemd ou componentes de outro sistema operacional.

## Limitações deliberadas do primeiro protótipo

Esta etapa usa uma imagem BIOS simples e um limite inicial de carregamento do kernel de 64 setores (32 KiB). Isso é uma fundação de bring-up, não o formato final do boot.

Próximas etapas:

1. validar a imagem em QEMU;
2. trocar o carregamento fixo por metadados do kernel;
3. adicionar verificação de integridade;
4. separar o código de firmware da lógica de boot do kernel;
5. adicionar suporte UEFI;
6. definir o formato final da imagem do Bennu OS.
