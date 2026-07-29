import argparse
import json
import os
import urllib.request
from datetime import datetime

def colored_text(text, color):
    colors = {"orange": "\033[38;5,208m", "green": "\033[32m", "red": "\033[31m", "reset": "\033[0m"}
    return f"{colors.get(color, '')}{text}{colors['reset']}"

def run_fuzz(target):
    if not target.startswith("http"):
        target = f"http://{target}"

    print(colored_text("=== BENNU OS // WEB FUZZING MODULE ===", "orange"))
    print(f"[*] Alvo: {target}")

    paths = ["admin", "login", "config.json", "dashboard", "api", "robots.txt", "backup.zip", "test.php"]
    discovered = []

    for path in paths:
        url = f"{target.rstrip('/')}/{path}"
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'BennuOS/0.1'})
            with urllib.request.urlopen(req, timeout=3) as response:
                if response.status == 200:
                    print(colored_text(f"[+] Encontrado: {url} (Status: 200)", "green"))
                    discovered.append(url)
        except Exception:
            pass

    os.makedirs("/opt/bennu/reports", exist_ok=True)
    report_name = f"/opt/bennu/reports/fuzz_{target.replace('http://', '').replace('https://', '').replace('/', '_')}.json"
    
    results = {
        "target": target,
        "timestamp": datetime.now().isoformat(),
        "discovered": discovered
    }
    
    with open(report_name, "w") as f:
        json.dump(results, f, indent=4)
        
    print(colored_text(f"\n[+] Relatório de Fuzzing salvo em: {report_name}", "green"))

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Bennu OS Web Fuzzing")
    parser.add_argument("--target", required=True, help="URL ou IP alvo")
    args = parser.parse_args()
    run_fuzz(args.target)
