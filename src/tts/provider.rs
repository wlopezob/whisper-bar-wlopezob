// src/tts/provider.rs
// Tipos compartidos para el patrón de proveedores TTS.

pub struct AudioData {
    pub bytes: Vec<u8>,
    pub ext: &'static str, // "wav"
}

pub struct TtsConfig {
    pub voice: String,
    pub gemini_key: String,
    pub scene: String,
    pub sample_context: String,
}

pub trait TtsProvider {
    fn name(&self) -> &'static str;
}
