# whisperwlopezob

App de barra de menú para macOS que graba tu voz con un hotkey, transcribe con `whisper-cli` y pega el texto donde está el cursor. Opcionalmente corrige gramática y pronunciación con un LLM local (`llama-cli`).

**Hotkey:** `⌘⌥W` — mantén para grabar, suelta para transcribir y pegar.

---

## Requisitos

- macOS 13 o superior (Apple Silicon o Intel)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/)
- [Homebrew](https://brew.sh)
- Rust (instalado vía `rustup`)

---

## Estructura de datos

Todo vive bajo un único directorio:

```
~/.config/whisperwlopezob/
├── whisperwlopezob.log     # log de la app (se trunca al iniciar)
├── data.db                 # configuración persistente (SQLite)
├── whisper-models/         # modelos de Whisper (.bin)
│   └── ggml-large-v3.bin
└── llm/                    # modelos LLM para corrección gramatical (.gguf)
    └── qwen2.5-1.5b-instruct-q4_k_m.gguf
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

Verifica que esté disponible:

```bash
which whisper-cli
# /opt/homebrew/bin/whisper-cli  (Apple Silicon)
# /usr/local/bin/whisper-cli     (Intel)
```

### 5. Descargar un modelo de Whisper

Crea el directorio y descarga el modelo:

```bash
mkdir -p ~/.config/whisperwlopezob/whisper-models
```

Elige uno según tu hardware:

```bash
# Recomendado para Apple Silicon — buena precisión y velocidad
curl -L -o ~/.config/whisperwlopezob/whisper-models/ggml-small.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin

# Más rápido — menor precisión
curl -L -o ~/.config/whisperwlopezob/whisper-models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

# Máxima precisión — más lento
curl -L -o ~/.config/whisperwlopezob/whisper-models/ggml-large-v3.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin
```

> La app detecta automáticamente el mejor modelo disponible con esta prioridad: `large-v3 → large-v2 → medium → small → base → tiny`

### 6. (Opcional) Corrección gramatical con LLM local

Para corregir automáticamente errores de gramática y pronunciación después de transcribir:

```bash
brew install llama.cpp
mkdir -p ~/.config/whisperwlopezob/llm

# Modelo recomendado (~1GB, <2s en Apple Silicon)
curl -L -o ~/.config/whisperwlopezob/llm/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf
```

> Puedes colocar múltiples modelos `.gguf` en `~/.config/whisperwlopezob/llm/`. Todos aparecerán en el menú para seleccionar.

### 7. Compilar e instalar como .app

```bash
git clone <url-del-repositorio>
cd whisper-bar-rust
bash bundle.sh
```

La primera ejecución pedirá contraseña para crear `/Applications/whisperwlopezob.app`. Las siguientes no requieren `sudo`.

---

## Permisos en macOS

### Accesibilidad (obligatorio)

Requerido para:
- Registrar el hotkey global `⌘⌥W`
- Simular `⌘V` para pegar el texto

Pasos:

1. Ejecuta la app: `open /Applications/whisperwlopezob.app`
2. Ve a **System Settings → Privacy & Security → Accessibility**
3. Activa el toggle de **whisperwlopezob**
4. Reinicia: `pkill whisperwlopezob; open /Applications/whisperwlopezob.app`

> **Importante:** cada rebuild revoca el permiso de Accessibility (el hash del binario cambia). Debes desactivar y reactivar el toggle tras cada `bash bundle.sh`.

### Micrófono

macOS lo solicita automáticamente la primera vez que grabes. Si fue denegado:

**System Settings → Privacy & Security → Microphone** → activa **whisperwlopezob**

---

## Ejecución

```bash
# Compilar e instalar (primera vez o tras cambios)
bash bundle.sh

# Lanzar
open /Applications/whisperwlopezob.app

# Relanzar tras cambios
pkill whisperwlopezob; open /Applications/whisperwlopezob.app

# Ver log en tiempo real
tail -f ~/.config/whisperwlopezob/whisperwlopezob.log
```

---

## Uso

| Acción | Resultado |
|--------|-----------|
| Mantener `⌘⌥W` | Inicia grabación — icono cambia a `🔴` |
| Soltar `⌘⌥W` | Detiene grabación — icono cambia a `⏳` |
| Transcripción completa | Texto pegado donde está el cursor — icono vuelve a `🎙` |

El clipboard original se restaura automáticamente 300ms después de pegar.

### Menú de estado

```
whisperwlopezob
─────────────────────────────────────────
Mantén ⌘⌥W para grabar / suelta para transcribir
─────────────────────────────────────────
✅ whisper-cli: /opt/homebrew/bin/whisper-cli
✅ Modelo: ggml-large-v3.bin
─────────────────────────────────────────
Idioma
  ✓ Español
    English
─────────────────────────────────────────
✅ llama-cli: /opt/homebrew/bin/llama-cli
Modelo LLM:
  ✓ qwen2.5-1.5b-instruct-q4_k_m.gguf
☐ Mejorar gramática
─────────────────────────────────────────
Log: ~/.config/whisperwlopezob/whisperwlopezob.log
Ver log
─────────────────────────────────────────
Salir
```

- **Idioma** — selecciona el idioma de transcripción (Español / English). Se guarda automáticamente.
- **Modelo LLM** — selecciona qué modelo usar para corrección. Se guarda automáticamente.
- **Mejorar gramática** — activa/desactiva corrección con LLM. Solo disponible si `llama-cli` y al menos un modelo `.gguf` están instalados.
- **Ver log** — abre el log en el editor de texto por defecto.

---

## Corrección gramatical

Cuando **Mejorar gramática** está activo, después de transcribir el audio el texto pasa por `llama-cli` con un prompt que corrige errores de gramática y pronunciación antes de pegarlo. Si el LLM falla o supera 30 segundos, se pega la transcripción original sin interrupción.

Útil para aprender inglés: habla aunque cometas errores — el texto que se pega siempre será correcto.

---

## Solución de problemas

**El icono no aparece en la barra de menú**
- Asegúrate de ejecutar la app desde una sesión con interfaz gráfica (no SSH)

**`❌ whisper-cli no encontrado`**
- `brew install whisper-cpp`

**`❌ Modelo no encontrado`**
- Verifica que existe algún archivo `ggml-*.bin` en `~/.config/whisperwlopezob/whisper-models/`

**`❌ llama-cli no encontrado`**
- `brew install llama.cpp`

**Sin modelos en `~/.config/whisperwlopezob/llm/`**
- Descarga un modelo `.gguf` en esa carpeta (ver paso 6)

**El hotkey `⌘⌥W` no responde**
- Habilita Accesibilidad: System Settings → Privacy & Security → Accessibility
- Si recompilaste, desactiva y reactiva el toggle

**El texto no se pega**
- Habilita Accesibilidad (necesario para simular `⌘V`)
- Asegúrate de que el cursor esté en un campo de texto

**La transcripción tarda mucho**
- Usa un modelo más pequeño (`ggml-base.bin` o `ggml-tiny.bin`)
- Timeout máximo: 60 segundos

**La corrección LLM tarda mucho**
- Usa un modelo más pequeño o desactiva **Mejorar gramática**
- Timeout máximo: 30 segundos (usa transcripción original como fallback)

---

## Arquitectura

```
main.rs        — Event loop (winit), tray icon, coordinador principal
config.rs      — Auto-detección de whisper-cli, llama-cli y modelos
defaults.rs    — Constantes y valores por defecto (fuente única de verdad)
recorder.rs    — Grabación de audio (cpal + hound, 16kHz mono PCM)
transcriber.rs — Invocación de whisper-cli con timeout de 60s
llm.rs         — Corrección gramatical con llama-cli (timeout 30s, fallback silencioso)
hotkey.rs      — Registro del hotkey global ⌘⌥W
db.rs          — Persistencia de configuración (SQLite, tabla settings clave-valor)
logger.rs      — Log dual: consola + archivo (auto-recreado si se elimina)
```

**Flujo completo:**

```
⌘⌥W (hold)
  └─▶ Recorder::start()              — CoreAudio captura audio
⌘⌥W (release)
  └─▶ Recorder::stop()               — Escribe WAV en /tmp/
  └─▶ transcriber::transcribe()      — Ejecuta whisper-cli en thread separado
  └─▶ [llm::correct_grammar()]       — Si "Mejorar gramática" activado
  └─▶ paste_text()
        ├─ Guarda clipboard actual
        ├─ Escribe texto al clipboard
        ├─ Simula ⌘V (enigo)
        └─ Restaura clipboard original tras 300ms
```

**Configuración persistente** (`~/.config/whisperwlopezob/data.db`):

| Clave | Valores | Default |
|-------|---------|---------|
| `language` | `"es"` \| `"en"` | `"es"` |
| `llm_enabled` | `"true"` \| `"false"` | `"false"` |
| `llm_model` | nombre del archivo `.gguf` | `""` |
