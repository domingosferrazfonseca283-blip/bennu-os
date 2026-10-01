# Bennu OS — Roadmap

## Fase 0 — Fundação
- [x] Definir identidade de sistema operacional independente
- [x] Definir arquitetura em camadas
- [x] Escolher x86_64 como primeira plataforma
- [x] Boot BIOS inicial
- [x] Kernel freestanding inicial
- [ ] Validação real em QEMU
- [x] CI de construção reproduzível
- [ ] Toolchain totalmente fixado por versões

## Fase 1 — Kernel Core
- [x] GDT própria do kernel
- [x] IDT própria do kernel
- [x] primeira camada de exceções CPU
- [x] protocolo Bennu BootInfo
- [x] recolha inicial do mapa físico E820
- [x] allocator inicial de frames físicos
- [x] paging gerido pelo kernel
- [x] heap inicial do kernel
- [x] ObjectId e tipos nativos de Object
- [x] Capability e direitos explícitos
- [x] Cell e isolamento lógico inicial
- [x] Event Fabric inicial sem alocação dinâmica
- [x] runtime inicial de Objects/Cells/Capabilities
- [x] sincronização base do runtime
- [x] scheduler cooperativo inicial
- [x] contexto de CPU com troca real de contexto entre stacks de Cell
- [x] raízes independentes de espaço de endereçamento por Cell
- [x] Ring 3 com GDT/TSS e entrada via iretq
- [x] páginas USER com permissões de escrita e NX
- [x] stack de kernel por Cell para transição Ring 3 → Ring 0
- [x] ABI nativa acessível por `int 0x80`
- [x] yield de userspace para o scheduler e retorno a Ring 3
- [ ] memória virtual completa (reclamação, copy-in/copy-out e proteção avançada)
- [x] raízes de espaço de memória independentes por Cell
- [ ] preempção orientada a eventos
- [ ] afinidade e quotas de recursos por Cell
- [ ] IPC nativo orientado a Events

## Fase 1.5 — Hardware e Resource Graph
- [x] controlador de interrupções PIC inicial
- [x] temporizador de hardware PIT inicial
- [x] Resource Graph persistente em memória
- [x] sincronização do Resource Graph
- [x] modelo inicial de dispositivos como Objects
- [x] modelo inicial de USB
- [x] modelo inicial de armazenamento em blocos
- [x] descoberta PCI legada e identificação de xHCI
- [x] descoberta real de portas xHCI e primitivas de reset/acknowledge
- [ ] modelo de drivers baseado em Cells
- [x] acesso a dispositivos exclusivamente por Capability
- [x] fila nativa de DeviceRequest com validação de Capability
- [x] fronteira ABI DeviceSubmit com buffer USER e comprimento validados
- [ ] APIC/IOAPIC
- [ ] interrupções MSI/MSI-X

## Fase 2 — Plataforma USB
- [ ] identificação robusta do dispositivo de boot
- [x] controlador xHCI: MMIO, reset, DMA, rings, doorbell e interrupções
- [x] enumeração USB: eventos, Enable Slot, Address Device e control-transfer primitives
- [x] topologia USB: interfaces/endpoints e identificação Bulk-Only Mass Storage
- [x] USB Mass Storage: CBW/CSW e comandos SCSI base
- [ ] ligação física completa entre TRBs DMA e dispositivos USB
- [x] consumo da fila DeviceRequest pelo Device Fabric/xHCI
- [x] Device Fabric persistente no runtime
- [x] confirmação de DeviceRequest pelo Event Ring xHCI
- [x] completions tipadas de DeviceRequest no Event Fabric
- [x] tracking persistente de Transfer Ring por slot
- [x] buffer DMA nativo para GET_DESCRIPTOR sem expor VA USER ao hardware
- [x] persistência do Device Descriptor como UsbDevice/Object
- [x] estado USB explícito Detached/Default/Addressed/Configured
- [x] SET_CONFIGURATION como Control Transfer EP0
- [x] parser nativo de Configuration Descriptor / interfaces / endpoints
- [ ] Bulk Transfer Rings persistentes
- [ ] SET_ADDRESS / enumeration end-to-end
- [ ] SET_ADDRESS/SET_CONFIGURATION end-to-end
- [ ] fila de requests de bloco assíncrona
- [ ] BennuFS
- [ ] journaling/recuperação
- [ ] relógio e temporizadores
- [ ] drivers básicos
- [ ] política explícita para discos internos

## Fase 3 — Boot moderno
- [ ] UEFI Bennu Boot
- [ ] imagem GPT
- [ ] partição EFI
- [ ] cadeia BIOS de compatibilidade
- [ ] verificação de integridade do boot
- [ ] recuperação

## Fase 4 — Userspace
- [x] primeira Cell real em Ring 3
- [x] fronteira Ring 3 → kernel → Ring 3
- [ ] init
- [ ] API nativa de Objects/Capabilities/Cells/Events
- [ ] runtime Bennu
- [ ] shell
- [ ] gerenciador de serviços
- [ ] permissões e sandbox
- [ ] ABI estável

## Fase 5 — Experiência
- [ ] compositor
- [ ] servidor gráfico
- [ ] Surfaces reais e buffers
- [ ] desktop Bennu
- [ ] sistema de aplicações
- [ ] SDK

## Fase 6 — Ecossistema
- [ ] formato de pacotes
- [ ] atualizações atômicas
- [ ] recuperação do sistema
- [ ] documentação completa
- [ ] testes automatizados de kernel
- [ ] suporte a outras arquiteturas
