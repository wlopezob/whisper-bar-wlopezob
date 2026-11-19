// src/tts/gemini.rs
// Proveedor TTS usando Gemini gemini-3.8-flash-lite-tts (Interactions API).
// Devuelve WAV (PCM L16 mono 24kHz) tal cual lo entrega la API.

use base64::Engine;
use reqwest::blocking::Client;
use std::time::Duration;

use super::provider::AudioData;

/// Reemplazo oficial de gemini-3.1-flash-tts-preview: ~2x más rápido y ~1/3 del
/// costo por minuto de audio.
const MODEL: &str = "gemini-3.8-flash-lite-tts";

pub struct GeminiProvider {
    pub api_key: String,
}

impl GeminiProvider {
    pub fn synthesize(
        &self,
        text: &str,
        voice: &str,
        scene: &str,
        sample_context: &str,
    ) -> Result<AudioData, String> {
        // Los modelos 3.8 leen el texto de forma literal: una "director's note"
        // dentro del texto se pronuncia en voz alta. La dirección de estilo va
        // en la anotación speech_metadata, que solo existe en la Interactions API
        // (generateContent rechaza systemInstruction para estos modelos).
        let style = [scene.trim(), sample_context.trim()]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut content = serde_json::json!({ "type": "text", "text": text });
        if !style.is_empty() {
            content["annotations"] =
                serde_json::json!([{ "type": "speech_metadata", "style": style }]);
        }

        let body = serde_json::json!({
            "model": MODEL,
            "input": [{ "type": "user_input", "content": [content] }],
            "response_format": { "type": "audio", "mime_type": "audio/wav" },
            "generation_config": { "speech_config": [{ "voice": voice }] }
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client: {}", e))?;

        let resp = client
            .post("https://generativelanguage.googleapis.com/v1beta/interactions")
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .body(body.to_string())
            .send()
            .map_err(|e| format!("Gemini network: {}", e))?;

        let status = resp.status();
        let body_text = resp
            .text()
            .map_err(|e| format!("Gemini response read: {}", e))?;

        if !status.is_success() {
            return Err(format!("Gemini HTTP {}: {}", status.as_u16(), body_text));
        }

        let json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| format!("Gemini response parse: {}", e))?;

        let b64 = json["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|step| step["content"].as_array())
            .flatten()
            .find(|c| c["type"] == "audio")
            .and_then(|c| c["data"].as_str())
            .ok_or_else(|| format!("Gemini: no audio data. Response: {}", body_text))?;

        let wav = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("base64 decode: {}", e))?;

        if !wav.starts_with(b"RIFF") {
            return Err("Gemini: el audio no es WAV (falta cabecera RIFF)".to_string());
        }

        Ok(AudioData {
            bytes: wav,
            ext: "wav",
        })
    }
}
