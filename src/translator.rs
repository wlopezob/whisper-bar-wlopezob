// src/translator.rs
// Traducción vía Azure Translator REST API v3
// Reutiliza las mismas credenciales que Azure MAI Transcribe.

use reqwest::blocking::Client;
use std::time::Duration;

pub struct TranslateResult {
    pub text: String,
    pub detected_lang: String,
    pub was_translated: bool,
}

/// Traduce `text` al idioma `dest_lang` (p.ej. "en", "es").
/// Auto-detecta el idioma origen.
/// Si el idioma detectado ya es `dest_lang`, devuelve el texto original sin cambios.
pub fn translate(
    text: &str,
    dest_lang: &str,
    api_key: &str,
    region: &str,
) -> Result<TranslateResult, String> {
    let url = format!(
        "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&to={}",
        dest_lang
    );

    let body = serde_json::json!([{"Text": text}]).to_string();

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Translator HTTP client: {}", e))?;

    let response = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", api_key)
        .header("Ocp-Apim-Subscription-Region", region)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .map_err(|e| format!("Translator network: {}", e))?;

    let status = response.status();
    let body_text = response
        .text()
        .map_err(|e| format!("Translator response read: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Translator HTTP {}: {}",
            status.as_u16(),
            &body_text[..body_text.len().min(300)]
        ));
    }

    let json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Translator JSON parse: {} — {}", e, &body_text[..body_text.len().min(200)]))?;

    let detected_lang = json[0]["detectedLanguage"]["language"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Si el idioma detectado ya es el destino, no hace falta traducir
    if detected_lang == dest_lang {
        log::info!(
            "Translator: idioma detectado '{}' == destino '{}', sin traducción",
            detected_lang, dest_lang
        );
        return Ok(TranslateResult {
            text: text.to_string(),
            detected_lang,
            was_translated: false,
        });
    }

    let translated = json[0]["translations"][0]["text"]
        .as_str()
        .ok_or_else(|| format!("Translator: sin texto en respuesta: {}", json))?
        .to_string();

    log::info!(
        "Translator: '{}' → '{}' ({} chars → {} chars)",
        detected_lang, dest_lang, text.len(), translated.len()
    );

    Ok(TranslateResult {
        text: translated,
        detected_lang,
        was_translated: true,
    })
}
