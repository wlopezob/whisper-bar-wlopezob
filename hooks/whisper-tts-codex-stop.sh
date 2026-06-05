#!/usr/bin/env bash
# hooks/whisper-tts-codex-stop.sh
# Codex CLI Stop hook: lee last_assistant_message del payload y lo pasa a
# whisper-tts como JSON {"user": "...", "assistant": "..."}.
#
# Diferencias vs Claude Code:
#   - last_assistant_message viene en el payload (no hay race condition)
#   - el hook es SÍNCRONO → whisper-tts corre en background con &
#   - Codex requiere JSON en stdout ({} mínimo)

set -uo pipefail

WHISPER_TTS="${HOME}/.local/bin/whisper-tts"

if [ ! -x "$WHISPER_TTS" ]; then
    echo '{}'
    exit 0
fi

PAYLOAD=$(cat | python3 -c '
import sys, json, os

hook = json.load(sys.stdin)

assistant = (hook.get("last_assistant_message") or "").strip()
if not assistant:
    sys.exit(0)

# Extraer último mensaje del usuario desde el transcript (para contexto del formatter)
user = ""
transcript = hook.get("transcript_path", "")
if transcript and os.path.isfile(transcript):
    def extract_text(msg):
        content = msg.get("content", "")
        if isinstance(content, str):
            s = content.strip()
            return "" if (not s or s.startswith("<")) else s
        if isinstance(content, list):
            parts = [p.get("text","").strip() for p in content
                     if isinstance(p, dict) and p.get("type") == "text"]
            return "\n".join(p for p in parts if p)
        return ""

    entries = []
    with open(transcript, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                e = json.loads(line)
                if not e.get("isSidechain", False):
                    entries.append(e)
            except json.JSONDecodeError:
                continue

    for entry in reversed(entries):
        if entry.get("type") == "user":
            msg = entry.get("message", {})
            if isinstance(msg, dict):
                text = extract_text(msg)
                if text:
                    user = text
                    break

print(json.dumps({"user": user, "assistant": assistant}))
' 2>/dev/null) || PAYLOAD=""

if [ -n "$PAYLOAD" ]; then
    printf '%s' "$PAYLOAD" | "$WHISPER_TTS" &
fi

# Codex requiere respuesta JSON en stdout
echo '{}'
exit 0
