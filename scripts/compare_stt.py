#!/usr/bin/env python3
"""
Compara MAI-Transcribe-2 (Azure) contra gemini-3.5-transcribe sobre los mismos WAV.

Lee las credenciales de la misma SQLite que usa la app
(~/.config/whisperwlopezob/data.db), así que no hay que pasar keys.

Uso:
  # el último dictado de la app (⌘⌥W deja el WAV en /tmp)
  python3 scripts/compare_stt.py

  # varios ficheros
  python3 scripts/compare_stt.py /tmp/stt-samples/*.wav

  # sesgar hacia tu jerga técnica en ambos motores
  python3 scripts/compare_stt.py --vocab rusqlite,cpal,whisper-cli,Claude Code muestras/*.wav

  # estilo verbatim (con muletillas) en vez de clean
  python3 scripts/compare_stt.py --style verbatim
"""

import argparse
import base64
import difflib
import json
import os
import sqlite3
import sys
import time
import urllib.error
import urllib.request
import uuid
import wave

DB_PATH = os.path.expanduser("~/.config/whisperwlopezob/data.db")
DEFAULT_WAV = "/tmp/whisperbar_recording.wav"

AZURE_API_VERSION = "2025-10-15"
AZURE_MODEL = "MAI-Transcribe-2"
GEMINI_MODEL = "gemini-3.5-transcribe"
GEMINI_URL = "https://generativelanguage.googleapis.com/v1beta/interactions"

# $/hora de audio. Azure: promo hasta 31-dic-2026. Gemini: $0.003/min de input.
PRICE_PER_HOUR = {"azure": 0.10, "gemini": 0.18}

# $/1M tokens para Gemini, usado cuando la respuesta trae `usage` real.
GEMINI_PER_M = {"audio_in": 2.00, "text_out": 12.00}

TIMEOUT = 120

# ── Utilidades ────────────────────────────────────────────────────────────────


def db_get(key, default=""):
    """Lee una clave de la tabla settings; igual que db.rs::get."""
    try:
        with sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True) as conn:
            row = conn.execute(
                "SELECT value FROM settings WHERE key = ?", (key,)
            ).fetchone()
            return row[0] if row else default
    except sqlite3.Error:
        return default


def wav_duration(path):
    """Duración en segundos leyendo la cabecera WAV."""
    try:
        with wave.open(path, "rb") as w:
            return w.getnframes() / float(w.getframerate())
    except (wave.Error, EOFError):
        return 0.0


def cost(engine, seconds):
    return seconds / 3600.0 * PRICE_PER_HOUR[engine]


def encode_multipart(fields, files):
    """multipart/form-data sin dependencias externas."""
    boundary = "----whisperbar" + uuid.uuid4().hex
    body = bytearray()
    for name, value in fields.items():
        body += f"--{boundary}\r\n".encode()
        body += f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode()
        body += value.encode() + b"\r\n"
    for name, (filename, content, ctype) in files.items():
        body += f"--{boundary}\r\n".encode()
        body += (
            f'Content-Disposition: form-data; name="{name}"; '
            f'filename="{filename}"\r\n'
        ).encode()
        body += f"Content-Type: {ctype}\r\n\r\n".encode()
        body += content + b"\r\n"
    body += f"--{boundary}--\r\n".encode()
    return bytes(body), f"multipart/form-data; boundary={boundary}"


def post(url, data, headers):
    req = urllib.request.Request(url, data=data, headers=headers, method="POST")
    with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
        return json.loads(resp.read().decode())


# ── Motores ───────────────────────────────────────────────────────────────────


def transcribe_azure(path, key, region, style, vocab):
    """MAI-Transcribe-2 vía Fast Transcription API (mismo endpoint que azure_transcriber.rs)."""
    definition = {
        "enhancedMode": {
            "enabled": True,
            "model": AZURE_MODEL,
            "modelOptions": {"transcribeStyle": style},
        }
    }
    if vocab:
        definition["phraseList"] = {"phrases": vocab}

    with open(path, "rb") as f:
        audio = f.read()

    body, content_type = encode_multipart(
        {"definition": json.dumps(definition)},
        {"audio": (os.path.basename(path), audio, "audio/wav")},
    )

    url = (
        f"https://{region}.api.cognitive.microsoft.com"
        f"/speechtotext/transcriptions:transcribe?api-version={AZURE_API_VERSION}"
    )
    js = post(
        url,
        body,
        {"Ocp-Apim-Subscription-Key": key, "Content-Type": content_type},
    )

    phrases = js.get("combinedPhrases") or []
    return (phrases[0].get("text", "") if phrases else "").strip(), None


def transcribe_gemini(path, key, style, vocab):
    """gemini-3.5-transcribe vía Interactions API, audio inline en base64."""
    with open(path, "rb") as f:
        audio_b64 = base64.b64encode(f.read()).decode()

    # Azure llama "clean" a lo que Gemini llama "smart".
    tconfig = {"mode": {"type": "smart" if style == "clean" else "verbatim"}}
    if vocab:
        tconfig["custom_vocabulary"] = vocab

    body = json.dumps(
        {
            "model": GEMINI_MODEL,
            "input": [
                {"type": "audio", "data": audio_b64, "mime_type": "audio/wav"}
            ],
            "generation_config": {"transcription_config": tconfig},
        }
    ).encode()

    js = post(
        GEMINI_URL,
        body,
        {"x-goog-api-key": key, "Content-Type": "application/json"},
    )

    # La API devuelve el transcript dentro de steps[].content[], no en un
    # campo plano. `output_text` se acepta como fallback por si vuelve.
    text = js.get("output_text") or ""
    if not text:
        chunks = [
            c.get("text", "")
            for step in js.get("steps", [])
            for c in step.get("content", [])
            if c.get("type") == "text"
        ]
        text = " ".join(t for t in chunks if t)

    return text.strip(), gemini_cost(js.get("usage") or {})


def gemini_cost(usage):
    """Coste desde los tokens facturados. None si la respuesta no trae usage."""
    if not usage:
        return None
    audio = sum(
        m.get("tokens", 0)
        for m in usage.get("input_tokens_by_modality", [])
        if m.get("modality") == "audio"
    )
    text_out = usage.get("total_output_tokens", 0)
    if not text_out:  # algunas respuestas solo lo reportan en el desglose
        text_out = sum(
            d.get("tokens", 0)
            for inv in usage.get("model_invocation_token_counts", [])
            for d in inv.get("candidates_tokens_details", [])
            if d.get("modality") == "text"
        )
    return (audio * GEMINI_PER_M["audio_in"] + text_out * GEMINI_PER_M["text_out"]) / 1e6


def run(fn, engine, seconds):
    """Ejecuta un motor midiendo tiempo. Nunca lanza: devuelve el error como texto."""
    start = time.monotonic()
    billed = None
    try:
        text, billed = fn()
        err = None if text else "respuesta sin transcripción"
    except urllib.error.HTTPError as e:
        detail = e.read().decode(errors="replace")[:300]
        text, err = None, f"HTTP {e.code}: {detail}"
    except Exception as e:  # red, timeout, JSON inesperado
        text, err = None, f"{type(e).__name__}: {e}"
    return {
        "engine": engine,
        "text": text,
        "error": err,
        "elapsed": time.monotonic() - start,
        "cost": billed if billed is not None else cost(engine, seconds),
    }


# ── Presentación ──────────────────────────────────────────────────────────────


def normalize(text):
    """Palabras en minúscula sin puntuación de borde, para comparar sin ruido."""
    return [w.strip(".,;:¿?¡!\"'()[]…").lower() for w in text.split()]


def word_diff(a, b):
    """Tramos donde difieren. Devuelve [(texto_a, texto_b), ...]."""
    wa, wb = normalize(a), normalize(b)
    out = []
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, wa, wb).get_opcodes():
        if tag == "equal":
            continue
        out.append((" ".join(wa[i1:i2]) or "∅", " ".join(wb[j1:j2]) or "∅"))
    return out


def report(path, seconds, results):
    name = os.path.basename(path)
    print(f"\n{'─' * 72}")
    print(f" {name}  ·  {seconds:.1f}s")
    print("─" * 72)

    for r in results:
        label = AZURE_MODEL if r["engine"] == "azure" else GEMINI_MODEL
        if r["error"]:
            print(f"\n  {label:<22} {r['elapsed']:>5.2f}s   ✗ {r['error']}")
        else:
            print(f"\n  {label:<22} {r['elapsed']:>5.2f}s   ${r['cost']:.6f}")
            print(f"    {r['text'] or '(vacío)'}")

    ok = [r for r in results if r["text"]]
    if len(ok) == 2:
        a, b = ok[0]["text"], ok[1]["text"]
        if normalize(a) == normalize(b):
            print("\n  ✓ transcripciones idénticas (ignorando puntuación)")
        else:
            diffs = word_diff(a, b)
            print(f"\n  ⚠ difieren en {len(diffs)} tramo(s)  [azure → gemini]")
            for da, db in diffs[:12]:
                print(f"      {da!r} → {db!r}")
            if len(diffs) > 12:
                print(f"      … y {len(diffs) - 12} más")


def summary(rows):
    print(f"\n{'═' * 72}")
    print(" RESUMEN")
    print("═" * 72)
    print(f"  {'motor':<22} {'ok':>4} {'fallos':>7} {'media':>8} {'coste total':>13}")
    for engine in ("azure", "gemini"):
        rs = [r for r in rows if r["engine"] == engine]
        if not rs:
            continue
        ok = [r for r in rs if not r["error"]]
        label = AZURE_MODEL if engine == "azure" else GEMINI_MODEL
        avg = sum(r["elapsed"] for r in ok) / len(ok) if ok else 0.0
        total = sum(r["cost"] for r in ok)
        print(
            f"  {label:<22} {len(ok):>4} {len(rs) - len(ok):>7} "
            f"{avg:>7.2f}s {total:>12.6f}$"
        )
    print("\n  Nota: la latencia incluye el round-trip de red, no solo el modelo.")
    print("  Gemini se cobra con los tokens reales que devuelve la API; Azure se")
    print("  estima por duración a $%.2f/h (tarifa promocional hasta 31-dic-2026)."
          % PRICE_PER_HOUR["azure"])


# ── main ──────────────────────────────────────────────────────────────────────


def main():
    ap = argparse.ArgumentParser(
        description="Compara MAI-Transcribe-2 vs gemini-3.5-transcribe sobre los mismos WAV."
    )
    ap.add_argument("wavs", nargs="*", default=[DEFAULT_WAV],
                    help=f"ficheros WAV (por defecto: {DEFAULT_WAV})")
    ap.add_argument("--style", choices=("clean", "verbatim"), default="clean",
                    help="clean quita muletillas; verbatim las conserva (default: clean)")
    ap.add_argument("--vocab", default="",
                    help="términos separados por coma para sesgar el reconocimiento")
    ap.add_argument("--only", choices=("azure", "gemini"),
                    help="ejecutar un solo motor")
    args = ap.parse_args()

    vocab = [v.strip() for v in args.vocab.split(",") if v.strip()]

    azure_key = db_get("azure_mai_key")
    azure_region = db_get("azure_mai_region")
    gemini_key = db_get("gemini_api_key")

    use_azure = args.only != "gemini" and bool(azure_key and azure_region)
    use_gemini = args.only != "azure" and bool(gemini_key)

    if args.only != "gemini" and not use_azure:
        print("⚠ Azure omitido: falta azure_mai_key o azure_mai_region en la DB", file=sys.stderr)
    if args.only != "azure" and not use_gemini:
        print("⚠ Gemini omitido: falta gemini_api_key en la DB", file=sys.stderr)
    if not (use_azure or use_gemini):
        print("Sin credenciales utilizables. Configúralas en la app.", file=sys.stderr)
        return 1

    wavs = [w for w in args.wavs if os.path.isfile(w)]
    for missing in set(args.wavs) - set(wavs):
        print(f"⚠ no existe: {missing}", file=sys.stderr)
    if not wavs:
        print("No hay WAVs que procesar.", file=sys.stderr)
        return 1

    if vocab:
        print(f"Vocabulario sesgado: {', '.join(vocab)}")
    print(f"Estilo: {args.style}  ·  {len(wavs)} fichero(s)")

    all_results = []
    for path in wavs:
        seconds = wav_duration(path)
        results = []
        # Secuencial a propósito: en paralelo las dos subidas compiten por el
        # uplink y los tiempos dejan de ser comparables.
        if use_azure:
            results.append(run(
                lambda: transcribe_azure(path, azure_key, azure_region, args.style, vocab),
                "azure", seconds))
        if use_gemini:
            results.append(run(
                lambda: transcribe_gemini(path, gemini_key, args.style, vocab),
                "gemini", seconds))
        report(path, seconds, results)
        all_results.extend(results)

    if len(wavs) > 1 or len(all_results) > 1:
        summary(all_results)
    return 0


if __name__ == "__main__":
    sys.exit(main())
