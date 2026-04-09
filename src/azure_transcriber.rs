// src/azure_transcriber.rs
// Transcripción vía Azure MAI Transcribe REST API
// Documentación: https://learn.microsoft.com/azure/ai-services/speech-service/mai-transcribe

use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

/// Envía el audio WAV a Azure MAI Transcribe y devuelve el texto transcrito.
///
/// - `api_key`  : Clave de suscripción de Azure Cognitive Services
/// - `region`   : Región de Azure (p.ej. "eastus", "westeurope")
/// - `model`    : Nombre del modelo base (p.ej. "whisper"). Vacío → usa el modelo por defecto del servicio
/// - `language` : Código de idioma interno ("es" | "en")
/// - `audio_path`: Ruta al archivo WAV grabado
pub fn transcribe(
    api_key: &str,
    region: &str,
    model: &str,
    language: &str,
    audio_path: &str,
) -> Result<String, String> {
    let url = format!(
        "https://{}.api.cognitive.microsoft.com/speechtotext/transcriptions:transcribe?api-version={}",
        region,
        crate::defaults::AZURE_MAI_API_VERSION,
    );

    // Locale BCP-47 según idioma seleccionado
    let locale = lang_to_locale(language);

    // Definición de la transcripción — con o sin modelo explícito
    let definition = if model.is_empty() {
        json!({
            "locales": [locale],
            "channels": [0]
        })
    } else {
        json!({
            "model": {
                "self": format!(
                    "https://{}.api.cognitive.microsoft.com/speechtotext/models/base/{}?api-version={}",
                    region, model, crate::defaults::AZURE_MAI_API_VERSION
                )
            },
            "locales": [locale],
            "channels": [0]
        })
    };

    // Leer el archivo de audio
    let audio_bytes = std::fs::read(audio_path)
        .map_err(|e| format!("Error leyendo audio para Azure: {}", e))?;

    // Construir el formulario multipart
    let audio_part = reqwest::blocking::multipart::Part::bytes(audio_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Error MIME audio/wav: {}", e))?;

    let form = reqwest::blocking::multipart::Form::new()
        .text("definition", definition.to_string())
        .part("audio", audio_part);

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let model_label = if model.is_empty() { "default" } else { model };
    log::info!(
        "Azure MAI: enviando audio → región={} modelo={} locale={}",
        region, model_label, locale
    );

    let response = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", api_key)
        .multipart(form)
        .send()
        .map_err(|e| format!("Error de red con Azure MAI: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|e| format!("Error leyendo respuesta de Azure: {}", e))?;

    if !status.is_success() {
        // Mostrar los primeros 300 caracteres del body para ayudar al diagnóstico
        let preview = &body[..body.len().min(300)];
        return Err(format!(
            "Azure MAI respondió con error HTTP {}: {}",
            status.as_u16(),
            preview
        ));
    }

    // Parsear JSON de respuesta
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        let preview = &body[..body.len().min(200)];
        format!("Error parseando JSON de Azure: {} — respuesta: {}", e, preview)
    })?;

    // Extraer texto de combinedPhrases[0].text
    let text = json["combinedPhrases"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|p| p["text"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    if text.is_empty() {
        return Err("Azure MAI no detectó texto en el audio".to_string());
    }

    Ok(text)
}

/// Mapea código de idioma interno a locale BCP-47 requerido por Azure
fn lang_to_locale(lang: &str) -> &str {
    match lang {
        "es" => "es-ES",
        "en" => "en-US",
        _ => lang,
    }
}
