# Bennu OS em pen drive

O Bennu OS será distribuído e executado diretamente a partir de um pen drive USB.

## Princípio

O computador hospedeiro fornece apenas o firmware e o hardware físico. O Bennu não depende do sistema operacional instalado no disco interno.

Fluxo:

USB → firmware BIOS/UEFI → Bennu Boot → Bennu Kernel → Bennu Userspace

O disco interno do computador não é requisito para inicialização.

## Dois modos de utilização

### Live persistente

A imagem do Bennu reside no USB e o sistema mantém dados, configurações e aplicações no próprio dispositivo.

### Sistema instalado no USB

O USB funciona como o dispositivo de armazenamento principal do Bennu. O kernel, userspace, sistema de arquivos e aplicações ficam no USB.

A arquitetura deve evitar assumir que exista um disco SATA/NVMe interno.

## Segurança

O acesso a discos internos deve ser um recurso explícito de hardware e de política do sistema. O Bennu não deve montar nem modificar automaticamente discos internos.

## Compatibilidade de boot

A primeira cadeia usa BIOS para bring-up.

A próxima cadeia necessária para computadores modernos será:

UEFI → Bennu EFI Loader → Bennu Kernel

A imagem final deverá possuir uma estrutura de disco compatível com USB e UEFI, além do caminho BIOS de compatibilidade.

## Persistência

A camada de armazenamento será projetada para:

- identificar o próprio dispositivo de boot;
- manter uma partição de sistema;
- manter dados persistentes;
- permitir atualização atômica;
- suportar recuperação;
- evitar corrupção após perda de energia sempre que possível.

O formato definitivo será definido junto com o BennuFS.
