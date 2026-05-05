// src/main.rs

mod azure_transcriber;
mod config;
mod db;
mod defaults;
mod hotkey;
mod llm;
mod logger;
mod recorder;
mod settings_window;
mod transcriber;

use settings_window::{SettingsValues, show_settings_modal};

use config::Config;
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use hotkey::HotkeyHandler;
use recorder::Recorder;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    Idle,
    Recording,
    Processing,
}

enum UiMsg {
    SetTitle(&'static str),
    PasteText(String),
}

// ── Struct principal que implementa el ApplicationHandler de winit ─────────────
struct WhisperApp {
    // Config e estado (inicializados en new())
    config: Arc<Config>,
    db: Arc<db::Db>,
    app_state: Arc<Mutex<AppState>>,
    recorder: Arc<Mutex<Recorder>>,
    current_language: Arc<Mutex<String>>,
    llm_enabled: Arc<Mutex<bool>>,
    llm_model: Arc<Mutex<String>>,
    grammar_prompt_es: Arc<Mutex<String>>,
    grammar_prompt_en: Arc<Mutex<String>>,
    translate_enabled: Arc<Mutex<bool>>,
    translate_dest_lang: Arc<Mutex<String>>,
    // Azure MAI Transcribe
    azure_mai_enabled: Arc<Mutex<bool>>,
    azure_mai_key: Arc<Mutex<String>>,
    azure_mai_region: Arc<Mutex<String>>,
    azure_mai_model: Arc<Mutex<String>>,
    azure_mai_api_version: Arc<Mutex<String>>,
    azure_mai_definition: Arc<Mutex<String>>,
    ui_tx: mpsc::Sender<UiMsg>,
    ui_rx: mpsc::Receiver<UiMsg>,

    // UI (inicializado en resumed(), requiere NSApp activo)
    tray: Option<TrayIcon>,
    quit_id: Option<tray_icon::menu::MenuId>,
    log_id: Option<tray_icon::menu::MenuId>,
    settings_id: Option<tray_icon::menu::MenuId>,
    lang_es_item: Option<tray_icon::menu::MenuItem>,
    lang_en_item: Option<tray_icon::menu::MenuItem>,
    improve_item: Option<tray_icon::menu::MenuItem>,
    azure_item: Option<tray_icon::menu::MenuItem>,
    // (MenuItem, filename) — uno por cada .gguf encontrado (solo lectura en tray)
    llm_model_items: Vec<(tray_icon::menu::MenuItem, String)>,
    _hotkey_handler: Option<HotkeyHandler>,
    hotkey_id: Option<u32>,
}

impl WhisperApp {
    fn new() -> Self {
        let config = Arc::new(Config::new());

        let db = db::Db::open().expect("No se pudo abrir la base de datos");
        let language = db.get("language", defaults::LANGUAGE);
        let llm_enabled = db.get("llm_enabled", "false") == "true";
        let llm_model = db.get("llm_model", "");
        let grammar_prompt_es = db.get("grammar_prompt_es", defaults::GRAMMAR_PROMPT_ES);
        let grammar_prompt_en = db.get("grammar_prompt_en", defaults::GRAMMAR_PROMPT_EN);
        let translate_enabled = db.get("translate_enabled", "false") == "true";
        let translate_dest_lang = db.get("translate_dest_lang", defaults::TRANSLATE_DEST_LANG);
        let azure_mai_enabled = db.get("azure_mai_enabled", "false") == "true";
        let azure_mai_key = db.get("azure_mai_key", "");
        let azure_mai_region = db.get("azure_mai_region", "");
        let azure_mai_model = db.get("azure_mai_model", "");
        let azure_mai_api_version = db.get("azure_mai_api_version", defaults::AZURE_MAI_API_VERSION);
        let azure_mai_definition = db.get("azure_mai_definition", defaults::AZURE_MAI_DEFINITION);

        log::info!("=== whisperwlopezob v0.1.0 ===");
        log::info!(
            "whisper-cli: {} {}",
            if config.is_whisper_cli_valid() { "✅" } else { "❌" },
            if config.is_whisper_cli_valid() { &config.whisper_cli_path } else { "no encontrado" }
        );
        log::info!(
            "modelo:      {} {}",
            if config.is_model_valid() { "✅" } else { "❌" },
            config.model_filename()
        );
        log::info!("idioma:      {}", language);
        log::info!(
            "llama-cli:   {} {}",
            if config.is_llama_cli_valid() { "✅" } else { "❌" },
            if config.is_llama_cli_valid() { &config.llama_cli_path } else { "no encontrado" }
        );
        log::info!("modelos LLM: {} encontrados", config.llm_models.len());
        log::info!(
            "Azure MAI:   {} región={}",
            if azure_mai_enabled { "✅ activo" } else { "☐ inactivo" },
            if azure_mai_region.is_empty() { "—" } else { &azure_mai_region }
        );

        let (ui_tx, ui_rx) = mpsc::channel();

        WhisperApp {
            config,
            db,
            app_state: Arc::new(Mutex::new(AppState::Idle)),
            recorder: Arc::new(Mutex::new(Recorder::new())),
            current_language: Arc::new(Mutex::new(language)),
            llm_enabled: Arc::new(Mutex::new(llm_enabled)),
            llm_model: Arc::new(Mutex::new(llm_model)),
            grammar_prompt_es: Arc::new(Mutex::new(grammar_prompt_es)),
            grammar_prompt_en: Arc::new(Mutex::new(grammar_prompt_en)),
            translate_enabled: Arc::new(Mutex::new(translate_enabled)),
            translate_dest_lang: Arc::new(Mutex::new(translate_dest_lang)),
            azure_mai_enabled: Arc::new(Mutex::new(azure_mai_enabled)),
            azure_mai_key: Arc::new(Mutex::new(azure_mai_key)),
            azure_mai_region: Arc::new(Mutex::new(azure_mai_region)),
            azure_mai_model: Arc::new(Mutex::new(azure_mai_model)),
            azure_mai_api_version: Arc::new(Mutex::new(azure_mai_api_version)),
            azure_mai_definition: Arc::new(Mutex::new(azure_mai_definition)),
            ui_tx,
            ui_rx,
            tray: None,
            quit_id: None,
            log_id: None,
            settings_id: None,
            lang_es_item: None,
            lang_en_item: None,
            improve_item: None,
            azure_item: None,
            llm_model_items: Vec::new(),
            _hotkey_handler: None,
            hotkey_id: None,
        }
    }

    /// Crea tray icon y registra hotkey — llamado una vez desde resumed()
    /// cuando NSApplication ya está activo
    fn setup_ui(&mut self) {
        log::debug!("setup_ui: creando tray icon y hotkey...");

        // ── Menú ──────────────────────────────────────────────────────────────
        let menu = Menu::new();

        let title_item = MenuItem::new("whisperwlopezob", false, None);
        let hint_item = MenuItem::new("Mantén ⌘⌥W para grabar / suelta para transcribir", false, None);

        let cli_label = if self.config.is_whisper_cli_valid() {
            format!("✅ whisper-cli: {}", self.config.whisper_cli_path)
        } else {
            "❌ whisper-cli no encontrado".to_string()
        };
        let model_label = if self.config.is_model_valid() {
            format!("✅ Modelo: {}", self.config.model_filename())
        } else {
            "❌ Modelo no encontrado".to_string()
        };

        let lang = self.current_language.lock().unwrap().clone();
        let es_label = if lang == "es" { "✓ Español" } else { "  Español" };
        let en_label = if lang == "en" { "✓ English" } else { "  English" };

        let cli_item = MenuItem::new(cli_label, false, None);
        let model_item = MenuItem::new(model_label, false, None);
        let lang_header = MenuItem::new("Idioma", false, None);
        let lang_es_item = MenuItem::new(es_label, false, None);
        let lang_en_item = MenuItem::new(en_label, false, None);
        // ── Sección LLM ───────────────────────────────────────────────────────
        let llama_label = if self.config.is_llama_cli_valid() {
            format!("✅ llama-cli: {}", self.config.llama_cli_path)
        } else {
            "❌ llama-cli no encontrado (brew install llama.cpp)".to_string()
        };
        let llama_item = MenuItem::new(llama_label, false, None);

        let selected_model = self.llm_model.lock().unwrap().clone();
        let llm_enabled_val = *self.llm_enabled.lock().unwrap();

        // Un MenuItem informativo (solo lectura) por cada .gguf encontrado
        let mut llm_model_entries: Vec<(tray_icon::menu::MenuItem, String)> = vec![];
        let llm_models_header = MenuItem::new("Modelo LLM:", false, None);

        if self.config.llm_models.is_empty() {
            // Sin modelos disponibles
        } else {
            for model_path in &self.config.llm_models {
                let filename = std::path::Path::new(model_path)
                    .file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                let label = if filename == selected_model {
                    format!("✓ {}", filename)
                } else {
                    format!("  {}", filename)
                };
                let item = MenuItem::new(label, false, None);
                llm_model_entries.push((item, filename));
            }
        }

        let no_models_item = if self.config.llm_models.is_empty() {
            Some(MenuItem::new(
                format!(
                    "❌ Sin modelos en ~/{}/{}/",
                    defaults::APP_CONFIG_DIR,
                    defaults::LLM_MODELS_DIR
                ),
                false,
                None,
            ))
        } else {
            None
        };

        let improve_enabled = self.config.is_llm_available();
        let improve_label = if llm_enabled_val { "☑ Mejorar gramática" } else { "☐ Mejorar gramática" };
        let improve_item = MenuItem::new(
            if improve_enabled {
                improve_label
            } else {
                "☐ Mejorar gramática (no disponible)"
            },
            false,
            None,
        );

        // ── Azure MAI ─────────────────────────────────────────────────────────
        let azure_enabled_val = *self.azure_mai_enabled.lock().unwrap();
        let azure_region_val = self.azure_mai_region.lock().unwrap().clone();
        let azure_item = MenuItem::new(
            azure_status_label(azure_enabled_val, &azure_region_val),
            false,
            None,
        );

        let settings_item = MenuItem::new("Configuración...", true, None);
        let settings_id = settings_item.id().clone();
        let ver_log_item = MenuItem::new("Ver log", true, None);
        let ver_log_id = ver_log_item.id().clone();
        let log_path_item = MenuItem::new(format!("Log: {}", logger::log_path()), false, None);
        let quit_item = MenuItem::new("Salir", true, None);
        let quit_id = quit_item.id().clone();

        let _ = menu.append(&title_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&hint_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&cli_item);
        let _ = menu.append(&model_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&lang_header);
        let _ = menu.append(&lang_es_item);
        let _ = menu.append(&lang_en_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&llama_item);
        let _ = menu.append(&llm_models_header);
        if let Some(ref item) = no_models_item {
            let _ = menu.append(item);
        }
        for (item, _) in &llm_model_entries {
            let _ = menu.append(item);
        }
        let _ = menu.append(&improve_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&azure_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&settings_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&log_path_item);
        let _ = menu.append(&ver_log_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit_item);

        // ── Tray icon ─────────────────────────────────────────────────────────
        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_title("🎙")
            .with_tooltip("whisperwlopezob — ⌘⌥W para grabar")
            .build()
        {
            Ok(tray) => {
                log::info!("Tray icon creado");
                self.tray = Some(tray);
                self.quit_id = Some(quit_id);
                self.log_id = Some(ver_log_id);
                self.settings_id = Some(settings_id);
                self.lang_es_item = Some(lang_es_item);
                self.lang_en_item = Some(lang_en_item);
                self.improve_item = Some(improve_item);
                self.azure_item = Some(azure_item);
                self.llm_model_items = llm_model_entries;
            }
            Err(e) => log::error!("Error creando tray icon: {}", e),
        }

        // ── Hotkey ────────────────────────────────────────────────────────────
        match HotkeyHandler::new() {
            Ok(h) => {
                let id = h.hotkey_id();
                log::info!("hotkey: ✅ ⌘⌥W registrado (id={})", id);
                self._hotkey_handler = Some(h);
                self.hotkey_id = Some(id);
            }
            Err(e) => {
                log::error!("hotkey: ❌ {}", e);
                log::error!("→ System Settings → Privacy & Security → Accessibility");
            }
        }

        // Verificar permiso Accessibility
        if is_accessibility_trusted() {
            log::info!("Accessibility: ✅ permiso concedido");
        } else {
            log::error!("Accessibility: ❌ SIN PERMISO — hotkey no funcionará");
            log::error!("→ System Settings → Privacy & Security → Accessibility → activa whisperwlopezob");
        }

        log::info!("whisperwlopezob activo. Mantén ⌘⌥W para grabar, suelta para transcribir.");
        log::info!("Log en tiempo real: tail -f {}", logger::log_path());

        // Solicitar permiso de micrófono al arrancar (antes del primer uso)
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            probe_microphone();
        });
    }

    /// Cambia el idioma activo, actualiza checkmarks en el menú y persiste en DB
    fn set_language(&self, lang: &str) {
        *self.current_language.lock().unwrap() = lang.to_string();
        self.db.set("language", lang);

        let (es_label, en_label) = if lang == "es" {
            ("✓ Español", "  English")
        } else {
            ("  Español", "✓ English")
        };
        if let Some(ref item) = self.lang_es_item { item.set_text(es_label); }
        if let Some(ref item) = self.lang_en_item { item.set_text(en_label); }

        let name = if lang == "es" { "Español" } else { "English" };
        log::info!("Idioma cambiado a: {}", name);
    }

    fn apply_settings(&self, v: SettingsValues) {
        // Idioma
        self.set_language(&v.language);

        // Gramática
        *self.llm_enabled.lock().unwrap() = v.grammar_enabled;
        self.db.set("llm_enabled", if v.grammar_enabled { "true" } else { "false" });
        let label = if !self.config.is_llm_available() {
            "☐ Mejorar gramática (no disponible)".to_string()
        } else if v.grammar_enabled {
            "☑ Mejorar gramática".to_string()
        } else {
            "☐ Mejorar gramática".to_string()
        };
        if let Some(ref item) = self.improve_item { item.set_text(label); }

        *self.llm_model.lock().unwrap() = v.grammar_model.clone();
        self.db.set("llm_model", &v.grammar_model);
        for (item, name) in &self.llm_model_items {
            item.set_text(if *name == v.grammar_model {
                format!("✓ {}", name)
            } else {
                format!("  {}", name)
            });
        }

        let prompt_es = if v.grammar_prompt_es.is_empty() {
            defaults::GRAMMAR_PROMPT_ES.to_string()
        } else {
            v.grammar_prompt_es.clone()
        };
        let prompt_en = if v.grammar_prompt_en.is_empty() {
            defaults::GRAMMAR_PROMPT_EN.to_string()
        } else {
            v.grammar_prompt_en.clone()
        };
        *self.grammar_prompt_es.lock().unwrap() = prompt_es.clone();
        *self.grammar_prompt_en.lock().unwrap() = prompt_en.clone();
        self.db.set("grammar_prompt_es", &prompt_es);
        self.db.set("grammar_prompt_en", &prompt_en);

        // Traducción
        *self.translate_enabled.lock().unwrap() = v.translate_enabled;
        self.db.set("translate_enabled", if v.translate_enabled { "true" } else { "false" });
        *self.translate_dest_lang.lock().unwrap() = v.translate_dest.clone();
        self.db.set("translate_dest_lang", &v.translate_dest);

        // Azure MAI Transcribe
        *self.azure_mai_enabled.lock().unwrap() = v.azure_mai_enabled;
        self.db.set("azure_mai_enabled", if v.azure_mai_enabled { "true" } else { "false" });
        *self.azure_mai_key.lock().unwrap() = v.azure_mai_key.clone();
        self.db.set("azure_mai_key", &v.azure_mai_key);
        *self.azure_mai_region.lock().unwrap() = v.azure_mai_region.clone();
        self.db.set("azure_mai_region", &v.azure_mai_region);
        *self.azure_mai_model.lock().unwrap() = v.azure_mai_model.clone();
        self.db.set("azure_mai_model", &v.azure_mai_model);
        let api_ver = if v.azure_mai_api_version.is_empty() {
            defaults::AZURE_MAI_API_VERSION.to_string()
        } else {
            v.azure_mai_api_version.clone()
        };
        let definition = if v.azure_mai_definition.is_empty() {
            defaults::AZURE_MAI_DEFINITION.to_string()
        } else {
            v.azure_mai_definition.clone()
        };
        *self.azure_mai_api_version.lock().unwrap() = api_ver.clone();
        self.db.set("azure_mai_api_version", &api_ver);
        *self.azure_mai_definition.lock().unwrap() = definition.clone();
        self.db.set("azure_mai_definition", &definition);
        if let Some(ref item) = self.azure_item {
            item.set_text(azure_status_label(v.azure_mai_enabled, &v.azure_mai_region));
        }
        log::info!(
            "Azure MAI: {} región={} api-version={} definition={}",
            if v.azure_mai_enabled { "activo" } else { "inactivo" },
            if v.azure_mai_region.is_empty() { "—" } else { &v.azure_mai_region },
            api_ver,
            definition,
        );

        log::info!("Configuración aplicada");
    }
}

impl ApplicationHandler for WhisperApp {
    /// Llamado cuando NSApplication está listo — aquí creamos la UI
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.tray.is_none() {
            self.setup_ui();
        }
    }

    /// No usamos ventanas pero winit requiere implementarlo
    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    /// Llamado antes de que el event loop espere nuevos eventos — nuestro "tick"
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Actualizar tray icon desde worker thread
        if let Some(ref tray) = self.tray {
            while let Ok(msg) = self.ui_rx.try_recv() {
                match msg {
                    UiMsg::SetTitle(title) => {
                        log::debug!("UI: set_title → {}", title);
                        let _ = tray.set_title(Some(title));
                    }
                    UiMsg::PasteText(text) => {
                        log::debug!("UI: paste_text ({} chars)", text.chars().count());
                        paste_text(&text);
                    }
                }
            }
        }

        // Eventos de menú
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if self.quit_id.as_ref() == Some(event.id()) {
                log::info!("Saliendo...");
                event_loop.exit();
            } else if self.log_id.as_ref() == Some(event.id()) {
                let _ = std::process::Command::new("open")
                    .arg("-t")
                    .arg(logger::log_path())
                    .spawn();
            } else if self.settings_id.as_ref() == Some(event.id()) {
                let current = SettingsValues {
                    language: self.current_language.lock().unwrap().clone(),
                    grammar_enabled: *self.llm_enabled.lock().unwrap(),
                    grammar_model: self.llm_model.lock().unwrap().clone(),
                    grammar_prompt_es: self.grammar_prompt_es.lock().unwrap().clone(),
                    grammar_prompt_en: self.grammar_prompt_en.lock().unwrap().clone(),
                    translate_enabled: *self.translate_enabled.lock().unwrap(),
                    translate_dest: self.translate_dest_lang.lock().unwrap().clone(),
                    azure_mai_enabled: *self.azure_mai_enabled.lock().unwrap(),
                    azure_mai_key: self.azure_mai_key.lock().unwrap().clone(),
                    azure_mai_region: self.azure_mai_region.lock().unwrap().clone(),
                    azure_mai_model: self.azure_mai_model.lock().unwrap().clone(),
                    azure_mai_api_version: self.azure_mai_api_version.lock().unwrap().clone(),
                    azure_mai_definition: self.azure_mai_definition.lock().unwrap().clone(),
                };
                let models: Vec<String> = self.config.llm_models.iter()
                    .filter_map(|p| std::path::Path::new(p).file_name()?.to_str().map(|s| s.to_string()))
                    .collect();
                if let Some(values) = show_settings_modal(&current, &models) {
                    self.apply_settings(values);
                }
            }
        }

        // Eventos de hotkey
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if Some(event.id) == self.hotkey_id {
                match event.state {
                    HotKeyState::Pressed => handle_hotkey_pressed(
                        &self.app_state,
                        &self.recorder,
                        &self.config,
                        &self.ui_tx,
                        &self.azure_mai_enabled,
                        &self.azure_mai_key,
                        &self.azure_mai_region,
                    ),
                    HotKeyState::Released => handle_hotkey_released(
                        &self.app_state,
                        &self.recorder,
                        &self.config,
                        &self.ui_tx,
                        &self.current_language,
                        &self.llm_enabled,
                        &self.llm_model,
                        &self.grammar_prompt_es,
                        &self.grammar_prompt_en,
                        &self.translate_enabled,
                        &self.translate_dest_lang,
                        &self.azure_mai_enabled,
                        &self.azure_mai_key,
                        &self.azure_mai_region,
                        &self.azure_mai_model,
                        &self.azure_mai_api_version,
                        &self.azure_mai_definition,
                    ),
                }
            }
        }

        // Volver a llamar en 20ms
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(20),
        ));
    }
}

fn main() {
    logger::init();
    install_panic_hook();

    // winit se encarga de inicializar NSApplication con política Accessory
    // (sin icono en Dock, solo barra de menú) — reemplaza init_macos_app()
    let event_loop = EventLoop::builder()
        .with_activation_policy(ActivationPolicy::Accessory)
        .with_default_menu(true)
        .build()
        .expect("Error creando event loop");

    let mut app = WhisperApp::new();

    event_loop.run_app(&mut app).expect("Error en event loop");
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {}", info);
        if let Some(loc) = info.location() {
            log::error!("PANIC location: {}:{}:{}", loc.file(), loc.line(), loc.column());
        }
        let bt = std::backtrace::Backtrace::force_capture();
        log::error!("PANIC backtrace:\n{:?}", bt);
    }));
}

/// Verifica si el proceso tiene permiso de Accessibility en macOS
fn is_accessibility_trusted() -> bool {
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

/// Pressed: inicia grabación si está en reposo
fn handle_hotkey_pressed(
    app_state: &Arc<Mutex<AppState>>,
    recorder: &Arc<Mutex<Recorder>>,
    config: &Arc<Config>,
    ui_tx: &mpsc::Sender<UiMsg>,
    azure_mai_enabled: &Arc<Mutex<bool>>,
    azure_mai_key: &Arc<Mutex<String>>,
    azure_mai_region: &Arc<Mutex<String>>,
) {
    let mut state = app_state.lock().unwrap();

    match *state {
        AppState::Idle => {
            // Permitir grabación si whisper local está listo O si Azure MAI está configurado
            let azure_on = *azure_mai_enabled.lock().unwrap();
            let azure_key = azure_mai_key.lock().unwrap().clone();
            let azure_region = azure_mai_region.lock().unwrap().clone();
            let azure_ready = azure_on && !azure_key.is_empty() && !azure_region.is_empty();

            if !azure_ready && !config.is_valid() {
                log::warn!(
                    "Config inválida: verifica whisper-cli y modelo, o activa Azure MAI con clave y región"
                );
                return;
            }
            let mut rec = recorder.lock().unwrap();
            match rec.start() {
                Ok(()) => {
                    *state = AppState::Recording;
                    let _ = ui_tx.send(UiMsg::SetTitle("🔴"));
                    log::info!("🔴 Grabando... (suelta ⌘⌥W para transcribir)");
                }
                Err(e) => log::error!("Error al iniciar grabación: {}", e),
            }
        }
        AppState::Recording | AppState::Processing => {
            // Tecla repetida por autorepeat del SO — ignorar
        }
    }
}

/// Released: detiene grabación y lanza transcripción (+ corrección LLM y/o traducción si están activas)
fn handle_hotkey_released(
    app_state: &Arc<Mutex<AppState>>,
    recorder: &Arc<Mutex<Recorder>>,
    config: &Arc<Config>,
    ui_tx: &mpsc::Sender<UiMsg>,
    current_language: &Arc<Mutex<String>>,
    llm_enabled: &Arc<Mutex<bool>>,
    llm_model: &Arc<Mutex<String>>,
    grammar_prompt_es: &Arc<Mutex<String>>,
    grammar_prompt_en: &Arc<Mutex<String>>,
    translate_enabled: &Arc<Mutex<bool>>,
    translate_dest_lang: &Arc<Mutex<String>>,
    azure_mai_enabled: &Arc<Mutex<bool>>,
    azure_mai_key: &Arc<Mutex<String>>,
    azure_mai_region: &Arc<Mutex<String>>,
    azure_mai_model_ref: &Arc<Mutex<String>>,
    azure_mai_api_version_ref: &Arc<Mutex<String>>,
    azure_mai_definition_ref: &Arc<Mutex<String>>,
) {
    let mut state = app_state.lock().unwrap();

    match *state {
        AppState::Recording => {
            let mut rec = recorder.lock().unwrap();
            let duration_result = rec.stop();
            let audio_path = rec.output_path().to_string();
            drop(rec);

            match duration_result {
                Ok(dur) if dur < config.min_recording_duration => {
                    log::warn!(
                        "Grabación muy corta ({:.1}s < {:.1}s mínimo), ignorando",
                        dur, config.min_recording_duration
                    );
                    *state = AppState::Idle;
                    let _ = ui_tx.send(UiMsg::SetTitle("🎙"));
                }
                Ok(dur) => {
                    log::info!("⏳ Transcribiendo {:.1}s de audio...", dur);
                    *state = AppState::Processing;
                    let _ = ui_tx.send(UiMsg::SetTitle("⏳"));
                    drop(state);

                    let app_state = app_state.clone();
                    let ui_tx = ui_tx.clone();
                    let cli = config.whisper_cli_path.clone();
                    let model = config.model_path.clone();
                    let lang = current_language.lock().unwrap().clone();
                    let use_llm = *llm_enabled.lock().unwrap();
                    let selected_model = llm_model.lock().unwrap().clone();
                    let llama_cli = config.llama_cli_path.clone();
                    let prompt_es = grammar_prompt_es.lock().unwrap().clone();
                    let prompt_en = grammar_prompt_en.lock().unwrap().clone();
                    let llm_available = config.is_llm_available();
                    let llm_model_path = if use_llm && !selected_model.is_empty() {
                        config.llm_model_path(&selected_model)
                    } else {
                        None
                    };
                    let use_translate = *translate_enabled.lock().unwrap();
                    let dest_lang = translate_dest_lang.lock().unwrap().clone();
                    // Azure MAI
                    let use_azure = *azure_mai_enabled.lock().unwrap();
                    let azure_key = azure_mai_key.lock().unwrap().clone();
                    let azure_region = azure_mai_region.lock().unwrap().clone();
                    let _azure_model = azure_mai_model_ref.lock().unwrap().clone();
                    let azure_api_version = azure_mai_api_version_ref.lock().unwrap().clone();
                    let azure_definition = azure_mai_definition_ref.lock().unwrap().clone();
                    log::info!(
                        "🧭 Pipeline: 1/3 transcripción → 2/3 corrección LLM → 3/3 traducción (origen: {}, destino: {}, llm: {}, traducción: {})",
                        lang,
                        dest_lang,
                        if use_llm { "activa" } else { "desactivada" },
                        if use_translate { "activa" } else { "desactivada" }
                    );
                    if use_llm {
                        if !llm_available {
                            log::warn!(
                                "LLM habilitado, pero no disponible (falta llama-cli o modelos .gguf)"
                            );
                        } else if selected_model.is_empty() {
                            log::warn!("LLM habilitado, pero sin modelo seleccionado");
                        } else if llm_model_path.is_some() {
                            log::info!(
                                "LLM habilitado para esta transcripción (modelo: {})",
                                selected_model
                            );
                        } else {
                            log::warn!(
                                "LLM habilitado, pero el modelo '{}' no existe en disco",
                                selected_model
                            );
                        }
                    } else {
                        log::debug!("LLM deshabilitado para esta transcripción");
                    }

                    std::thread::spawn(move || {
                        log::info!("🧭 Paso 1/3: iniciando transcripción");
                        // Elegir backend: Azure MAI o local whisper-cli
                        let transcription_result = if use_azure {
                            log::info!("🔵 Azure MAI: transcribiendo (región={})...", azure_region);
                            azure_transcriber::transcribe(
                                &azure_key,
                                &azure_region,
                                &azure_api_version,
                                &azure_definition,
                                &audio_path,
                            )
                        } else {
                            transcriber::transcribe(&cli, &model, &lang, &audio_path)
                        };

                        match transcription_result {
                            Ok(text) => {
                                log::info!("✅ Paso 1/3 completado");
                                log::info!("✅ Transcripción: \"{}\"", text);

                                let final_text = if let Some(ref lm_path) = llm_model_path {
                                    log::info!("🧭 Paso 2/3: iniciando corrección gramatical");
                                    log::info!(
                                        "🔧 LLM: iniciando corrección (modelo: {})",
                                        selected_model
                                    );
                                    let llm_start = Instant::now();
                                    let system_prompt = if lang == "es" {
                                        prompt_es.as_str()
                                    } else {
                                        prompt_en.as_str()
                                    };
                                    log::info!(
                                        "🧩 LLM prompt activo (idioma: {}): {}",
                                        if lang == "es" { "es" } else { "en" },
                                        system_prompt.replace('\n', "\\n")
                                    );
                                    match llm::correct_grammar(&llama_cli, lm_path, &text, system_prompt) {
                                        Ok(corrected) => {
                                            log::info!(
                                                "✅ LLM aplicado en {:.2}s",
                                                llm_start.elapsed().as_secs_f64()
                                            );
                                            if corrected == text {
                                                log::info!("LLM completado: sin cambios en el texto");
                                            } else {
                                                log::info!("LLM completado: texto corregido");
                                            }
                                            log::info!("✅ Paso 2/3 completado");
                                            log::info!("✅ Corregido: \"{}\"", corrected);
                                            corrected
                                        }
                                        Err(e) => {
                                            log::warn!("⚠️ Paso 2/3 falló; se mantiene transcripción original");
                                            log::warn!("LLM falló (usando transcripción original): {}", e);
                                            text
                                        }
                                    }
                                } else {
                                    if use_llm {
                                        log::warn!("⏭ Paso 2/3 omitido: mejora gramatical sin modelo/configuración válida");
                                        log::warn!(
                                            "LLM no se aplicó por configuración incompleta en esta transcripción"
                                        );
                                    } else {
                                        log::info!("⏭ Paso 2/3 omitido: mejora gramatical desactivada");
                                    }
                                    text
                                };

                                let final_text = if use_translate && dest_lang != lang {
                                    if let Some(ref lm_path) = llm_model_path {
                                        log::info!(
                                            "🧭 Paso 3/3: iniciando traducción ({} → {})",
                                            lang,
                                            dest_lang
                                        );
                                        match llm::translate_text(&llama_cli, lm_path, &final_text, &dest_lang) {
                                            Ok(t) => {
                                                log::info!("✅ Paso 3/3 completado");
                                                log::info!("✅ Traducción completada");
                                                t
                                            }
                                            Err(e) => {
                                                log::warn!("⚠️ Paso 3/3 falló; se mantiene texto previo");
                                                log::warn!("Traducción falló: {}", e);
                                                final_text
                                            }
                                        }
                                    } else {
                                        log::warn!("⏭ Paso 3/3 omitido: traducción activa pero sin modelo LLM disponible");
                                        log::warn!("Traducción solicitada pero sin modelo LLM disponible");
                                        final_text
                                    }
                                } else {
                                    if !use_translate {
                                        log::info!("⏭ Paso 3/3 omitido: traducción desactivada");
                                    } else {
                                        log::info!(
                                            "⏭ Paso 3/3 omitido: idioma destino igual al origen ({})",
                                            lang
                                        );
                                    }
                                    final_text
                                };

                                log::info!("🏁 Pipeline completado");
                                if ui_tx.send(UiMsg::PasteText(final_text)).is_err() {
                                    log::error!("No se pudo enviar PasteText al hilo UI");
                                }
                            }
                            Err(e) => {
                                log::error!("❌ Paso 1/3 falló: {}", e);
                                log::error!("Error de transcripción: {}", e);
                            }
                        }
                        let mut s = app_state.lock().unwrap();
                        *s = AppState::Idle;
                        let _ = ui_tx.send(UiMsg::SetTitle("🎙"));
                    });
                }
                Err(e) => {
                    if e.starts_with("No se capturó audio") {
                        log::warn!("Grabación descartada: {}", e);
                    } else {
                        log::error!("Error al detener grabación: {}", e);
                    }
                    *state = AppState::Idle;
                    let _ = ui_tx.send(UiMsg::SetTitle("🎙"));
                }
            }
        }
        AppState::Idle | AppState::Processing => {
            // Released sin grabación activa — ignorar
        }
    }
}

fn paste_text(text: &str) {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(c) => c,
        Err(e) => { log::error!("Error accediendo al clipboard: {}", e); return; }
    };

    let previous = clipboard.get_text().ok();

    if let Err(e) = clipboard.set_text(text) {
        log::error!("Error escribiendo al clipboard: {}", e);
        return;
    }
    drop(clipboard);

    std::thread::sleep(Duration::from_millis(50));

    match simulate_paste() {
        Ok(()) => log::info!("⌘V simulado correctamente"),
        Err(e) => log::error!("Error simulando ⌘V: {}", e),
    }

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        if let Some(prev) = previous {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(&prev);
                log::debug!("Clipboard original restaurado");
            }
        }
    });
}

/// Abre y cierra un stream de audio brevemente para que macOS muestre
/// el diálogo de permiso de micrófono al arrancar la app, no al grabar.
fn probe_microphone() {
    use cpal::traits::{DeviceTrait, HostTrait};
    use cpal::SampleFormat;

    let host = cpal::default_host();
    let Some(device) = host.default_input_device() else {
        log::warn!("Micrófono: ❌ no se detectó dispositivo de entrada");
        log::warn!("→ System Settings → Privacy & Security → Microphone → activa whisperwlopezob");
        return;
    };
    let Ok(supported) = device.default_input_config() else {
        return;
    };
    let config: cpal::StreamConfig = supported.clone().into();
    let result = match supported.sample_format() {
        SampleFormat::I16 => device
            .build_input_stream(&config, |_: &[i16], _| {}, |_| {}, None)
            .map(|_| ()),
        SampleFormat::F32 => device
            .build_input_stream(&config, |_: &[f32], _| {}, |_| {}, None)
            .map(|_| ()),
        SampleFormat::U8 => device
            .build_input_stream(&config, |_: &[u8], _| {}, |_| {}, None)
            .map(|_| ()),
        _ => return,
    };
    match result {
        Ok(_stream) => log::info!("Micrófono: ✅ permiso concedido"),
        Err(e) => {
            log::warn!("Micrófono: ❌ sin permiso ({})", e);
            log::warn!("→ System Settings → Privacy & Security → Microphone → activa whisperwlopezob");
        }
    }
    // _stream se descarta aquí, liberando el audio unit
}

/// Genera la etiqueta del ítem Azure MAI en el tray menu
fn azure_status_label(enabled: bool, region: &str) -> String {
    if enabled && !region.is_empty() {
        format!("☑ Azure MAI Transcribe ({})", region)
    } else if enabled {
        "☑ Azure MAI Transcribe (configurar región)".to_string()
    } else {
        "☐ Azure MAI Transcribe".to_string()
    }
}

fn simulate_paste() -> Result<(), String> {
    use enigo::{Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Error creando enigo: {}", e))?;

    enigo.key(Key::Meta, enigo::Direction::Press)
        .map_err(|e| format!("Error key down ⌘: {}", e))?;
    enigo.key(Key::Unicode('v'), enigo::Direction::Click)
        .map_err(|e| format!("Error key click v: {}", e))?;
    enigo.key(Key::Meta, enigo::Direction::Release)
        .map_err(|e| format!("Error key up ⌘: {}", e))?;

    Ok(())
}
