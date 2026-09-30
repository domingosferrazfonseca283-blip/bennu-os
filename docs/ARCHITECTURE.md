# Bennu OS — Arquitetura

## Objetivo

Bennu OS é um sistema operacional independente. O projeto não usa Linux como kernel nem pretende ser uma distribuição Linux.

O alvo operacional do projeto é um sistema pessoal, inicializado diretamente de um dispositivo USB e capaz de funcionar sem depender de um sistema operacional instalado no computador hospedeiro.

## Princípios

- kernel próprio e pequeno;
- Cells como unidades nativas de execução e isolamento;
- Objects como unidades nativas de recursos e estado;
- Capabilities como autoridade explícita sobre Objects;
- Events como mecanismo nativo de comunicação;
- APIs nativas do Bennu;
- interface gráfica própria;
- arquitetura modular;
- evolução incremental, mantendo cada etapa inicializável;
- USB como plataforma de primeira classe;
- armazenamento interno do computador nunca é acessado ou alterado automaticamente.

## Modelo fundamental

O Bennu não adota processos, threads, ficheiros, sockets ou syscalls tradicionais como modelo conceptual central.

As primitivas fundamentais são:

1. Object — representa um recurso, estado, superfície, dispositivo ou entidade persistente.
2. Capability — representa autoridade explícita sobre um Object.
3. Cell — espaço isolado de execução que possui um conjunto limitado de capabilities.
4. Event — unidade nativa de comunicação e mudança de estado.
5. Resource Graph — relações entre Objects, Cells e recursos.

Uma Cell só pode operar sobre um Object quando possui uma Capability compatível.

## Camadas

1. Boot — entrada da plataforma e preparação mínima da CPU.
2. Kernel Core — memória, CPU, isolamento, Objects, Capabilities e Events.
3. Hardware — acesso físico e descoberta de recursos.
4. Resource Graph — dispositivos, memória, armazenamento e outros recursos.
5. Cells — unidades isoladas de execução.
6. Surfaces — representação nativa da experiência visual.
7. Applications — conjuntos de Cells e Objects.

Scheduler, IPC e outros mecanismos podem existir como implementação interna, mas não definem a arquitetura pública do Bennu.

## Estado atual do kernel

Além do bring-up x86_64, o kernel já começou a implementar o modelo próprio do Bennu:

- ObjectId com geração para distinguir instâncias reutilizadas;
- tipos nativos de Object para memória, dispositivos, dados, superfícies, eventos e Cells;
- Capability com direitos explícitos;
- Cell com tabela limitada de capabilities;
- Event Fabric inicial com fila fixa e sem alocação dinâmica;
- runtime inicial para criação de Objects, Cells, concessão de capabilities e eventos;
- Cell raiz criada durante a inicialização do kernel.

O bring-up x86_64 já possui uma fronteira clara entre o bootloader e o kernel:

- Stage 2 entra em long mode;
- o kernel instala sua própria GDT;
- o kernel instala sua própria IDT;
- exceções iniciais têm handlers nativos;
- diagnóstico pré-userspace usa VGA apenas como canal temporário;
- Stage 2 publica um protocolo `BootInfo` próprio do Bennu;
- o protocolo transporta o dispositivo de boot, localização/tamanho do kernel e o mapa físico E820;
- o kernel já possui a primeira camada de alocação de frames físicos, recusando memória abaixo de 1 MiB e a região ocupada pelo próprio kernel;
- o próximo salto é completar a memória virtual e tornar o modelo de estado seguro para concorrência e interrupções.

A ABI de interrupção x86-interrupt do Rust é usada apenas na camada de baixo nível; ela não define a API pública do sistema operacional.

## Primeira plataforma

A primeira plataforma-alvo é x86_64. QEMU é um ambiente de validação durante o desenvolvimento, não uma dependência do sistema final. O alvo físico principal é um computador inicializado diretamente a partir do USB.

## Regra de engenharia

Nenhum componente de alto nível deve exigir Linux para funcionar. Ferramentas externas podem ser usadas durante o desenvolvimento, mas o sistema final deve possuir suas próprias abstrações.
