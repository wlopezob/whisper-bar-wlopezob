# whisperwlopezob

App de barra de menú para macOS que graba tu voz, transcribe con `whisper-cli` (o Azure MAI) y pega el texto donde está el cursor. Lee en voz alta las respuestas de Claude Code / Codex usando Gemini TTS.

**Hotkeys:**
- `⌘⌥W` — mantén para grabar, suelta para transcribir y pegar
- `⌘⌥R` — reproduce la última respuesta TTS; si ya está reproduciéndose, la para

---

## Requisitos

- macOS 13 o superior (Apple Silicon o Intel)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/)
- [Homebrew](https://brew.sh)
- Rust (instalado vía `rustup`)

---

## Estructura de datos

```
~/.config/whisperwlopezob/
├── whisperwlopezob.log     # log de la app
├── data.db                 # configuración persistente (SQLite)
├── whisper-models/         # modelos de Whisper (.bin)
│   └── ggml-large-v3.bin
├── llm/                    # modelos LLM para corrección y traducción (.gguf)
│   └── qwen2.5-1.5b-instruct-q4_k_m.gguf
└── audio/
    └── last-tts.wav        # última respuesta TTS (reproducir con ⌘⌥R)
```

---

## Instalación paso a paso

### 1. Instalar Xcode Command Line Tools

```bash
xcode-select --install
```

### 2. Instalar Homebrew (si no lo tienes)

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

### 3. Instalar Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 4. Instalar whisper-cpp

```bash
brew install whisper-cpp
```

### 5. Descargar un modelo de Whisper

```bash
mkdir -p ~/.config/whisperwlopezob/whisper-models

# Recomendado para Apple Silicon
curl -L -o ~/.config/whisperwlopezob/whisper-models/ggml-small.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin
```

> La app detecta automáticamente el mejor modelo disponible: `large-v3 → large-v2 → medium → small → base → tiny`

### 6. (Opcional) Corrección gramatical y traducción con LLM local

```bash
brew install llama.cpp
mkdir -p ~/.config/whisperwlopezob/llm

curl -L -o ~/.config/whisperwlopezob/llm/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf
```

### 7. (Opcional) Lectura de respuestas con Gemini TTS

Requiere una API key de Google Gemini. Configurable desde **Configuración → Lectura de respuestas**.

Para que las respuestas de Claude Code / Codex se lean automáticamente al terminar cada respuesta, instala el hook:

```bash
bash bundle.sh   # instala también el hook de Claude Code y Codex
```

### 8. Compilar e instalar como .app

```bash
git clone <url-del-repositorio>
cd whisper-bar-wlopezob
bash bundle.sh
```

---

## Permisos en macOS

### Accesibilidad (obligatorio)

Requerido para registrar `⌘⌥W` / `⌘⌥R` y simular `⌘V` para pegar.

1. Ejecuta la app: `open /Applications/whisperwlopezob.app`
2. Ve a **System Settings → Privacy & Security → Accessibility**
3. Activa el toggle de **whisperwlopezob**
4. Reinicia: `pkill whisperwlopezob; open /Applications/whisperwlopezob.app`

> **Importante:** cada rebuild revoca el permiso. Desactiva y reactiva el toggle tras cada `bash bundle.sh`.

### Micrófono

La app solicita el permiso automáticamente al arrancar. Si fue denegado:

**System Settings → Privacy & Security → Microphone** → activa **whisperwlopezob**

---

## Ejecución

```bash
bash bundle.sh                                          # compilar e instalar
open /Applications/whisperwlopezob.app                 # lanzar
pkill whisperwlopezob; open /Applications/whisperwlopezob.app  # relanzar
tail -f ~/.config/whisperwlopezob/whisperwlopezob.log  # ver log
```

---

## Uso

| Acción | Resultado |
|--------|-----------|
| Mantener `⌘⌥W` | Inicia grabación — icono cambia a `🔴` |
| Soltar `⌘⌥W` | Detiene grabación — icono `⏳` — transcribe y pega |
| Presionar `⌘⌥R` (sin audio) | Reproduce la última respuesta TTS |
| Presionar `⌘⌥R` (con audio) | Para la reproducción en curso |

El clipboard original se restaura automáticamente 300ms después de pegar.

### Menú de estado

```
whisperwlopezob
─────────────────────────────────────────────────────────────
Mantén ⌘⌥W para grabar / suelta para transcribir  ·  ⌘⌥R reproducir o parar última respuesta
─────────────────────────────────────────────────────────────
✅ whisper-cli: /opt/homebrew/bin/whisper-cli
✅ Modelo: ggml-large-v3.bin
─────────────────────────────────────────────────────────────
Idioma
  ✓ Español
    English
─────────────────────────────────────────────────────────────
Configuración...
─────────────────────────────────────────────────────────────
Ver log
Salir
```

---

## Ventana de configuración

Panel nativo macOS con scroll vertical que agrupa todas las opciones:

### Lectura de respuestas (TTS)

| Campo | Descripción |
|-------|-------------|
| Activar lectura | Habilita la síntesis de voz con Gemini TTS |
| Gemini API Key | Clave de Google Gemini |
| Voz | Nombre de voz Gemini (default: `Sulafat`) |
| Scene | Descripción del estilo de narración |
| Sample Context | Contexto de la conversación para el director's note |

**Formatear respuesta para voz** — si está activo, antes de sintetizar la respuesta pasa por `gemini-3.1-flash-lite` que la convierte a prosa natural (elimina código, markdown, listas). El prompt es configurable.

### Transcripción

- **Backend**: local (whisper-cli) o Azure MAI Transcribe
- Para Azure MAI: API Key, Region, API Version, Definition JSON

### Mejora gramatical

- Corrección automática con LLM local tras transcribir
- Prompt configurable por idioma (español / inglés)

### Traducción

- Traduce el texto transcrito al idioma destino configurado

---

## Lectura automática de respuestas (Stop Hook)

Al terminar cada respuesta, Claude Code / Codex ejecuta automáticamente `whisper-tts` que:

1. Extrae el último par usuario/asistente del transcript
2. (Opcional) Formatea la respuesta con `gemini-3.1-flash-lite` para que suene natural
3. Sintetiza con Gemini TTS (`gemini-3.1-flash-tts-preview`)
4. Reproduce el audio con `afplay`
5. Guarda el audio en `~/.config/whisperwlopezob/audio/last-tts.wav`

### Hook para Claude Code

Instalado automáticamente por `bundle.sh` en `~/.claude/hooks/whisper-tts-stop.sh`.

Config en `~/.claude/settings.json`:
```json
{
  "hooks": {
    "Stop": [{ "hooks": [{ "type": "command", "command": "~/.claude/hooks/whisper-tts-stop.sh", "async": true }] }]
  }
}
```

### Hook para Codex Desktop

Instalado automáticamente por `bundle.sh` en `~/.codex/hooks/whisper-tts-stop.sh`.

Config en `~/.codex/hooks.json`:
```json
{
  "hooks": {
    "Stop": [{ "hooks": [{ "type": "command", "command": "/Users/<user>/.codex/hooks/whisper-tts-stop.sh", "timeout": 30 }] }]
  }
}
```

### Prueba manual del hook

```bash
echo '{"user": "raíz cuadrada de 144", "assistant": "La raíz cuadrada de 144 es 12."}' \
  | ~/.local/bin/whisper-tts

tail -20 ~/.config/whisperwlopezob/whisperwlopezob.log
```

---

## Arquitectura

```
src/
├── main.rs              — Event loop (winit), tray icon, coordinador principal
├── lib.rs               — Declaración de módulos
├── config.rs            — Auto-detección de whisper-cli, CLI LLM y modelos
├── defaults.rs          — Constantes y valores por defecto
├── hotkey.rs            — Hotkeys globales: ⌘⌥W (grabar) y ⌘⌥R (repetir TTS)
├── recorder.rs          — Grabación de audio (cpal + hound, 16kHz mono PCM)
├── transcriber.rs       — Invocación de whisper-cli con timeout 60s
├── azure_transcriber.rs — Backend Azure MAI Transcribe
├── llm.rs               — Corrección gramatical y traducción con CLI LLM
├── formatter.rs         — Formateador Gemini Flash Lite para TTS
├── tts/
│   ├── mod.rs           — Orquestador TTS, PID management, replay path
│   ├── gemini.rs        — Síntesis con gemini-3.1-flash-tts-preview (PCM→WAV)
│   ├── say.rs           — Fallback: say -v Paulina
│   └── provider.rs      — Trait TtsProvider + structs AudioData, TtsConfig
├── db.rs                — Persistencia SQLite (tabla settings clave-valor)
├── logger.rs            — Log dual: consola + archivo
└── settings_window.rs   — Ventana nativa macOS (NSPanel modal, scrollable)

src/bin/
└── whisper-tts.rs       — Binario CLI para el Stop hook
```

**Flujo de transcripción:**

```
⌘⌥W (hold) → Recorder::start()
⌘⌥W (release) → Recorder::stop() → WAV en /tmp/
  └─▶ transcriber::transcribe()    (whisper-cli local)
   OR azure_transcriber::transcribe() (Azure MAI)
  └─▶ [llm::correct_grammar()]     si activo
  └─▶ [llm::translate_text()]      si activo
  └─▶ paste_text()                 simula ⌘V
```

**Flujo de lectura de respuestas:**

```
Claude / Codex termina respuesta
  └─▶ Stop hook → whisper-tts (stdin: {user, assistant})
  └─▶ [GeminiFormatter]           si formatter activo
  └─▶ GeminiProvider::synthesize()
  └─▶ afplay audio.wav
  └─▶ guarda audio/last-tts.wav   (⌘⌥R para repetir)
```

---

## Configuración persistente (data.db)

| Clave | Valores | Default |
|-------|---------|---------|
| `language` | `"es"` \| `"en"` | `"es"` |
| `llm_enabled` | `"true"` \| `"false"` | `"false"` |
| `llm_model` | nombre `.gguf` | `""` |
| `translate_enabled` | `"true"` \| `"false"` | `"false"` |
| `translate_dest_lang` | `"es"` \| `"en"` | `"es"` |
| `azure_mai_enabled` | `"true"` \| `"false"` | `"false"` |
| `azure_mai_key` | string | `""` |
| `azure_mai_region` | string | `""` |
| `tts_enabled` | `"true"` \| `"false"` | `"false"` |
| `tts_voice` | nombre de voz Gemini | `"Sulafat"` |
| `gemini_api_key` | string | `""` |
| `tts_scene` | string | (descripción bilingual) |
| `tts_sample_context` | string | (contexto conversacional) |
| `tts_formatter_enabled` | `"true"` \| `"false"` | `"false"` |
| `tts_formatter_prompt` | string | (8 reglas de formato) |

---

## Solución de problemas

**El hotkey no responde**
- Habilita Accessibility: System Settings → Privacy & Security → Accessibility
- Si recompilaste, desactiva y reactiva el toggle

**`⌘⌥R` no reproduce nada**
- Verifica que existe `~/.config/whisperwlopezob/audio/last-tts.wav`
- Requiere que TTS esté activado y que haya habido al menos una respuesta leída

**TTS no se activa al terminar respuesta de Claude/Codex**
- Verifica que TTS esté habilitado en Configuración
- Verifica que haya una Gemini API Key configurada
- Revisa el log: `tail -20 ~/.config/whisperwlopezob/whisperwlopezob.log`
- Prueba el binario manualmente (ver sección anterior)

**El texto no se pega**
- Habilita Accessibility (necesario para simular `⌘V`)

**`❌ whisper-cli no encontrado`** → `brew install whisper-cpp`

**`❌ Modelo no encontrado`** → descarga un `.bin` en `~/.config/whisperwlopezob/whisper-models/`

**La transcripción tarda mucho** → usa un modelo más pequeño (`ggml-base.bin`)
