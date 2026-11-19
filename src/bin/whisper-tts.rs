// src/bin/whisper-tts.rs
// Binary CLI: recibe JSON {user, assistant} por stdin, lee config de SQLite,
// formatea con LLM si es necesario y reproduce via Gemini TTS.
// Flujo: {user, assistant} → [needs_formatter?] → [formatter LLM] → Gemini TTS → afplay
// Invocado desde el Stop hook de Claude Code (async: true).
// Siempre termina con exit code 0.

use std::io::Read;
use whisper_bar_rust::settings::Settings;
use whisper_bar_rust::{db, defaults, formatter, logger, text, tts};

/// Evalúa si el texto necesita pasar por el formatter LLM antes de TTS.
/// Retorna true si el contenido tiene elementos que suenan mal al leerlos en voz alta.
fn needs_formatter(text: &str) -> bool {
    let has_code_block  = text.contains("```");
    let has_inline_code = text.contains('`');
    let has_markdown    = text.contains("**") || text.contains("##") || text.contains("__");
    let has_url         = text.contains("http://") || text.contains("https://");
    let is_long         = text.len() > 400;

    let has_list = text.lines().any(|l| {
        let t = l.trim();
        t.starts_with("- ")
            || t.starts_with("* ")
            || t.starts_with("• ")
            || (t.len() > 2 && t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && t.contains(". "))
    });

    has_code_block || has_inline_code || has_markdown || has_url || is_long || has_list
}

fn main() {
    logger::init_append();

    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() {
        std::process::exit(0);
    }
    let raw = raw.trim().to_string();

    if raw.is_empty() {
        log::info!("TTS: stdin vacío, omitiendo");
        std::process::exit(0);
    }

    // Intentar parsear JSON {user, assistant}; si falla, tratar como texto plano
    // `session` solo lo envía el hook de Claude Code (nombre de /rename, "" si
    // no tiene); su ausencia (Codex, prueba manual) desactiva el filtro.
    let (user_msg, assistant_msg, session) = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(v) if v.is_object() => {
            let user      = v["user"].as_str().unwrap_or("").trim().to_string();
            let assistant = v["assistant"].as_str().unwrap_or("").trim().to_string();
            let session   = v.get("session").and_then(|s| s.as_str()).map(str::to_string);
            (user, assistant, session)
        }
        _ => (String::new(), raw.clone(), None),
    };

    if assistant_msg.is_empty() {
        log::info!("TTS: mensaje del asistente vacío, omitiendo");
        std::process::exit(0);
    }

    log::info!(
        "TTS [USUARIO] ({} chars):\n---\n{}\n---",
        user_msg.len(),
        text::truncate(&user_msg, 300)
    );
    log::info!(
        "TTS [ASISTENTE] ({} chars):\n---\n{}\n---",
        assistant_msg.len(),
        text::truncate(&assistant_msg, 500)
    );

    let db = match db::Db::open() {
        Ok(d) => d,
        Err(e) => {
            log::error!("TTS: no se pudo abrir DB: {}", e);
            std::process::exit(0);
        }
    };

    let settings = Settings::load(&db);
    if !settings.tts_enabled {
        log::info!("TTS: desactivado (tts_enabled=false), omitiendo");
        std::process::exit(0);
    }

    // Antes de formatear o matar la reproducción en curso: una sesión que no se
    // escucha no debe gastar llamadas ni cortar el audio de otra.
    if !settings.should_speak_session(session.as_deref()) {
        log::info!(
            "TTS: sesión {:?} no está en 'Sesiones a escuchar' ({:?}), omitiendo",
            session.as_deref().unwrap_or(""),
            settings.tts_sessions
        );
        std::process::exit(0);
    }
    if let Some(name) = session.as_deref().filter(|n| !n.is_empty()) {
        log::info!("TTS: sesión '{}'", name);
    }

    let gemini_key = &settings.gemini_api_key;
    let (final_text, modal_title) = if settings.tts_formatter_enabled && !gemini_key.is_empty() {
        if needs_formatter(&assistant_msg) {
            log::info!(
                "TTS formatter: necesario — enviando a {} (usuario={}c asistente={}c)",
                defaults::GEMINI_FORMATTER_MODEL,
                user_msg.len(),
                assistant_msg.len()
            );
            match (formatter::GeminiFormatter { api_key: gemini_key.clone() })
                .format(&user_msg, &assistant_msg, &settings.tts_formatter_prompt)
            {
                Ok(formatted) => {
                    log::info!(
                        "TTS formatter: ok ({} chars → {} chars):\n---\n{}\n---",
                        assistant_msg.len(),
                        formatted.len(),
                        text::truncate(&formatted, 500)
                    );
                    (formatted, "TTS — Formatter activo")
                }
                Err(e) => {
                    log::error!("TTS formatter: error, usando texto original — {}", e);
                    (assistant_msg.clone(), "TTS — Directo (error formatter)")
                }
            }
        } else {
            log::info!("TTS formatter: no necesario (texto conversacional simple), usando original");
            (assistant_msg.clone(), "TTS — Directo")
        }
    } else {
        if !settings.tts_formatter_enabled {
            log::info!("TTS formatter: desactivado, usando texto original");
        } else {
            log::info!("TTS formatter: sin clave Gemini, usando texto original");
        }
        (assistant_msg.clone(), "TTS — Directo")
    };

    // Guardar texto para re-mostrar con ⌘⌥V
    save_tts_text(modal_title, &final_text);

    if settings.tts_show_modal {
        show_modal_async(modal_title, &final_text);
    }

    tts::kill_previous_instance();
    tts::write_pid_file();
    tts::speak(&final_text, &settings.tts_config());
    tts::cleanup_pid_file();

    std::process::exit(0);
}

/// Persiste el texto final en last-tts-text.txt (primera línea = título, resto = texto)
fn save_tts_text(title: &str, text: &str) {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };
    let dir = std::path::PathBuf::from(&home)
        .join(defaults::APP_CONFIG_DIR)
        .join(defaults::TTS_AUDIO_DIR);
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(defaults::TTS_LAST_TEXT_FILE), format!("{}\n{}", title, text));
}

/// Muestra modal osascript en background (no bloquea el TTS)
fn show_modal_async(title: &str, text: &str) {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };
    let tmp = std::path::PathBuf::from(&home)
        .join(defaults::APP_CONFIG_DIR)
        .join("last-tts-modal.txt");
    if std::fs::write(&tmp, text).is_err() { return; }
    let script = format!(
        "set t to (do shell script \"cat '{}'\")\n\
         display dialog t with title \"{}\" buttons {{\"OK\"}} default button \"OK\"",
        tmp.display(), title
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e").arg(&script)
        .spawn();
}
