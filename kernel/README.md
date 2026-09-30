# Bennu Kernel

Núcleo independente do Bennu OS.

Este diretório será dividido por responsabilidade:

- `arch/` — código específico da arquitetura;
- `mm/` — memória;
- `sched/` — scheduler;
- `ipc/` — comunicação entre processos;
- `drivers/` — drivers;
- `fs/` — armazenamento e sistema de arquivos;
- `net/` — rede;
- `security/` — capacidades e isolamento.

A regra é manter o núcleo pequeno e mover serviços para userspace sempre que isso não comprometer a inicialização.
