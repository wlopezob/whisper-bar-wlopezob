// src/settings.rs
// Configuración persistente: única definición de claves SQLite y defaults.
// La usan la app (main.rs), la ventana de Configuración y el binario whisper-tts.

use crate::db::Db;
use crate::defaults;
use crate::tts::TtsConfig;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    // Traducción
    pub translate_enabled: bool,
    pub translate_dest: String,
    pub translate_provider: String,
    pub translate_ollama_prompt: String,
    // Azure MAI Transcribe
    pub azure_mai_enabled: bool,
    pub azure_mai_key: String,
    pub azure_mai_region: String,
    pub azure_mai_api_version: String,
    pub azure_mai_definition: String,
    // TTS
    pub tts_enabled: bool,
    pub tts_provider: String,
    pub tts_qwen_prompt: String,
    pub tts_qwen_temperature: String,
    pub tts_voice: String,
    pub gemini_api_key: String,
    pub tts_scene: String,
    pub tts_sample_context: String,
    pub tts_formatter_enabled: bool,
    pub tts_formatter_prompt: String,
    pub tts_playback_rate: String,
    pub tts_show_modal: bool,
    /// Nombres de sesión de Claude Code (los de /rename) cuyas respuestas se
    /// leen, separados por comas. Vacío = todas las sesiones.
    pub tts_sessions: String,
}

impl Settings {
    /// Lee toda la configuración de la DB; las claves ausentes toman su default.
    pub fn load(db: &Db) -> Self {
        let s = |key: &str, default: &str| db.get(key, default);
        let b = |key: &str| db.get(key, "false") == "true";
        Settings {
            translate_enabled: b("translate_enabled"),
            translate_dest: s("translate_dest_lang", defaults::TRANSLATE_DEST_LANG),
            translate_provider: s("translate_provider", defaults::TRANSLATE_DEFAULT_PROVIDER),
            translate_ollama_prompt: s("translate_ollama_prompt", defaults::TRANSLATE_OLLAMA_DEFAULT_PROMPT),
            azure_mai_enabled: b("azure_mai_enabled"),
            azure_mai_key: s("azure_mai_key", ""),
            azure_mai_region: s("azure_mai_region", ""),
            azure_mai_api_version: s("azure_mai_api_version", defaults::AZURE_MAI_API_VERSION),
            azure_mai_definition: s("azure_mai_definition", defaults::AZURE_MAI_DEFINITION),
            tts_enabled: b("tts_enabled"),
            tts_provider: s("tts_provider", defaults::TTS_DEFAULT_PROVIDER),
            tts_qwen_prompt: s("tts_qwen_prompt", defaults::TTS_QWEN_DEFAULT_PROMPT),
            tts_qwen_temperature: s("tts_qwen_temperature", defaults::TTS_QWEN_DEFAULT_TEMPERATURE),
            tts_voice: s("tts_voice", defaults::TTS_DEFAULT_VOICE),
            gemini_api_key: s("gemini_api_key", ""),
            tts_scene: s("tts_scene", defaults::TTS_DEFAULT_SCENE),
            tts_sample_context: s("tts_sample_context", defaults::TTS_DEFAULT_SAMPLE_CONTEXT),
            tts_formatter_enabled: b("tts_formatter_enabled"),
            tts_formatter_prompt: s("tts_formatter_prompt", defaults::FORMATTER_DEFAULT_PROMPT),
            tts_playback_rate: s("tts_playback_rate", defaults::TTS_DEFAULT_PLAYBACK_RATE),
            tts_show_modal: b("tts_show_modal"),
            tts_sessions: s("tts_sessions", ""),
        }
    }

    /// Persiste toda la configuración en la DB.
    pub fn save(&self, db: &Db) {
        let b = |v: bool| if v { "true" } else { "false" };
        let pairs: [(&str, &str); 22] = [
            ("translate_enabled", b(self.translate_enabled)),
            ("translate_dest_lang", &self.translate_dest),
            ("translate_provider", &self.translate_provider),
            ("translate_ollama_prompt", &self.translate_ollama_prompt),
            ("azure_mai_enabled", b(self.azure_mai_enabled)),
            ("azure_mai_key", &self.azure_mai_key),
            ("azure_mai_region", &self.azure_mai_region),
            ("azure_mai_api_version", &self.azure_mai_api_version),
            ("azure_mai_definition", &self.azure_mai_definition),
            ("tts_enabled", b(self.tts_enabled)),
            ("tts_provider", &self.tts_provider),
            ("tts_qwen_prompt", &self.tts_qwen_prompt),
            ("tts_qwen_temperature", &self.tts_qwen_temperature),
            ("tts_voice", &self.tts_voice),
            ("gemini_api_key", &self.gemini_api_key),
            ("tts_scene", &self.tts_scene),
            ("tts_sample_context", &self.tts_sample_context),
            ("tts_formatter_enabled", b(self.tts_formatter_enabled)),
            ("tts_formatter_prompt", &self.tts_formatter_prompt),
            ("tts_playback_rate", &self.tts_playback_rate),
            ("tts_show_modal", b(self.tts_show_modal)),
            ("tts_sessions", &self.tts_sessions),
        ];
        for (key, value) in pairs {
            db.set(key, value);
        }
    }

    /// Corrige lo que llega de la ventana antes de guardarlo.
    ///
    /// Un Definition inválido no se nota al guardar: revienta después, al
    /// transcribir, con un HTTP 400 de Azure. Se conserva el valor anterior
    /// antes que persistir algo que rompe la transcripción.
    pub fn sanitized(mut self, previous: &Settings) -> Settings {
        if self.azure_mai_api_version.trim().is_empty() {
            self.azure_mai_api_version = defaults::AZURE_MAI_API_VERSION.to_string();
        }
        if self.azure_mai_definition.trim().is_empty() {
            self.azure_mai_definition = defaults::AZURE_MAI_DEFINITION.to_string();
        } else if serde_json::from_str::<serde_json::Value>(&self.azure_mai_definition).is_err() {
            log::error!(
                "Azure MAI: Definition JSON inválido, se conserva el anterior. \
                 Revisa que las comillas sean rectas (\") y no tipográficas: {}",
                self.azure_mai_definition
            );
            self.azure_mai_definition = previous.azure_mai_definition.clone();
        }
        self
    }

    /// Azure MAI activo y con credenciales suficientes para transcribir.
    pub fn azure_ready(&self) -> bool {
        self.azure_mai_enabled && self.has_azure_credentials()
    }

    pub fn has_azure_credentials(&self) -> bool {
        !self.azure_mai_key.is_empty() && !self.azure_mai_region.is_empty()
    }

    /// ¿Se deben leer las respuestas de esta sesión?
    ///
    /// `session` es el nombre puesto con /rename en Claude Code (`Some("")` si
    /// la sesión no tiene nombre). `None` significa que el origen no informa de
    /// sesión (Codex, prueba manual): el filtro no aplica.
    pub fn should_speak_session(&self, session: Option<&str>) -> bool {
        let Some(session) = session else { return true };
        let mut allowed = self
            .tts_sessions
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .peekable();
        if allowed.peek().is_none() {
            return true;
        }
        let session = session.trim();
        !session.is_empty() && allowed.any(|n| n.eq_ignore_ascii_case(session))
    }

    pub fn playback_rate(&self) -> f32 {
        self.tts_playback_rate.parse().unwrap_or(1.0)
    }

    pub fn tts_config(&self) -> TtsConfig {
        TtsConfig {
            provider: self.tts_provider.clone(),
            voice: self.tts_voice.clone(),
            gemini_key: self.gemini_api_key.clone(),
            scene: self.tts_scene.clone(),
            sample_context: self.tts_sample_context.clone(),
            playback_rate: self.playback_rate(),
            qwen_prompt: self.tts_qwen_prompt.clone(),
            qwen_temperature: self.tts_qwen_temperature.parse().unwrap_or(0.7),
        }
    }
}

/// Migra la definition de Azure MAI de una versión anterior a la actual.
///
/// El valor vive en SQLite, así que cambiar el default de `defaults.rs` no basta:
/// las instalaciones existentes seguirían llamando a MAI-Transcribe-1.5. Solo se
/// reemplaza si el valor guardado coincide exactamente con un default antiguo —
/// si el usuario lo editó, se respeta.
pub fn migrate(db: &Db) {
    let stored = db.get("azure_mai_definition", defaults::AZURE_MAI_DEFINITION);
    if defaults::AZURE_MAI_LEGACY_DEFINITIONS.contains(&stored.as_str()) {
        log::info!("Azure MAI: migrando definition a MAI-Transcribe-2");
        db.set("azure_mai_definition", defaults::AZURE_MAI_DEFINITION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Settings {
        Settings {
            translate_enabled: false,
            translate_dest: "es".into(),
            translate_provider: "azure".into(),
            translate_ollama_prompt: String::new(),
            azure_mai_enabled: true,
            azure_mai_key: "k".into(),
            azure_mai_region: "eastus".into(),
            azure_mai_api_version: "2025-10-15".into(),
            azure_mai_definition: r#"{"a":1}"#.into(),
            tts_enabled: true,
            tts_provider: "gemini".into(),
            tts_qwen_prompt: String::new(),
            tts_qwen_temperature: "0.7".into(),
            tts_voice: "Sulafat".into(),
            gemini_api_key: String::new(),
            tts_scene: String::new(),
            tts_sample_context: String::new(),
            tts_formatter_enabled: false,
            tts_formatter_prompt: String::new(),
            tts_playback_rate: "1.25".into(),
            tts_show_modal: false,
            tts_sessions: String::new(),
        }
    }

    /// Si una clave de `save` no coincide con la de `load`, el valor se pierde
    /// al reiniciar. Todos los campos de `base()` difieren de su default.
    #[test]
    fn save_y_load_usan_las_mismas_claves() {
        let db = Db::open_in_memory().unwrap();
        let mut s = base();
        s.translate_enabled = true;
        s.translate_ollama_prompt = "p".into();
        s.tts_qwen_prompt = "q".into();
        s.tts_qwen_temperature = "0.3".into();
        s.gemini_api_key = "g".into();
        s.tts_scene = "e".into();
        s.tts_sample_context = "c".into();
        s.tts_formatter_enabled = true;
        s.tts_formatter_prompt = "f".into();
        s.tts_show_modal = true;
        s.tts_provider = "qwen".into();
        s.tts_voice = "Kore".into();
        s.translate_dest = "en".into();
        s.translate_provider = "ollama".into();
        s.azure_mai_api_version = "2099-01-01".into();
        s.tts_sessions = "mods, voz".into();

        s.save(&db);
        assert_eq!(Settings::load(&db), s);
    }

    #[test]
    fn migra_solo_definitions_legacy() {
        let db = Db::open_in_memory().unwrap();
        db.set("azure_mai_definition", defaults::AZURE_MAI_LEGACY_DEFINITIONS[0]);
        migrate(&db);
        assert_eq!(Settings::load(&db).azure_mai_definition, defaults::AZURE_MAI_DEFINITION);

        db.set("azure_mai_definition", r#"{"editada":true}"#);
        migrate(&db);
        assert_eq!(Settings::load(&db).azure_mai_definition, r#"{"editada":true}"#);
    }

    #[test]
    fn definition_invalida_conserva_la_anterior() {
        let previous = base();
        let mut nuevo = base();
        nuevo.azure_mai_definition = "{“comillas”: tipográficas}".into();
        assert_eq!(nuevo.sanitized(&previous).azure_mai_definition, r#"{"a":1}"#);
    }

    #[test]
    fn vacios_toman_el_default() {
        let mut nuevo = base();
        nuevo.azure_mai_definition = "  ".into();
        nuevo.azure_mai_api_version = String::new();
        let s = nuevo.sanitized(&base());
        assert_eq!(s.azure_mai_definition, defaults::AZURE_MAI_DEFINITION);
        assert_eq!(s.azure_mai_api_version, defaults::AZURE_MAI_API_VERSION);
    }

    #[test]
    fn azure_ready_exige_clave_y_region() {
        assert!(base().azure_ready());
        let mut s = base();
        s.azure_mai_region.clear();
        assert!(!s.azure_ready());
        s = base();
        s.azure_mai_enabled = false;
        assert!(!s.azure_ready());
    }

    #[test]
    fn sin_filtro_se_leen_todas_las_sesiones() {
        let s = base();
        assert!(s.should_speak_session(Some("cualquiera")));
        assert!(s.should_speak_session(Some("")));
        assert!(s.should_speak_session(None));
    }

    #[test]
    fn con_filtro_solo_las_sesiones_listadas() {
        let mut s = base();
        s.tts_sessions = " mods , Voz ,,".into();
        assert!(s.should_speak_session(Some("mods")));
        assert!(s.should_speak_session(Some("voz")), "sin distinguir mayúsculas");
        assert!(s.should_speak_session(Some(" mods ")));
        assert!(!s.should_speak_session(Some("otra")));
        assert!(!s.should_speak_session(Some("")), "sesión sin nombre");
        assert!(!s.should_speak_session(Some("mod")), "coincidencia exacta, no prefijo");
    }

    #[test]
    fn origen_sin_sesion_ignora_el_filtro() {
        let mut s = base();
        s.tts_sessions = "mods".into();
        assert!(s.should_speak_session(None));
    }

    #[test]
    fn rate_invalido_cae_a_1() {
        assert_eq!(base().playback_rate(), 1.25);
        let mut s = base();
        s.tts_playback_rate = "rápido".into();
        assert_eq!(s.playback_rate(), 1.0);
    }
}
