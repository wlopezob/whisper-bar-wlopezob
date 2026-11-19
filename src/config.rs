// src/config.rs

use crate::defaults;
use std::path::Path;
use std::process::Command;

pub struct Config {
    pub whisper_cli_path: String,
    pub model_path: String,
    pub min_recording_duration: f64,
}

impl Config {
    pub fn new() -> Self {
        let whisper_cli_path = Self::detect_whisper_cli().unwrap_or_default();
        let model_path = Self::detect_model().unwrap_or_default();

        Config {
            whisper_cli_path,
            model_path,
            min_recording_duration: defaults::MIN_RECORDING_DURATION,
        }
    }

    /// Busca whisper-cli en rutas conocidas de Homebrew y fallback con `which`
    fn detect_whisper_cli() -> Option<String> {
        for path in defaults::WHISPER_CLI_CANDIDATES {
            if is_executable(path) {
                return Some(path.to_string());
            }
        }

        // Fallback: `which whisper-cli`
        if let Ok(output) = Command::new("which").arg("whisper-cli").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() && is_executable(&path) {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Busca el mejor modelo Whisper disponible según prioridad definida en defaults
    fn detect_model() -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        let base_dir = format!("{}/{}/{}", home, defaults::APP_CONFIG_DIR, defaults::WHISPER_MODELS_DIR);

        for name in defaults::MODEL_PRIORITY {
            let full_path = format!("{}/{}", base_dir, name);
            if Path::new(&full_path).exists() {
                return Some(full_path);
            }
        }

        None
    }

    pub fn is_valid(&self) -> bool {
        self.is_whisper_cli_valid() && self.is_model_valid()
    }

    pub fn is_whisper_cli_valid(&self) -> bool {
        is_executable(&self.whisper_cli_path)
    }

    pub fn is_model_valid(&self) -> bool {
        !self.model_path.is_empty() && Path::new(&self.model_path).exists()
    }

    /// Extrae solo el nombre del archivo del modelo (para mostrar en menú)
    pub fn model_filename(&self) -> &str {
        Path::new(&self.model_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("no encontrado")
    }
}

/// Verifica si un archivo existe y es ejecutable
fn is_executable(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = Path::new(path);
    if !p.exists() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(p) {
            return meta.permissions().mode() & 0o111 != 0;
        }
    }
    false
}
