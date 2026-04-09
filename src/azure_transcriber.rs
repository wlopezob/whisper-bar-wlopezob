// src/azure_transcriber.rs
// Transcripción vía Azure MAI Transcribe REST API (LLM Speech API)
// Documentación: https://learn.microsoft.com/azure/ai-services/speech-service/mai-transcribe

use reqwest::blocking::Client;
use std::time::Duration;

/// Envía el audio WAV a Azure MAI Transcribe y devuelve el texto transcrito.
///
/// - `api_key`    : Clave de suscripción de Azure Cognitive Services
/// - `region`     : Región de Azure (p.ej. "eastus", "westeurope")
/// - `api_version`: Versión de la API (default: crate::defaults::AZURE_MAI_API_VERSION)
/// - `definition` : JSON del campo `definition` en el form-data
///                  (default: crate::defaults::AZURE_MAI_DEFINITION)
/// - `audio_path` : Ruta al archivo WAV grabado
pub fn transcribe(
    api_key: &str,
    region: &str,
    api_version: &str,
    definition: &str,
    audio_path: &str,
) -> Result<String, String> {
    let api_ver = if api_version.trim().is_empty() {
        crate::defaults::AZURE_MAI_API_VERSION
    } else {
        api_version.trim()
    };

    let definition_str = if definition.trim().is_empty() {
        crate::defaults::AZURE_MAI_DEFINITION
    } else {
        definition.trim()
    };

    let url = format!(
        "https://{}.api.cognitive.microsoft.com/speechtotext/transcriptions:transcribe?api-version={}",
        region, api_ver,
    );

    // Leer el archivo de audio
    let audio_bytes = std::fs::read(audio_path)
        .map_err(|e| format!("Error leyendo audio para Azure: {}", e))?;

    // Construir el formulario multipart
    let audio_part = reqwest::blocking::multipart::Part::bytes(audio_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Error MIME audio/wav: {}", e))?;

    let form = reqwest::blocking::multipart::Form::new()
        .text("definition", definition_str.to_string())
        .part("audio", audio_part);

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    log::info!(
        "Azure MAI: enviando audio → región={} api-version={} definition={}",
        region, api_ver, definition_str
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
