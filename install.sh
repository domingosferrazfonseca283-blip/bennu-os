#!/usr/bin/env bash
set -e

echo "[*] Instalando o Bennu OS e a interface gráfica..."

INSTALL_DIR="/opt/bennu"

if [ "$EUID" -ne 0 ]; then
  echo "[!] Por favor, execute como root (sudo)."
  exit 1
fi

mkdir -p "$INSTALL_DIR"/{tools,scripts,config,reports,logs,ui}

if [ -f "bennu-desktop-v2.html" ]; then
  cp bennu-desktop-v2.html "$INSTALL_DIR/ui/"
fi

chmod +x "$INSTALL_DIR"/*.py 2>/dev/null || true

if [ -f "$INSTALL_DIR/bennu.py" ]; then
  ln -sf "$INSTALL_DIR/bennu.py" /usr/local/bin/bennu
  chmod +x /usr/local/bin/bennu
  echo "[+] Bennu OS e interface instalados com sucesso!"
else
  echo "[x] Erro: bennu.py não encontrado em $INSTALL_DIR."
  exit 1
fi
