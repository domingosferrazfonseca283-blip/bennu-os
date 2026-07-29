import sys
import os
import json
import glob

def colored_text(text, color):
    colors = {"orange": "\033[38;5,208m", "green": "\033[32m", "red": "\033[31m", "reset": "\033[0m"}
    return f"{colors.get(color, '')}{text}{colors['reset']}"

def show_last_report():
    print(colored_text("=== BENNU OS // REPORT MANAGER ===", "orange"))
    reports_dir = "/opt/bennu/reports"
    if not os.path.exists(reports_dir) or not os.listdir(reports_dir):
        print(colored_text("[-] Nenhum relatório encontrado na base.", "red"))
        return

    list_of_files = glob.glob(f"{reports_dir}/*")
    latest_file = max(list_of_files, key=os.path.getmtime)
    
    print(f"[*] Último relatório gerado: {latest_file}\n")
    with open(latest_file, "r") as f:
        data = json.load(f)
        print(json.dumps(data, indent=4))

if __name__ == "__main__":
    show_last_report()
