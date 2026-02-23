// src/llm.rs

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;

const LLM_TIMEOUT_SECS: u64 = 30;

const GRAMMAR_PROMPT_EN: &str =
    "Fix grammar and pronunciation errors in this English text. Return ONLY the corrected text, no explanations, no extra words.";

const GRAMMAR_PROMPT_ES: &str =
    "Corrige los errores gramaticales de este texto en español. Devuelve ÚNICAMENTE el texto corregido.";

const TRANSLATE_PROMPT_TO_ES: &str =
    "Translate the following text to Spanish. Return ONLY the Spanish translation.";

const TRANSLATE_PROMPT_TO_EN: &str =
    "Traduce el siguiente texto al inglés. Devuelve ÚNICAMENTE la traducción en inglés.";

/// Corrige errores gramaticales y de pronunciación usando el CLI LLM configurado.
/// `language` debe ser "en" o "es"; cualquier otro valor usa el prompt en inglés.
pub fn correct_grammar(
    llama_cli_path: &str,
    model_path: &str,
    text: &str,
    language: &str,
) -> Result<String, String> {
    let prompt = if language == "es" { GRAMMAR_PROMPT_ES } else { GRAMMAR_PROMPT_EN };
    run_llm(llama_cli_path, model_path, prompt, text)
}

/// Traduce texto usando el CLI LLM configurado.
/// `dest_lang`: "es" → español; cualquier otro valor → inglés.
pub fn translate_text(
    llama_cli_path: &str,
    model_path: &str,
    text: &str,
    dest_lang: &str,
) -> Result<String, String> {
    let prompt = if dest_lang == "es" { TRANSLATE_PROMPT_TO_ES } else { TRANSLATE_PROMPT_TO_EN };
    run_llm(llama_cli_path, model_path, prompt, text)
}

/// Lanza `llama-cli` con el prompt de sistema indicado y retorna el texto parseado.
fn run_llm(
    llama_cli_path: &str,
    model_path: &str,
    system_prompt: &str,
    text: &str,
) -> Result<String, String> {
    let mut child = Command::new(llama_cli_path)
        .args([
            "-m", model_path,
            "-sys", system_prompt,
            "-p", text,
            "-n", "512",
            "-ngl", "99",
        ])
        // Cierra stdin para evitar que el proceso quede esperando entrada interactiva.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Error ejecutando LLM CLI: {}", e))?;

    let timeout = Duration::from_secs(LLM_TIMEOUT_SECS);
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
                    return Err(format!(
                        "LLM CLI falló (exit {}): {}",
                        status,
                        stderr.trim()
                    ));
                }

                let stdout = child.stdout.take().map(|mut s| {
                    let mut buf = String::new();
                    s.read_to_string(&mut buf).ok();
                    buf
                }).unwrap_or_default();

                let result = parse_llm_output(&stdout);

                if result.is_empty() {
                    return Err("llama-cli no devolvió texto".to_string());
                }

                return Ok(result);
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(format!("LLM CLI timeout (>{}s)", LLM_TIMEOUT_SECS));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error esperando llama-cli: {}", e)),
        }
    }
}

/// Limpia la salida del CLI LLM.
fn parse_llm_output(raw: &str) -> String {
    let clean = strip_ansi(raw);

    // Formato esperado en llama-completion:
    // system\n...\nuser\n...\nassistant\n<respuesta>\n> EOF by user
    if let Some(range) = clean.rfind("assistant\n") {
        let after_assistant = &clean[range + "assistant\n".len()..];
        let parsed = collect_llm_lines(after_assistant, true);
        if !parsed.is_empty() {
            return parsed;
        }
    }

    // Fallback si no aparece el marcador "assistant"
    collect_llm_lines(&clean, false)
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

fn collect_llm_lines(s: &str, after_assistant: bool) -> String {
    let mut lines = Vec::new();

    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains("EOF by user") {
            break;
        }
        if trimmed.starts_with('>')
            || trimmed.starts_with("▄")
            || trimmed.starts_with("█")
            || trimmed.starts_with("▀")
            || trimmed.starts_with('[')
            || trimmed.starts_with("|-")
        {
            continue;
        }
        if !after_assistant
            && (trimmed == "system"
                || trimmed == "user"
                || trimmed == "assistant"
                || trimmed.starts_with("build")
                || trimmed.starts_with("available commands:")
                || trimmed.starts_with("/exit")
                || trimmed.starts_with("/regen")
                || trimmed.starts_with("/clear")
                || trimmed.starts_with("/read")
                || trimmed.starts_with("Loading model"))
        {
            continue;
        }
        lines.push(trimmed);
    }

    lines.join(" ").trim().to_string()
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

    #[test]
    fn test_parse_llm_output_chat_format() {
        let input = "system\nFix grammar\nuser\nCould you help me send a message for Mama?\nassistant\nCould you help me send a message to Mama?\n\n> EOF by user\n";
        assert_eq!(
            parse_llm_output(input),
            "Could you help me send a message to Mama?"
        );
    }

    #[test]
    fn test_translate_prompt_to_es() {
        let prompt = if "es" == "es" { TRANSLATE_PROMPT_TO_ES } else { TRANSLATE_PROMPT_TO_EN };
        assert_eq!(prompt, TRANSLATE_PROMPT_TO_ES);
    }

    #[test]
    fn test_translate_prompt_to_en() {
        let prompt = if "en" == "es" { TRANSLATE_PROMPT_TO_ES } else { TRANSLATE_PROMPT_TO_EN };
        assert_eq!(prompt, TRANSLATE_PROMPT_TO_EN);
    }
}
