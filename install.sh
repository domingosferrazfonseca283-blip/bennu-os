#!/usr/bin/env bash
set -e

echo "[*] Iniciando a instalação do Bennu OS Toolkit..."

INSTALL_DIR="/opt/bennu"
BIN_DIR="/usr/local/bin"

if [ "$EUID" -ne 0 ]; then
    echo "[x] Por favor, execute como root."
    exit 1
fi

mkdir -p "$INSTALL_DIR"

if [ -d "toolkit" ]; then
    cp -r toolkit/* "$INSTALL_DIR/"
else
    cp -r * "$INSTALL_DIR/"
fi

chmod +x "$INSTALL_DIR"/*.py

for script in "$INSTALL_DIR"/*.py; do
    filename=$(basename "$script")
    name="${filename%.py}"
    ln -sf "$script" "$BIN_DIR/$name"
    echo "[+] Link criado para: $name"
done

if [ ! -f "$INSTALL_DIR/config.json" ]; then
    echo '{"anthropic_api_key": ""}' > "$INSTALL_DIR/config.json"
    chmod 600 "$INSTALL_DIR/config.json"
    echo "[+] Arquivo de configuração gerado em $INSTALL_DIR/config.json"
fi

echo "[*] Instalação concluída com sucesso!"
