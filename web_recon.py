#!/usr/bin/env python3
import sys
import urllib.request
import json
from datetime import datetime

def check_web(target):
    if not target.startswith("http://") and not target.startswith("https://"):
        target = "http://" + target

    print(f"[*] Analisando headers e status de: {target}")
    
    req = urllib.request.Request(
        target,
        headers={"User-Agent": "BennuOSINT/1.0"}
    )

    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            status = response.getcode()
            headers = dict(response.info())
            print(f"[+] Status Code: {status}")
            print("[+] Headers principais:")
            for k, v in headers.items():
                print(f"    {k}: {v}")
                
            report = {
                "target": target,
                "status_code": status,
                "headers": headers,
                "timestamp": str(datetime.now())
            }
            
            with open("web_recon_report.json", "w") as f:
                json.dump(report, f, indent=4)
            print("[+] Relatório salvo em web_recon_report.json")

    except Exception as e:
        print(f"[x] Erro ao conectar no alvo: {e}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Uso: python3 {sys.argv[0]} <url_ou_dominio>")
        sys.exit(1)
    check_web(sys.argv[1])
