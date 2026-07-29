#!/usr/bin/env python3
import os
import sys
import urllib.request
import json

def ask_bennu(prompt_text):
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("[x] Erro: Variável de ambiente ANTHROPIC_API_KEY não definida.")
        print("Defina com: export ANTHROPIC_API_KEY='sua-chave'")
        sys.exit(1)

    url = "https://api.anthropic.com/v1/messages"
    headers = {
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json"
    }

    payload = {
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 1024,
        "system": "Você é o Bennu AI, um assistente técnico especializado em segurança ofensiva legítima, OSINT e análise de infraestrutura rodando em um ambiente mobile Debian. Seja direto, técnico e use o contexto de pentest autorizado.",
        "messages": [
            {"role": "user", "content": prompt_text}
        ]
    }

    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode('utf-8'),
        headers=headers,
        method='POST'
    )

    print("[*] Consultando o copiloto Bennu...")
    try:
        with urllib.request.urlopen(req) as response:
            res_data = json.loads(response.read().decode('utf-8'))
            content = res_data['content'][0]['text']
            print("\n--- BENNU AI COPILOT ---")
            print(content)
            print("------------------------\n")
    except Exception as e:
        print(f"[x] Erro na requisição à API: {e}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Uso: bennu-ai \"sua dúvida ou comando para análise\"")
        sys.exit(1)
    ask_bennu(" ".join(sys.argv[1:]))
