# Implementation Plan: Azure TTS Core

**Branch**: `001-azure-tts-core` | **Date**: 2026-05-16 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-azure-tts-core/spec.md`

## Summary

Agregar síntesis de voz Azure TTS a whisperwlopezob mediante tres cambios integrados:
(A) nuevo módulo `src/tts.rs` con lógica de síntesis, limpieza de markdown, fallback a
`say` y gestión de PID para interrumpir reproducción activa; (B) nuevo binary
`src/bin/whisper-tts.rs` que lee config desde SQLite y llama a `tts::speak()`; (C)
bloque "LECTURA DE RESPUESTAS" en la ventana de Settings (condicional a Azure MAI).

Requiere añadir `src/lib.rs` para exponer módulos compartidos al nuevo binary, y
actualizar `main.rs` para importar desde la librería en lugar de declarar módulos
directamente.

## Technical Context

**Language/Version**: Rust edition 2024
**Primary Dependencies**: reqwest 0.12 blocking (ya en Cargo.toml), rusqlite 0.31 (ya existe), simplelog 0.12 (ya existe), objc2/AppKit (ya existe)
**Storage**: SQLite — keys nuevas: `tts_enabled`, `tts_voice`; reutiliza `azure_mai_key`, `azure_mai_region`
**Testing**: Manual — `cargo build --bin whisper-tts`, pipe de echo, inspección SQLite con `sqlite3`
**Target Platform**: macOS 13+ Apple Silicon e Intel
**Project Type**: Desktop app (main binary) + CLI binary independiente (whisper-tts)
**Performance Goals**: <3s desde texto hasta inicio de audio (SC-001); <11s en timeout Azure (FR-003 + SC-005)
**Constraints**: Exit 0 siempre (FR-006); log en append sin truncar (FR-014); interrumpir TTS previo al iniciar (FR-015)
**Scale/Scope**: Un usuario, invocaciones secuenciales desde Stop hook de Claude Code

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principio | Estado | Notas |
|-----------|--------|-------|
| I. Native macOS | ✅ Pass | Usa `afplay`/`say` (herramientas macOS). UI en NSPanel/AppKit existente. |
| II. Local-First | ✅ Pass | TTS usa Azure por elección explícita del usuario (spec Assumptions). El spec acota: "el TTS sí usa Azure porque es explícito del usuario". |
| III. Latency Minimization | ✅ Pass | Binary invocado async desde hook; event loop de la app no bloqueado. Timeout 10s en HTTP. |
| IV. Graceful Degradation | ✅ Pass | Fallback a `say` en cualquier fallo; exit 0 siempre; timeout 10s; PID kill antes de nueva síntesis. |
| V. Single Responsibility | ✅ Pass | `tts.rs` = síntesis únicamente; `whisper-tts.rs` = orquestación únicamente; `settings_window.rs`/`main.rs` reciben adiciones mínimas dentro de su scope. |
| VI. SQLite-Backed Config | ✅ Pass | `tts_enabled` y `tts_voice` en tabla `settings`. Sin archivos de config nuevos. |
| VII. Model Compatibility | N/A | TTS no usa modelos locales. |
| VIII. Minimal Permissions | ✅ Pass | Sin permisos nuevos. Red ya permitida por Azure MAI Transcribe. |

**Resultado: PASS** — sin violaciones. No requiere Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/001-azure-tts-core/
├── plan.md              ← este archivo
├── research.md          ← Phase 0
├── data-model.md        ← Phase 1
├── quickstart.md        ← Phase 1
├── contracts/
│   └── whisper-tts-cli.md
└── tasks.md             ← Phase 2 (/speckit-tasks, no generado aquí)
```

### Source Code (repository root)

```text
src/
├── lib.rs               ← NUEVO: expone todos los módulos compartidos
├── main.rs              ← MODIFICADO: importa desde lib; añade tts_enabled/tts_voice
├── tts.rs               ← NUEVO: speak(), clean_markdown(), fallback_say(), PID utils
├── defaults.rs          ← MODIFICADO: añade TTS_DEFAULT_VOICE = "es-MX-DaliaNeural"
├── settings_window.rs   ← MODIFICADO: bloque TTS, panel 800px, SettingsValues ampliado
├── db.rs                ← sin cambios (reutilizado vía lib)
├── azure_transcriber.rs ← sin cambios
├── config.rs            ← sin cambios
├── hotkey.rs            ← sin cambios
├── llm.rs               ← sin cambios
├── logger.rs            ← sin cambios
├── recorder.rs          ← sin cambios
└── transcriber.rs       ← sin cambios

src/bin/
└── whisper-tts.rs       ← NUEVO: entry point del binary CLI

Cargo.toml               ← MODIFICADO: añade [lib] + [[bin]] whisper-tts
```

**Structure Decision**: Proyecto Cargo único con librería (`src/lib.rs`) que expone todos
los módulos compartidos. El binary principal (`src/main.rs`) y el nuevo binary
(`src/bin/whisper-tts.rs`) importan desde la librería. Sin workspace separado.

## Complexity Tracking

> No hay violaciones de constitución que justificar.
