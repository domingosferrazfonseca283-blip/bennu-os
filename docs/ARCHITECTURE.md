# Bennu OS — Arquitetura

## Objetivo

Bennu OS é um sistema operacional independente. O projeto não usa Linux como kernel.

## Princípios

- kernel próprio e pequeno;
- isolamento entre kernel e serviços;
- APIs nativas do Bennu;
- segurança por capacidade e sandbox;
- interface gráfica própria;
- arquitetura modular;
- evolução incremental, mantendo cada etapa inicializável.

## Camadas

1. Boot — entrada da plataforma e preparação mínima da CPU.
2. Kernel — memória, interrupções, processos, IPC e scheduler.
3. HAL — abstração de hardware.
4. Serviços — armazenamento, rede, dispositivos e segurança.
5. Runtime — ABI/API para aplicações.
6. Shell/GUI — experiência do usuário.
7. Apps — aplicações nativas.

## Primeira plataforma

A primeira plataforma-alvo será x86_64, com execução inicial em máquina virtual/emulador. Outras arquiteturas serão adicionadas depois.

## Regra de engenharia

Nenhum componente de alto nível deve exigir Linux para funcionar. Ferramentas externas podem ser usadas durante o desenvolvimento, mas o sistema final deve possuir suas próprias abstrações.
