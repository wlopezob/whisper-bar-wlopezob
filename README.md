# whisperwlopezob

App de barra de menú para macOS que graba tu voz, transcribe con `whisper-cli` o Azure MAI Transcribe, traduce automáticamente si el idioma detectado difiere del destino, y pega el texto donde está el cursor. Lee en voz alta las respuestas de Claude Code / Codex usando Gemini TTS.

**Hotkeys:**
- `⌘⌥W` — mantén para grabar, suelta para transcribir y pegar
- `⌘⌥R` — reproduce la última respuesta TTS; si ya está reproduciéndose, la para
- `⌘⌥V` — muestra modal con el último texto TTS

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
└── audio/
    ├── last-tts.wav        # último audio TTS (reproducir con ⌘⌥R)
    └── last-tts-text.txt   # último texto TTS (ver con ⌘⌥V)
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

### 4. Instalar whisper-cpp (backend local)

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

### 6. (Recomendado) Azure MAI Transcribe

Transcripción de alta precisión vía Azure Cognitive Services. Requiere:
- Suscripción de Azure con Speech Services habilitado
- API Key y región (p.ej. `eastus`)

Configurable desde **Configuración → AZURE MAI TRANSCRIBE → Backend → Azure MAI**.

El modelo por defecto es `mai-transcribe-1.5` con enhanced mode activado.

### 7. (Opcional) Traducción automática

Reutiliza las mismas credenciales de Azure MAI. Cuando está activa:
- Detecta automáticamente el idioma del texto transcrito
- Solo traduce si el idioma detectado ≠ idioma destino (sin coste innecesario)

Configurable desde **Configuración → TRADUCCIÓN**.

### 8. (Opcional) Lectura de respuestas con Gemini TTS

Requiere una API key de Google Gemini. Configurable desde **Configuración → LECTURA DE RESPUESTAS**.

Para que las respuestas de Claude Code / Codex se lean automáticamente al terminar cada respuesta:

```bash
bash bundle.sh   # instala también los hooks de Claude Code y Codex
```

### 9. Compilar e instalar como .app

```bash
git clone <url-del-repositorio>
cd whisper-bar-wlopezob
bash bundle.sh
```

---

## Permisos en macOS

### Accesibilidad (obligatorio)

Requerido para registrar `⌘⌥W` / `⌘⌥R` / `⌘⌥V` y simular `⌘V` para pegar.

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
| Soltar `⌘⌥W` | Detiene grabación — icono `⏳` — transcribe, traduce (si activo) y pega |
| Presionar `⌘⌥R` (sin audio) | Reproduce la última respuesta TTS |
| Presionar `⌘⌥R` (con audio) | Para la reproducción en curso |
| Presionar `⌘⌥V` | Muestra modal con el último texto TTS |

El clipboard original se restaura automáticamente 300ms después de pegar.

---

## Ventana de configuración

Panel nativo macOS con scroll vertical. Secciones en orden de flujo:

### AZURE MAI TRANSCRIBE

| Campo | Descripción |
|-------|-------------|
| Backend | `Whisper local` (whisper-cli) o `Azure MAI` |
| API Key | Clave de Azure Cognitive Services |
| Región | Región de Azure (ej: `eastus`) |
| API Version | Versión de la API (default: `2025-10-15`) |
| Definition JSON | Configuración del modelo (default: `mai-transcribe-1.5` con enhanced mode) |

### TRADUCCIÓN

| Campo | Descripción |
|-------|-------------|
| Activar traducción | Habilita traducción automática post-transcripción |
| Idioma destino | `Español` o `English` |

Usa las mismas credenciales de Azure MAI. Si el idioma detectado ya es el destino, se salta la llamada.

### LECTURA DE RESPUESTAS

| Campo | Descripción |
|-------|-------------|
| Leer respuestas de Claude | Habilita síntesis de voz con Gemini TTS |
| Clave Gemini | API key de Google Gemini (usada para TTS y formatter) |
| Formatear respuesta para voz | Pre-procesa con `gemini-3.1-flash-lite` antes de sintetizar |
| Prompt TTS | Instrucciones para el formatter (configurable) |
| Voz | Nombre de voz Gemini (default: `Sulafat`) |
| Vel | Velocidad de reproducción afplay (default: `1.0`) |
| Escena | Director's note: estilo de narración para Gemini TTS |
| Contexto | Contexto de la conversación para Gemini TTS |
| Mostrar texto al leer (`⌘⌥V`) | Muestra modal con el texto antes de reproducirlo |

---

## Lectura automática de respuestas (Stop Hook)

Al terminar cada respuesta, Claude Code / Codex ejecuta automáticamente `whisper-tts` que:

1. Recibe el par `{user, assistant}` por stdin
2. Comprueba si `tts_enabled` está activo en la DB
3. (Opcional) Formatea con `gemini-3.1-flash-lite` si el texto tiene código, markdown o listas
4. Sintetiza con Gemini TTS (`gemini-3.1-flash-tts-preview`)
5. Reproduce con `afplay` (respetando la velocidad configurada)
6. Guarda el audio en `audio/last-tts.wav` y el texto en `audio/last-tts-text.txt`

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
├── config.rs            — Auto-detección de whisper-cli y modelos
├── defaults.rs          — Constantes y valores por defecto
├── hotkey.rs            — Hotkeys globales: ⌘⌥W, ⌘⌥R, ⌘⌥V
├── recorder.rs          — Grabación de audio (cpal + hound, 16kHz mono PCM)
├── transcriber.rs       — Invocación de whisper-cli con timeout 60s
├── azure_transcriber.rs — Backend Azure MAI Transcribe (mai-transcribe-1.5)
├── translator.rs        — Traducción Azure Translator v3 con auto-detección
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
  └─▶ transcriber::transcribe()       (whisper-cli local)
   OR azure_transcriber::transcribe() (Azure MAI Transcribe)
  └─▶ [translator::translate()]       si activo y detected_lang ≠ dest_lang
  └─▶ paste_text()                    simula ⌘V
```

**Flujo de lectura de respuestas:**

```
Claude Code / Codex termina respuesta
  └─▶ Stop hook → whisper-tts (stdin: {user, assistant})
  └─▶ [GeminiFormatter]               si formatter activo y texto lo requiere
  └─▶ GeminiProvider::synthesize()
  └─▶ afplay audio.wav                (velocidad configurable)
  └─▶ guarda last-tts.wav + last-tts-text.txt
  └─▶ [modal osascript]               si show_modal activo
```

---

## Configuración persistente (data.db)

| Clave | Valores | Default |
|-------|---------|---------|
| `translate_enabled` | `"true"` \| `"false"` | `"false"` |
| `translate_dest_lang` | `"es"` \| `"en"` | `"es"` |
| `azure_mai_enabled` | `"true"` \| `"false"` | `"false"` |
| `azure_mai_key` | string | `""` |
| `azure_mai_region` | string | `""` |
| `azure_mai_api_version` | string | `"2025-10-15"` |
| `azure_mai_definition` | JSON string | `{"enhancedMode":{"enabled":true,"model":"mai-transcribe-1.5"}}` |
| `tts_enabled` | `"true"` \| `"false"` | `"false"` |
| `gemini_api_key` | string | `""` |
| `tts_voice` | nombre de voz Gemini | `"Sulafat"` |
| `tts_playback_rate` | float string | `"1.0"` |
| `tts_scene` | string | (descripción bilingual) |
| `tts_sample_context` | string | (contexto conversacional) |
| `tts_formatter_enabled` | `"true"` \| `"false"` | `"false"` |
| `tts_formatter_prompt` | string | (7 reglas de formato) |
| `tts_show_modal` | `"true"` \| `"false"` | `"false"` |

---

## Solución de problemas

**El hotkey no responde**
- Habilita Accessibility: System Settings → Privacy & Security → Accessibility
- Si recompilaste, desactiva y reactiva el toggle

**`⌘⌥R` no reproduce nada**
- Verifica que existe `~/.config/whisperwlopezob/audio/last-tts.wav`
- Requiere que TTS esté activado y que haya habido al menos una respuesta leída

**TTS no se activa al terminar respuesta de Claude/Codex**
- Verifica que `tts_enabled = true` en Configuración
- Verifica que haya una Gemini API Key configurada
- Revisa el log: `tail -20 ~/.config/whisperwlopezob/whisperwlopezob.log`
- Prueba el binario manualmente con el comando de la sección anterior

**La traducción no funciona**
- Requiere Azure MAI configurado (reutiliza la misma API Key y región)
- Verifica que `translate_enabled = true` y el idioma destino esté configurado

**El texto no se pega**
- Habilita Accessibility (necesario para simular `⌘V`)

**`❌ whisper-cli no encontrado`** → `brew install whisper-cpp`

**`❌ Modelo no encontrado`** → descarga un `.bin` en `~/.config/whisperwlopezob/whisper-models/`

**La transcripción tarda mucho** → usa Azure MAI (más rápido) o un modelo más pequeño (`ggml-base.bin`)
