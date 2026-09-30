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


## Expansão arquitetural atual

A arquitetura está a evoluir para uma cadeia de recursos única:

```
Hardware → Objects → Capabilities → Cells → Events
                    ↓             ↓
              Resource Graph   Domains
                    ↓             ↓
             Device Fabric   Address Spaces
                    ↓
              Async I/O Fabric
                    ↓
             USB / Block Storage
                    ↓
                  BennuFS
                    ↓
             Surfaces / Buffers
                    ↓
              Bennu Applications
```

Foram adicionados modelos nativos para PCI, xHCI, USB, block I/O, drivers, event ports, Address Spaces, Domains, BennuFS e buffers gráficos.

O armazenamento segue uma política explícita: dispositivos internos não recebem autoridade implícita. Um dispositivo deve ser descoberto como Object e a sua utilização deve atravessar uma Capability e a política de armazenamento do sistema.

A camada de armazenamento é desenhada para I/O assíncrono. Requests de bloco, envelopes de I/O e completions são objetos de protocolo; drivers serão Cells que consomem esses protocolos.

BennuFS não trata caminhos e ficheiros como a primitiva arquitetural central. A representação persistente é baseada em Objects/Nodes, relações de parentagem e um journal transacional. Uma interface de ficheiros poderá existir por conveniência no runtime, mas será uma visão sobre Objects persistentes.

A camada gráfica segue a mesma regra: buffers e surfaces são Objects com autoridade explícita. O compositor futuro será uma composição de Cells e Events, não um subsistema externo obrigatório.

## Execution Fabric

O scheduler já é orientado pelo relógio PIT e só executa uma Cell quando existe um tick pendente. A troca completa de contexto CPU ainda é uma etapa separada: o snapshot atual não é declarado como context switch seguro.

A implementação futura do contexto deverá definir explicitamente:
- conjunto de registradores preservados;
- stack de entrada e retorno;
- trampoline de Cell;
- política de interrupções;
- relação entre Context e Address Space;
- comportamento de yield, wait, wake e stop.

Isso mantém a fronteira entre o modelo conceptual de Cell e os mecanismos internos necessários para executar código com segurança.


## Native I/O Fabric

Bennu models block I/O as an asynchronous fabric rather than synchronous device calls. A BlockRequest enters a bounded I/O queue, is authorized by the USB-first AccessPolicy, becomes an IoEnvelope, and is completed independently of the submitting Cell. Completion carries a token and status so an Event can wake the owning Cell without exposing a conventional blocking syscall model.

Internal storage is not implicitly eligible. The default policy permits the removable boot medium and explicitly removable devices while denying internal storage unless a future authority grants it deliberately.

## PCI Discovery

The kernel contains a first legacy PCI configuration-space discovery path for x86_64. Discovered functions become Bennu Device Objects and emit DeviceAttached events. This is discovery only: BAR mapping, xHCI initialization, USB enumeration, and Mass Storage transport remain separate capabilities in the Device Fabric.


## USB Controller Fabric

PCI xHCI devices now expose their MMIO BAR as a hardware resource in the Device Fabric. The xHCI model contains capability-derived slot/port limits, command and event rings, TRB cycle state, and port state. These structures deliberately stop before direct MMIO access: the next hardware layer will map the controller region, perform controller reset, establish the Device Context Base Address Array, start the rings, and consume interrupt events.


## Execução nativa de Cells

A execução do Bennu já possui uma fronteira explícita entre o scheduler e as Cells:

- cada Cell executável recebe uma stack de kernel própria;
- a troca de contexto preserva o conjunto callee-saved da ABI x86_64;
- a entrada da Cell passa por um trampoline nativo que devolve o controlo ao scheduler através do Execution Fabric;
- cada Cell executável recebe uma raiz de espaço de endereçamento própria;
- a entrada numa Cell muda o CR3 para a sua raiz e o retorno ao scheduler restaura a raiz do kernel;
- a raiz da Cell é inicialmente derivada do espaço do kernel, portanto este passo fornece isolamento estrutural de raízes, mas **ainda não constitui isolamento de memória de utilizador completo**;
- permissões supervisor/user, NX, regiões de kernel partilhadas controladamente e mapeamentos de objetos por Capability continuam a ser trabalho futuro.

Isto mantém a identidade arquitetural do Bennu: a unidade pública de execução é a **Cell**, não um processo tradicional. O scheduler é uma implementação interna do Execution Fabric.


## ABI nativa e fronteira de chamadas

O Bennu agora possui uma primeira fronteira de chamada nativa x86_64 em int 0x80:

- a entrada é um stub assembly que preserva os registos da Cell;
- os argumentos são convertidos para o tipo Call da ABI Bennu;
- o dispatcher identifica a Operation;
- operações protegidas passam pela validação de Capability antes de tocar no recurso;
- o resultado regressa pelo mesmo frame de registos;
- o vetor é exposto como gate utilizável por código de nível utilizador quando existirem Cells em CPL3.

A implementação atual é deliberadamente pequena. ObjectQuery, EventEmit e o primeiro caminho de DeviceSubmit possuem semântica concreta; as restantes operações continuam explícitas até terem validação e execução completas.


## Userspace nativo

A primeira fronteira real de userspace já existe no x86_64. Uma Cell pode possuir uma raiz de Address Space própria, stack de kernel para entradas de privilégio, código e stack marcados como USER e uma entrada Ring 3 construída com `iretq`.

A fronteira de chamada nativa usa `int 0x80`. O número da operação entra em `RAX`, os handles e argumentos entram em registradores Bennu e o dispatcher devolve status/valor no mesmo frame. O yield é uma operação nativa: uma Cell Ring 3 pode devolver o CPU ao scheduler e posteriormente retomar a execução em Ring 3.

O primeiro programa de userspace é deliberadamente mínimo: executa em uma página não gravável, usa uma stack USER não executável e repete a operação de yield. Isso valida a direção completa Ring 3 → syscall → kernel scheduler → Ring 3 sem transformar o userspace em uma extensão privilegiada do kernel.

As páginas de userspace ocupam uma região virtual privada da raiz da Cell. A raiz começa com o espaço de kernel compartilhado estruturalmente, mas as novas tabelas para a região USER são criadas por Cell. A separação avançada continuará a eliminar dependências compartilhadas e a introduzir copy-in/copy-out, reclaim e políticas de mapeamento baseadas em Capability.


## Isolamento real de Address Spaces

Uma Cell executável agora nasce com um PML4 próprio vazio. Ela não herda automaticamente os mapeamentos do kernel. O espaço da Cell recebe apenas páginas supervisoras mínimas para a entrada de execução, stack de kernel e entrada de timer/syscall, além das páginas USER explicitamente instaladas.

O CR3 do kernel é mantido separadamente. A entrada int 0x80 captura o CR3 da Cell e troca imediatamente para o CR3 do kernel antes de acessar estado, capacidades ou objetos. Na entrada Ring 3, o caminho inverso é explícito: o trampolim carrega o CR3 da Cell antes de executar iretq.

A stack de kernel da Cell permanece mapeada como supervisor-only no espaço da própria Cell porque a CPU precisa de uma stack válida para a transição CPL3→CPL0 antes que qualquer código de entrada possa trocar CR3. Isso não concede leitura/escrita ao código Ring 3.

## Fronteira de memória do usuário

O kernel não deve dereferenciar diretamente ponteiros fornecidos pela ABI. A camada de memória oferece tradução validada de páginas USER e primitivas copy_from_user / copy_to_user, verificando canonicalidade, presença da página, bit USER e, para escrita, WRITABLE.

O primeiro backend de MemoryMap segue a mesma regra: somente Memory/Data Objects com Capability contendo MAP podem ser mapeados. Uma escrita exige também WRITE. O endereço físico não é fornecido pela Cell; o backing frame pertence ao Object e é resolvido pelo runtime.


## Delegação de Capabilities

A delegação de autoridade segue uma regra de não escalada:

- a Cell delegante identifica a Capability-fonte e o Object ao qual ela se refere;
- uma Capability com `SHARE` só pode delegar direitos que já estejam contidos na própria Capability-fonte;
- uma Capability com `ADMIN` pode atuar como autoridade administrativa para delegação;
- direitos desconhecidos são rejeitados pela ABI;
- a Cell de destino precisa existir antes da delegação;
- a Capability derivada recebe uma nova geração e um novo slot na tabela da Cell de destino;
- revogar a Capability original continua a invalidar somente aquela entrada, preservando a separação por geração.

Assim, uma cadeia de delegações não pode transformar uma autoridade limitada em uma autoridade maior. A autoridade flui pelo Resource Graph, mas não cresce por delegação.


## Device Fabric e I/O nativo

O acesso a hardware não é uma chamada privilegiada genérica. `DeviceSubmit` segue a mesma cadeia fundamental do restante do sistema:

`Cell → Capability → Device Object → Operation → Device I/O Queue → Completion/Event`

A operação valida simultaneamente:

- a Capability pertence à Cell chamadora e referencia o Object correto;
- a Capability possui `DEVICE` e `WRITE`;
- o Object referenciado é efetivamente do tipo `Device`;
- a requisição possui opcode e token válidos;
- a requisição é colocada na fila assíncrona do Device Fabric.

A execução física do driver pode consumir essa fila posteriormente, sem conceder à Cell acesso direto a MMIO, PCI ou DMA. O próximo estágio é ligar essa fila aos registros concretos de `DeviceFabric`/xHCI e devolver completions através do Event Fabric.


### ABI de DeviceSubmit

A fronteira Ring 3 não entrega uma estrutura de kernel diretamente. A chamada nativa usa registradores separados: Capability em RDI, Device Object em RSI, buffer USER em RDX, comprimento em R10, token em R8 e opcode/flags compactados em R11. O kernel valida o intervalo USER antes de construir o `DeviceRequest` interno.

O request interno conserva buffer, comprimento, argumento, valor e token como dados do kernel. A fila não recebe ponteiros físicos nem autoridade adicional. Assim, uma Cell só consegue submeter I/O para um Device Object que já possui através de Capability e dentro de um intervalo virtual que o kernel aceitou.

A implementação atual limita opcode e flags ao formato compacto da ABI; a execução física ainda pertence ao Device Fabric/driver e a conclusão será entregue pelo Event Fabric.


### Device Fabric persistente

O Device Fabric deixou de ser uma estrutura temporária do arranque. O runtime mantém uma instância persistente e o scheduler pode consumir a fila DeviceRequest através de service_device_io().

O primeiro executor concreto é xHCI: requests dirigidos ao Object do controlador podem produzir comandos nativos como Enable Slot. O evento emitido neste estágio é DeviceQueued, não DeviceCompleted; a conclusão só será declarada depois de o Event Ring/DMA físico confirmar o comando. Assim a arquitetura não confunde aceitação do request com conclusão de hardware.


### Command Ring → Event Ring → Completion

O primeiro caminho físico assíncrono do Device Fabric agora atravessa o hardware xHCI de ponta a ponta no modelo do kernel:

`DeviceRequest → DeviceFabric → Command Ring DMA → xHCI → Event Ring DMA → DeviceCompleted`

O Command Ring reserva explicitamente o TRB de Link, grava o TRB lógico na memória DMA física e toca o doorbell do controlador. O runtime mantém o token associado ao endereço físico do command TRB.

O Event Ring possui dequeue e cycle state próprios. O runtime lê os TRBs DMA produzidos pelo controlador, atualiza o Event Ring Dequeue Pointer e, para Command Completion Events, correlaciona o command TRB com o request pendente. A conclusão é então publicada como `DeviceCompleted` no Event Fabric.

O valor do evento preserva o token e o completion code xHCI. `DeviceQueued` continua significando apenas que o request foi aceito pelo Device Fabric; `DeviceCompleted` significa confirmação observável pelo Event Ring.

Este caminho ainda é código de bring-up de hardware e não é declarado como validado em QEMU ou em hardware físico até existir uma execução de CI/boot que o confirme.


## Transferência USB nativa: primeiro caminho de Control Transfer

A camada xHCI agora possui as primitivas físicas para sair do modelo apenas de comandos de controlador e preparar uma transferência USB real no endpoint 0.

O caminho é:

`Cell → Capability(Device) → Device Fabric → xHCI → Input Context → Address Device → EP0 Transfer Ring → Setup/Data/Status TRBs → Transfer Event → Event Fabric`

A preparação de `Address Device` constrói um Input Context DMA e um Device Context DMA, seleciona o tamanho de contexto anunciado pelo controlador, cria o Slot Context com porta/velocidade e prepara o Endpoint 0 para control transfers.

A primeira família de transferências nativas usa três estágios:

- Setup Stage: pacote USB de 8 bytes inline no TRB, com Immediate Data;
- Data Stage: opcional, com direção IN/OUT e buffer DMA;
- Status Stage: direção complementar e IOC.

A Transfer Ring é DMA-backed, termina com Link TRB e é acionada pelo doorbell do slot. O endereço físico do primeiro TRB é preservável para correlação posterior com um Transfer Event.

Isto ainda não significa enumeração USB completa. O próximo passo é manter persistentemente os transfer rings por slot/endpoint, correlacionar `TRANSFER_EVENT` com requests pendentes e então executar `GET_DESCRIPTOR`, `SET_ADDRESS` e `SET_CONFIGURATION`. Só depois disso a camada Bulk-Only Mass Storage poderá transportar CBW/CSW e SCSI sobre os endpoints reais.
