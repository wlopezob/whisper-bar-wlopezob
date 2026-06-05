# Fase 2: Claude Code Stop Hook — Lectura de respuestas con TTS

## ¿Qué hace esta fase?

Cuando Claude Code termina de responder, dispara un evento **Stop**. Este hook captura ese evento, extrae el último mensaje del asistente del transcript de la conversación, y lo envía al binario `whisper-tts` para que Gemini lo lea en voz alta.

```
Claude Code responde
       ↓
 Stop hook dispara (async, no bloquea)
       ↓
 whisper-tts-stop.sh lee el transcript JSONL
       ↓
 Extrae último mensaje del asistente (solo texto, sin thinking/tool_use)
       ↓
 Pasa el texto a whisper-tts via stdin
       ↓
 whisper-tts llama a Gemini TTS → WAV → afplay
```

---

## Prerequisitos

| Requisito | Verificar con |
|---|---|
| macOS con Python3 | `python3 --version` |
| Rust + cargo | `cargo --version` |
| Claude Code CLI | `claude --version` |
| Clave API de Gemini | [aistudio.google.com/apikey](https://aistudio.google.com/apikey) |

---

## Arquitectura de archivos

```
proyecto/
├── hooks/
│   └── whisper-tts-stop.sh     ← script del hook (fuente, en repo)
├── docs/
│   └── fase2-stop-hook.md      ← este archivo
└── bundle.sh                   ← instala todo automáticamente

~/.local/bin/
└── whisper-tts                 ← binario TTS (instalado por bundle.sh)

~/.claude/
├── hooks/
│   └── whisper-tts-stop.sh     ← script del hook (instalado por bundle.sh)
└── settings.json               ← configuración de Claude Code (modificado por bundle.sh)
```

---

## Instalación (opción A — automática con bundle.sh)

```bash
# 1. Clonar el proyecto
git clone <repo-url> ~/project/whisper-bar-wlopezob
cd ~/project/whisper-bar-wlopezob

# 2. Compilar e instalar todo
./bundle.sh
```

`bundle.sh` se encarga de:
- Compilar `whisper-bar-rust` y `whisper-tts` en modo release
- Instalar la app en `/Applications/whisperwlopezob.app`
- Instalar `whisper-tts` en `~/.local/bin/whisper-tts`
- Copiar `hooks/whisper-tts-stop.sh` a `~/.claude/hooks/`
- Agregar el Stop hook a `~/.claude/settings.json`

---

## Instalación (opción B — manual)

### Paso 1: Compilar el binario TTS
```bash
cd ~/project/whisper-bar-wlopezob
cargo build --release --bins
```

### Paso 2: Instalar whisper-tts
```bash
mkdir -p ~/.local/bin
cp target/release/whisper-tts ~/.local/bin/whisper-tts
chmod +x ~/.local/bin/whisper-tts
```

### Paso 3: Instalar el hook script
```bash
mkdir -p ~/.claude/hooks
cp hooks/whisper-tts-stop.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/whisper-tts-stop.sh
```

### Paso 4: Registrar el hook en Claude Code settings
Editar `~/.claude/settings.json` y agregar el bloque `Stop` dentro de `hooks`:

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/Users/TU_USUARIO/.claude/hooks/whisper-tts-stop.sh",
            "async": true
          }
        ]
      }
    ]
  }
}
```

> **`async: true`** es clave: el hook corre en background y no bloquea la respuesta de Claude Code.

### Paso 5: Configurar credenciales Gemini

Abrir la app en la barra de menú → **Configuración** → sección **LECTURA DE RESPUESTAS**:

- **Leer respuestas de Claude**: ✅ activado
- **Clave Gemini**: pegar la API key de [aistudio.google.com](https://aistudio.google.com/apikey)
- **Voz**: `Sulafat` (default) u otra voz de Gemini
- **Escena / Contexto**: personalizar el estilo de voz (opcional)

O directamente en SQLite:
```bash
sqlite3 ~/.config/whisperwlopezob/data.db \
  "INSERT OR REPLACE INTO settings VALUES
   ('tts_enabled',     'true'),
   ('tts_voice',       'Sulafat'),
   ('gemini_api_key',  'TU_API_KEY_AQUI');"
```

---

## Cómo funciona el hook internamente

### Payload de entrada (stdin del hook)
Claude Code envía este JSON al hook cuando dispara el evento Stop:
```json
{
  "session_id": "abc123",
  "transcript_path": "/Users/wlopezob/.claude/projects/-Users-.../session.jsonl",
  "cwd": "/Users/wlopezob/project/...",
  "hook_event_name": "Stop"
}
```

### Formato del transcript JSONL
Cada línea es un turno de conversación. Las líneas de interés son las del asistente:
```json
{
  "isSidechain": false,
  "message": {
    "role": "assistant",
    "content": [
      { "type": "thinking", "thinking": "..." },
      { "type": "text", "text": "El texto visible de la respuesta..." },
      { "type": "tool_use", "name": "Bash", "input": {...} }
    ]
  }
}
```

El script extrae **solo** las partes `type: "text"` del último mensaje del asistente, ignorando:
- `thinking` (razonamiento interno)
- `tool_use` (llamadas a herramientas)
- Mensajes de sub-agentes (`isSidechain: true`)

---

## Verificación

```bash
# 1. Confirmar que whisper-tts está instalado
~/.local/bin/whisper-tts --version 2>/dev/null || \
  echo "Hola, esta es una prueba manual." | ~/.local/bin/whisper-tts

# 2. Confirmar que el hook script es ejecutable
ls -la ~/.claude/hooks/whisper-tts-stop.sh

# 3. Confirmar que el hook está en settings.json
python3 -c "
import json
with open('$HOME/.claude/settings.json') as f: d = json.load(f)
stops = d.get('hooks', {}).get('Stop', [])
print('Stop hooks:', len(stops), 'registrados')
for s in stops:
    for h in s.get('hooks', []):
        print(' -', h.get('command'), '(async:', h.get('async'), ')')
"

# 4. Simular el hook manualmente con un transcript existente
TRANSCRIPT=$(ls ~/.claude/projects/-Users-wlopezob-project-*/  *.jsonl 2>/dev/null | tail -1)
echo "{\"transcript_path\": \"$TRANSCRIPT\"}" | ~/.claude/hooks/whisper-tts-stop.sh

# 5. Prueba real: usar Claude Code y verificar que la respuesta se lee
# Abrir una terminal y escribir cualquier prompt a Claude Code
```

---

## Voces disponibles de Gemini TTS

Configurar en la app → Configuración → Voz:

| Voz | Característica |
|---|---|
| `Sulafat` | Default — warm, conversational |
| `Kore` | Firm, confident |
| `Puck` | Upbeat, energetic |
| `Zephyr` | Calm, soothing |
| `Charon` | Informative, clear |
| `Leda` | Youthful, energetic |

Ver lista completa en [Google AI Studio](https://aistudio.google.com).

---

## Replicar en otra máquina

```bash
# 1. Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clonar e instalar
git clone <repo-url> ~/project/whisper-bar-wlopezob
cd ~/project/whisper-bar-wlopezob
./bundle.sh

# 3. Configurar clave Gemini (si no se sincroniza con iCloud/Dropbox)
sqlite3 ~/.config/whisperwlopezob/data.db \
  "INSERT OR REPLACE INTO settings VALUES ('gemini_api_key','TU_KEY'),('tts_enabled','true');"

# 4. Dar permisos de Accessibility a la app
# System Settings → Privacy & Security → Accessibility → agregar whisperwlopezob.app

# 5. Arrancar la app
open /Applications/whisperwlopezob.app
```

---

## Solución de problemas

| Síntoma | Causa probable | Solución |
|---|---|---|
| No hay voz después de respuestas | `tts_enabled = false` | Activar en Settings o DB |
| "sin clave Gemini, fallback a say" en log | `gemini_api_key` vacío | Configurar clave en Settings |
| Voz de macOS (`say`) en lugar de Gemini | Clave Gemini incorrecta | Verificar en [aistudio.google.com](https://aistudio.google.com/apikey) |
| Hook no dispara | No registrado en settings.json | Re-ejecutar `bundle.sh` |
| `whisper-tts: command not found` | No instalado en PATH | `./bundle.sh` o instalación manual paso 2 |

Ver log detallado:
```bash
tail -f ~/.config/whisperwlopezob/whisperwlopezob.log
```
