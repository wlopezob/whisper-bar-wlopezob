# whisper-bar-wlopezob

App de barra de menú para macOS (Rust, edition 2024): dictado por voz (whisper-cli
local o Azure MAI Transcribe) con traducción opcional (Azure Translator u Ollama) y
lectura en voz alta de las respuestas de Claude Code / Codex (Gemini TTS o Qwen3-TTS
local) vía Stop hook. Visión general, arquitectura y claves de config: `README.md`.

## Comandos

```bash
cargo test                      # tests unitarios (no tocan ~/.config: DB en memoria)
cargo clippy --all-targets
bash bundle.sh                  # build release + instala .app, whisper-tts y hooks
tail -f ~/.config/whisperwlopezob/whisperwlopezob.log
```

Tras cada `bundle.sh` macOS revoca Accessibility: desactivar y reactivar el toggle.

## Dos binarios, una librería

- `src/main.rs` → `whisperwlopezob.app` (tray, hotkeys, grabación, pegado).
- `src/bin/whisper-tts.rs` → `~/.local/bin/whisper-tts`, invocado por
  `hooks/whisper-tts-stop.sh` (Claude) y `hooks/whisper-tts-codex-stop.sh` (Codex).
  Debe terminar siempre con exit 0 y nunca hacer panic: corre en segundo plano y un
  fallo es silencioso.

Ambos comparten la config a través de SQLite (`~/.config/whisperwlopezob/data.db`).

## Convenciones

- **Config**: toda clave nueva va en `src/settings.rs` (`Settings` + `load` + `save`)
  y en la ventana (`settings_window.rs`). No leer claves sueltas con `db.get` en otros
  sitios. El test `save_y_load_usan_las_mismas_claves` cubre la consistencia.
- **Recortar strings** con `text::truncate`, nunca `&s[..n]`: el texto suele ser
  español y partir un carácter multibyte hace panic.
- **API keys** en headers (`x-goog-api-key`, `Ocp-Apim-Subscription-Key`), nunca en la
  URL: los errores de reqwest incluyen la URL y se escriben en el log.
- **Reproducción**: usar `tts::play_wav` (registra el PID para que `⌘⌥R` y una
  respuesta nueva puedan pararla).
- Comentarios, logs y mensajes de error en español.
- `specs/` contiene la especificación original (Spec Kit, mayo 2026): es histórica, el
  TTS ya no usa Azure.
