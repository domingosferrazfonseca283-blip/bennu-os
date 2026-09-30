# Bennu OS

Bennu OS é um sistema operacional independente, projetado para arrancar e executar diretamente de um pen drive USB.

O Bennu não é uma distribuição Linux e não usa o kernel Linux. O objetivo é possuir kernel, boot, drivers, armazenamento, userspace, segurança, GUI e APIs próprias.

## A ideia central

Bennu trata **Objects, Capabilities, Cells e Events** como primitivas de primeira classe.

- **Object** representa uma entidade ou recurso do sistema.
- **Capability** representa autoridade explícita sobre um Object.
- **Cell** representa uma unidade nativa de execução isolada.
- **Event** representa mudança, comunicação ou sinalização.
- **Resource Graph** relaciona recursos sem transformar arquivos/processos em centro conceitual.
- **Surface** representa uma superfície gráfica nativa.
- **Address Space** modela o espaço de memória de cada domínio de execução.

O objetivo é um núcleo pequeno, mas expressivo: complexidade nasce da composição das primitivas, não de um conjunto enorme de subsistemas obrigatórios no caminho crítico.

## Execução

O computador hospedeiro fornece firmware e hardware. O sistema operacional instalado no disco interno não é uma dependência.

```
USB → BIOS/UEFI → Bennu Boot → Kernel → Object Graph → Cells → Runtime → Apps
```

O USB será uma plataforma de primeira classe: boot, armazenamento, persistência, recuperação e atualização.

## Estado atual

- kernel freestanding x86_64;
- boot BIOS próprio com Bennu BootInfo/E820;
- GDT e IDT próprias;
- PIC + PIT a 100 Hz;
- allocator inicial de frames, paging e heap;
- runtime de Objects/Cells/Capabilities com sincronização base;
- scheduler cooperativo inicial orientado pelo relógio;
- Resource Graph sincronizado;
- modelo inicial de Address Spaces;
- modelo inicial de USB;
- modelo inicial de block storage;
- Surfaces como objetos nativos;
- política arquitetural USB-first, sem acesso automático a discos internos.

## O que já mudou estruturalmente

Bennu não trata processo, thread, arquivo ou socket como a identidade pública do sistema. Esses conceitos podem existir como mecanismos internos quando úteis, mas a API fundamental é composta por Objects, Capabilities, Cells e Events.

Isso permite que o mesmo modelo seja usado para memória, dispositivos, armazenamento, superfícies gráficas, execução e comunicação.

## Próxima grande vertical

A próxima integração é transformar os modelos atuais em uma cadeia funcional completa:

```
PIT → Event Fabric → Scheduler → Cell Context
                         ↓
                 Capability checks
                         ↓
                  Resource Graph
                         ↓
              USB Controller → Block Device
                         ↓
                      BennuFS
```

Ainda falta a troca real de contexto CPU, descoberta física de hardware, xHCI, armazenamento USB funcional, BennuFS e userspace. Esses pontos não são declarados como concluídos até existirem implementações e validação.

## Segurança

Bennu não deve acessar ou modificar automaticamente discos internos. Armazenamento adicional deve aparecer como Object e só ser usado por autoridade explícita.

## Desenvolvimento

A documentação de arquitetura, boot, armazenamento e execução em USB está em `docs/`.

A construção usa uma toolchain freestanding própria. O projeto pode usar ferramentas externas durante o desenvolvimento, mas o sistema operacional mantém suas próprias abstrações no runtime e no kernel.


### Latest kernel progress

- Native asynchronous block I/O fabric with USB-first access policy
- EventPort filtering and subscriptions
- Capability generations and revocation
- Legacy PCI configuration-space discovery
- PCI discovery emits native DeviceAttached events
