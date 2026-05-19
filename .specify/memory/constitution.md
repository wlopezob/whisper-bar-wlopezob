<!--
## Sync Impact Report
- Version change: N/A (initial fill) → 1.0.0
- Modified principles: All 8 replaced from template placeholders
  (template had 5 slots; expanded to 8 to match project principles)
- Added sections: Technology Stack, Development Workflow, Governance
- Removed sections: None (template placeholders fully resolved)
- Templates requiring updates:
  - ✅ .specify/memory/constitution.md (this file — initial population)
  - ✅ .specify/templates/plan-template.md (Constitution Check section is
       a per-feature dynamic gate — no static update required)
  - ✅ .specify/templates/spec-template.md (no constitution references — no changes needed)
  - ✅ .specify/templates/tasks-template.md (no constitution references — no changes needed)
- Follow-up TODOs: None — all placeholders resolved
-->

# whisperwlopezob Constitution

## Core Principles

### I. Native macOS

The app MUST be built exclusively with native macOS system frameworks (AppKit via
objc2, CoreAudio via cpal). Web views, Electron, Qt, or any cross-platform UI
framework are PROHIBITED. UI components MUST use NSPanel, NSButton, NSTextField, and
other AppKit primitives directly.

**Rationale**: macOS-native rendering, tray integration, and permission APIs require
direct AppKit access. Cross-platform layers introduce latency and behavioral
inconsistencies in system-level features (hotkeys, clipboard, accessibility).

### II. Local-First

All core processing — audio recording, transcription, grammar correction, and
translation — MUST execute on the user's machine. No cloud API calls are permitted
for these functions. Network access is PROHIBITED in the recording, transcription,
and LLM pipelines.

**Rationale**: User voice data is sensitive. Local execution guarantees privacy,
works offline, and eliminates per-use API costs and latency.

### III. Latency Minimization

The hotkey→record→transcribe→paste pipeline MUST be optimized for speed. All
blocking subprocess calls (whisper-cli, llama-completion) MUST run in dedicated
threads. The main event loop MUST remain unblocked at all times. Artificial delays
are PROHIBITED except for the 300ms clipboard restore (required to avoid clipboard
race conditions).

**Rationale**: The app is used mid-sentence in a conversation flow. Perceived
slowness breaks the dictation habit and makes the tool feel unreliable.

### IV. Graceful Degradation

Every external subprocess call MUST have an enforced timeout: whisper-cli at 60s,
llama-completion at 30s. On timeout or failure, the pipeline MUST fall back to the
last valid text (transcription without correction/translation). The user's original
clipboard MUST always be restored regardless of pipeline outcome.

**Rationale**: LLM and transcription models can stall under resource pressure. A
hung operation MUST never lock the UI or discard the user's dictated content.

### V. Single Responsibility per Module

Each `.rs` source file MUST own exactly one concern: `recorder.rs` records audio,
`transcriber.rs` invokes whisper-cli, `llm.rs` handles LLM post-processing,
`db.rs` manages persistence, and so on. Cross-cutting logic MUST NOT be embedded
inside a module that owns a different concern.

**Rationale**: The architecture defined in README.md is authoritative. Isolated
responsibilities make each subsystem independently replaceable — e.g., swapping
whisper-cli for a future native library should touch only `transcriber.rs`.

### VI. SQLite-Backed Configuration

All persistent configuration (language, LLM model path, toggle states) MUST be
stored in `~/.config/whisperwlopezob/data.db` as key-value rows in the `settings`
table. Plain-text config files (TOML, JSON, YAML, INI) are PROHIBITED.

**Rationale**: SQLite provides atomic writes, eliminates parse errors, and is
already bundled via `rusqlite` (no added dependency). A single storage format
reduces failure modes and simplifies migrations.

### VII. Model Compatibility

The app MUST auto-detect and work with whisper models (`.bin`) and LLM models
(`.gguf`) already present in the user's config directories. The app MUST NOT force
downloads or pin a specific model filename. Whisper model selection MUST follow the
documented priority order: large-v3 → large-v2 → medium → small → base → tiny.

**Rationale**: Users may have invested time downloading large models. Forcing
re-downloads or requiring a specific version degrades trust and adds friction.

### VIII. Minimal Permissions

The app MUST request only two macOS permissions: Accessibility (for global hotkey
registration and ⌘V paste simulation) and Microphone (for audio capture). No other
entitlements, background services, or network permissions are permitted.

**Rationale**: Unnecessary permissions erode user trust and may trigger Gatekeeper
or notarization issues. The two listed permissions are the minimum required to
deliver core functionality.

## Technology Stack

- **Language**: Rust (edition 2024)
- **Platform**: macOS 13+ (Apple Silicon and Intel)
- **Audio**: `cpal` 0.17 + `hound` 3.5 (16kHz mono PCM WAV)
- **Transcription**: `whisper-cli` subprocess (whisper-cpp via Homebrew)
- **LLM**: `llama-completion` subprocess (llama.cpp via Homebrew)
- **UI**: `tray-icon` 0.21 + `objc2`/`objc2-app-kit` 0.3 (native NSPanel dialogs)
- **Persistence**: `rusqlite` 0.31 with bundled SQLite
- **Hotkey**: `global-hotkey` 0.7 driven by `winit` 0.30 event loop
- **Clipboard/Paste**: `arboard` 3 + `enigo` 0.6
- **Logging**: `simplelog` 0.12 (file + console dual output)
- **Build/Deploy**: `bundle.sh` → `/Applications/whisperwlopezob.app`

New dependencies MUST be justified against crates already in scope. Prefer extending
existing integrations over introducing new ones.

## Development Workflow

- All changes MUST compile cleanly with `cargo build` before commit.
- Architecture violations (principle breaches) MUST be flagged in PR review.
- Module boundaries from the README architecture map are authoritative; deviations
  MUST be documented in the Complexity Tracking section of the feature's `plan.md`.
- Every new feature MUST preserve the timeout + fallback behavior (Principle IV).
- UI changes MUST be manually verified on macOS before merging (AppKit has no
  headless CI path).
- After each `bash bundle.sh` rebuild, the Accessibility permission toggle MUST be
  manually re-enabled — this MUST be noted in any PR that changes binary output.

## Governance

This constitution supersedes all other development guidelines for whisperwlopezob.
Amendments require: (1) a rationale referencing a concrete problem, (2) a version
bump to this file following the rules below, and (3) propagation to dependent
templates.

- **MAJOR** bump: removing or redefining an existing principle.
- **MINOR** bump: adding a new principle or materially expanding guidance.
- **PATCH** bump: clarifications, wording refinements, or typo fixes.

All feature plans (`plan.md`) MUST include a Constitution Check section that
identifies the principles most relevant to the feature before Phase 0 research.

**Version**: 1.0.0 | **Ratified**: 2026-05-16 | **Last Amended**: 2026-05-16
