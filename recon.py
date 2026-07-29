#!/usr/bin/env python3
import socket
import sys
import json
from datetime import datetime

def analyze_target(target):
    print(f"[*] Analisando alvo: {target}")
    try:
        ip = socket.gethostbyname(target)
        print(f"[+] IP Resolvido: {ip}")
    except socket.gaierror:
        print(f"[x] Erro: Não foi possível resolver o host {target}")
        sys.exit(1)

    common_ports = [21, 22, 23, 25, 53, 80, 110, 443, 445, 8080, 8443]
    open_ports = []

    print("[*] Varrendo portas comuns...")
    for port in common_ports:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(0.5)
        result = s.connect_ex((ip, port))
        if result == 0:
            open_ports.append(port)
            print(f"  [OPEN] Porta {port}")
        s.close()

    report = {
        "target": target,
        "ip": ip,
        "timestamp": str(datetime.now()),
        "open_ports": open_ports
    }
    
    with open("recon_report.json", "w") as f:
        json.dump(report, f, indent=4)
    print(f"[+] Relatório salvo em recon_report.json")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Uso: python3 {sys.argv[0]} <dominio_ou_ip>")
        sys.exit(1)
    analyze_target(sys.argv[1])
