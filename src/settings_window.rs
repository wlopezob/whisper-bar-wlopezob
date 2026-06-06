// src/settings_window.rs
// Ventana modal de configuración — NSPanel nativo vía objc2 0.6

use objc2::define_class;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{msg_send, sel, AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSButton, NSControlStateValueOff, NSControlStateValueOn,
    NSModalResponseOK, NSPanel, NSPopUpButton, NSScrollView, NSSegmentedControl,
    NSSegmentSwitchTracking, NSTextField, NSTextView, NSView, NSWindowButton, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub struct SettingsValues {
    pub translate_enabled: bool,
    pub translate_dest: String,
    pub translate_provider: String,
    pub translate_ollama_prompt: String,
    // Azure MAI Transcribe
    pub azure_mai_enabled: bool,
    pub azure_mai_key: String,
    pub azure_mai_region: String,
    pub azure_mai_model: String,
    pub azure_mai_api_version: String,
    pub azure_mai_definition: String,
    // TTS
    pub tts_enabled: bool,
    pub tts_voice: String,
    pub gemini_api_key: String,
    pub tts_scene: String,
    pub tts_sample_context: String,
    pub tts_formatter_enabled: bool,
    pub tts_formatter_prompt: String,
    pub tts_playback_rate: String,
    pub tts_show_modal: bool,
}

// ── Thread-local: referencias a los campos de Azure para el callback ─────────

#[derive(Copy, Clone)]
struct AzureFieldPtrs {
    tf_key: *const NSTextField,
    tf_region: *const NSTextField,
    tf_api_version: *const NSTextField,
    txt_definition: *const NSTextView,
    scroll_definition: *const NSScrollView,
    lbl_key: *const NSTextField,
    lbl_region: *const NSTextField,
    lbl_region_hint: *const NSTextField,
    lbl_api_version: *const NSTextField,
    lbl_definition: *const NSTextField,
}

// SAFETY: solo se accede desde el hilo principal (modal UI)
unsafe impl Send for AzureFieldPtrs {}
unsafe impl Sync for AzureFieldPtrs {}

thread_local! {
    static AZURE_FIELDS: std::cell::RefCell<Option<AzureFieldPtrs>> =
        std::cell::RefCell::new(None);
}

fn set_azure_fields_hidden(hidden: bool) {
    AZURE_FIELDS.with(|cell| {
        if let Some(ptrs) = *cell.borrow() {
            unsafe {
                let fields: [*const NSTextField; 6] = [
                    ptrs.tf_key, ptrs.tf_region, ptrs.tf_api_version,
                    ptrs.lbl_key, ptrs.lbl_region, ptrs.lbl_region_hint,
                ];
                let label_fields: [*const NSTextField; 2] = [
                    ptrs.lbl_api_version, ptrs.lbl_definition,
                ];
                for f in fields {
                    let _: () = msg_send![&*f, setHidden: hidden];
                }
                for f in label_fields {
                    let _: () = msg_send![&*f, setHidden: hidden];
                }
                let _: () = msg_send![&*ptrs.scroll_definition, setHidden: hidden];
            }
        }
    });
}

// ── Thread-local: referencias al prompt Ollama para show/hide ────────────────

#[derive(Copy, Clone)]
struct OllamaPromptPtrs {
    lbl_prompt: *const NSTextField,
    scroll_prompt: *const NSScrollView,
}

unsafe impl Send for OllamaPromptPtrs {}
unsafe impl Sync for OllamaPromptPtrs {}

thread_local! {
    static OLLAMA_PROMPT_FIELDS: std::cell::RefCell<Option<OllamaPromptPtrs>> =
        std::cell::RefCell::new(None);
}

fn set_ollama_prompt_hidden(hidden: bool) {
    OLLAMA_PROMPT_FIELDS.with(|cell| {
        if let Some(ptrs) = *cell.borrow() {
            unsafe {
                let _: () = msg_send![&*ptrs.lbl_prompt, setHidden: hidden];
                let _: () = msg_send![&*ptrs.scroll_prompt, setHidden: hidden];
            }
        }
    });
}

// ── Delegate: Aplicar / Cancelar + toggle de sección Azure ───────────────────
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "WhisperBarModalDelegate"]
    struct ModalDelegate;

    impl ModalDelegate {
        #[unsafe(method(applyClicked:))]
        fn apply_clicked(&self, _sender: &AnyObject) {
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let app = NSApplication::sharedApplication(mtm);
            app.stopModalWithCode(NSModalResponseOK);
        }

        #[unsafe(method(cancelClicked:))]
        fn cancel_clicked(&self, _sender: &AnyObject) {
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let app = NSApplication::sharedApplication(mtm);
            app.stopModal();
        }

        /// Llamado cuando el usuario cambia el selector Whisper local / Azure MAI
        #[unsafe(method(backendChanged:))]
        fn backend_changed(&self, sender: &NSSegmentedControl) {
            let show_azure = sender.selectedSegment() == 1;
            set_azure_fields_hidden(!show_azure);
        }

        /// Llamado cuando el usuario cambia el proveedor de traducción
        #[unsafe(method(providerChanged:))]
        fn provider_changed(&self, sender: &NSPopUpButton) {
            let is_ollama = sender
                .titleOfSelectedItem()
                .map(|s| s.to_string().contains("Ollama"))
                .unwrap_or(false);
            set_ollama_prompt_hidden(!is_ollama);
        }
    }
);

impl ModalDelegate {
    fn new() -> Retained<Self> {
        let this = ModalDelegate::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

// ── Helpers de layout ────────────────────────────────────────────────────────

fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}

fn label(text: &str, x: f64, y: f64, w: f64, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let s = NSString::from_str(text);
    let lbl = NSTextField::labelWithString(&s, mtm);
    lbl.setFrame(rect(x, y, w, 20.0));
    lbl
}

fn section_header(text: &str, x: f64, y: f64, mtm: MainThreadMarker) -> Retained<NSTextField> {
    label(text, x, y, 380.0, mtm)
}

/// Crea un NSTextField editable (campo de texto de entrada)
fn input_field(
    initial: &str,
    x: f64,
    y: f64,
    w: f64,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    let tf = NSTextField::initWithFrame(NSTextField::alloc(mtm), rect(x, y, w, 24.0));
    tf.setStringValue(&NSString::from_str(initial));
    tf
}

// ── API pública ───────────────────────────────────────────────────────────────

pub fn show_settings_modal(current: &SettingsValues) -> Option<SettingsValues> {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);

    // Delegate creado primero para usarlo como target de seg_backend
    let delegate = ModalDelegate::new();

    // ── Panel ─────────────────────────────────────────────────────────────────
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        rect(0.0, 0.0, 420.0, 720.0),
        NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setTitle(&NSString::from_str("Configuración"));
    panel.setFloatingPanel(true);
    panel.setBecomesKeyOnlyIfNeeded(false);
    panel.setHidesOnDeactivate(false);
    panel.center();

    let cv: Retained<NSView> = panel.contentView().unwrap();

    // ── Scroll container: panel fijo 720px, contenido scrolleable de 1260px ────
    // Los botones Cancelar/Aplicar van directamente en cv (fuera del scroll).
    let scroll_settings = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(0.0, 55.0, 420.0, 665.0), // y=55 (sobre botones) hasta y=720 (tope panel)
    );
    scroll_settings.setHasVerticalScroller(true);
    scroll_settings.setHasHorizontalScroller(false);

    let content = NSView::initWithFrame(
        NSView::alloc(mtm),
        rect(0.0, 0.0, 420.0, 1210.0),
    );

    // ── AZURE MAI TRANSCRIBE ──────────────────────────────────────────────────
    content.addSubview(&section_header("AZURE MAI TRANSCRIBE", 20.0, 1158.0, mtm));
    content.addSubview(&label("Backend:", 20.0, 1133.0, 65.0, mtm));

    let seg_backend = NSSegmentedControl::initWithFrame(
        NSSegmentedControl::alloc(mtm),
        rect(88.0, 1128.0, 250.0, 26.0),
    );
    seg_backend.setSegmentCount(2);
    seg_backend.setLabel_forSegment(&NSString::from_str("Whisper local"), 0);
    seg_backend.setLabel_forSegment(&NSString::from_str("Azure MAI"), 1);
    seg_backend.setTrackingMode(NSSegmentSwitchTracking::SelectOne);
    seg_backend.setSelectedSegment(if current.azure_mai_enabled { 1 } else { 0 });
    // Registrar callback para mostrar/ocultar campos Azure
    let delegate_obj: &AnyObject = &*delegate;
    unsafe {
        seg_backend.setTarget(Some(delegate_obj));
        seg_backend.setAction(Some(sel!(backendChanged:)));
    }
    content.addSubview(&seg_backend);

    // Campos Azure MAI (ocultos cuando Backend = Whisper local)
    let lbl_key = label("API Key:", 20.0, 1100.0, 60.0, mtm);
    content.addSubview(&lbl_key);
    let tf_key = input_field(&current.azure_mai_key, 85.0, 1097.0, 315.0, mtm);
    content.addSubview(&tf_key);

    let lbl_region = label("Región:", 20.0, 1068.0, 58.0, mtm);
    content.addSubview(&lbl_region);
    let tf_region = input_field(&current.azure_mai_region, 82.0, 1065.0, 180.0, mtm);
    content.addSubview(&tf_region);
    let lbl_region_hint = label("(ej: eastus)", 270.0, 1068.0, 130.0, mtm);
    content.addSubview(&lbl_region_hint);

    let lbl_api_version = label("API Version:", 20.0, 1036.0, 80.0, mtm);
    content.addSubview(&lbl_api_version);
    let tf_api_version = input_field(&current.azure_mai_api_version, 105.0, 1033.0, 140.0, mtm);
    content.addSubview(&tf_api_version);

    let lbl_definition = label("Definition JSON:", 20.0, 1004.0, 110.0, mtm);
    content.addSubview(&lbl_definition);
    let scroll_definition = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 947.0, 380.0, 54.0),
    );
    scroll_definition.setHasVerticalScroller(true);
    scroll_definition.setHasHorizontalScroller(false);
    let txt_definition = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 54.0),
    );
    txt_definition.setEditable(true);
    txt_definition.setSelectable(true);
    txt_definition.setRichText(false);
    txt_definition.setString(&NSString::from_str(&current.azure_mai_definition));
    scroll_definition.setDocumentView(Some(txt_definition.as_ref()));
    content.addSubview(&scroll_definition);

    // Registrar punteros en thread_local para el callback de toggle azure
    AZURE_FIELDS.with(|cell| {
        *cell.borrow_mut() = Some(AzureFieldPtrs {
            tf_key: &*tf_key as *const NSTextField,
            tf_region: &*tf_region as *const NSTextField,
            tf_api_version: &*tf_api_version as *const NSTextField,
            txt_definition: &*txt_definition as *const NSTextView,
            scroll_definition: &*scroll_definition as *const NSScrollView,
            lbl_key: &*lbl_key as *const NSTextField,
            lbl_region: &*lbl_region as *const NSTextField,
            lbl_region_hint: &*lbl_region_hint as *const NSTextField,
            lbl_api_version: &*lbl_api_version as *const NSTextField,
            lbl_definition: &*lbl_definition as *const NSTextField,
        });
    });

    // Aplicar estado inicial
    if !current.azure_mai_enabled {
        set_azure_fields_hidden(true);
    }

    // ── TRADUCCIÓN ────────────────────────────────────────────────────────────
    content.addSubview(&section_header("TRADUCCIÓN", 20.0, 918.0, mtm));

    let chk_translate = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("Activar traducción"),
            None,
            None,
            mtm,
        )
    };
    chk_translate.setFrame(rect(20.0, 891.0, 240.0, 22.0));
    chk_translate.setState(if current.translate_enabled {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    content.addSubview(&chk_translate);

    content.addSubview(&label("Proveedor:", 20.0, 862.0, 75.0, mtm));
    let popup_provider = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(100.0, 859.0, 210.0, 26.0),
        false,
    );
    popup_provider.addItemWithTitle(&NSString::from_str("Microsoft Translator"));
    popup_provider.addItemWithTitle(&NSString::from_str("Ollama (gemma4:e4b)"));
    popup_provider.selectItemWithTitle(&NSString::from_str(
        if current.translate_provider == "ollama" { "Ollama (gemma4:e4b)" } else { "Microsoft Translator" },
    ));
    unsafe {
        popup_provider.setTarget(Some(delegate_obj));
        popup_provider.setAction(Some(sel!(providerChanged:)));
    }
    content.addSubview(&popup_provider);

    content.addSubview(&label("Idioma destino:", 20.0, 828.0, 110.0, mtm));
    let popup_dest = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(135.0, 825.0, 140.0, 26.0),
        false,
    );
    popup_dest.addItemWithTitle(&NSString::from_str("Español"));
    popup_dest.addItemWithTitle(&NSString::from_str("English"));
    popup_dest.selectItemWithTitle(&NSString::from_str(
        if current.translate_dest == "es" { "Español" } else { "English" },
    ));
    content.addSubview(&popup_dest);

    let scroll_translate_prompt = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 736.0, 380.0, 57.0),
    );
    scroll_translate_prompt.setHasVerticalScroller(true);
    scroll_translate_prompt.setHasHorizontalScroller(false);
    let txt_translate_prompt = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 57.0),
    );
    txt_translate_prompt.setEditable(true);
    txt_translate_prompt.setSelectable(true);
    txt_translate_prompt.setRichText(false);
    let ollama_prompt_initial = if current.translate_ollama_prompt.is_empty() {
        crate::defaults::TRANSLATE_OLLAMA_DEFAULT_PROMPT
    } else {
        &current.translate_ollama_prompt
    };
    txt_translate_prompt.setString(&NSString::from_str(ollama_prompt_initial));
    scroll_translate_prompt.setDocumentView(Some(txt_translate_prompt.as_ref()));
    content.addSubview(&scroll_translate_prompt);

    // Registrar punteros para show/hide del prompt Ollama
    let lbl_translate_prompt = label("Prompt Ollama:", 20.0, 796.0, 105.0, mtm);
    content.addSubview(&lbl_translate_prompt);
    OLLAMA_PROMPT_FIELDS.with(|cell| {
        *cell.borrow_mut() = Some(OllamaPromptPtrs {
            lbl_prompt: &*lbl_translate_prompt as *const NSTextField,
            scroll_prompt: &*scroll_translate_prompt as *const NSScrollView,
        });
    });
    // Visibilidad inicial: oculto si no es Ollama
    set_ollama_prompt_hidden(current.translate_provider != "ollama");

    // ── LECTURA DE RESPUESTAS (TTS) — siempre visible ─────────────────────────
    content.addSubview(&section_header("LECTURA DE RESPUESTAS", 20.0, 702.0, mtm));

    let chk_tts = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("Leer respuestas de Claude"),
            None,
            None,
            mtm,
        )
    };
    chk_tts.setFrame(rect(20.0, 672.0, 280.0, 22.0));
    chk_tts.setState(if current.tts_enabled {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    content.addSubview(&chk_tts);

    content.addSubview(&label("Clave Gemini:", 20.0, 642.0, 90.0, mtm));
    let gemini_key_initial = current.gemini_api_key.as_str();
    let tf_gemini_key = input_field(gemini_key_initial, 115.0, 639.0, 285.0, mtm);
    content.addSubview(&tf_gemini_key);

    // ── Formatter (paso 1 del pipeline: preprocesa el texto antes de TTS) ─────
    let chk_formatter = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("Formatear respuesta para voz"),
            None,
            None,
            mtm,
        )
    };
    chk_formatter.setFrame(rect(20.0, 606.0, 280.0, 22.0));
    chk_formatter.setState(if current.tts_formatter_enabled {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    content.addSubview(&chk_formatter);

    content.addSubview(&label("Prompt TTS:", 20.0, 580.0, 80.0, mtm));
    let scroll_formatter_prompt = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 522.0, 380.0, 55.0),
    );
    scroll_formatter_prompt.setHasVerticalScroller(true);
    scroll_formatter_prompt.setHasHorizontalScroller(false);
    let txt_formatter_prompt = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 55.0),
    );
    txt_formatter_prompt.setEditable(true);
    txt_formatter_prompt.setSelectable(true);
    txt_formatter_prompt.setRichText(false);
    let formatter_prompt_initial = if current.tts_formatter_prompt.is_empty() {
        crate::defaults::FORMATTER_DEFAULT_PROMPT
    } else {
        &current.tts_formatter_prompt
    };
    txt_formatter_prompt.setString(&NSString::from_str(formatter_prompt_initial));
    scroll_formatter_prompt.setDocumentView(Some(txt_formatter_prompt.as_ref()));
    content.addSubview(&scroll_formatter_prompt);

    // ── Voz / Velocidad ───────────────────────────────────────────────────────
    content.addSubview(&label("Voz:", 20.0, 492.0, 35.0, mtm));
    let tts_voice_initial = if current.tts_voice.is_empty() {
        crate::defaults::TTS_DEFAULT_VOICE
    } else {
        &current.tts_voice
    };
    let tf_tts_voice = input_field(tts_voice_initial, 60.0, 489.0, 200.0, mtm);
    content.addSubview(&tf_tts_voice);

    content.addSubview(&label("Vel:", 268.0, 492.0, 30.0, mtm));
    let rate_initial = if current.tts_playback_rate.is_empty() {
        crate::defaults::TTS_DEFAULT_PLAYBACK_RATE
    } else {
        &current.tts_playback_rate
    };
    let tf_tts_rate = input_field(rate_initial, 300.0, 489.0, 100.0, mtm);
    content.addSubview(&tf_tts_rate);

    content.addSubview(&label("Escena:", 20.0, 462.0, 55.0, mtm));
    let scroll_tts_scene = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 392.0, 380.0, 67.0),
    );
    scroll_tts_scene.setHasVerticalScroller(true);
    scroll_tts_scene.setHasHorizontalScroller(false);
    let txt_tts_scene = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 67.0),
    );
    txt_tts_scene.setEditable(true);
    txt_tts_scene.setSelectable(true);
    txt_tts_scene.setRichText(false);
    let scene_initial = if current.tts_scene.is_empty() {
        crate::defaults::TTS_DEFAULT_SCENE
    } else {
        &current.tts_scene
    };
    txt_tts_scene.setString(&NSString::from_str(scene_initial));
    scroll_tts_scene.setDocumentView(Some(txt_tts_scene.as_ref()));
    content.addSubview(&scroll_tts_scene);

    content.addSubview(&label("Contexto:", 20.0, 367.0, 65.0, mtm));
    let scroll_tts_context = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 297.0, 380.0, 67.0),
    );
    scroll_tts_context.setHasVerticalScroller(true);
    scroll_tts_context.setHasHorizontalScroller(false);
    let txt_tts_context = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 67.0),
    );
    txt_tts_context.setEditable(true);
    txt_tts_context.setSelectable(true);
    txt_tts_context.setRichText(false);
    let context_initial = if current.tts_sample_context.is_empty() {
        crate::defaults::TTS_DEFAULT_SAMPLE_CONTEXT
    } else {
        &current.tts_sample_context
    };
    txt_tts_context.setString(&NSString::from_str(context_initial));
    scroll_tts_context.setDocumentView(Some(txt_tts_context.as_ref()));
    content.addSubview(&scroll_tts_context);

    // ── Mostrar texto modal (⌘⌥V) ────────────────────────────────────────────
    let chk_show_modal = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("Mostrar texto al leer (⌘⌥V)"),
            None,
            None,
            mtm,
        )
    };
    chk_show_modal.setFrame(rect(20.0, 265.0, 280.0, 22.0));
    chk_show_modal.setState(if current.tts_show_modal {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    content.addSubview(&chk_show_modal);

    // ── Montar content en scroll y scroll en panel ────────────────────────────
    scroll_settings.setDocumentView(Some(&content));
    cv.addSubview(&scroll_settings);

    // Scroll inicial: mostrar la parte de arriba (TRANSCRIPCIÓN, y=1230 en content)
    unsafe {
        let clip: *mut AnyObject = msg_send![&*scroll_settings, contentView];
        // top_y = content_height(1210) - visible_height(665) = 545
        let _: () = msg_send![clip, scrollToPoint: NSPoint::new(0.0, 545.0_f64)];
        let _: () = msg_send![&*scroll_settings, reflectScrolledClipView: clip];
    }

    // ── Botones Cancelar / Aplicar ────────────────────────────────────────────
    // Fuerza que el botón nativo "X" siga el mismo flujo que Cancelar.
    if let Some(close_btn) = panel.standardWindowButton(NSWindowButton::CloseButton) {
        let delegate_obj: &AnyObject = &*delegate;
        unsafe {
            close_btn.setTarget(Some(delegate_obj));
            close_btn.setAction(Some(sel!(cancelClicked:)));
        }
    }

    let btn_cancel = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("Cancelar"),
            Some(&*delegate),
            Some(sel!(cancelClicked:)),
            mtm,
        )
    };
    btn_cancel.setFrame(rect(210.0, 15.0, 90.0, 30.0));
    cv.addSubview(&btn_cancel);

    let btn_apply = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str("Aplicar"),
            Some(&*delegate),
            Some(sel!(applyClicked:)),
            mtm,
        )
    };
    btn_apply.setFrame(rect(310.0, 15.0, 90.0, 30.0));
    cv.addSubview(&btn_apply);

    // ── Ejecutar modal ────────────────────────────────────────────────────────
    app.activate();
    panel.center();
    panel.makeKeyAndOrderFront(None);
    panel.orderFrontRegardless();
    let f = panel.frame();
    log::debug!(
        "UI: settings panel abierto (x={}, y={}, w={}, h={})",
        f.origin.x, f.origin.y, f.size.width, f.size.height
    );
    let response = app.runModalForWindow(&panel);
    log::debug!("UI: settings panel cerrado (response={:?})", response);

    panel.orderOut(None);

    // Limpiar referencias de thread_locals
    AZURE_FIELDS.with(|cell| { *cell.borrow_mut() = None; });
    OLLAMA_PROMPT_FIELDS.with(|cell| { *cell.borrow_mut() = None; });

    if response != NSModalResponseOK {
        return None;
    }

    // ── Leer estado de los controles ──────────────────────────────────────────
    let azure_mai_enabled = seg_backend.selectedSegment() == 1;
    let azure_mai_key = tf_key.stringValue().to_string().trim().to_string();
    let azure_mai_region = tf_region.stringValue().to_string().trim().to_string();
    let azure_mai_model = String::new(); // campo eliminado de UI; modelo va dentro de definition JSON
    let azure_mai_api_version = tf_api_version.stringValue().to_string().trim().to_string();
    let azure_mai_definition = txt_definition.string().to_string().trim().to_string();

    let translate_enabled = chk_translate.state() == NSControlStateValueOn;

    let translate_provider = popup_provider
        .titleOfSelectedItem()
        .map(|s| {
            if s.to_string().contains("Ollama") { "ollama".to_string() } else { "azure".to_string() }
        })
        .unwrap_or_else(|| "azure".to_string());

    let translate_dest = popup_dest
        .titleOfSelectedItem()
        .map(|s| {
            if s.to_string() == "Español" { "es".to_string() } else { "en".to_string() }
        })
        .unwrap_or_else(|| "es".to_string());

    let translate_ollama_prompt = {
        let v = txt_translate_prompt.string().to_string().trim().to_string();
        if v.is_empty() { crate::defaults::TRANSLATE_OLLAMA_DEFAULT_PROMPT.to_string() } else { v }
    };

    // Leer valores TTS
    let tts_enabled = chk_tts.state() == NSControlStateValueOn;
    let tts_voice = {
        let v = tf_tts_voice.stringValue().to_string().trim().to_string();
        if v.is_empty() { crate::defaults::TTS_DEFAULT_VOICE.to_string() } else { v }
    };
    let gemini_api_key = tf_gemini_key.stringValue().to_string().trim().to_string();
    let tts_scene = {
        let v = txt_tts_scene.string().to_string().trim().to_string();
        if v.is_empty() { crate::defaults::TTS_DEFAULT_SCENE.to_string() } else { v }
    };
    let tts_sample_context = {
        let v = txt_tts_context.string().to_string().trim().to_string();
        if v.is_empty() { crate::defaults::TTS_DEFAULT_SAMPLE_CONTEXT.to_string() } else { v }
    };
    let tts_formatter_enabled = chk_formatter.state() == NSControlStateValueOn;
    let tts_formatter_prompt = {
        let v = txt_formatter_prompt.string().to_string().trim().to_string();
        if v.is_empty() { crate::defaults::FORMATTER_DEFAULT_PROMPT.to_string() } else { v }
    };
    let tts_playback_rate = {
        let v = tf_tts_rate.stringValue().to_string().trim().to_string();
        if v.parse::<f32>().is_ok() { v } else { crate::defaults::TTS_DEFAULT_PLAYBACK_RATE.to_string() }
    };
    let tts_show_modal = chk_show_modal.state() == NSControlStateValueOn;

    Some(SettingsValues {
        translate_enabled,
        translate_dest,
        translate_provider,
        translate_ollama_prompt,
        azure_mai_enabled,
        azure_mai_key,
        azure_mai_region,
        azure_mai_model,
        azure_mai_api_version,
        azure_mai_definition,
        tts_enabled,
        tts_voice,
        gemini_api_key,
        tts_scene,
        tts_sample_context,
        tts_formatter_enabled,
        tts_formatter_prompt,
        tts_playback_rate,
        tts_show_modal,
    })
}
