// src/llm.rs

use std::io::Read;
use std::process::Command;
use std::time::Duration;

/// Corrige errores gramaticales y de pronunciación en el texto usando llama-cli
pub fn correct_grammar(
    llama_cli_path: &str,
    model_path: &str,
    text: &str,
) -> Result<String, String> {
    let prompt = format!(
        "Fix grammar and pronunciation errors in this English text. \
         Return ONLY the corrected text, no explanations, no extra words.\n\n\
         Text: {}\nCorrected:",
        text
    );

    let mut child = Command::new(llama_cli_path)
        .args([
            "-m", model_path,
            "-p", &prompt,
            "--no-display-prompt",
            "-n", "300",
            "--temp", "0",
            "-c", "1024",
            "--log-disable",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Error ejecutando llama-cli: {}", e))?;

    let timeout = Duration::from_secs(30);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(format!("llama-cli falló (exit {})", status));
                }

                let stdout = child.stdout.take().map(|mut s| {
                    let mut buf = String::new();
                    s.read_to_string(&mut buf).ok();
                    buf
                }).unwrap_or_default();

                let corrected = parse_llm_output(&stdout);

                if corrected.is_empty() {
                    return Err("llama-cli no devolvió texto".to_string());
                }

                return Ok(corrected);
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err("llama-cli timeout (>30s)".to_string());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error esperando llama-cli: {}", e)),
        }
    }
}

/// Limpia la salida de llama-cli: elimina ANSI codes, líneas vacías y espacios extra
fn parse_llm_output(raw: &str) -> String {
    // Eliminar secuencias ANSI (ej: \x1b[0m)
    let clean = strip_ansi(raw);

    clean
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Elimina secuencias de escape ANSI del texto
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Saltar hasta el final de la secuencia (letra después de '[')
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() { break; }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[0mhello\x1b[1;32m world\x1b[0m";
        assert_eq!(strip_ansi(input), "hello world");
    }

    #[test]
    fn test_parse_llm_output_clean() {
        let input = "  I want to learn English, but it is hard for me.  \n";
        assert_eq!(
            parse_llm_output(input),
            "I want to learn English, but it is hard for me."
        );
    }

    #[test]
    fn test_parse_llm_output_multiline() {
        let input = "Hello, my name is John.\nI am learning English.\n";
        assert_eq!(
            parse_llm_output(input),
            "Hello, my name is John. I am learning English."
        );
    }

    #[test]
    fn test_parse_llm_output_with_ansi() {
        let input = "\x1b[0m I want to learn English. \x1b[0m\n";
        assert_eq!(parse_llm_output(input), "I want to learn English.");
    }
}
