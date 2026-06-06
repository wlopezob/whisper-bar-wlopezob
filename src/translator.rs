// src/translator.rs
// Traducción con patrón factory: Azure Translator v3 u Ollama local.

use reqwest::blocking::Client;
use std::time::Duration;

pub struct TranslateResult {
    pub text: String,
    pub detected_lang: String,
    pub was_translated: bool,
}

pub trait TranslatorProvider: Send {
    fn translate(&self, text: &str, dest_lang: &str) -> Result<TranslateResult, String>;
}

// ── Azure Translator v3 ───────────────────────────────────────────────────────

pub struct AzureTranslator {
    pub api_key: String,
    pub region: String,
}

impl TranslatorProvider for AzureTranslator {
    fn translate(&self, text: &str, dest_lang: &str) -> Result<TranslateResult, String> {
        if self.api_key.is_empty() || self.region.is_empty() {
            return Err("Azure Translator: credenciales no configuradas".to_string());
        }

        let url = format!(
            "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&to={}",
            dest_lang
        );
        let body = serde_json::json!([{"Text": text}]).to_string();

        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Azure HTTP client: {}", e))?;

        let response = client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &self.api_key)
            .header("Ocp-Apim-Subscription-Region", &self.region)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map_err(|e| format!("Azure network: {}", e))?;

        let status = response.status();
        let body_text = response.text()
            .map_err(|e| format!("Azure response read: {}", e))?;

        if !status.is_success() {
            return Err(format!("Azure HTTP {}: {}", status.as_u16(), &body_text[..body_text.len().min(300)]));
        }

        let json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| format!("Azure JSON parse: {} — {}", e, &body_text[..body_text.len().min(200)]))?;

        let detected_lang = json[0]["detectedLanguage"]["language"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if detected_lang == dest_lang {
            log::info!("Azure translator: '{}' == destino '{}', sin traducción", detected_lang, dest_lang);
            return Ok(TranslateResult { text: text.to_string(), detected_lang, was_translated: false });
        }

        let translated = json[0]["translations"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("Azure: sin texto en respuesta: {}", json))?
            .to_string();

        log::info!("Azure translator: '{}' → '{}' ({} → {} chars)", detected_lang, dest_lang, text.len(), translated.len());

        Ok(TranslateResult { text: translated, detected_lang, was_translated: true })
    }
}

// ── Ollama local ──────────────────────────────────────────────────────────────

pub struct OllamaTranslator {
    pub model: String,
    pub prompt_template: String,
}

impl TranslatorProvider for OllamaTranslator {
    fn translate(&self, text: &str, dest_lang: &str) -> Result<TranslateResult, String> {
        let prompt = self.prompt_template
            .replace("{dest_lang}", dest_lang)
            .replace("{input_text}", text);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {"temperature": 0.1}
        }).to_string();

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Ollama HTTP client: {}", e))?;

        let response = client
            .post("http://localhost:11434/api/generate")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map_err(|e| format!("Ollama network (¿está corriendo?): {}", e))?;

        let status = response.status();
        let body_text = response.text()
            .map_err(|e| format!("Ollama response read: {}", e))?;

        if !status.is_success() {
            return Err(format!("Ollama HTTP {}: {}", status.as_u16(), &body_text[..body_text.len().min(200)]));
        }

        let json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| format!("Ollama response parse: {}", e))?;

        let raw = json["response"].as_str()
            .ok_or_else(|| format!("Ollama: sin campo response: {}", json))?
            .trim()
            .to_string();

        // Limpiar markdown si el modelo envuelve el JSON
        let clean = if raw.contains("```") {
            let parts: Vec<&str> = raw.splitn(3, "```").collect();
            if parts.len() >= 2 {
                let inner = parts[1];
                if inner.starts_with("json") { inner[4..].trim().to_string() } else { inner.trim().to_string() }
            } else { raw.clone() }
        } else { raw.clone() };

        let result_json: serde_json::Value = serde_json::from_str(&clean)
            .map_err(|e| format!("Ollama JSON parse: {} — raw: {}", e, &raw[..raw.len().min(200)]))?;

        let detected_lang = result_json["detected_lang"].as_str().unwrap_or("").to_string();
        let translated_text = result_json["text"]
            .as_str()
            .ok_or_else(|| format!("Ollama: sin campo text en: {}", result_json))?
            .to_string();

        let was_translated = !detected_lang.is_empty() && detected_lang != dest_lang;

        if was_translated {
            log::info!("Ollama translator: '{}' → '{}' ({} → {} chars)", detected_lang, dest_lang, text.len(), translated_text.len());
        } else {
            log::info!("Ollama translator: '{}' == destino '{}', sin traducción", detected_lang, dest_lang);
        }

        Ok(TranslateResult { text: translated_text, detected_lang, was_translated })
    }
}

// ── Factory ───────────────────────────────────────────────────────────────────

pub fn create_translator(
    provider: &str,
    azure_key: &str,
    azure_region: &str,
    ollama_prompt: &str,
) -> Box<dyn TranslatorProvider> {
    match provider {
        "ollama" => Box::new(OllamaTranslator {
            model: crate::defaults::TRANSLATE_OLLAMA_MODEL.to_string(),
            prompt_template: if ollama_prompt.is_empty() {
                crate::defaults::TRANSLATE_OLLAMA_DEFAULT_PROMPT.to_string()
            } else {
                ollama_prompt.to_string()
            },
        }),
        _ => Box::new(AzureTranslator {
            api_key: azure_key.to_string(),
            region: azure_region.to_string(),
        }),
    }
}
