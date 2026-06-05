# Data Model: Azure TTS Core

## Entidades

### 1. TTS Config (SQLite — tabla `settings`)

Configuración persistente del usuario para TTS. Vive en la misma tabla `settings`
key-value que el resto de la configuración de la app.

| Clave SQLite | Tipo lógico | Default | Descripción |
|--------------|-------------|---------|-------------|
| `tts_enabled` | bool (string "true"/"false") | "false" | Activa/desactiva síntesis de voz |
| `tts_voice` | string | "es-MX-DaliaNeural" | Nombre de voz Azure Neural |

**Claves reutilizadas** (ya existen, no se crean):

| Clave SQLite | Usada por TTS para |
|--------------|-------------------|
| `azure_mai_key` | API Key de Azure Speech TTS |
| `azure_mai_region` | Región de Azure (ej: "eastus") |

**Acceso**: vía `db::Db::get(key, default)` y `db::Db::set(key, value)`.
**Carga en la app**: en `WhisperApp::new()` junto con el resto de settings.
**Carga en el binary**: al inicio de `whisper-tts`, antes de llamar a `tts::speak()`.

---

### 2. Audio Temporal

Archivo MP3 generado durante la síntesis. Runtime artifact — no persiste.

| Atributo | Valor |
|----------|-------|
| Path | `/tmp/whisper-tts-audio.mp3` |
| Formato | audio-16khz-128kbitrate-mono-mp3 |
| Ciclo de vida | Creado por `tts::speak()` → reproducido por `afplay` → eliminado tras reproducción |
| Error de eliminación | Ignorado (el archivo se limpia en la siguiente invocación) |

---

### 3. PID de Control

Archivo que permite a una nueva instancia interrumpir la reproducción activa.

| Atributo | Valor |
|----------|-------|
| Path | `/tmp/whisper-tts.pid` |
| Contenido | PID del proceso `whisper-tts` activo (número decimal, sin newline) |
| Ciclo de vida | Escrito al inicio de cada invocación → eliminado al finalizar |
| Error de lectura/escritura | Ignorado silenciosamente (fallo no bloquea TTS) |

---

## Cambios en Structs Rust

### `SettingsValues` (src/settings_window.rs)

Añadir dos campos al struct existente:

```rust
pub struct SettingsValues {
    // ... campos existentes ...
    // Azure MAI Transcribe (ya existen)
    pub azure_mai_enabled: bool,
    pub azure_mai_key: String,
    pub azure_mai_region: String,
    pub azure_mai_model: String,
    pub azure_mai_api_version: String,
    pub azure_mai_definition: String,
    // TTS (nuevos)
    pub tts_enabled: bool,
    pub tts_voice: String,
}
```

### `WhisperApp` (src/main.rs)

Añadir dos campos `Arc<Mutex<>>` siguiendo el patrón existente:

```rust
// TTS (nuevos)
tts_enabled: Arc<Mutex<bool>>,
tts_voice: Arc<Mutex<String>>,
```

### `AzureFieldPtrs` (src/settings_window.rs)

Añadir punteros a los nuevos controles TTS (mostrados/ocultados con el toggle Azure):

```rust
struct AzureFieldPtrs {
    // ... campos existentes ...
    chk_tts: *const NSButton,
    tf_tts_voice: *const NSTextField,
    lbl_tts_section: *const NSTextField,
    lbl_tts_voice: *const NSTextField,
}
```

---

## Constante Nueva (src/defaults.rs)

```rust
pub const TTS_DEFAULT_VOICE: &str = "es-MX-DaliaNeural";
```
