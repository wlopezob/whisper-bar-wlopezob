// src/tts/provider.rs
// Tipos compartidos para el patrón de proveedores TTS.

pub struct AudioData {
    pub bytes: Vec<u8>,
    pub ext: &'static str, // "wav"
}

pub struct TtsConfig {
    pub provider: String, // "gemini" | "qwen"
    pub voice: String,
    pub gemini_key: String,
    pub scene: String,
    pub sample_context: String,
    pub playback_rate: f32,
    pub qwen_prompt: String,
    pub qwen_temperature: f32,
}

pub trait TtsProvider {
    fn name(&self) -> &'static str;
}
