# Guía de Implementación y Pruebas — Azure TTS Core

**Feature**: `001-azure-tts-core` | **Branch**: `001-azure-tts-core`
**Documentos de referencia**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

---

## Visión General

Esta feature añade síntesis de voz al proyecto `whisperwlopezob` mediante tres
componentes integrados:

| Componente | Archivo | Qué hace |
|------------|---------|----------|
| Módulo TTS | `src/tts.rs` | Sintetiza texto con Azure Speech TTS + fallback a `say` |
| Binary CLI | `src/bin/whisper-tts.rs` | Recibe texto por stdin, lee config de SQLite, llama a tts |
| Settings UI | `src/settings_window.rs` | Permite activar TTS y elegir voz desde la app |
| Logger append | `src/logger.rs` | Nueva función `init_append()` para log sin truncar |
| Librería | `src/lib.rs` | Expone módulos compartidos al binary `whisper-tts` |

---

## Parte 1: Cómo Construir la Implementación

### Orden de implementación

```
Phase 1: Setup Cargo (T001–T005)
  ↓
Phase 2: src/tts.rs + src/logger.rs (T006–T011)
  ↓
Phase 3: src/bin/whisper-tts.rs — MVP (T012–T014)
  ↓
Phase 4: Validación fallback (T015–T017)
  ↓ (en paralelo con Phase 2 si hay 2 personas)
Phase 5: Settings UI (T018–T030)
  ↓
Phase Final: Validaciones transversales (T031–T033)
```

---

### Phase 1 — Restructura Cargo

**Objetivo**: Hacer que `whisper-tts` pueda usar los módulos existentes sin duplicar código.

#### 1.1 Modificar `Cargo.toml`

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

#### 1.2 Crear `src/lib.rs`

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

#### 1.3 Actualizar `src/main.rs`

Reemplazar todas las declaraciones `mod X;` por importaciones desde la librería:

```rust
use whisper_bar_rust::{
    azure_transcriber, config, db, defaults, hotkey, llm,
    logger, recorder, settings_window, transcriber,
};
```

#### 1.4 Añadir constante en `src/defaults.rs`

```rust
pub const TTS_DEFAULT_VOICE: &str = "es-MX-DaliaNeural";
```

#### 1.5 Checkpoint

```bash
cargo build
# Debe compilar sin errores antes de continuar
```

---

### Phase 2 — Módulo TTS y Logger

#### 2.1 `src/logger.rs` — añadir `init_append()`

```rust
/// Inicializa logger en modo append (sin truncar).
/// Usar en binaries secundarios que no deben borrar el log de la app principal.
pub fn init_append() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{}/{}/whisperwlopezob.log", home, defaults::APP_CONFIG_DIR);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("No se pudo abrir el log para append");
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        file,
    ).ok();
}
```

#### 2.2 `src/tts.rs` — estructura del módulo

```rust
// src/tts.rs

// ── Constantes ────────────────────────────────────────────────────────────────
const PID_FILE: &str = "/tmp/whisper-tts.pid";
const AUDIO_TMP: &str = "/tmp/whisper-tts-audio.mp3";

// ── Limpieza de texto ─────────────────────────────────────────────────────────
pub fn clean_markdown(text: &str) -> String { ... }

// ── Fallback local ────────────────────────────────────────────────────────────
fn fallback_say(text: &str) {
    // ✅ CORRECTO: args separados, nunca interpolados en string de shell
    std::process::Command::new("say")
        .args(["-v", "Paulina", text])
        .status()
        .ok();
}

// ── Gestión de PID ────────────────────────────────────────────────────────────
pub fn kill_previous_instance() {
    if let Ok(pid_str) = std::fs::read_to_string(PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if pid != std::process::id() {
                let _ = std::process::Command::new("kill")
                    .args(["-15", &pid.to_string()])
                    .status();
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

// ── Llamada HTTP a Azure TTS ──────────────────────────────────────────────────
fn call_azure_tts(text: &str, voice: &str, key: &str, region: &str) -> Result<Vec<u8>, String> {
    // Extraer xml:lang del nombre de la voz
    // "es-MX-DaliaNeural" → "es-MX"
    // Si la voz tiene menos de 2 segmentos → fallback "es-MX"
    let lang: String = {
        let parts: Vec<&str> = voice.splitn(3, '-').collect();
        if parts.len() >= 2 {
            format!("{}-{}", parts[0], parts[1])
        } else {
            "es-MX".to_string()
        }
    };

    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{lang}'>\
         <voice name='{voice}'>{text}</voice></speak>"
    );

    let url = format!(
        "https://{region}.tts.speech.microsoft.com/cognitiveservices/v1"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let resp = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", key)
        .header("Content-Type", "application/ssml+xml")
        .header("X-Microsoft-OutputFormat", "audio-16khz-128kbitrate-mono-mp3")
        .body(ssml)
        .send()
        .map_err(|e| format!("Error de red: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Azure TTS error HTTP {}", resp.status().as_u16()));
    }

    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("Error leyendo respuesta: {}", e))
}

// ── Función pública principal ─────────────────────────────────────────────────
pub fn speak(text: &str, voice: &str, key: &str, region: &str) {
    log::info!("TTS: síntesis iniciada — voz={} chars={}", voice, text.len());
    let clean = clean_markdown(text);
    let clean = if clean.len() > 5000 { &clean[..5000] } else { &clean };
    if key.is_empty() || region.is_empty() {
        log::info!("TTS: fallback a say (credenciales vacías)");
        fallback_say(clean);
        return;
    }
    match call_azure_tts(clean, voice, key, region) {
        Ok(bytes) => {
            std::fs::write(AUDIO_TMP, &bytes).ok();
            let status = std::process::Command::new("afplay")
                .arg(AUDIO_TMP)
                .status();
            let _ = std::fs::remove_file(AUDIO_TMP);
            match status {
                Ok(_) => log::info!("TTS: audio reproducido correctamente"),
                Err(e) => log::error!("TTS: error en afplay: {}", e),
            }
        }
        Err(e) => {
            log::error!("TTS: error Azure, fallback a say — {}", e);
            fallback_say(clean);
        }
    }
}
```

#### 2.3 Checkpoint

```bash
cargo build
# tts.rs y logger.rs deben compilar sin errores
```

---

### Phase 3 — Binary `whisper-tts`

#### 3.1 Crear `src/bin/whisper-tts.rs`

```rust
use std::io::Read;
use whisper_bar_rust::{db, defaults, logger, tts};

fn main() {
    logger::init_append();

    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).ok();
    let text = text.trim().to_string();

    if text.is_empty() {
        log::info!("TTS: stdin vacío, omitiendo");
        std::process::exit(0);
    }

    let db = match db::Db::open() {
        Ok(d) => d,
        Err(e) => {
            log::error!("TTS: no se pudo abrir DB: {}", e);
            std::process::exit(0);
        }
    };

    let tts_enabled = db.get("tts_enabled", "false") == "true";
    if !tts_enabled {
        log::info!("TTS: desactivado (tts_enabled=false), omitiendo");
        std::process::exit(0);
    }

    let voice   = db.get("tts_voice",      defaults::TTS_DEFAULT_VOICE);
    let key     = db.get("azure_mai_key",   "");
    let region  = db.get("azure_mai_region","");

    tts::kill_previous_instance();
    tts::write_pid_file();
    tts::speak(&text, &voice, &key, &region);
    tts::cleanup_pid_file();

    std::process::exit(0);
}
```

#### 3.2 Compilar

```bash
cargo build --bin whisper-tts
```

---

### Phase 5 — Settings UI (resumen de cambios)

| Archivo | Cambio |
|---------|--------|
| `src/settings_window.rs` | Añadir `tts_enabled: bool`, `tts_voice: String` a `SettingsValues` |
| `src/settings_window.rs` | Añadir 4 punteros TTS a `AzureFieldPtrs` |
| `src/settings_window.rs` | `set_azure_fields_hidden()` muestra/oculta controles TTS |
| `src/settings_window.rs` | Panel 730px → 800px; desplazar todos los y-coords +70px |
| `src/settings_window.rs` | Bloque "LECTURA DE RESPUESTAS" en y=440/415/388/385 |
| `src/main.rs` | `tts_enabled` y `tts_voice` en `WhisperApp` + carga desde DB + guarda al Aplicar |
| `src/defaults.rs` | `TTS_DEFAULT_VOICE = "es-MX-DaliaNeural"` |

Ver coordenadas exactas en [tasks.md T021 y T022](../tasks.md).

---

## Parte 2: Cómo Probar la Implementación

### Preparación común

```bash
# Configurar DB de prueba
sqlite3 ~/.config/whisperwlopezob/data.db <<'SQL'
INSERT OR REPLACE INTO settings VALUES
  ('tts_enabled',    'true'),
  ('tts_voice',      'es-MX-DaliaNeural'),
  ('azure_mai_key',  'TU_API_KEY_AQUI'),
  ('azure_mai_region','eastus');
SQL
```

---

### Prueba 1 — Flujo básico Azure TTS ✅ SC-001, FR-001

```bash
echo "La compilación fue exitosa. Se modificaron tres archivos." \
  | ./target/debug/whisper-tts
```

**Esperado**: Voz de Dalia suena en el altavoz en menos de 3 segundos. Exit 0.

---

### Prueba 2 — Limpieza de markdown ✅ FR-002, SC-003

```bash
printf "## Resultado\n**Error**: el archivo \`main.rs\` no existe\n- Línea 42\n" \
  | ./target/debug/whisper-tts
```

**Esperado**: Audio suena natural sin leer `##`, `**`, backticks ni guiones de lista.

---

### Prueba 3 — TTS desactivado ✅ FR-008, SC-002

```bash
sqlite3 ~/.config/whisperwlopezob/data.db \
  "UPDATE settings SET value='false' WHERE key='tts_enabled';"
echo "Este texto no debe sonar" | ./target/debug/whisper-tts
echo "Exit code: $?"
```

**Esperado**: Silencio total. Exit code 0. Log muestra "omitiendo".

---

### Prueba 4 — Stdin vacío ✅ FR-009

```bash
echo "" | ./target/debug/whisper-tts
echo "Exit code: $?"
```

**Esperado**: Sin audio. Exit code 0. Log muestra "stdin vacío".

---

### Prueba 5 — Fallback por credenciales vacías ✅ FR-005, US2

```bash
sqlite3 ~/.config/whisperwlopezob/data.db <<'SQL'
UPDATE settings SET value='true'  WHERE key='tts_enabled';
UPDATE settings SET value=''      WHERE key='azure_mai_key';
SQL
echo "Prueba de fallback" | ./target/debug/whisper-tts
```

**Esperado**: Voz de Paulina (español macOS) reproduce el texto. Exit 0.
Log muestra "fallback a say (credenciales vacías)".

---

### Prueba 6 — Fallback por timeout ✅ FR-003, SC-005

```bash
sqlite3 ~/.config/whisperwlopezob/data.db <<'SQL'
UPDATE settings SET value='true'             WHERE key='tts_enabled';
UPDATE settings SET value='TU_KEY'           WHERE key='azure_mai_key';
UPDATE settings SET value='invalid-xyz-404'  WHERE key='azure_mai_region';
SQL
time echo "Prueba de timeout" | ./target/debug/whisper-tts
```

**Esperado**: Termina en ≤ 11 segundos. Paulina reproduce el texto. Exit 0.
Log muestra error Azure con motivo + "fallback a say".

---

### Prueba 7 — Log append (sin borrar log previo) ✅ FR-014, SC-006

```bash
# Terminal 1: monitorear log en tiempo real mientras la app corre
open /Applications/whisperwlopezob.app
tail -f ~/.config/whisperwlopezob/whisperwlopezob.log &

# Terminal 2: invocar TTS
sqlite3 ~/.config/whisperwlopezob/data.db "UPDATE settings SET value='true' WHERE key='tts_enabled';"
echo "Prueba de log" | ./target/debug/whisper-tts
```

**Esperado**: Las entradas TTS aparecen en el log SIN que desaparezcan las entradas
previas de la app principal. El log sigue creciendo, no se trunca.

---

### Prueba 8 — Concurrencia: interrumpir y reemplazar ✅ FR-015

```bash
# Restaurar credenciales válidas primero
# Terminal 1: texto largo para que tarde en reproducir
python3 -c "print('Esta es una frase muy larga. ' * 50)" \
  | ./target/debug/whisper-tts &

# Terminal 2: 2 segundos después, texto nuevo (debe interrumpir al anterior)
sleep 2 && echo "Nuevo mensaje que interrumpe al anterior" \
  | ./target/debug/whisper-tts
```

**Esperado**: El primer audio se corta y se escucha el segundo. No hay solapamiento.

---

### Prueba 9 — Voice format inesperado ✅ FR-013 (Obs 3)

```bash
sqlite3 ~/.config/whisperwlopezob/data.db \
  "UPDATE settings SET value='InvalidVoiceName' WHERE key='tts_voice';"
echo "Prueba con voz inválida" | ./target/debug/whisper-tts
```

**Esperado**: Azure puede fallar (401/400) → fallback a Paulina reproduce el texto.
Exit 0. Log muestra el error Azure con motivo.

---

### Prueba 10 — Settings UI ✅ FR-011, FR-012, US3

```bash
bash bundle.sh && open /Applications/whisperwlopezob.app
```

1. Clic en icono tray → **Configuración...**
2. Cambiar Backend a **Azure MAI** → verificar que aparece sección **LECTURA DE RESPUESTAS**
3. Cambiar a **Whisper local** → verificar que desaparece
4. Volver a **Azure MAI** → activar checkbox → cambiar voz a `en-US-JennyNeural` → **Aplicar**
5. Verificar en DB:

```bash
sqlite3 ~/.config/whisperwlopezob/data.db \
  "SELECT key, value FROM settings WHERE key LIKE 'tts%';"
```

**Esperado**:
```
tts_enabled|true
tts_voice|en-US-JennyNeural
```

6. Pulsar **Cancelar** → verificar que los valores en DB no cambiaron respecto al Aplicar anterior.

---

### Tabla de cobertura

| Prueba | FR/SC cubiertos | Verifica |
|--------|-----------------|---------|
| 1 | FR-001, FR-004, SC-001 | Flujo completo Azure → afplay |
| 2 | FR-002, SC-003 | Markdown limpio en audio |
| 3 | FR-008, SC-002 | TTS desactivado → silencio + exit 0 |
| 4 | FR-009, SC-002 | Stdin vacío → silencio + exit 0 |
| 5 | FR-005, FR-006 | Fallback Paulina sin credenciales |
| 6 | FR-003, FR-005, SC-005 | Timeout 10s + fallback |
| 7 | FR-014, SC-006 | Log append sin truncar |
| 8 | FR-015 | Interrupt-and-replace PID |
| 9 | FR-010, FR-013 | Voice inválida → fallback |
| 10 | FR-011, FR-012, US3 | Settings UI muestra/oculta/guarda |
