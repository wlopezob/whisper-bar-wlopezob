// src/tts/say.rs
// Fallback TTS local usando el comando `say` de macOS.

use super::provider::TtsProvider;

pub struct SayProvider;

impl TtsProvider for SayProvider {
    fn name(&self) -> &'static str {
        "say"
    }
}

impl SayProvider {
    pub fn say(&self, text: &str) {
        std::process::Command::new("say")
            .args(["-v", "Paulina", text])
            .status()
            .ok();
    }
}
