import sys
import socket
import argparse
import json
import os
from datetime import datetime

def colored_text(text, color):
    colors = {"orange": "\033[38;5,208m", "green": "\033[32m", "red": "\033[31m", "reset": "\033[0m"}
    return f"{colors.get(color, '')}{text}{colors['reset']}"

def run_recon(target, ports=[80, 443, 22, 21, 8080]):
    print(colored_text("=== BENNU OS // RECON MODULE ===", "orange"))
    print(f"[*] Alvo: {target}")
    print(f"[*] Iniciando varredura de portas e banner grabbing...\n")
    
    results = {"target": target, "timestamp": datetime.now().isoformat(), "open_ports": []}
    
    for port in ports:
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(1.5)
            result = sock.connect_ex((target, port))
            if result == 0:
                print(colored_text(f"[+] Porta {port} Aberta", "green"))
                banner_data = ""
                try:
                    sock.sendall(b"HEAD / HTTP/1.0\r\n\r\n")
                    banner_data = sock.recv(1024).decode('utf-8', errors='ignore').split('\n')[0]
                    print(f"    |_ Banner: {banner_data.strip()}")
                except:
                    banner_data = "Indisponível/Não HTTP"
                
                results["open_ports"].append({"port": port, "banner": banner_data})
            sock.close()
        except socket.gaierror:
            print(colored_text("[-] Erro: Alvo não resolvido ou inválido.", "red"))
            sys.exit(1)
        except socket.timeout:
            print(colored_text(f"[-] Timeout ao tentar conectar na porta {port}.", "red"))
        except Exception as e:
            print(colored_text(f"[-] Erro inesperado: {e}", "red"))

    os.makedirs("/opt/bennu/reports", exist_ok=True)
    report_path = f"/opt/bennu/reports/recon_{target.replace('/', '_')}.json"
    with open(report_path, "w") as f:
        json.dump(results, f, indent=4)
    print(f"\n[+] Relatório salvo em: {report_path}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Bennu Recon Tool")
    parser.add_argument("--target", required=True, help="IP ou domínio alvo")
    args = parser.parse_args()
    run_recon(args.target)
