# Bennu OS

Bennu OS é um sistema operacional independente, projetado para arrancar e executar diretamente de um pen drive USB.

O Bennu não é uma distribuição Linux e não usa o kernel Linux. O objetivo é possuir kernel, boot, drivers, armazenamento, userspace, segurança, GUI e APIs próprias.

## Execução

O computador hospedeiro fornece firmware e hardware. O sistema operacional do disco interno não é uma dependência.

Arquitetura de execução:

```
USB → BIOS/UEFI → Bennu Boot → Bennu Kernel → Bennu Userspace → Apps
```

O USB será tratado como plataforma de primeira classe, incluindo boot, armazenamento, persistência, recuperação e atualização.

## Estado atual

- kernel freestanding x86_64;
- primeiro estágio de boot BIOS;
- segundo estágio de boot com transição para long mode;
- GDT e IDT próprias;
- BootInfo e mapa físico E820;
- allocator inicial de frames físicos;
- paging e heap iniciais;
- Object/Cell/Capability/Event model inicial;
- runtime de recursos sem alocação dinâmica;
- arquitetura USB-first documentada.

## Próximos subsistemas

1. completar memória virtual;
2. tornar Objects/Cells/Events seguros para concorrência;
3. execução real de Cells;
4. controlador de interrupções e temporização;
5. Resource Graph;
6. dispositivos USB como Objects;
7. armazenamento orientado a Objects;
8. UEFI;
9. runtime nativo e aplicações;
10. Surfaces e experiência gráfica.

## Princípio de segurança

O Bennu não deve acessar ou modificar automaticamente discos internos. Dispositivos de armazenamento adicionais serão recursos explícitos controlados pela política do sistema.

## Desenvolvimento

A documentação de arquitetura, boot, armazenamento e execução em USB está em `docs/`.

As ferramentas de desenvolvimento podem usar componentes externos, mas o sistema operacional produzido pelo projeto deve manter suas próprias abstrações.
