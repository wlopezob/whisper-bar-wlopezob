// src/tts/gemini.rs
// Proveedor TTS usando Gemini gemini-3.1-flash-tts-preview.
// Devuelve PCM L16 mono 24kHz envuelto en cabecera WAV.

use base64::Engine;
use reqwest::blocking::Client;
use std::time::Duration;

use super::provider::{AudioData, TtsProvider};

pub struct GeminiProvider {
    pub api_key: String,
}

impl TtsProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }
}

impl GeminiProvider {
    pub fn synthesize(
        &self,
        text: &str,
        voice: &str,
        scene: &str,
        sample_context: &str,
    ) -> Result<AudioData, String> {
        let full_prompt = format!(
            "Read the following transcript based on the director's note.\n\n\
             ## Scene:\n{scene}\n\n\
             ## Sample Context:\n{sample_context}\n\n\
             ## Transcript:\n{text}"
        );

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/\
             gemini-3.1-flash-tts-preview:generateContent?key={}",
            self.api_key
        );

        let body = serde_json::json!({
            "contents": [{"role": "user", "parts": [{"text": full_prompt}]}],
            "generationConfig": {
                "responseModalities": ["audio"],
                "temperature": 1,
                "speech_config": {
                    "voice_config": {
                        "prebuilt_voice_config": { "voice_name": voice }
                    }
                }
            }
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client: {}", e))?;

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
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

        let b64 = json["candidates"][0]["content"]["parts"][0]["inlineData"]["data"]
            .as_str()
            .ok_or_else(|| format!("Gemini: no audio data. Response: {}", json))?;

        let pcm = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("base64 decode: {}", e))?;

        Ok(AudioData {
            bytes: wrap_pcm_in_wav(&pcm, 24000),
            ext: "wav",
        })
    }
}

// PCM L16 signed 16-bit LE mono → WAV (cabecera 44 bytes, sin crate extra)
fn wrap_pcm_in_wav(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits/sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}
