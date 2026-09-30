# Bennu OS — Roadmap

## Fase 0 — Fundação
- [x] Definir identidade de sistema operacional independente
- [x] Definir arquitetura em camadas
- [x] Escolher x86_64 como primeira plataforma
- [x] Boot BIOS inicial
- [x] Kernel freestanding inicial
- [ ] Validação em QEMU
- [ ] Toolchain totalmente reprodutível

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
- [ ] memória virtual completa
- [ ] proteção e sincronização do estado global
- [x] execução cooperativa mínima de Cells
- [x] contexto de CPU por Cell
- [ ] preempção orientada a eventos
- [ ] afinidade e recursos por Cell

## Fase 1.5 — Hardware e Resource Graph
- [x] controlador de interrupções PIC inicial
- [x] temporizador de hardware PIT inicial
- [ ] descoberta de dispositivos como Objects
- [ ] Resource Graph persistente em memória
- [ ] modelo de drivers baseado em Cells
- [ ] acesso a dispositivos exclusivamente por Capability

## Fase 2 — Plataforma USB
- [ ] identificação robusta do dispositivo de boot
- [ ] controladores USB
- [ ] armazenamento USB
- [ ] BennuFS
- [ ] journaling/recuperação
- [ ] relógio e temporizadores
- [ ] rede
- [ ] drivers básicos

## Fase 3 — Boot moderno
- [ ] UEFI Bennu Boot
- [ ] imagem GPT
- [ ] partição EFI
- [ ] cadeia BIOS de compatibilidade
- [ ] verificação de integridade do boot
- [ ] recuperação

## Fase 4 — Userspace
- [ ] init
- [ ] syscall/API
- [ ] runtime Bennu
- [ ] shell
- [ ] gerenciador de serviços
- [ ] permissões e sandbox

## Fase 5 — Experiência
- [ ] compositor
- [ ] servidor gráfico
- [ ] desktop Bennu
- [ ] sistema de aplicações
- [ ] SDK

## Fase 6 — Ecossistema
- [ ] formato de pacotes
- [ ] atualizações atômicas
- [ ] recuperação do sistema
- [ ] documentação
- [ ] testes automatizados
- [ ] suporte a outras arquiteturas
