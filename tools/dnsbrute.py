import sys
import socket
import argparse
import json
import os
from datetime import datetime

def colored_text(text, color):
    colors = {"orange": "\033[38;5,208m", "green": "\033[32m", "red": "\033[31m", "reset": "\033[0m"}
    return f"{colors.get(color, '')}{text}{colors['reset']}"

def run_dns(target, wordlist=None):
    print(colored_text("=== BENNU OS // DNS SUBDOMAIN MODULE ===", "orange"))
    print(f"[*] Alvo: {target}")
    
    subdomains = ["www", "mail", "ftp", "admin", "test", "api", "dev", "vpn", "portal", "panel"]
    if wordlist and os.path.exists(wordlist):
        with open(wordlist, "r") as f:
            subdomains = [line.strip() for line in f if line.strip()]

    results = {"target": target, "timestamp": datetime.now().isoformat(), "found": []}
    
    for sub in subdomains:
        domain = f"{sub}.{target}"
        try:
            ip = socket.gethostbyname(domain)
            print(colored_text(f"[+] Encontrado: {domain} -> {ip}", "green"))
            results["found"].append({"subdomain": domain, "ip": ip})
        except socket.gaierror:
            pass
        except Exception as e:
            print(colored_text(f"[-] Erro em {domain}: {e}", "red"))

    os.makedirs("/opt/bennu/reports", exist_ok=True)
    report_path = f"/opt/bennu/reports/dns_{target.replace('/', '_')}.json"
    with open(report_path, "w") as f:
        json.dump(results, f, indent=4)
    print(f"\n[+] Relatório de DNS salvo em: {report_path}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Bennu DNS Subdomain Enumeration")
    parser.add_argument("--target", required=True, help="Domínio alvo")
    parser.add_argument("--wordlist", help="Caminho para wordlist customizada")
    args = parser.parse_args()
    run_dns(args.target, args.wordlist)
