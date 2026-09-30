# Bennu OS — Roadmap

## Fase 0 — Fundação
- [x] Definir identidade de sistema operacional independente
- [x] Definir arquitetura em camadas
- [x] Escolher x86_64 como primeira plataforma
- [x] Boot BIOS inicial
- [x] Kernel freestanding inicial
- [ ] Validação em QEMU
- [ ] Toolchain totalmente reprodutível

## Fase 1 — Kernel
- [x] GDT própria do kernel
- [x] IDT própria do kernel
- [x] primeira camada de exceções CPU
- [ ] interrupções externas (IRQ0)
- [ ] PIC 8259A inicial
- [ ] timer PIT a 100 Hz
- [ ] APIC/IOAPIC
- [ ] timer HPET/APIC
- [x] protocolo Bennu BootInfo
- [x] recolha inicial do mapa físico E820
- [x] allocator inicial de frames físicos
- [ ] paging gerido pelo kernel
- [ ] heap do kernel
- [ ] scheduler
- [ ] processos e threads
- [ ] IPC

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
