# Bennu Storage

O armazenamento do Bennu foi projetado primeiro para execução em USB.

## Requisitos

- boot independente do disco interno;
- identificação estável do dispositivo;
- sistema de arquivos próprio;
- journaling ou mecanismo equivalente de recuperação;
- metadados com integridade;
- separação entre sistema e dados;
- suporte a atualização atômica;
- modo de recuperação.

## Layout conceitual

A estrutura final será aproximadamente:

- área de boot;
- sistema Bennu;
- dados persistentes;
- área de recuperação;
- espaço livre.

O layout físico exato ainda não está congelado porque depende do desenho final do BennuFS e do carregador UEFI.

## Regra

O kernel não deve assumir que o armazenamento principal é SATA, NVMe ou um disco interno. USB é uma plataforma de primeira classe.
