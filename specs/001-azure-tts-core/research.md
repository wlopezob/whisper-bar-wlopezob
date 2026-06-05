# Research: Azure TTS Core

## 1. Azure Speech TTS REST API

**Decision**: Usar la API REST v1 de Azure Cognitive Services Speech (mismas credenciales
que `azure_transcriber.rs`, sin SDK adicional).

**Endpoint**:
```
POST https://{region}.tts.speech.microsoft.com/cognitiveservices/v1
```

**Headers requeridos**:
```
Ocp-Apim-Subscription-Key: {api_key}
Content-Type: application/ssml+xml
X-Microsoft-OutputFormat: audio-16khz-128kbitrate-mono-mp3
```

**Body — formato SSML**:
```xml
<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{lang}'>
  <voice name='{voice_name}'>{text}</voice>
</speak>
```

donde `{lang}` se extrae del nombre de la voz: `es-MX-DaliaNeural` → `es-MX`
(tomar las dos primeras partes separadas por `-`).

**Respuesta exitosa**: bytes MP3 binarios (Content-Type: audio/mpeg).
**Respuesta de error**: 4xx/5xx con cuerpo de texto describiendo el error.

**Rationale**: Mismo patrón que `azure_transcriber.rs` (reqwest blocking, timeout,
header Ocp-Apim-Subscription-Key). Sin nueva dependencia.

**Alternativas consideradas**: Azure Speech SDK (Rust binding no oficial, añade complejidad);
`edge-tts` Python (dependencia externa, requiere Python instalado).

---

## 2. Inicialización de Log en Modo Append (sin truncar)

**Decision**: Inicializar `simplelog::WriteLogger` en modo append para el binary
`whisper-tts`, evitando el `truncate(true)` que usa `logger::init()` de la app principal.

**Implementación**:
```rust
fn init_append_logger() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let log_path = format!("{}/{}/whisperwlopezob.log",
        home, whisper_bar_rust::defaults::APP_CONFIG_DIR);

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)           // ← diferencia clave vs logger::init()
        .open(&log_path)
        .unwrap_or_else(|_| {
            // Si falla el log, continuar silenciosamente
            return;
        });

    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        file,
    ).ok(); // .ok() porque puede fallar si ya hay otro logger inicializado
}
```

**Rationale**: `simplelog` ya está en Cargo.toml. Sin nueva dependencia. El binary
no llama a `logger::init()` (que trunca) sino a esta función propia.

**Alternativas consideradas**: `tracing` (nueva dependencia); `env_logger` (nueva
dependencia); escribir al log manualmente con `std::fs::write` (sin formato estándar).

---

## 3. Gestión de PID para Interrumpir Reproducción Activa (FR-015)

**Decision**: Archivo `/tmp/whisper-tts.pid` contiene el PID del proceso `whisper-tts`
activo. Al iniciar, se lee el PID previo, se envía SIGTERM via `kill` CLI (sin libc),
se espera 300ms, y se escribe el PID propio.

**Implementación**:
```rust
const PID_FILE: &str = "/tmp/whisper-tts.pid";

pub fn kill_previous_instance() {
    if let Ok(pid_str) = std::fs::read_to_string(PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let current = std::process::id();
            if pid != current {
                let _ = std::process::Command::new("kill")
                    .args(["-15", &pid.to_string()])
                    .status();
                // Espera breve para que el proceso hijo (afplay) termine
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }
}

pub fn write_pid_file() {
    let _ = std::fs::write(PID_FILE, std::process::id().to_string());
}

pub fn cleanup_pid_file() {
    let _ = std::fs::remove_file(PID_FILE);
}
```

SIGTERM (15) al proceso `whisper-tts` anterior provoca que su hijo `afplay` reciba
SIGHUP y termine, porque `afplay` es lanzado con `Command::new("afplay").spawn()`
y Rust maneja la señal del padre.

**Rationale**: Sin dependencia `libc` ni `nix`. Usa `kill` CLI disponible en macOS.
El SIGTERM es suficiente: `afplay` como proceso hijo termina cuando su padre muere.

**Alternativas consideradas**: `libc::kill()` (requiere crate `libc`); pkill por nombre
(demasiado amplio, podría matar afplay de otras apps); lock file con espera activa
(complejo y propenso a deadlocks).

---

## 4. Coexistencia [lib] + binaries en Cargo (Rust)

**Decision**: Añadir `[lib]` a Cargo.toml con `src/lib.rs` que declara todos los módulos
compartidos. `main.rs` y `whisper-tts.rs` importan desde la librería.

**Cargo.toml**:
```toml
[lib]
name = "whisper_bar_rust"
path = "src/lib.rs"

[[bin]]
name = "whisper-bar-rust"
path = "src/main.rs"

[[bin]]
name = "whisper-tts"
path = "src/bin/whisper-tts.rs"
```

**src/lib.rs** (declara todos los módulos):
```rust
pub mod azure_transcriber;
pub mod config;
pub mod db;
pub mod defaults;
pub mod hotkey;
pub mod llm;
pub mod logger;
pub mod recorder;
pub mod settings_window;
pub mod transcriber;
pub mod tts;
```

**src/main.rs** (importa desde lib en lugar de declarar módulos):
```rust
use whisper_bar_rust::{
    azure_transcriber, config, db, defaults, hotkey, llm,
    logger, recorder, settings_window, transcriber
};
```

Todas las referencias `crate::defaults::X` dentro de los módulos seguirán funcionando
porque cuando la librería se compila, `crate::` refiere a `whisper_bar_rust`.

**Rationale**: Patrón estándar de Rust para proyectos con múltiples binaries. Sin
workspace separado. Mínimo impacto en código existente (solo la forma de declarar
los módulos en main.rs cambia).

**Alternativas consideradas**: Binary auto-contenido con código duplicado (viola DRY);
workspace separado (overhead innecesario para esta escala).

---

## 5. Limpieza de Markdown para TTS (FR-002)

**Decision**: Reemplazos simples de string sin regex crate adicional.

**Implementación**:
```rust
pub fn clean_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line
            .replace("**", "")
            .replace('*', "")
            .replace('`', "")
            .replace("~~", "");
        // Remover # al inicio de línea (headings)
        let line = if line.starts_with('#') {
            line.trim_start_matches('#').trim_start().to_string()
        } else {
            line
        };
        // Remover guiones de lista al inicio
        let line = if line.starts_with("- ") || line.starts_with("* ") {
            line[2..].to_string()
        } else {
            line
        };
        result.push_str(&line);
        result.push('\n');
    }
    result.trim().to_string()
}
```

**Rationale**: Sin dependencia `regex`. El texto de respuesta de Claude no requiere
parsing sofisticado — la limpieza básica es suficiente para TTS natural.
