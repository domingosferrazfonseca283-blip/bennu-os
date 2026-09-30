# Bennu OS — Roadmap

## Fase 0 — Fundação
- [x] Definir identidade de sistema operacional independente
- [x] Definir arquitetura em camadas
- [x] Escolher x86_64 como primeira plataforma
- [ ] Toolchain reprodutível
- [ ] Boot mínimo
- [ ] Kernel que inicializa e escreve diagnóstico

## Fase 1 — Kernel
- [ ] GDT/IDT
- [ ] interrupções
- [ ] memória física
- [ ] paging
- [ ] heap do kernel
- [ ] scheduler
- [ ] processos e threads
- [ ] IPC

## Fase 2 — Plataforma
- [ ] drivers básicos
- [ ] armazenamento
- [ ] sistema de arquivos BennuFS
- [ ] relógio e temporizadores
- [ ] rede

## Fase 3 — Userspace
- [ ] init
- [ ] syscall/API
- [ ] runtime Bennu
- [ ] shell
- [ ] gerenciador de serviços
- [ ] permissões e sandbox

## Fase 4 — Experiência
- [ ] compositor
- [ ] servidor gráfico
- [ ] desktop Bennu
- [ ] sistema de aplicações
- [ ] SDK

## Fase 5 — Ecossistema
- [ ] formato de pacotes
- [ ] atualizações atômicas
- [ ] documentação
- [ ] testes automatizados
- [ ] suporte a outras arquiteturas
