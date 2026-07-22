// src/tts/qwen.rs
// Proveedor TTS local usando Qwen3-TTS VoiceDesign vía mlx-audio (Apple Silicon).
// Ejecuta el script Python embebido con el venv de ~/.config/<app>/tts-venv.

use super::provider::{AudioData, TtsProvider};

const SYNTH_SCRIPT: &str = include_str!("../../scripts/qwen_tts_synth.py");
const SYNTH_SCRIPT_TMP: &str = "/tmp/whisper-qwen-tts-synth.py";
const SYNTH_OUT_TMP: &str = "/tmp/whisper-qwen-tts-out.wav";

pub struct QwenProvider {
    pub prompt: String,
    pub temperature: f32,
}

impl TtsProvider for QwenProvider {
    fn name(&self) -> &'static str {
        "qwen"
    }
}

impl QwenProvider {
    pub fn synthesize(&self, text: &str) -> Result<AudioData, String> {
        let home = std::env::var("HOME").map_err(|_| "HOME no definido".to_string())?;
        let base = std::path::PathBuf::from(&home).join(crate::defaults::APP_CONFIG_DIR);

        let python = base.join(crate::defaults::TTS_QWEN_VENV_DIR).join("bin/python");
        if !python.exists() {
            return Err(format!("Qwen TTS: venv no encontrado en {:?}", python));
        }
        let model_dir = base.join(crate::defaults::TTS_QWEN_MODEL_DIR);
        if !model_dir.exists() {
            return Err(format!("Qwen TTS: modelo no encontrado en {:?}", model_dir));
        }

        std::fs::write(SYNTH_SCRIPT_TMP, SYNTH_SCRIPT)
            .map_err(|e| format!("Qwen TTS: no se pudo escribir script: {}", e))?;
        let _ = std::fs::remove_file(SYNTH_OUT_TMP);

        let language = detect_dominant_language(text);
        log::info!("Qwen TTS: idioma dominante detectado: {}", language);

        let cfg = serde_json::json!({
            "text": text,
            "instruct": self.prompt,
            "language": language,
            "temperature": self.temperature,
            "model_dir": model_dir.to_string_lossy(),
            "out_path": SYNTH_OUT_TMP,
        });

        let mut child = std::process::Command::new(&python)
            .arg(SYNTH_SCRIPT_TMP)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Qwen TTS: no se pudo lanzar python: {}", e))?;

        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().ok_or("Qwen TTS: sin stdin")?;
            stdin
                .write_all(cfg.to_string().as_bytes())
                .map_err(|e| format!("Qwen TTS: error escribiendo stdin: {}", e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Qwen TTS: error esperando proceso: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Qwen TTS: síntesis falló: {}",
                stderr.lines().last().unwrap_or("sin detalle")
            ));
        }

        let bytes = std::fs::read(SYNTH_OUT_TMP)
            .map_err(|e| format!("Qwen TTS: no se pudo leer WAV generado: {}", e))?;
        let _ = std::fs::remove_file(SYNTH_OUT_TMP);

        log::info!(
            "Qwen TTS: síntesis ok — {}",
            String::from_utf8_lossy(&output.stdout).trim()
        );

        Ok(AudioData { bytes, ext: "wav" })
    }
}

/// Detecta el idioma dominante (español/inglés) contando stopwords.
/// El token de idioma ancla la prosodia del modelo: con texto mixto evita
/// que las palabras inglesas arrastren el acento (no usar "auto").
fn detect_dominant_language(text: &str) -> &'static str {
    const ES: &[&str] = &[
        "el", "la", "los", "las", "de", "del", "que", "y", "en", "un", "una",
        "es", "para", "con", "se", "por", "no", "ya", "más", "como", "pero",
        "está", "este", "esta", "todo", "muy", "si", "le", "lo", "su",
    ];
    const EN: &[&str] = &[
        "the", "is", "are", "and", "of", "to", "in", "that", "it", "for",
        "with", "on", "this", "was", "you", "have", "be", "not", "all", "now",
    ];

    let mut es_hits = 0usize;
    let mut en_hits = 0usize;
    for word in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != 'á' && c != 'é' && c != 'í' && c != 'ó' && c != 'ú' && c != 'ñ')
        .filter(|w| !w.is_empty())
    {
        if ES.contains(&word) {
            es_hits += 1;
        } else if EN.contains(&word) {
            en_hits += 1;
        }
    }

    if en_hits > es_hits { "english" } else { "spanish" }
}
