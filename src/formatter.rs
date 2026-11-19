// src/formatter.rs
// Formatea respuestas de IA para TTS usando Gemini 3.1 Flash Lite.
// Recibe la pregunta del usuario y la respuesta del asistente para tener
// contexto completo y evitar interpretaciones incorrectas del LLM.

use reqwest::blocking::Client;
use std::time::Duration;

const MODEL: &str = crate::defaults::GEMINI_FORMATTER_MODEL;

pub struct GeminiFormatter {
    pub api_key: String,
}

impl GeminiFormatter {
    /// Formatea la respuesta del asistente para TTS.
    /// `user_msg`: pregunta original del usuario (puede estar vacío)
    /// `assistant_msg`: respuesta del asistente a formatear
    /// `system_prompt`: instrucciones de formateo configurables
    pub fn format(
        &self,
        user_msg: &str,
        assistant_msg: &str,
        system_prompt: &str,
    ) -> Result<String, String> {
        // La clave va en el header, no en la URL: los errores de reqwest
        // (timeout, red) incluyen la URL completa y acaban en el log.
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            MODEL
        );

        // Construir el contenido incluyendo la pregunta del usuario como contexto
        let user_content = if user_msg.trim().is_empty() {
            // Sin contexto de pregunta: solo la respuesta
            assistant_msg.to_string()
        } else {
            // Con contexto: pregunta + respuesta para que el LLM entienda el par
            format!(
                "[Pregunta del usuario]: {}\n\n[Respuesta del asistente]: {}",
                crate::text::truncate(user_msg, 500),
                assistant_msg
            )
        };

        let body = serde_json::json!({
            "systemInstruction": {
                "parts": [{"text": system_prompt}]
            },
            "contents": [{"role": "user", "parts": [{"text": user_content}]}],
            "generationConfig": {
                "temperature": 1.0,
                "maxOutputTokens": 500
            }
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Formatter HTTP client: {}", e))?;

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .body(body.to_string())
            .send()
            .map_err(|e| format!("Formatter network: {}", e))?;

        let status = resp.status();
        let body_text = resp
            .text()
            .map_err(|e| format!("Formatter response read: {}", e))?;

        if !status.is_success() {
            return Err(format!("Formatter HTTP {}: {}", status.as_u16(), body_text));
        }

        let json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| format!("Formatter response parse: {}", e))?;

        let result = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("Formatter: no text in response: {}", json))?
            .trim()
            .to_string();

        Ok(result)
    }
}
