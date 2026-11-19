# whisperwlopezob

App de barra de menú para macOS que graba tu voz, transcribe con `whisper-cli` o Azure MAI Transcribe, traduce automáticamente (Azure Translator u Ollama local) si el idioma detectado difiere del destino, y pega el texto donde está el cursor. Lee en voz alta las respuestas de Claude Code / Codex con Gemini TTS o Qwen3-TTS local.

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

El modelo por defecto es `MAI-Transcribe-2` con enhanced mode activado, estilo
`clean` (elimina muletillas) y una lista de términos técnicos en `phraseList` para
que no destroce la jerga del proyecto.

> Las instalaciones que venían de `mai-transcribe-1.5` se migran solas al arrancar.
> Si editaste el Definition JSON a mano, se respeta y no se toca.
>
> Si Azure rechaza el estilo `clean` (`unsupported_transcript_mode`, visto en
> eastus en 2026-09), la app reintenta sin `transcribeStyle` (verbatim) y lo omite
> hasta reiniciar. Cuando Azure lo arregle, vuelve a usar `clean` sin tocar nada.

### 7. (Opcional) Traducción automática

Dos proveedores, configurables desde **Configuración → TRADUCCIÓN**:

- **Microsoft Translator** (default): reutiliza la API Key y región de Azure MAI.
  Detecta el idioma y solo traduce si difiere del destino.
- **Ollama local** (`gemma4:e4b`): requiere [Ollama](https://ollama.com) corriendo en
  `localhost:11434` con el modelo descargado (`ollama pull gemma4:e4b`). El prompt
  es editable y usa los placeholders `{dest_lang}` e `{input_text}`.

Si la traducción falla, se pega el texto original.

### 8. (Opcional) Lectura de respuestas (TTS)

Configurable desde **Configuración → LECTURA DE RESPUESTAS → Proveedor**:

- **Gemini TTS** (default): requiere una API key de Google Gemini.
- **Qwen3-TTS (local)**: 100% offline en Apple Silicon vía `mlx-audio`. Requiere:
  ```bash
  python3.12 -m venv ~/.config/whisperwlopezob/tts-venv
  ~/.config/whisperwlopezob/tts-venv/bin/pip install mlx-audio
  # modelo mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-8bit en:
  #   ~/.config/whisperwlopezob/tts-models/Qwen3-TTS-12Hz-1.7B-VoiceDesign-8bit
  ```

Si el proveedor falla (o no hay clave Gemini), se usa `say` de macOS como fallback.

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
| Definition JSON | Configuración del modelo (default: `MAI-Transcribe-2`, estilo `clean` + `phraseList`) |

### TRADUCCIÓN

| Campo | Descripción |
|-------|-------------|
| Activar traducción | Habilita traducción automática post-transcripción |
| Proveedor | `Microsoft Translator` (credenciales de Azure MAI) u `Ollama (gemma4:e4b)` |
| Idioma destino | `Español` o `English` |
| Prompt Ollama | Instrucciones para Ollama (solo visible con ese proveedor) |

Si el idioma detectado ya es el destino, el texto se pega sin traducir.

### LECTURA DE RESPUESTAS

| Campo | Descripción |
|-------|-------------|
| Leer respuestas de Claude | Habilita la síntesis de voz |
| Proveedor | `Gemini TTS` o `Qwen3-TTS (local)` |
| Clave Gemini | API key de Google Gemini (usada para TTS y formatter) |
| Formatear respuesta para voz | Pre-procesa con `gemini-3.1-flash-lite` antes de sintetizar |
| Prompt TTS | Instrucciones para el formatter (configurable) |
| Voz | Nombre de voz Gemini (default: `Sulafat`) |
| Vel | Velocidad de reproducción afplay (default: `1.0`) |
| Escena | Director's note: estilo de narración para Gemini TTS |
| Contexto | Contexto de la conversación para Gemini TTS |
| Prompt voz Qwen | Descripción de la voz para Qwen3-TTS VoiceDesign (acento primero) |
| Temp | Temperatura de Qwen3-TTS (default `0.7`; más baja = timbre más estable) |
| Mostrar texto al leer (`⌘⌥V`) | Muestra modal con el texto antes de reproducirlo |
| Sesiones a escuchar | Nombres de sesión de Claude Code (los de `/rename`) separados por comas. Vacío = todas |

---

## Lectura automática de respuestas (Stop Hook)

Al terminar cada respuesta, Claude Code / Codex ejecuta automáticamente `whisper-tts` que:

1. Recibe `{user, assistant, session}` por stdin (`session` = nombre puesto con `/rename`)
2. Comprueba si `tts_enabled` está activo en la DB y si la sesión está en **Sesiones a escuchar**
3. (Opcional) Formatea con `gemini-3.1-flash-lite` si el texto tiene código, markdown o listas
4. Sintetiza con el proveedor configurado: Gemini TTS (`gemini-3.8-flash-lite-tts`, Interactions API; escena y contexto van como `speech_metadata.style`) o Qwen3-TTS local; si falla, `say`
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

### Escuchar solo algunas sesiones

Si trabajas con varios agentes a la vez, ponle nombre a los que quieres escuchar con
`/rename <nombre>` en Claude Code y escribe esos nombres en **Configuración → Sesiones a
escuchar** (p. ej. `mods, review`). El resto de sesiones no se leen ni cortan el audio
en curso. La coincidencia es exacta y no distingue mayúsculas; una sesión sin nombre no
se lee mientras haya filtro.

El hook lee el nombre de las entradas `custom-title` del transcript, así que renombrar
una sesión a mitad de trabajo se aplica desde la siguiente respuesta. El filtro no
afecta a Codex (su hook no envía sesión).

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
├── main.rs              — Event loop (winit), tray icon, hotkeys, flujo grabar → transcribir → pegar
├── lib.rs               — Declaración de módulos
├── settings.rs          — Struct Settings: carga/guarda toda la config de data.db (fuente única de claves)
├── settings_window.rs   — Ventana nativa macOS (NSPanel modal, scrollable)
├── db.rs                — Persistencia SQLite (tabla settings clave-valor)
├── config.rs            — Auto-detección de whisper-cli y modelos
├── defaults.rs          — Constantes y valores por defecto
├── hotkey.rs            — Hotkeys globales: ⌘⌥W, ⌘⌥R, ⌘⌥V
├── recorder.rs          — Grabación (cpal + hound), resample a 16kHz mono con filtro anti-aliasing
├── transcriber.rs       — Invocación de whisper-cli con timeout 60s
├── azure_transcriber.rs — Backend Azure MAI Transcribe (MAI-Transcribe-2, fallback sin transcribeStyle)
├── translator.rs        — Traducción: Azure Translator v3 u Ollama local (factory)
├── formatter.rs         — Formateador Gemini Flash Lite para TTS
├── text.rs              — Utilidades de texto (recorte seguro UTF-8)
├── logger.rs            — Log dual: consola + archivo
└── tts/
    ├── mod.rs           — Orquestador TTS: elige proveedor, reproduce con afplay, gestión de PIDs
    ├── gemini.rs        — Síntesis con gemini-3.8-flash-lite-tts (WAV)
    ├── qwen.rs          — Síntesis local Qwen3-TTS (ejecuta scripts/qwen_tts_synth.py en el venv)
    ├── say.rs           — Fallback: say -v Paulina / Samantha
    └── provider.rs      — Structs AudioData, TtsConfig

src/bin/
└── whisper-tts.rs       — Binario CLI para el Stop hook
```

**Flujo de transcripción:**

```
⌘⌥W (hold) → Recorder::start()
⌘⌥W (release) → Recorder::stop() → WAV en /tmp/
  └─▶ transcriber::transcribe()       (whisper-cli local)
   OR azure_transcriber::transcribe() (Azure MAI Transcribe)
  └─▶ [translator::translate()]       si activo y detected_lang ≠ dest_lang (Azure u Ollama)
  └─▶ paste_text()                    simula ⌘V
```

**Flujo de lectura de respuestas:**

```
Claude Code / Codex termina respuesta
  └─▶ Stop hook → whisper-tts (stdin: {user, assistant})
  └─▶ [GeminiFormatter]               si formatter activo y texto lo requiere
  └─▶ GeminiProvider | QwenProvider   (fallback: say)
  └─▶ afplay audio.wav                (velocidad configurable)
  └─▶ guarda last-tts.wav + last-tts-text.txt
  └─▶ [modal osascript]               si show_modal activo
```

---

## Configuración persistente (data.db)

Todas las claves se definen en un único sitio: `src/settings.rs` (`Settings::load` / `save`).

| Clave | Valores | Default |
|-------|---------|---------|
| `translate_enabled` | `"true"` \| `"false"` | `"false"` |
| `translate_dest_lang` | `"es"` \| `"en"` | `"es"` |
| `translate_provider` | `"azure"` \| `"ollama"` | `"azure"` |
| `translate_ollama_prompt` | string | (prompt JSON con `{dest_lang}` / `{input_text}`) |
| `azure_mai_enabled` | `"true"` \| `"false"` | `"false"` |
| `azure_mai_key` | string | `""` |
| `azure_mai_region` | string | `""` |
| `azure_mai_api_version` | string | `"2025-10-15"` |
| `azure_mai_definition` | JSON string | `{"enhancedMode":{"enabled":true,"model":"MAI-Transcribe-2",…}}` (ver `defaults.rs`) |
| `tts_enabled` | `"true"` \| `"false"` | `"false"` |
| `tts_provider` | `"gemini"` \| `"qwen"` | `"gemini"` |
| `tts_qwen_prompt` | string | (descripción de voz, acento primero) |
| `tts_qwen_temperature` | float string | `"0.7"` |
| `gemini_api_key` | string | `""` |
| `tts_voice` | nombre de voz Gemini | `"Sulafat"` |
| `tts_playback_rate` | float string | `"1.0"` |
| `tts_scene` | string | (descripción bilingual) |
| `tts_sample_context` | string | (contexto conversacional) |
| `tts_formatter_enabled` | `"true"` \| `"false"` | `"false"` |
| `tts_formatter_prompt` | string | (7 reglas de formato) |
| `tts_show_modal` | `"true"` \| `"false"` | `"false"` |
| `tts_sessions` | nombres separados por comas | `""` (todas) |

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
- Verifica que `translate_enabled = true` y el idioma destino esté configurado
- Microsoft Translator: requiere API Key y región de Azure MAI
- Ollama: verifica que esté corriendo (`curl localhost:11434`) y que el modelo esté descargado (`ollama pull gemma4:e4b`)

**El texto no se pega**
- Habilita Accessibility (necesario para simular `⌘V`)

**`❌ whisper-cli no encontrado`** → `brew install whisper-cpp`

**`❌ Modelo no encontrado`** → descarga un `.bin` en `~/.config/whisperwlopezob/whisper-models/`

**La transcripción tarda mucho** → usa Azure MAI (más rápido) o un modelo más pequeño (`ggml-base.bin`)
