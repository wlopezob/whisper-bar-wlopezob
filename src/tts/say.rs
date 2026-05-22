// src/tts/say.rs
// Fallback TTS local usando el comando `say` de macOS.

use super::provider::TtsProvider;

pub struct SayProvider;

impl TtsProvider for SayProvider {
    fn name(&self) -> &'static str {
        "say"
    }
}

const SAY_PID_FILE: &str = "/tmp/whisper-tts-say.pid";

fn detect_voice(text: &str) -> &'static str {
    let spanish_chars = ['á','é','í','ó','ú','ü','ñ','Á','É','Í','Ó','Ú','Ü','Ñ'];
    if text.chars().any(|c| spanish_chars.contains(&c)) { "Paulina" } else { "Samantha" }
}

impl SayProvider {
    pub fn say(&self, text: &str, rate: f32) {
        let voice = detect_voice(text);
        let wpm = (175.0 * rate).round() as u32;
        let mut cmd = std::process::Command::new("say");
        cmd.args(["-v", voice, "-r", &wpm.to_string()]);
        cmd.arg(text);
        match cmd.spawn()
        {
            Ok(mut child) => {
                let _ = std::fs::write(SAY_PID_FILE, child.id().to_string());
                let _ = child.wait();
                let _ = std::fs::remove_file(SAY_PID_FILE);
            }
            Err(e) => log::error!("TTS say: error — {}", e),
        }
    }
}
