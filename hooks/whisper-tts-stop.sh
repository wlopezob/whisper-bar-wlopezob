#!/usr/bin/env bash
# hooks/whisper-tts-stop.sh
# Claude Code Stop hook: extrae el último par usuario/asistente del transcript
# y lo envía a whisper-tts como JSON {"user": "...", "assistant": "..."}.
#
# El hook se lanza async; espera un momento para que el transcript termine
# de escribirse antes de leerlo (evita race condition con la escritura async).
# Usa iteración reversa para encontrar siempre el par más reciente.

set -uo pipefail  # sin -e para no abortar si python3 falla

WHISPER_TTS="${HOME}/.local/bin/whisper-tts"

if [ ! -x "$WHISPER_TTS" ]; then
    exit 0
fi

# Esperar a que el transcript termine de escribirse (race condition)
sleep 0.8

PAYLOAD=$(python3 -c '
import sys, json, os

def extract_text_parts(msg):
    content = msg.get("content", "")
    if isinstance(content, str):
        stripped = content.strip()
        if not stripped or stripped.startswith("<"):
            return ""
        return stripped
    if isinstance(content, list):
        parts = []
        for p in content:
            if not isinstance(p, dict):
                continue
            if p.get("type") == "text":
                t = p.get("text", "").strip()
                if t:
                    parts.append(t)
        return "\n".join(parts)
    return ""

try:
    hook_data = json.load(sys.stdin)
    transcript = hook_data.get("transcript_path", "")
    if not transcript or not os.path.isfile(transcript):
        sys.exit(0)

    entries = []
    with open(transcript, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not entry.get("isSidechain", False):
                entries.append(entry)

    # Iterar en reversa: encontrar el último assistant con texto,
    # luego el último user con texto real ANTES de ese assistant.
    last_assistant = ""
    last_user = ""

    for entry in reversed(entries):
        entry_type = entry.get("type", "")
        msg = entry.get("message", {})
        if not isinstance(msg, dict):
            continue

        if entry_type == "assistant" and not last_assistant:
            text = extract_text_parts(msg)
            if text:
                last_assistant = text

        elif entry_type == "user" and last_assistant and not last_user:
            text = extract_text_parts(msg)
            if text:
                last_user = text
                break  # par completo encontrado

    if last_assistant:
        print(json.dumps({"user": last_user, "assistant": last_assistant}))

except Exception as e:
    sys.stderr.write(f"whisper-tts-stop error: {e}\n")
    sys.exit(0)
' 2>/dev/null) || true

if [ -n "$PAYLOAD" ]; then
    printf '%s' "$PAYLOAD" | "$WHISPER_TTS"
fi

exit 0
