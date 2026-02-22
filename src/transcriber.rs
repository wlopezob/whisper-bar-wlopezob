// src/transcriber.rs

use std::io::Read;
use std::process::Command;
use std::time::Duration;

/// Ejecuta whisper-cli y retorna el texto transcrito
///
/// Comando: whisper-cli -m <modelo> -l <idioma> --no-timestamps -f <audio>
/// Timeout: 60 segundos
pub fn transcribe(
    whisper_cli_path: &str,
    model_path: &str,
    language: &str,
    audio_path: &str,
) -> Result<String, String> {
    let mut child = Command::new(whisper_cli_path)
        .args([
            "-m", model_path,
            "-l", language,
            "--no-timestamps",
            "-f", audio_path,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Error ejecutando whisper-cli: {}", e))?;

    let timeout = Duration::from_secs(60);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let stderr = child.stderr.take().map(|mut s| {
                        let mut buf = String::new();
                        s.read_to_string(&mut buf).ok();
                        buf
                    }).unwrap_or_default();
                    return Err(format!("whisper-cli falló (exit {}): {}", status, stderr));
                }

                let stdout = child.stdout.take().map(|mut s| {
                    let mut buf = String::new();
                    s.read_to_string(&mut buf).ok();
                    buf
                }).unwrap_or_default();

                let text = parse_whisper_output(&stdout);

                if text.is_empty() {
                    return Err("No se detectó texto en el audio".to_string());
                }

                return Ok(text);
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(
                        "Tiempo de espera agotado (>60s). Prueba un modelo más pequeño."
                            .to_string(),
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("Error esperando whisper-cli: {}", e));
            }
        }
    }
}

/// Parsea la salida de whisper-cli:
/// 1. Divide por líneas
/// 2. Trim whitespace
/// 3. Filtra líneas vacías y las que empiezan con '[' (timestamps)
/// 4. Une con espacios
fn parse_whisper_output(raw: &str) -> String {
    raw.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('['))
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_output() {
        // Salida real de whisper-cli --no-timestamps: solo texto sin prefijos
        let input = "  Hola mundo  \n  esto es una prueba  \n\n";
        let result = parse_whisper_output(input);
        assert_eq!(result, "Hola mundo esto es una prueba");
    }

    #[test]
    fn test_parse_empty_output() {
        let result = parse_whisper_output("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_only_timestamps() {
        // Líneas que empiezan con '[' se filtran completamente
        let input = "[00:00.000 --> 00:02.000]\n[00:02.000 --> 00:04.000]\n";
        let result = parse_whisper_output(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_lines_starting_with_bracket_are_filtered() {
        // Líneas tipo "[timestamp] texto" — se filtra la línea completa
        // ya que empieza con '['. Con --no-timestamps esto no ocurre en la práctica.
        let input = "[00:00.000 --> 00:02.000]   Hola mundo\n[00:02.000 --> 00:04.000]   esto es una prueba\n\n";
        let result = parse_whisper_output(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_info_lines_filtered() {
        // Líneas de progreso/info que whisper-cli puede emitir en stdout
        let input = "[INFO] modelo cargado\n Hola mundo\n[WARN] algo\n esto es una prueba\n";
        let result = parse_whisper_output(input);
        assert_eq!(result, "Hola mundo esto es una prueba");
    }
}
