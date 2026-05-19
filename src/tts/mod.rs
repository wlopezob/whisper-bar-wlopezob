// src/tts/mod.rs
// Orquestador TTS: selecciona proveedor y reproduce audio.

mod gemini;
mod provider;
mod say;

pub use provider::{AudioData, TtsConfig, TtsProvider};

use gemini::GeminiProvider;
use say::SayProvider;
use std::time::Duration;

const PID_FILE: &str = "/tmp/whisper-tts.pid";

// ── Limpieza de markdown ──────────────────────────────────────────────────────

pub fn clean_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line
            .replace("**", "")
            .replace("~~", "")
            .replace('*', "")
            .replace('`', "");
        let line = if line.starts_with('#') {
            line.trim_start_matches('#').trim_start().to_string()
        } else {
            line
        };
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

// ── Gestión de PID (interrumpir-y-reemplazar) ─────────────────────────────────

pub fn kill_previous_instance() {
    if let Ok(pid_str) = std::fs::read_to_string(PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if pid != std::process::id() {
                let _ = std::process::Command::new("kill")
                    .args(["-15", &pid.to_string()])
                    .status();
                std::thread::sleep(Duration::from_millis(300));
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

// ── Síntesis principal ────────────────────────────────────────────────────────

pub fn speak(text: &str, config: &TtsConfig) {
    log::info!(
        "TTS: síntesis iniciada — proveedor=gemini voz={} chars={}",
        config.voice,
        text.len()
    );

    let clean = clean_markdown(text);
    let clean = if clean.len() > 5000 {
        &clean[..5000]
    } else {
        clean.as_str()
    };

    if config.gemini_key.is_empty() {
        log::info!("TTS: sin clave Gemini, fallback a say");
        SayProvider.say(clean);
        return;
    }

    let provider = GeminiProvider {
        api_key: config.gemini_key.clone(),
    };

    match provider.synthesize(clean, &config.voice, &config.scene, &config.sample_context) {
        Ok(audio) => play_audio_file(audio),
        Err(e) => {
            log::error!("TTS Gemini error: {} — fallback a say", e);
            SayProvider.say(clean);
        }
    }
}

fn play_audio_file(audio: AudioData) {
    let tmp = format!("/tmp/whisper-tts-audio.{}", audio.ext);
    if std::fs::write(&tmp, &audio.bytes).is_ok() {
        let status = std::process::Command::new("afplay").arg(&tmp).status();
        let _ = std::fs::remove_file(&tmp);
        match status {
            Ok(_) => log::info!("TTS: audio reproducido correctamente"),
            Err(e) => log::error!("TTS: error en afplay: {}", e),
        }
    } else {
        log::error!("TTS: no se pudo escribir archivo temporal de audio");
    }
}
