// src/bin/whisper-tts.rs
// Binary CLI: recibe JSON {user, assistant} por stdin, lee config de SQLite,
// formatea con LLM (opcional) y reproduce via Gemini TTS.
// Flujo: {user, assistant} → [formatter LLM] → Gemini TTS → afplay
// Invocado desde el Stop hook de Claude Code (async: true).
// Siempre termina con exit code 0.

use std::io::Read;
use whisper_bar_rust::{db, defaults, formatter, logger, tts};

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
    let (user_msg, assistant_msg) = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(v) if v.is_object() => {
            let user      = v["user"].as_str().unwrap_or("").trim().to_string();
            let assistant = v["assistant"].as_str().unwrap_or("").trim().to_string();
            (user, assistant)
        }
        _ => (String::new(), raw.clone()),
    };

    if assistant_msg.is_empty() {
        log::info!("TTS: mensaje del asistente vacío, omitiendo");
        std::process::exit(0);
    }

    log::info!(
        "TTS [USUARIO] ({} chars):\n---\n{}\n---",
        user_msg.len(),
        &user_msg[..user_msg.len().min(300)]
    );
    log::info!(
        "TTS [ASISTENTE] ({} chars):\n---\n{}\n---",
        assistant_msg.len(),
        &assistant_msg[..assistant_msg.len().min(500)]
    );

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

    let voice          = db.get("tts_voice",          defaults::TTS_DEFAULT_VOICE);
    let gemini_key     = db.get("gemini_api_key",      "");
    let scene          = db.get("tts_scene",           defaults::TTS_DEFAULT_SCENE);
    let sample_context = db.get("tts_sample_context",  defaults::TTS_DEFAULT_SAMPLE_CONTEXT);

    let formatter_enabled = db.get("tts_formatter_enabled", "false") == "true";
    let formatter_prompt  = db.get("tts_formatter_prompt",  defaults::FORMATTER_DEFAULT_PROMPT);

    let final_text = if formatter_enabled && !gemini_key.is_empty() {
        log::info!(
            "TTS formatter: enviando a {} — usuario={}c asistente={}c",
            defaults::GEMINI_FORMATTER_MODEL,
            user_msg.len(),
            assistant_msg.len()
        );
        match (formatter::GeminiFormatter { api_key: gemini_key.clone() })
            .format(&user_msg, &assistant_msg, &formatter_prompt)
        {
            Ok(formatted) => {
                log::info!(
                    "TTS formatter: ok ({} chars → {} chars):\n---\n{}\n---",
                    assistant_msg.len(),
                    formatted.len(),
                    &formatted[..formatted.len().min(500)]
                );
                formatted
            }
            Err(e) => {
                log::error!("TTS formatter: error, usando texto original — {}", e);
                assistant_msg.clone()
            }
        }
    } else {
        if !formatter_enabled {
            log::info!("TTS formatter: desactivado, usando texto original");
        } else {
            log::info!("TTS formatter: sin clave Gemini, usando texto original");
        }
        assistant_msg.clone()
    };

    tts::kill_previous_instance();
    tts::write_pid_file();
    tts::speak(&final_text, &tts::TtsConfig {
        voice,
        gemini_key,
        scene,
        sample_context,
    });
    tts::cleanup_pid_file();

    std::process::exit(0);
}
