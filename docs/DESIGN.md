# Bennu OS — Decisões de Design

## Por que um kernel próprio?

O objetivo do projeto é construir um sistema operacional independente, e não uma distribuição Linux. Portanto, o kernel é uma parte central do produto.

## Por que começar pequeno?

Um sistema operacional completo é grande demais para ser construído de uma vez. O primeiro marco é uma imagem inicializável que prove a cadeia:

firmware/boot → Bennu boot → kernel → diagnóstico.

Depois adicionamos memória, interrupções, execução concorrente, armazenamento, rede e userspace.

## Segurança

O modelo de segurança será projetado desde o início. A direção é capability-based security, isolamento de processos e permissões explícitas para recursos.

## Interface

A GUI será uma camada nativa do Bennu. Ela não será uma adaptação de GNOME, KDE ou outro desktop existente.
