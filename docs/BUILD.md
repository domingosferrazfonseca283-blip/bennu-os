# Build e bring-up

O objetivo desta etapa é gerar uma imagem BIOS mínima do Bennu OS.

## Dependências

- Rust nightly
- componente `rust-src`
- `llvm-tools-preview`
- NASM
- QEMU x86_64 para execução da imagem

## Gerar a imagem

Na raiz do repositório:

```sh
make image
```

O resultado esperado é:

```
build/bennu.img
```

## Executar em QEMU

```sh
qemu-system-x86_64 -drive format=raw,file=build/bennu.img
```

O primeiro diagnóstico do kernel escreve:

```
BENNU OS KERNEL OK
```

diretamente no framebuffer VGA texto.

## Próxima validação

Antes de adicionar subsistemas grandes, precisamos validar a cadeia inteira:

BIOS → Stage 1 → Stage 2 → long mode → `kmain`

Se a cadeia funcionar, a próxima fundação do kernel será:

1. GDT;
2. IDT;
3. exceções de CPU;
4. controlador de interrupções;
5. timer;
6. physical frame allocator;
7. paging;
8. heap do kernel.
