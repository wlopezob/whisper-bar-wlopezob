// src/main.rs

use whisper_bar_rust::{
    azure_transcriber, config, db, defaults, hotkey,
    logger, recorder, transcriber, translator, tts,
};
use whisper_bar_rust::settings_window::{SettingsValues, show_settings_modal};

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
    translate_enabled: Arc<Mutex<bool>>,
    translate_dest_lang: Arc<Mutex<String>>,
    // Azure MAI Transcribe
    azure_mai_enabled: Arc<Mutex<bool>>,
    azure_mai_key: Arc<Mutex<String>>,
    azure_mai_region: Arc<Mutex<String>>,
    azure_mai_model: Arc<Mutex<String>>,
    azure_mai_api_version: Arc<Mutex<String>>,
    azure_mai_definition: Arc<Mutex<String>>,
    // TTS
    tts_enabled: Arc<Mutex<bool>>,
    tts_voice: Arc<Mutex<String>>,
    gemini_api_key: Arc<Mutex<String>>,
    tts_scene: Arc<Mutex<String>>,
    tts_sample_context: Arc<Mutex<String>>,
    tts_formatter_enabled: Arc<Mutex<bool>>,
    tts_formatter_prompt: Arc<Mutex<String>>,
    tts_playback_rate: Arc<Mutex<String>>,
    tts_show_modal: Arc<Mutex<bool>>,
    ui_tx: mpsc::Sender<UiMsg>,
    ui_rx: mpsc::Receiver<UiMsg>,

    // UI (inicializado en resumed(), requiere NSApp activo)
    tray: Option<TrayIcon>,
    quit_id: Option<tray_icon::menu::MenuId>,
    log_id: Option<tray_icon::menu::MenuId>,
    settings_id: Option<tray_icon::menu::MenuId>,
    azure_item: Option<tray_icon::menu::MenuItem>,
    _hotkey_handler: Option<HotkeyHandler>,
    hotkey_id: Option<u32>,
    replay_hotkey_id: Option<u32>,
    modal_hotkey_id: Option<u32>,
}

impl WhisperApp {
    fn new() -> Self {
        let config = Arc::new(Config::new());

        let db = db::Db::open().expect("No se pudo abrir la base de datos");
        let translate_enabled = db.get("translate_enabled", "false") == "true";
        let translate_dest_lang = db.get("translate_dest_lang", defaults::TRANSLATE_DEST_LANG);
        let azure_mai_enabled = db.get("azure_mai_enabled", "false") == "true";
        let azure_mai_key = db.get("azure_mai_key", "");
        let azure_mai_region = db.get("azure_mai_region", "");
        let azure_mai_model = db.get("azure_mai_model", "");
        let azure_mai_api_version = db.get("azure_mai_api_version", defaults::AZURE_MAI_API_VERSION);
        let azure_mai_definition = db.get("azure_mai_definition", defaults::AZURE_MAI_DEFINITION);
        // TTS
        let tts_enabled = db.get("tts_enabled", "false") == "true";
        let tts_voice = db.get("tts_voice", defaults::TTS_DEFAULT_VOICE);
        let gemini_api_key = db.get("gemini_api_key", "");
        let tts_scene = db.get("tts_scene", defaults::TTS_DEFAULT_SCENE);
        let tts_sample_context = db.get("tts_sample_context", defaults::TTS_DEFAULT_SAMPLE_CONTEXT);
        let tts_formatter_enabled = db.get("tts_formatter_enabled", "false") == "true";
        let tts_formatter_prompt = db.get("tts_formatter_prompt", defaults::FORMATTER_DEFAULT_PROMPT);
        let tts_playback_rate = db.get("tts_playback_rate", defaults::TTS_DEFAULT_PLAYBACK_RATE);
        let tts_show_modal = db.get("tts_show_modal", "false") == "true";

        log::info!("=== whisperwlopezob v2.0.0 ===");
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
        log::info!(
            "TTS:         {} voz={} gemini_key={}",
            if tts_enabled { "✅ activo" } else { "☐ inactivo" },
            tts_voice,
            if gemini_api_key.is_empty() { "vacía" } else { "configurada" }
        );

        let (ui_tx, ui_rx) = mpsc::channel();

        WhisperApp {
            config,
            db,
            app_state: Arc::new(Mutex::new(AppState::Idle)),
            recorder: Arc::new(Mutex::new(Recorder::new())),
            translate_enabled: Arc::new(Mutex::new(translate_enabled)),
            translate_dest_lang: Arc::new(Mutex::new(translate_dest_lang)),
            azure_mai_enabled: Arc::new(Mutex::new(azure_mai_enabled)),
            azure_mai_key: Arc::new(Mutex::new(azure_mai_key)),
            azure_mai_region: Arc::new(Mutex::new(azure_mai_region)),
            azure_mai_model: Arc::new(Mutex::new(azure_mai_model)),
            azure_mai_api_version: Arc::new(Mutex::new(azure_mai_api_version)),
            azure_mai_definition: Arc::new(Mutex::new(azure_mai_definition)),
            tts_enabled: Arc::new(Mutex::new(tts_enabled)),
            tts_voice: Arc::new(Mutex::new(tts_voice)),
            gemini_api_key: Arc::new(Mutex::new(gemini_api_key)),
            tts_scene: Arc::new(Mutex::new(tts_scene)),
            tts_sample_context: Arc::new(Mutex::new(tts_sample_context)),
            tts_formatter_enabled: Arc::new(Mutex::new(tts_formatter_enabled)),
            tts_formatter_prompt: Arc::new(Mutex::new(tts_formatter_prompt)),
            tts_playback_rate: Arc::new(Mutex::new(tts_playback_rate)),
            tts_show_modal: Arc::new(Mutex::new(tts_show_modal)),
            ui_tx,
            ui_rx,
            tray: None,
            quit_id: None,
            log_id: None,
            settings_id: None,
            azure_item: None,
            _hotkey_handler: None,
            hotkey_id: None,
            replay_hotkey_id: None,
            modal_hotkey_id: None,
        }
    }

    /// Crea tray icon y registra hotkey — llamado una vez desde resumed()
    /// cuando NSApplication ya está activo
    fn setup_ui(&mut self) {
        log::debug!("setup_ui: creando tray icon y hotkey...");

        // ── Menú ──────────────────────────────────────────────────────────────
        let menu = Menu::new();

        let title_item = MenuItem::new("whisperwlopezob", false, None);
        let hint_item = MenuItem::new("Mantén ⌘⌥W para grabar / suelta para transcribir  ·  ⌘⌥R reproducir o parar última respuesta", false, None);

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

        let cli_item = MenuItem::new(cli_label, false, None);
        let model_item = MenuItem::new(model_label, false, None);
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
            .with_tooltip("whisperwlopezob — ⌘⌥W grabar · ⌘⌥R reproducir/parar")
            .build()
        {
            Ok(tray) => {
                log::info!("Tray icon creado");
                self.tray = Some(tray);
                self.quit_id = Some(quit_id);
                self.log_id = Some(ver_log_id);
                self.settings_id = Some(settings_id);
                self.azure_item = Some(azure_item);
            }
            Err(e) => log::error!("Error creando tray icon: {}", e),
        }

        // ── Hotkey ────────────────────────────────────────────────────────────
        match HotkeyHandler::new() {
            Ok(h) => {
                let id = h.hotkey_id();
                let replay_id = h.replay_hotkey_id();
                let modal_id  = h.modal_hotkey_id();
                log::info!("hotkey: ✅ ⌘⌥W registrado (id={})", id);
                log::info!("hotkey: ✅ ⌘⌥R registrado (id={})", replay_id);
                log::info!("hotkey: ✅ ⌘⌥V registrado (id={})", modal_id);
                self._hotkey_handler = Some(h);
                self.hotkey_id = Some(id);
                self.replay_hotkey_id = Some(replay_id);
                self.modal_hotkey_id = Some(modal_id);
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

        log::info!("whisperwlopezob activo. Mantén ⌘⌥W para grabar, suelta para transcribir. ⌘⌥R repite la última respuesta TTS.");
        log::info!("Log en tiempo real: tail -f {}", logger::log_path());

        // Solicitar permiso de micrófono al arrancar (antes del primer uso)
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            probe_microphone();
        });
    }

    fn apply_settings(&self, v: SettingsValues) {
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

        // TTS
        *self.tts_enabled.lock().unwrap() = v.tts_enabled;
        self.db.set("tts_enabled", if v.tts_enabled { "true" } else { "false" });
        *self.tts_voice.lock().unwrap() = v.tts_voice.clone();
        self.db.set("tts_voice", &v.tts_voice);
        *self.gemini_api_key.lock().unwrap() = v.gemini_api_key.clone();
        self.db.set("gemini_api_key", &v.gemini_api_key);
        *self.tts_scene.lock().unwrap() = v.tts_scene.clone();
        self.db.set("tts_scene", &v.tts_scene);
        *self.tts_sample_context.lock().unwrap() = v.tts_sample_context.clone();
        self.db.set("tts_sample_context", &v.tts_sample_context);
        *self.tts_formatter_enabled.lock().unwrap() = v.tts_formatter_enabled;
        self.db.set("tts_formatter_enabled", if v.tts_formatter_enabled { "true" } else { "false" });
        *self.tts_formatter_prompt.lock().unwrap() = v.tts_formatter_prompt.clone();
        self.db.set("tts_formatter_prompt", &v.tts_formatter_prompt);
        *self.tts_playback_rate.lock().unwrap() = v.tts_playback_rate.clone();
        self.db.set("tts_playback_rate", &v.tts_playback_rate);
        *self.tts_show_modal.lock().unwrap() = v.tts_show_modal;
        self.db.set("tts_show_modal", if v.tts_show_modal { "true" } else { "false" });
        log::info!(
            "TTS: {} voz={} gemini_key={}",
            if v.tts_enabled { "activo" } else { "inactivo" },
            v.tts_voice,
            if v.gemini_api_key.is_empty() { "vacía" } else { "configurada" },
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
                    translate_enabled: *self.translate_enabled.lock().unwrap(),
                    translate_dest: self.translate_dest_lang.lock().unwrap().clone(),
                    azure_mai_enabled: *self.azure_mai_enabled.lock().unwrap(),
                    azure_mai_key: self.azure_mai_key.lock().unwrap().clone(),
                    azure_mai_region: self.azure_mai_region.lock().unwrap().clone(),
                    azure_mai_model: self.azure_mai_model.lock().unwrap().clone(),
                    azure_mai_api_version: self.azure_mai_api_version.lock().unwrap().clone(),
                    azure_mai_definition: self.azure_mai_definition.lock().unwrap().clone(),
                    tts_enabled: *self.tts_enabled.lock().unwrap(),
                    tts_voice: self.tts_voice.lock().unwrap().clone(),
                    gemini_api_key: self.gemini_api_key.lock().unwrap().clone(),
                    tts_scene: self.tts_scene.lock().unwrap().clone(),
                    tts_sample_context: self.tts_sample_context.lock().unwrap().clone(),
                    tts_formatter_enabled: *self.tts_formatter_enabled.lock().unwrap(),
                    tts_formatter_prompt: self.tts_formatter_prompt.lock().unwrap().clone(),
                    tts_playback_rate: self.tts_playback_rate.lock().unwrap().clone(),
                    tts_show_modal: *self.tts_show_modal.lock().unwrap(),
                };
                if let Some(values) = show_settings_modal(&current) {
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
                        &self.azure_mai_enabled,
                        &self.azure_mai_key,
                        &self.azure_mai_region,
                        &self.azure_mai_model,
                        &self.azure_mai_api_version,
                        &self.azure_mai_definition,
                        &self.translate_enabled,
                        &self.translate_dest_lang,
                    ),
                }
            } else if Some(event.id) == self.replay_hotkey_id
                && event.state == HotKeyState::Pressed
            {
                let rate       = self.tts_playback_rate.lock().unwrap().parse::<f32>().unwrap_or(1.0);
                let show_modal = *self.tts_show_modal.lock().unwrap();
                replay_last_tts(rate, show_modal);
            } else if Some(event.id) == self.modal_hotkey_id
                && event.state == HotKeyState::Pressed
            {
                show_last_tts_modal();
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

/// ⌘⌥R: toggle — para si hay audio reproduciéndose, inicia replay si no hay nada
fn replay_last_tts(rate: f32, show_modal: bool) {
    if tts::is_afplay_running() {
        log::info!("TTS replay: parando afplay en curso");
        tts::kill_afplay();
        return;
    }
    if tts::is_say_running() {
        log::info!("TTS replay: parando say en curso (fallback)");
        tts::kill_say();
        return;
    }

    if show_modal {
        show_last_tts_modal();
    }

    // Parar también cualquier instancia de whisper-tts del hook antes de reproducir
    tts::kill_previous_instance();

    match tts::last_audio_path() {
        Some(path) if path.exists() => {
            log::info!("TTS replay: reproduciendo {:?}", path);
            std::thread::spawn(move || {
                let mut cmd = std::process::Command::new("afplay");
                if (rate - 1.0).abs() > 0.01 {
                    cmd.args(["-r", &format!("{:.2}", rate), "-q", "1"]);
                }
                cmd.arg(&path);
                if let Ok(mut child) = cmd.spawn() {
                    let _ = std::fs::write("/tmp/whisper-tts-afplay.pid", child.id().to_string());
                    let _ = child.wait();
                    let _ = std::fs::remove_file("/tmp/whisper-tts-afplay.pid");
                }
            });
        }
        _ => log::info!("TTS replay: no hay audio previo para reproducir"),
    }
}

/// ⌘⌥V: muestra modal con el último texto TTS
fn show_last_tts_modal() {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => { log::warn!("TTS modal: HOME no definido"); return; }
    };
    let text_path = std::path::PathBuf::from(&home)
        .join(defaults::APP_CONFIG_DIR)
        .join(defaults::TTS_AUDIO_DIR)
        .join(defaults::TTS_LAST_TEXT_FILE);

    let content = match std::fs::read_to_string(&text_path) {
        Ok(s) if !s.is_empty() => s,
        _ => {
            log::info!("TTS modal: no hay texto previo");
            return;
        }
    };

    // Formato del archivo: primera línea = título, resto = texto
    let mut lines = content.splitn(2, '\n');
    let title = lines.next().unwrap_or("TTS").trim().to_string();
    let text  = lines.next().unwrap_or("").trim().to_string();
    if text.is_empty() { return; }

    std::thread::spawn(move || {
        let tmp = std::path::PathBuf::from(&home)
            .join(defaults::APP_CONFIG_DIR)
            .join("last-tts-modal.txt");
        if std::fs::write(&tmp, &text).is_err() { return; }
        let script = format!(
            "set t to (do shell script \"cat '{}'\")\n\
             display dialog t with title \"{}\" buttons {{\"OK\"}} default button \"OK\"",
            tmp.display(), title
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e").arg(&script)
            .status();
    });
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

/// Released: detiene grabación y lanza transcripción (+ traducción opcional)
fn handle_hotkey_released(
    app_state: &Arc<Mutex<AppState>>,
    recorder: &Arc<Mutex<Recorder>>,
    config: &Arc<Config>,
    ui_tx: &mpsc::Sender<UiMsg>,
    azure_mai_enabled: &Arc<Mutex<bool>>,
    azure_mai_key: &Arc<Mutex<String>>,
    azure_mai_region: &Arc<Mutex<String>>,
    azure_mai_model_ref: &Arc<Mutex<String>>,
    azure_mai_api_version_ref: &Arc<Mutex<String>>,
    azure_mai_definition_ref: &Arc<Mutex<String>>,
    translate_enabled: &Arc<Mutex<bool>>,
    translate_dest_lang: &Arc<Mutex<String>>,
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
                    // Azure MAI
                    let use_azure = *azure_mai_enabled.lock().unwrap();
                    let azure_key = azure_mai_key.lock().unwrap().clone();
                    let azure_region = azure_mai_region.lock().unwrap().clone();
                    let _azure_model = azure_mai_model_ref.lock().unwrap().clone();
                    let azure_api_version = azure_mai_api_version_ref.lock().unwrap().clone();
                    let azure_definition = azure_mai_definition_ref.lock().unwrap().clone();
                    // Traducción
                    let do_translate = *translate_enabled.lock().unwrap();
                    let dest_lang = translate_dest_lang.lock().unwrap().clone();
                    std::thread::spawn(move || {
                        log::info!("🧭 Paso 1: iniciando transcripción");
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
                            transcriber::transcribe(&cli, &model, defaults::LANGUAGE, &audio_path)
                        };

                        match transcription_result {
                            Ok(text) => {
                                log::info!("✅ Transcripción: \"{}\"", text);

                                // Paso opcional: traducción
                                let final_text = if do_translate && !azure_key.is_empty() && !azure_region.is_empty() {
                                    log::info!("🌐 Traduciendo a '{}'...", dest_lang);
                                    match translator::translate(&text, &dest_lang, &azure_key, &azure_region) {
                                        Ok(result) => {
                                            if result.was_translated {
                                                log::info!(
                                                    "✅ Traducción: '{}' → '{}': \"{}\"",
                                                    result.detected_lang, dest_lang, result.text
                                                );
                                            }
                                            result.text
                                        }
                                        Err(e) => {
                                            log::error!("❌ Traducción falló, usando texto original: {}", e);
                                            text
                                        }
                                    }
                                } else {
                                    text
                                };

                                if ui_tx.send(UiMsg::PasteText(final_text)).is_err() {
                                    log::error!("No se pudo enviar PasteText al hilo UI");
                                }
                            }
                            Err(e) => {
                                log::error!("❌ Transcripción falló: {}", e);
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
