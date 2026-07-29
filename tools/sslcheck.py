import sys
import ssl
import socket
import argparse
import json
import os
from datetime import datetime

def colored_text(text, color):
    colors = {"orange": "\033[38;5,208m", "green": "\033[32m", "red": "\033[31m", "reset": "\033[0m"}
    return f"{colors.get(color, '')}{text}{colors['reset']}"

def run_ssl_check(target):
    print(colored_text("=== BENNU OS // SSL CHECK MODULE ===", "orange"))
    print(f"[*] Alvo: {target}:443")
    print(f"[*] Analisando certificado SSL/TLS...\n")
    
    context = ssl.create_default_context()
    results = {"target": target, "timestamp": datetime.now().isoformat(), "ssl_info": {}}
    
    try:
        with socket.create_connection((target, 443), timeout=3) as sock:
            with context.wrap_socket(sock, server_hostname=target) as ssock:
                cert = ssock.getpeercert()
                results["ssl_info"] = {
                    "subject": dict(x[0] for x in cert.get('subject', [])),
                    "issuer": dict(x[0] for x in cert.get('issuer', [])),
                    "version": cert.get('version'),
                    "notAfter": cert.get('notAfter')
                }
                print(colored_text("[+] Certificado SSL válido obtido com sucesso!", "green"))
                print(f"    |_ Emissor: {results['ssl_info']['issuer'].get('organizationName', 'Desconhecido')}")
                print(f"    |_ Expira em: {results['ssl_info']['notAfter']}")
    except Exception as e:
        print(colored_text(f"[-] Erro ao analisar SSL: {e}", "red"))
        results["error"] = str(e)

    os.makedirs("/opt/bennu/reports", exist_ok=True)
    report_path = f"/opt/bennu/reports/ssl_{target.replace('/', '_')}.json"
    with open(report_path, "w") as f:
        json.dump(results, f, indent=4)
    print(f"\n[+] Relatório SSL salvo em: {report_path}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Bennu SSL Checker")
    parser.add_argument("--target", required=True, help="Domínio alvo")
    args = parser.parse_args()
    run_ssl_check(args.target)
