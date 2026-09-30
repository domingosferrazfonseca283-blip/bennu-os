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
- [ ] contexto de CPU completo com troca real de contexto
- [ ] memória virtual completa
- [ ] espaços de memória independentes por Cell
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
- [ ] descoberta real de dispositivos PCI/USB
- [ ] modelo de drivers baseado em Cells
- [ ] acesso a dispositivos exclusivamente por Capability
- [ ] APIC/IOAPIC
- [ ] interrupções MSI/MSI-X

## Fase 2 — Plataforma USB
- [ ] identificação robusta do dispositivo de boot
- [ ] controlador xHCI
- [ ] enumeração USB
- [ ] USB Mass Storage
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
