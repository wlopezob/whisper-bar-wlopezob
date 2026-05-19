---
description: "Task list for Azure TTS Core feature"
---

# Tasks: Azure TTS Core — Síntesis de Voz para Respuestas de Claude

**Input**: Design documents from `/specs/001-azure-tts-core/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/whisper-tts-cli.md ✅

**Tests**: No se incluyen tareas de test automatizado (no solicitado en spec). Las
validaciones son manuales según quickstart.md.

**Organization**: Tareas agrupadas por user story para implementación y test independiente.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Puede correr en paralelo (archivos distintos, sin dependencias)
- **[Story]**: A qué user story pertenece (US1, US2, US3)
- Rutas de archivo exactas en cada descripción

## Path Conventions

- Single Cargo project: `src/` at repository root
- New binary: `src/bin/whisper-tts.rs`
- Shared library: `src/lib.rs`

---

## Phase 1: Setup — Restructura Cargo (prerequisito para todo)

**Purpose**: Convertir el proyecto en una librería + binaries para que `whisper-tts`
pueda reutilizar los módulos existentes sin duplicar código.

**⚠️ CRÍTICO**: Nada del código TTS puede compilar hasta que esta fase esté completa.

- [X] T001 Añadir sección `[lib]` con `path = "src/lib.rs"` y `[[bin]]` `whisper-tts` con `path = "src/bin/whisper-tts.rs"` en `Cargo.toml` (mantener el `[[bin]]` implícito de `src/main.rs` añadiendo `[[bin]]` explícito con `name = "whisper-bar-rust"` y `path = "src/main.rs"`)
- [X] T002 Crear `src/lib.rs` con `pub mod` para los 11 módulos existentes (sin `tts` aún — se añade en T006 cuando exista el archivo): `azure_transcriber`, `config`, `db`, `defaults`, `hotkey`, `llm`, `logger`, `recorder`, `settings_window`, `transcriber`
- [X] T003 Actualizar `src/main.rs`: (a) reemplazar todas las declaraciones `mod X;` por `use whisper_bar_rust::{azure_transcriber, config, db, defaults, hotkey, llm, logger, recorder, settings_window, transcriber};`; (b) actualizar también las líneas `use X::{Y, Z}` del mismo archivo que referencian esos módulos — por ejemplo `use settings_window::{SettingsValues, show_settings_modal}` pasa a `use whisper_bar_rust::settings_window::{SettingsValues, show_settings_modal}` — el código dentro de las funciones (ej: `config::Config::new()`) no cambia
- [X] T004 [P] Añadir constante `pub const TTS_DEFAULT_VOICE: &str = "es-MX-DaliaNeural";` en `src/defaults.rs`
- [X] T005 Ejecutar `cargo build` y verificar que compila sin errores antes de continuar

**Checkpoint**: `cargo build` pasa ✅ — restructura lista.

---

## Phase 2: Foundational — Módulo src/tts.rs

**Purpose**: Toda la lógica de síntesis TTS en un único módulo. Requerido por US1 y US2.

**⚠️ CRÍTICO**: US1 y US2 dependen de este módulo completo.

- [X] T006 Crear `src/tts.rs` con función `pub fn clean_markdown(text: &str) -> String` que elimina `**`, `*`, `` ` ``, `~~`, `#` al inicio de línea (headings), y guiones/asteriscos de lista (`- `, `* `) al inicio de línea; también añadir `pub mod tts;` al final de `src/lib.rs` (ahora que el archivo existe, el compilador puede resolver el módulo)
- [X] T007 Añadir `fn fallback_say(text: &str)` en `src/tts.rs` que invoca `say` via `std::process::Command::new("say").args(["-v", "Paulina", text]).status()` — cada argumento como elemento separado (nunca interpolado en string de shell para evitar inyección); si `Paulina` no está disponible el comando falla silenciosamente; silencioso ante cualquier error (`.ok()` en el resultado)
- [X] T008 Añadir `pub fn init_append()` en `src/logger.rs` (no en tts.rs — Principio V: logging es concern de logger.rs) que inicializa `simplelog::WriteLogger` con `OpenOptions::append(true)` (sin truncar) apuntando a `~/.config/whisperwlopezob/whisperwlopezob.log` — llamar `.ok()` para ignorar si ya hay logger inicializado; diferencia clave con `init()`: no llama a `truncate(true)`
- [X] T009 Añadir funciones PID en `src/tts.rs`: `pub fn kill_previous_instance()` (lee `/tmp/whisper-tts.pid`, envía `kill -15 {pid}` via Command, espera 300ms), `pub fn write_pid_file()` (escribe `std::process::id()` en `/tmp/whisper-tts.pid`), `pub fn cleanup_pid_file()` (elimina `/tmp/whisper-tts.pid`)
- [X] T010 Añadir `fn call_azure_tts(text: &str, voice: &str, key: &str, region: &str) -> Result<Vec<u8>, String>` en `src/tts.rs`: POST a `https://{region}.tts.speech.microsoft.com/cognitiveservices/v1`, headers `Ocp-Apim-Subscription-Key`, `Content-Type: application/ssml+xml`, `X-Microsoft-OutputFormat: audio-16khz-128kbitrate-mono-mp3`, body SSML con `xml:lang` extraído del nombre de voz via `voice.splitn(3, '-').take(2).collect::<Vec<_>>().join("-")` con fallback a `"es-MX"` si el resultado tiene menos de 2 segmentos o está vacío (ej: `es-MX-DaliaNeural` → `es-MX`; `InvalidVoice` → `es-MX`), timeout 10s via `reqwest::blocking::Client`
- [X] T011 Añadir `pub fn speak(text: &str, voice: &str, key: &str, region: &str)` en `src/tts.rs`: (1) `log::info!` síntesis iniciada, (2) `clean_markdown`, (3) truncar a 5000 chars, (4) si key/region vacíos → `fallback_say` + return, (5) `call_azure_tts` → en éxito: escribir MP3 a `/tmp/whisper-tts-audio.mp3`, reproducir con `afplay` via Command, borrar tmp, `log::info!` éxito; en error: `log::error!` motivo + `fallback_say`; (6) `log::info!` o `log::error!` según resultado

**Checkpoint**: `cargo build` pasa con `tts.rs` compilado ✅

---

## Phase 3: User Story 1 — Reproducción CLI (Priority: P1) 🎯 MVP

**Goal**: El binario `whisper-tts` recibe texto por stdin y reproduce audio.

**Independent Test**: `echo "La compilación fue exitosa." | ./target/debug/whisper-tts`
reproduce audio con voz Azure; `echo $?` devuelve 0.

### Implementation for User Story 1

- [X] T012 [US1] Crear `src/bin/whisper-tts.rs`: (1) llamar `whisper_bar_rust::logger::init_append()`, (2) leer stdin completo hasta EOF, (3) si vacío → `log::info!("TTS: stdin vacío, omitiendo")` + `std::process::exit(0)`, (4) abrir DB con `whisper_bar_rust::db::Db::open()`, (5) leer `tts_enabled`, `tts_voice`, `azure_mai_key`, `azure_mai_region`, (6) si `tts_enabled != "true"` → `log::info!("TTS: desactivado (tts_enabled=false), omitiendo")` + exit(0), (7) `tts::kill_previous_instance()`, (8) `tts::write_pid_file()`, (9) `tts::speak(&text, &voice, &key, &region)`, (10) `tts::cleanup_pid_file()`, (11) `std::process::exit(0)`
- [X] T013 [US1] Ejecutar `cargo build --bin whisper-tts` y verificar que compila sin errores
- [ ] T014 [US1] Validación manual US1: configurar DB (`tts_enabled=true`, `tts_voice=es-MX-DaliaNeural`, key y región válidos via `sqlite3`), ejecutar `echo "La compilación fue exitosa." | ./target/debug/whisper-tts`, verificar audio reproducido y `echo $?` = 0

**Checkpoint**: User Story 1 funcional — `whisper-tts` reproduce audio desde stdin ✅

---

## Phase 4: User Story 2 — Fallback Automático (Priority: P2)

**Goal**: El binario termina con exit 0 y reproduce audio local cuando Azure no está disponible.

**Independent Test**: `sqlite3 ~/.config/whisperwlopezob/data.db "UPDATE settings SET value='' WHERE key='azure_mai_key';"` seguido de `echo "texto" | ./target/debug/whisper-tts` — debe sonar con `say`, exit 0.

### Implementation for User Story 2

*(La lógica de fallback está en `tts.rs` Phase 2 — esta fase valida los escenarios)*

- [ ] T015 [US2] Validación fallback por credenciales vacías: `sqlite3` → `azure_mai_key=""` → `echo "Fallback test" | ./target/debug/whisper-tts` → verificar `say` suena, exit 0, log muestra "fallback" con motivo
- [ ] T016 [US2] Validación fallback por timeout: `sqlite3` → `azure_mai_region="invalid-xyz-region-404"` → `echo "Timeout test" | ./target/debug/whisper-tts` → verificar termina en ≤11s, `say` suena, exit 0
- [ ] T017 [US2] Validación log append: mientras la app principal corre (`open /Applications/whisperwlopezob.app`), ejecutar `echo "log test" | ./target/debug/whisper-tts` y verificar con `tail -20 ~/.config/whisperwlopezob/whisperwlopezob.log` que las entradas TTS se añaden sin borrar las entradas previas de la app

**Checkpoint**: User Stories 1 y 2 funcionan independientemente ✅

---

## Phase 5: User Story 3 — Settings UI (Priority: P3)

**Goal**: El usuario configura TTS desde la ventana nativa de Configuración.

**Independent Test**: Abrir app → Configuración → seleccionar "Azure MAI" → verificar
sección "LECTURA DE RESPUESTAS" visible → cambiar voz → Aplicar → `sqlite3` verifica
`tts_enabled=true` y `tts_voice=<voz ingresada>`.

### Implementation for User Story 3

- [X] T018 [US3] Añadir `pub tts_enabled: bool` y `pub tts_voice: String` al struct `SettingsValues` en `src/settings_window.rs`
- [X] T019 [US3] Añadir campos al struct `AzureFieldPtrs` en `src/settings_window.rs`: `chk_tts: *const NSButton`, `tf_tts_voice: *const NSTextField`, `lbl_tts_section: *const NSTextField`, `lbl_tts_voice: *const NSTextField`
- [X] T020 [US3] Actualizar `set_azure_fields_hidden()` en `src/settings_window.rs` para ocultar/mostrar los 4 nuevos controles TTS junto con el resto de campos Azure
- [X] T021 [US3] Expandir el NSPanel de 730px a 800px en `show_settings_modal()` en `src/settings_window.rs` y desplazar +70px los y-coords de TODAS las secciones: TRANSCRIPCIÓN header (680→750), label Idioma (655→725), segmento idioma (650→720), AZURE MAI header (608→678), label Backend (583→653), segmento backend (578→648), label API Key (550→620), tf_key (547→617), label Región (518→588), tf_region (515→585), label hint región (518→588), label API Version (486→556), tf_api_version (483→553), label Definition (454→524), scroll_definition (397→467), MEJORA GRAMATICAL header (388→458), checkbox grammar (361→431), label Modelo (331→401), popup_model (328→398), label Prompt ES (301→371), scroll_prompt_es (236→306), label Prompt EN (209→279), scroll_prompt_en (144→214), TRADUCCIÓN header (106→176), checkbox translate (79→149), label Idioma destino (49→119), popup_dest (46→116); botones Cancelar/Aplicar mantienen y=15 (anclan al fondo)
- [X] T022 [US3] **Prerequisito: T021 completado.** Añadir controles TTS en `show_settings_modal()` en `src/settings_window.rs` entre Definition JSON y MEJORA GRAMATICAL usando y-coords del nuevo layout 800px: label "LECTURA DE RESPUESTAS" (y=440, w=380), `NSButton::checkboxWithTitle` "Leer respuestas de Claude" (y=415, w=280, estado inicial = `current.tts_enabled`), label "Voz:" (y=388, w=40), `NSTextField` editable `tf_tts_voice` (x=65, y=385, w=335, valor inicial = `current.tts_voice` o `defaults::TTS_DEFAULT_VOICE` si vacío)
- [X] T023 [US3] Registrar punteros de controles TTS en `AZURE_FIELDS` thread_local dentro de `show_settings_modal()` en `src/settings_window.rs`
- [X] T024 [US3] Leer estado de controles TTS al cierre con Aplicar en `show_settings_modal()` en `src/settings_window.rs`: `tts_enabled = chk_tts.state() == NSControlStateValueOn`, `tts_voice = tf_tts_voice.stringValue().trim()` (si vacío usar `defaults::TTS_DEFAULT_VOICE`); incluir en `SettingsValues` retornado
- [X] T025 [US3] Añadir `tts_enabled: Arc<Mutex<bool>>` y `tts_voice: Arc<Mutex<String>>` al struct `WhisperApp` en `src/main.rs`
- [X] T026 [US3] Cargar `tts_enabled` y `tts_voice` desde DB en `WhisperApp::new()` en `src/main.rs` siguiendo el patrón de los campos `azure_mai_*` existentes
- [X] T027 [US3] Guardar `tts_enabled` y `tts_voice` a DB en el handler de settings apply en `src/main.rs` — mismo patrón que el guardado de `azure_mai_*`
- [X] T028 [US3] Añadir `log::info!` para estado TTS al iniciar en `src/main.rs`, siguiendo el patrón de la línea de Azure MAI (`log::info!("TTS: {} voz={}", if tts_enabled {"✅ activo"} else {"☐ inactivo"}, tts_voice)`)
- [X] T029 [US3] Ejecutar `cargo build` (app completa) y verificar compilación sin errores
- [ ] T030 [US3] Validación manual US3: `bash bundle.sh` → `open /Applications/whisperwlopezob.app` → Configuración → cambiar backend a "Azure MAI" → verificar que aparece "LECTURA DE RESPUESTAS" → cambiar a "Whisper local" → verificar que desaparece → volver a "Azure MAI" → activar checkbox → escribir voz distinta → Aplicar → verificar DB: `sqlite3 ~/.config/whisperwlopezob/data.db "SELECT key,value FROM settings WHERE key LIKE 'tts%';"`

**Checkpoint**: Todas las User Stories (1, 2, 3) funcionan independientemente ✅

---

## Phase Final: Polish & Cross-Cutting Concerns

**Purpose**: Validaciones de comportamientos transversales (SC-006, FR-015)

- [ ] T031 [P] Validar SC-006 — concurrencia interrumpir-y-reemplazar (FR-015): Terminal 1: `python3 -c "print('texto ' * 300)" | ./target/debug/whisper-tts &` → Terminal 2 (2s después): `echo "Nuevo texto" | ./target/debug/whisper-tts` → verificar que el primer audio se corta y suena el segundo
- [ ] T032 [P] Validar tts_enabled=false silencioso: `sqlite3` → `tts_enabled=false` → `echo "No debe sonar" | ./target/debug/whisper-tts` → verificar silencio total, exit 0, entrada en log de omisión
- [ ] T033 Ejecutar todos los escenarios de `specs/001-azure-tts-core/quickstart.md` y verificar que cada uno pasa

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Sin dependencias — empezar inmediatamente. T001→T002→T003 secuenciales; T004 paralelo con T002.
- **Foundational (Phase 2)**: Depende de Phase 1 completa. T006→T007→T008→T009→T010→T011 secuenciales (mismo archivo).
- **US1 (Phase 3)**: Depende de Phase 2 completa.
- **US2 (Phase 4)**: Depende de US1 completa (T015-T017 son validaciones, no nueva implementación).
- **US3 (Phase 5)**: Depende de Phase 1. Puede empezar en paralelo con Phase 2 (archivos distintos: settings_window.rs, main.rs).
- **Polish (Phase Final)**: Depende de US1, US2, US3 completas.

### User Story Dependencies

- **US1 (P1)**: Puede empezar tras Phase 2 — sin dependencias de US2/US3
- **US2 (P2)**: Puede empezar tras US1 — no requiere US3
- **US3 (P3)**: Puede empezar tras Phase 1 — independiente de US1/US2 en implementación

### Within Each Phase

- En Phase 2, T006-T011 son secuenciales (mismo archivo `tts.rs`)
- En Phase 5, T018-T024 son secuenciales (`settings_window.rs`); T025-T028 son secuenciales (`main.rs`); T029 después de ambos grupos

### Parallel Opportunities

- T004 [P] puede hacerse junto con T002
- T015-T017 [US2] pueden hacerse en cualquier orden (son tests independientes)
- T031-T032 [P] pueden hacerse en paralelo
- US3 (Phase 5) puede iniciarse en paralelo con Phase 2 por un segundo desarrollador

---

## Implementation Strategy

### MVP First (User Story 1 — el binary funciona)

1. Completar Phase 1: Setup (T001–T005)
2. Completar Phase 2: Foundational — `src/tts.rs` (T006–T011)
3. Completar Phase 3: US1 — binary `whisper-tts` (T012–T014)
4. **STOP y VALIDAR**: `echo "texto" | ./target/debug/whisper-tts` → audio suena
5. Continuar con US2 y US3

### Incremental Delivery

1. Setup + Foundational + US1 → `whisper-tts` CLI funciona (**MVP**)
2. US2 → fallback validado → sistema robusto
3. US3 → configuración desde UI → experiencia de usuario completa
4. Polish → comportamientos transversales validados

---

## Notes

- [P] = diferentes archivos, sin dependencias bloqueantes
- [Story] mapea cada tarea a su user story para trazabilidad
- Cada user story es completable y testeable de forma independiente
- No hay tests automatizados en esta fase (no solicitado en spec)
- Hacer `cargo build` después de T003 (Phase 1) y después de T011 (Phase 2) como checkpoints intermedios
- El tag `⚠️ CRÍTICO` en cada checkpoint indica que no se puede avanzar sin pasar ese punto
