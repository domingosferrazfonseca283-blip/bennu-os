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
- [ ] GDT
- [ ] IDT
- [ ] exceções de CPU
- [ ] interrupções
- [ ] timer
- [ ] memória física
- [ ] paging
- [ ] heap do kernel
- [ ] scheduler
- [ ] processos e threads
- [ ] IPC

## Fase 2 — Plataforma USB
- [ ] controladores USB
- [ ] armazenamento USB
- [ ] identificação do dispositivo de boot
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
