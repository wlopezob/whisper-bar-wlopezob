#!/usr/bin/env python3
"""
Prueba de traducción con Ollama (gemma4:e4b).
Valida detección de idioma + traducción estructurada antes de integrar en Rust.

Uso:
  python3 scripts/test_ollama_translate.py
  python3 scripts/test_ollama_translate.py "tu texto aquí" en
"""

import json
import sys
import urllib.request

OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "gemma4:e4b"

PROMPT_TEMPLATE = """\
Eres un traductor. Analiza el texto y devuelve ÚNICAMENTE un JSON con este formato exacto:
{{"text": "...", "detected_lang": "es"}}

Reglas:
- detected_lang: idioma del texto original ("es" o "en")
- text: si detected_lang == "{dest_lang}", devuelve el texto original sin cambios
- text: si detected_lang != "{dest_lang}", devuelve la traducción a {dest_lang}
- Nada más que el JSON. Sin explicaciones, sin markdown, sin backticks.

Texto: "{input_text}"
"""

TEST_CASES = [
    ("Hola, ¿cómo estás hoy?",              "en"),
    ("The weather is great today.",          "es"),
    ("Necesito revisar el código de Rust.",  "en"),
    ("I need to fix this bug.",              "es"),
    ("Buenos días a todos.",                 "es"),  # mismo idioma, no debe traducir
    ("Good morning everyone.",              "en"),  # mismo idioma, no debe traducir
]


def translate(text: str, dest_lang: str) -> dict:
    prompt = PROMPT_TEMPLATE.format(dest_lang=dest_lang, input_text=text)
    payload = json.dumps({
        "model": MODEL,
        "prompt": prompt,
        "stream": False,
        "options": {"temperature": 0.1}
    }).encode()

    req = urllib.request.Request(
        OLLAMA_URL,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read())

    raw = data.get("response", "").strip()

    # Limpiar si el modelo envuelve en markdown
    if raw.startswith("```"):
        raw = raw.split("```")[1]
        if raw.startswith("json"):
            raw = raw[4:]
    raw = raw.strip()

    return json.loads(raw), data.get("response", "")


def main():
    # Modo interactivo: argumento directo
    if len(sys.argv) >= 2:
        text = sys.argv[1]
        dest = sys.argv[2] if len(sys.argv) >= 3 else "en"
        print(f"\nTexto:   {text}")
        print(f"Destino: {dest}")
        try:
            result, raw = translate(text, dest)
            print(f"Raw:     {raw}")
            print(f"Parsed:  {result}")
            was_translated = result.get("detected_lang") != dest
            print(f"Traducido: {was_translated}")
        except Exception as e:
            print(f"ERROR: {e}")
        return

    # Batería de pruebas
    print(f"Modelo: {MODEL}\n")
    print(f"{'#':<3} {'Texto original':<45} {'→'} {'dest':<4} {'detected':<10} {'Traducido':<5}  Resultado")
    print("-" * 120)

    passed = 0
    for i, (text, dest) in enumerate(TEST_CASES, 1):
        try:
            result, _ = translate(text, dest)
            detected = result.get("detected_lang", "?")
            translated_text = result.get("text", "?")
            was_translated = detected != dest
            flag = "✓" if True else "✗"  # siempre mostramos
            print(f"{i:<3} {text:<45} → {dest:<4} {detected:<10} {'sí' if was_translated else 'no':<5}  {translated_text}")
            passed += 1
        except Exception as e:
            print(f"{i:<3} {text:<45} → {dest:<4} ERROR: {e}")

    print(f"\n{passed}/{len(TEST_CASES)} completados sin error")


if __name__ == "__main__":
    main()
