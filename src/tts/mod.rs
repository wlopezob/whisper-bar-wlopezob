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
const AFPLAY_PID_FILE: &str = "/tmp/whisper-tts-afplay.pid";
const SAY_PID_FILE: &str = "/tmp/whisper-tts-say.pid";

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

/// Devuelve true si afplay está corriendo actualmente (PID file existe y el proceso vive)
pub fn is_afplay_running() -> bool {
    if let Ok(pid_str) = std::fs::read_to_string(AFPLAY_PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // kill -0 verifica si el proceso existe sin matarlo
            return std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }
    false
}

pub fn is_say_running() -> bool {
    if let Ok(pid_str) = std::fs::read_to_string(SAY_PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            return std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }
    false
}

pub fn kill_say() {
    if let Ok(pid_str) = std::fs::read_to_string(SAY_PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let _ = std::process::Command::new("kill")
                .args(["-15", &pid.to_string()])
                .status();
        }
    }
    let _ = std::fs::remove_file(SAY_PID_FILE);
}

/// Para cualquier reproducción de afplay en curso
pub fn kill_afplay() {
    if let Ok(pid_str) = std::fs::read_to_string(AFPLAY_PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let _ = std::process::Command::new("kill")
                .args(["-15", &pid.to_string()])
                .status();
        }
    }
    let _ = std::fs::remove_file(AFPLAY_PID_FILE);
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
        SayProvider.say(clean, config.playback_rate);
        return;
    }

    let provider = GeminiProvider {
        api_key: config.gemini_key.clone(),
    };

    match provider.synthesize(clean, &config.voice, &config.scene, &config.sample_context) {
        Ok(audio) => play_audio_file(audio, config.playback_rate),
        Err(e) => {
            log::error!("TTS Gemini error: {} — fallback a say", e);
            SayProvider.say(clean, config.playback_rate);
        }
    }
}

pub fn last_audio_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::PathBuf::from(home)
            .join(crate::defaults::APP_CONFIG_DIR)
            .join(crate::defaults::TTS_AUDIO_DIR)
            .join(crate::defaults::TTS_LAST_AUDIO_FILE),
    )
}

fn play_audio_file(audio: AudioData, rate: f32) {
    let tmp = format!("/tmp/whisper-tts-audio.{}", audio.ext);
    if std::fs::write(&tmp, &audio.bytes).is_err() {
        log::error!("TTS: no se pudo escribir archivo temporal de audio");
        return;
    }

    // Persistir como última respuesta para poder repetirla con ⌘⌥R
    if let Some(last) = last_audio_path() {
        if let Some(dir) = last.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::copy(&tmp, &last).is_ok() {
            log::info!("TTS: audio guardado en {:?}", last);
        }
    }

    // spawn() en vez de status() para obtener el PID y permitir kill externo
    let mut cmd = std::process::Command::new("afplay");
    if (rate - 1.0).abs() > 0.01 {
        cmd.args(["-r", &format!("{:.2}", rate), "-q", "1"]);
    }
    cmd.arg(&tmp);
    match cmd.spawn() {
        Ok(mut child) => {
            let _ = std::fs::write(AFPLAY_PID_FILE, child.id().to_string());
            let _ = child.wait();
            let _ = std::fs::remove_file(AFPLAY_PID_FILE);
            log::info!("TTS: audio reproducido correctamente");
        }
        Err(e) => log::error!("TTS: error en afplay: {}", e),
    }
    let _ = std::fs::remove_file(&tmp);
}
