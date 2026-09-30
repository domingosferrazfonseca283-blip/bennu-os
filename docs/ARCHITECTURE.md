# Bennu OS — Arquitetura

## Objetivo

Bennu OS é um sistema operacional independente. O projeto não usa Linux como kernel nem pretende ser uma distribuição Linux.

O alvo operacional do projeto é um sistema pessoal, inicializado diretamente de um dispositivo USB e capaz de funcionar sem depender de um sistema operacional instalado no computador hospedeiro.

## Princípios

- kernel próprio e pequeno;
- isolamento entre kernel e serviços;
- APIs nativas do Bennu;
- segurança por capacidade e sandbox;
- interface gráfica própria;
- arquitetura modular;
- evolução incremental, mantendo cada etapa inicializável;
- USB como plataforma de primeira classe;
- armazenamento interno do computador nunca é acessado ou alterado automaticamente.

## Camadas

1. Boot — entrada da plataforma e preparação mínima da CPU.
2. Kernel — memória, interrupções, processos, IPC e scheduler.
3. HAL — abstração de hardware.
4. Serviços — armazenamento, rede, dispositivos e segurança.
5. Runtime — ABI/API para aplicações.
6. Shell/GUI — experiência do usuário.
7. Apps — aplicações nativas.

## Estado atual do kernel

O bring-up x86_64 já possui uma fronteira clara entre o bootloader e o kernel:

- Stage 2 entra em long mode;
- o kernel instala sua própria GDT;
- o kernel instala sua própria IDT;
- exceções iniciais têm handlers nativos;
- diagnóstico pré-userspace usa VGA apenas como canal temporário;
- Stage 2 publica um protocolo `BootInfo` próprio do Bennu;
- o protocolo transporta o dispositivo de boot, localização/tamanho do kernel e o mapa físico E820;
- o kernel já possui a primeira camada de alocação de frames físicos, recusando memória abaixo de 1 MiB e a região ocupada pelo próprio kernel;
- o próximo salto é fazer o kernel assumir as próprias tabelas de páginas e construir o heap.

A ABI de interrupção x86-interrupt do Rust é usada apenas na camada de baixo nível; ela não define a API pública do sistema operacional.

## Primeira plataforma

A primeira plataforma-alvo é x86_64. QEMU é um ambiente de validação durante o desenvolvimento, não uma dependência do sistema final. O alvo físico principal é um computador inicializado diretamente a partir do USB.

## Regra de engenharia

Nenhum componente de alto nível deve exigir Linux para funcionar. Ferramentas externas podem ser usadas durante o desenvolvimento, mas o sistema final deve possuir suas próprias abstrações.
