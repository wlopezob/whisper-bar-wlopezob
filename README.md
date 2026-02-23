# whisperwlopezob

App de barra de menú para macOS que graba tu voz con un hotkey, transcribe con `whisper-cli` y pega el texto donde está el cursor. Opcionalmente corrige gramática con un LLM local y/o traduce el resultado al idioma destino configurado.

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
└── llm/                    # modelos LLM para corrección y traducción (.gguf)
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

### 6. (Opcional) Corrección gramatical y traducción con LLM local

Para corregir automáticamente errores de gramática o traducir después de transcribir:

```bash
brew install llama.cpp
mkdir -p ~/.config/whisperwlopezob/llm

# Modelo recomendado (~1GB, <2s en Apple Silicon)
curl -L -o ~/.config/whisperwlopezob/llm/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf
```

> Puedes colocar múltiples modelos `.gguf` en `~/.config/whisperwlopezob/llm/`. Todos aparecerán en la ventana de configuración para seleccionar.

#### Probar `llama-completion` directamente en consola

Este comando replica el flujo que usa la app (prompt de sistema + entrada de usuario):

```bash
llama-completion \
  -m ~/.config/whisperwlopezob/llm/qwen2.5-1.5b-instruct-q4_k_m.gguf \
  -sys "Fix grammar and pronunciation errors in this English text. Return ONLY the corrected text, no explanations, no extra words." \
  -p "Could you help me send a message for Mama?" \
  -n 128 -ngl 99 \
  > /tmp/llm_out.txt 2> /tmp/llm_err.txt

# Texto generado (incluye bloque assistant)
cat /tmp/llm_out.txt

# Logs técnicos del motor (carga de modelo, performance, etc.)
head -n 40 /tmp/llm_err.txt
```

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

La app solicita el permiso de micrófono automáticamente **500ms después de arrancar** (antes de la primera grabación). Si fue denegado:

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
✅ llama-cli: /opt/homebrew/bin/llama-completion
Modelo LLM:
  ✓ qwen2.5-1.5b-instruct-q4_k_m.gguf
☐ Mejorar gramática
─────────────────────────────────────────
Configuración...
─────────────────────────────────────────
Log: ~/.config/whisperwlopezob/whisperwlopezob.log
Ver log
─────────────────────────────────────────
Salir
```

- **Idioma** — muestra el idioma de transcripción activo (solo lectura, cámbialo en Configuración).
- **Modelo LLM** — muestra el modelo activo con checkmark (solo lectura, cámbialo en Configuración).
- **Mejorar gramática** — muestra si la corrección gramatical está activa (solo lectura).
- **Configuración...** — abre la ventana nativa de configuración.
- **Ver log** — abre el log en el editor de texto por defecto.

---

## Ventana de configuración

Al hacer clic en **Configuración...** se abre un panel nativo macOS (NSPanel modal) con tres secciones:

```
┌──────────────────────────────────────────┐
│  Configuración                     [X]  │
├──────────────────────────────────────────┤
│  TRANSCRIPCIÓN                           │
│  Idioma:  (●) Español   (○) English     │
│                                          │
│  MEJORA GRAMATICAL                       │
│  [☐] Activar mejora gramatical          │
│  Modelo: [▼ — seleccionar modelo —  ]   │
│                                          │
│  TRADUCCIÓN                              │
│  [☐] Activar traducción                 │
│  Idioma destino: [▼ Español         ]   │
│                                          │
│               [Cancelar]   [Aplicar]    │
└──────────────────────────────────────────┘
```

**TRANSCRIPCIÓN**
- Selecciona el idioma de transcripción: Español (`es`) o English (`en`).

**MEJORA GRAMATICAL**
- Activa o desactiva la corrección automática de gramática y pronunciación usando el LLM local.
- Selecciona el modelo `.gguf` a usar. Si no hay modelo seleccionado (muestra `— seleccionar modelo —`), la corrección no se aplica aunque el toggle esté activo.
- El prompt de corrección se ajusta automáticamente al idioma de transcripción.

**TRADUCCIÓN**
- Activa o desactiva la traducción del texto transcrito (y corregido) al idioma destino.
- Solo se traduce si el idioma destino es diferente al idioma de transcripción.
- Requiere que un modelo LLM esté seleccionado.

Pulsar **Aplicar** guarda todos los cambios en la base de datos y actualiza el tray. Pulsar **Cancelar** o cerrar la ventana descarta los cambios.

---

## Corrección gramatical

Cuando **Mejorar gramática** está activo y hay un modelo seleccionado, después de transcribir el audio el texto pasa por el CLI LLM con un prompt de sistema que varía según el idioma:

- **Español:** _"Corrige los errores gramaticales de este texto en español. Devuelve ÚNICAMENTE el texto corregido."_
- **English:** _"Fix grammar and pronunciation errors in this English text. Return ONLY the corrected text, no explanations, no extra words."_

Si el LLM falla o supera 30 segundos, se pega la transcripción original sin interrupción.

Útil para aprender inglés: habla aunque cometas errores — el texto que se pega siempre será correcto.

---

## Traducción

Cuando **Traducción** está activa y el idioma destino es distinto al idioma de transcripción, el texto (ya corregido si corresponde) pasa por el LLM con un prompt de traducción:

- **Destino Español:** _"Translate the following text to Spanish. Return ONLY the Spanish translation."_
- **Destino English:** _"Traduce el siguiente texto al inglés. Devuelve ÚNICAMENTE la traducción en inglés."_

Si la traducción falla o supera 30 segundos, se pega el texto sin traducir.

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
- Verifica si existe `llama-completion`: `which llama-completion`

**Sin modelos en `~/.config/whisperwlopezob/llm/`**
- Descarga un modelo `.gguf` en esa carpeta (ver paso 6)

**El hotkey `⌘⌥W` no responde**
- Habilita Accesibilidad: System Settings → Privacy & Security → Accessibility
- Si recompilaste, desactiva y reactiva el toggle

**El texto no se pega**
- Habilita Accesibilidad (necesario para simular `⌘V`)
- Asegúrate de que el cursor esté en un campo de texto

**Mejora gramatical no se aplica aunque esté activada**
- Abre Configuración... y selecciona un modelo LLM en el desplegable (si aparece `— seleccionar modelo —` no hay modelo activo)

**La traducción no se aplica**
- Verifica que el idioma destino sea diferente al idioma de transcripción
- Verifica que haya un modelo LLM seleccionado en Configuración...

**La transcripción tarda mucho**
- Usa un modelo más pequeño (`ggml-base.bin` o `ggml-tiny.bin`)
- Timeout máximo: 60 segundos

**La corrección LLM o traducción tarda mucho**
- Usa un modelo más pequeño o desactiva las opciones en Configuración...
- Timeout máximo: 30 segundos por operación (usa el texto previo como fallback)

---

## Arquitectura

```
main.rs           — Event loop (winit), tray icon, coordinador principal
config.rs         — Auto-detección de whisper-cli, CLI LLM y modelos
defaults.rs       — Constantes y valores por defecto (fuente única de verdad)
recorder.rs       — Grabación de audio (cpal + hound, 16kHz mono PCM)
transcriber.rs    — Invocación de whisper-cli con timeout de 60s
llm.rs            — Corrección gramatical y traducción con CLI LLM (timeout 30s)
hotkey.rs         — Registro del hotkey global ⌘⌥W
db.rs             — Persistencia de configuración (SQLite, tabla settings clave-valor)
logger.rs         — Log dual: consola + archivo (auto-recreado si se elimina)
settings_window.rs — Ventana nativa de configuración (NSPanel modal vía objc2)
```

**Flujo completo:**

```
⌘⌥W (hold)
  └─▶ Recorder::start()              — CoreAudio captura audio
⌘⌥W (release)
  └─▶ Recorder::stop()               — Escribe WAV en /tmp/
  └─▶ transcriber::transcribe()      — Ejecuta whisper-cli en thread separado
  └─▶ [llm::correct_grammar()]       — Si "Mejorar gramática" activo y modelo seleccionado
  └─▶ [llm::translate_text()]        — Si "Traducción" activa y destino ≠ origen
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
| `translate_enabled` | `"true"` \| `"false"` | `"false"` |
| `translate_dest_lang` | `"es"` \| `"en"` | `"es"` |
