#!/usr/bin/env python3
import sys
import socket
import json
from datetime import datetime

def enumerate_subdomains(domain, wordlist_path):
    print(f"[*] Iniciando enumeração de subdomínios para: {domain}")
    
    try:
        with open(wordlist_path, "r") as f:
            subdomains = [line.strip() for line in f if line.strip()]
    except FileNotFoundError:
        print(f"[x] Erro: Wordlist não encontrada em {wordlist_path}")
        sys.exit(1)

    found = []

    for sub in subdomains:
        target = f"{sub}.{domain}"
        try:
            ip = socket.gethostbyname(target)
            print(f"  [FOUND] {target} -> {ip}")
            found.append({"subdomain": target, "ip": ip})
        except socket.gaierror:
            pass

    report = {
        "domain": domain,
        "timestamp": str(datetime.now()),
        "results": found
    }

    with open("sub_enum_report.json", "w") as f:
        json.dump(report, f, indent=4)
    print(f"[+] Varredura concluída. Relatório salvo em sub_enum_report.json")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(f"Uso: python3 {sys.argv[0]} <dominio> <caminho_wordlist>")
        sys.exit(1)
    enumerate_subdomains(sys.argv[1], sys.argv[2])
