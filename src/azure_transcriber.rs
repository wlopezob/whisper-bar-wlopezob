// src/azure_transcriber.rs
// Transcripción vía Azure MAI Transcribe REST API (LLM Speech API)
// Documentación: https://learn.microsoft.com/azure/ai-services/speech-service/mai-transcribe

use reqwest::blocking::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// MAI-Transcribe-2 rechaza `transcribeStyle: "clean"` con 400
/// `unsupported_transcript_mode` aunque la validación de la API y la doc lo
/// aceptan (visto en eastus, 2026-09). Una vez rechazado, se omite el estilo el
/// resto de la sesión para no pagar un round-trip fallido en cada dictado; al
/// reiniciar la app se vuelve a intentar, así que se recupera solo cuando Azure
/// lo arregle.
static TRANSCRIBE_STYLE_REJECTED: AtomicBool = AtomicBool::new(false);

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

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    // Si el estilo ya fue rechazado en esta sesión se envía directamente sin
    // él, y no hay reintento posible: sería la misma petición otra vez.
    let stripped = without_transcribe_style(definition_str);
    let (definition_str, fallback) = match stripped.as_deref() {
        Some(d) if TRANSCRIBE_STYLE_REJECTED.load(Ordering::Relaxed) => (d, None),
        other => (definition_str, other),
    };

    log::info!(
        "Azure MAI: enviando audio → región={} api-version={} definition={}",
        region, api_ver, definition_str
    );

    // Solo se copia el audio si puede hacer falta para el reintento.
    let mut audio_bytes = audio_bytes;
    let first_bytes = if fallback.is_some() {
        audio_bytes.clone()
    } else {
        std::mem::take(&mut audio_bytes)
    };
    let (mut status, mut body) = post_audio(&client, &url, api_key, definition_str, first_bytes)?;

    if status == 400
        && body.contains("unsupported_transcript_mode")
        && let Some(fallback) = fallback
    {
        log::warn!(
            "Azure MAI: el servicio rechazó transcribeStyle, reintentando sin él \
             (verbatim) por el resto de la sesión"
        );
        TRANSCRIBE_STYLE_REJECTED.store(true, Ordering::Relaxed);
        (status, body) = post_audio(&client, &url, api_key, fallback, audio_bytes)?;
    }

    if !(200..300).contains(&status) {
        let preview = crate::text::truncate(&body, 300);
        return Err(format!(
            "Azure MAI respondió con error HTTP {}: {}",
            status, preview
        ));
    }

    // Parsear JSON de respuesta
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        let preview = crate::text::truncate(&body, 200);
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

/// Envía un request multipart (definition + audio) y devuelve (status, body).
fn post_audio(
    client: &Client,
    url: &str,
    api_key: &str,
    definition: &str,
    audio_bytes: Vec<u8>,
) -> Result<(u16, String), String> {
    let audio_part = reqwest::blocking::multipart::Part::bytes(audio_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Error MIME audio/wav: {}", e))?;

    let form = reqwest::blocking::multipart::Form::new()
        .text("definition", definition.to_string())
        .part("audio", audio_part);

    let response = client
        .post(url)
        .header("Ocp-Apim-Subscription-Key", api_key)
        .multipart(form)
        .send()
        .map_err(|e| format!("Error de red con Azure MAI: {}", e))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|e| format!("Error leyendo respuesta de Azure: {}", e))?;

    Ok((status, body))
}

/// Devuelve la definition sin `enhancedMode.modelOptions.transcribeStyle`
/// (y sin `modelOptions` si queda vacío), o `None` si no tenía estilo o no es
/// JSON válido — en ese caso no hay fallback posible.
fn without_transcribe_style(definition: &str) -> Option<String> {
    let mut v: serde_json::Value = serde_json::from_str(definition).ok()?;
    let enhanced = v.get_mut("enhancedMode")?.as_object_mut()?;
    let options = enhanced.get_mut("modelOptions")?.as_object_mut()?;
    options.remove("transcribeStyle")?;
    if options.is_empty() {
        enhanced.remove("modelOptions");
    }
    Some(v.to_string())
}

#[cfg(test)]
mod tests {
    use super::without_transcribe_style;

    #[test]
    fn quita_transcribe_style_y_modeloptions_vacio() {
        let out = without_transcribe_style(crate::defaults::AZURE_MAI_DEFINITION)
            .expect("el default tiene transcribeStyle");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert!(v["enhancedMode"].get("modelOptions").is_none());
        assert_eq!(v["enhancedMode"]["model"], "MAI-Transcribe-2");
        assert!(v["phraseList"]["phrases"].as_array().is_some_and(|a| !a.is_empty()));
    }

    #[test]
    fn conserva_otras_model_options() {
        let def = r#"{"enhancedMode":{"enabled":true,"model":"MAI-Transcribe-2","modelOptions":{"transcribeStyle":"clean","timestamps":"word"}}}"#;
        let v: serde_json::Value =
            serde_json::from_str(&without_transcribe_style(def).unwrap()).unwrap();

        assert_eq!(v["enhancedMode"]["modelOptions"]["timestamps"], "word");
        assert!(v["enhancedMode"]["modelOptions"].get("transcribeStyle").is_none());
    }

    #[test]
    fn sin_estilo_o_json_invalido_no_hay_fallback() {
        assert!(without_transcribe_style(r#"{"enhancedMode":{"enabled":true}}"#).is_none());
        assert!(without_transcribe_style("no es json").is_none());
    }
}
