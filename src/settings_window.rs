// src/settings_window.rs
// Ventana modal de configuración — NSPanel nativo vía objc2 0.6

use objc2::define_class;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{msg_send, sel, AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSButton, NSControlStateValueOff, NSControlStateValueOn,
    NSModalResponseOK, NSPanel, NSPopUpButton, NSSegmentedControl, NSSegmentSwitchTracking,
    NSTextField, NSView, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub struct SettingsValues {
    pub language: String,
    pub grammar_enabled: bool,
    pub grammar_model: String,
    pub translate_enabled: bool,
    pub translate_dest: String,
    // Azure MAI Transcribe
    pub azure_mai_enabled: bool,
    pub azure_mai_key: String,
    pub azure_mai_region: String,
    pub azure_mai_model: String,
}

// ── Delegate mínimo: solo para capturar Aplicar / Cancelar ───────────────────
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

/// Muestra el panel modal de configuración.
/// Retorna `None` si el usuario pulsa Cancelar o cierra la ventana.
/// Debe llamarse desde el hilo principal.
pub fn show_settings_modal(
    current: &SettingsValues,
    available_models: &[String],
) -> Option<SettingsValues> {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    // ── Panel (560px de alto para incluir la sección Azure MAI) ──────────────
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        rect(0.0, 0.0, 420.0, 560.0),
        NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setTitle(&NSString::from_str("Configuración"));
    panel.center();

    let cv: Retained<NSView> = panel.contentView().unwrap();

    // ── TRANSCRIPCIÓN ──────────────────────────────────────────────────────────
    cv.addSubview(&section_header("TRANSCRIPCIÓN", 20.0, 510.0, mtm));
    cv.addSubview(&label("Idioma:", 20.0, 485.0, 60.0, mtm));

    let seg_lang = NSSegmentedControl::initWithFrame(
        NSSegmentedControl::alloc(mtm),
        rect(80.0, 480.0, 210.0, 26.0),
    );
    seg_lang.setSegmentCount(2);
    seg_lang.setLabel_forSegment(&NSString::from_str("Español"), 0);
    seg_lang.setLabel_forSegment(&NSString::from_str("English"), 1);
    seg_lang.setTrackingMode(NSSegmentSwitchTracking::SelectOne);
    seg_lang.setSelectedSegment(if current.language == "es" { 0 } else { 1 });
    cv.addSubview(&seg_lang);

    cv.addSubview(&label("Backend:", 20.0, 448.0, 65.0, mtm));
    let seg_backend = NSSegmentedControl::initWithFrame(
        NSSegmentedControl::alloc(mtm),
        rect(88.0, 443.0, 240.0, 26.0),
    );
    seg_backend.setSegmentCount(2);
    seg_backend.setLabel_forSegment(&NSString::from_str("Local (Whisper)"), 0);
    seg_backend.setLabel_forSegment(&NSString::from_str("Azure MAI"), 1);
    seg_backend.setTrackingMode(NSSegmentSwitchTracking::SelectOne);
    seg_backend.setSelectedSegment(if current.azure_mai_enabled { 1 } else { 0 });
    cv.addSubview(&seg_backend);

    // ── AZURE MAI TRANSCRIBE ───────────────────────────────────────────────────
    cv.addSubview(&section_header("AZURE MAI TRANSCRIBE", 20.0, 405.0, mtm));

    cv.addSubview(&label("API Key:", 20.0, 378.0, 60.0, mtm));
    let tf_key = input_field(&current.azure_mai_key, 85.0, 375.0, 315.0, mtm);
    cv.addSubview(&tf_key);

    cv.addSubview(&label("Región:", 20.0, 346.0, 58.0, mtm));
    let tf_region = input_field(&current.azure_mai_region, 82.0, 343.0, 180.0, mtm);
    cv.addSubview(&tf_region);
    cv.addSubview(&label("(ej: eastus)", 270.0, 346.0, 130.0, mtm));

    cv.addSubview(&label("Modelo:", 20.0, 314.0, 60.0, mtm));
    let tf_model_mai = input_field(&current.azure_mai_model, 85.0, 311.0, 180.0, mtm);
    cv.addSubview(&tf_model_mai);
    cv.addSubview(&label("(ej: whisper, vacío=default)", 272.0, 314.0, 140.0, mtm));

    // ── MEJORA GRAMATICAL ──────────────────────────────────────────────────────
    cv.addSubview(&section_header("MEJORA GRAMATICAL", 20.0, 273.0, mtm));

    let chk_grammar = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("Activar mejora gramatical"),
            None,
            None,
            mtm,
        )
    };
    chk_grammar.setFrame(rect(20.0, 246.0, 280.0, 22.0));
    chk_grammar.setState(if current.grammar_enabled {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    cv.addSubview(&chk_grammar);

    cv.addSubview(&label("Modelo:", 20.0, 216.0, 60.0, mtm));
    let popup_model = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(85.0, 213.0, 295.0, 26.0),
        false,
    );
    if available_models.is_empty() {
        popup_model.addItemWithTitle(&NSString::from_str("(sin modelos)"));
    } else {
        popup_model.addItemWithTitle(&NSString::from_str("— seleccionar modelo —"));
        for m in available_models {
            popup_model.addItemWithTitle(&NSString::from_str(m));
        }
        if !current.grammar_model.is_empty() {
            popup_model.selectItemWithTitle(&NSString::from_str(&current.grammar_model));
        }
    }
    cv.addSubview(&popup_model);

    // ── TRADUCCIÓN ─────────────────────────────────────────────────────────────
    cv.addSubview(&section_header("TRADUCCIÓN", 20.0, 175.0, mtm));

    let chk_translate = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("Activar traducción"),
            None,
            None,
            mtm,
        )
    };
    chk_translate.setFrame(rect(20.0, 148.0, 240.0, 22.0));
    chk_translate.setState(if current.translate_enabled {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    cv.addSubview(&chk_translate);

    cv.addSubview(&label("Idioma destino:", 20.0, 118.0, 110.0, mtm));
    let popup_dest = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(135.0, 115.0, 140.0, 26.0),
        false,
    );
    popup_dest.addItemWithTitle(&NSString::from_str("Español"));
    popup_dest.addItemWithTitle(&NSString::from_str("English"));
    popup_dest.selectItemWithTitle(&NSString::from_str(
        if current.translate_dest == "es" {
            "Español"
        } else {
            "English"
        },
    ));
    cv.addSubview(&popup_dest);

    // ── Botones Cancelar / Aplicar ─────────────────────────────────────────────
    let delegate = ModalDelegate::new();

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

    // ── Ejecutar modal (bloquea hasta Aplicar / Cancelar / cierre) ────────────
    let app = NSApplication::sharedApplication(mtm);
    let response = app.runModalForWindow(&panel);

    // Ocultar el panel explícitamente — runModalForWindow no lo hace solo
    panel.orderOut(None);

    if response != NSModalResponseOK {
        return None;
    }

    // ── Leer estado de los controles ───────────────────────────────────────────
    let language = if seg_lang.selectedSegment() == 0 {
        "es"
    } else {
        "en"
    }
    .to_string();

    let azure_mai_enabled = seg_backend.selectedSegment() == 1;

    let azure_mai_key = tf_key.stringValue().to_string().trim().to_string();
    let azure_mai_region = tf_region.stringValue().to_string().trim().to_string();
    let azure_mai_model = tf_model_mai.stringValue().to_string().trim().to_string();

    let grammar_enabled = chk_grammar.state() == NSControlStateValueOn;

    let grammar_model = if available_models.is_empty() {
        String::new()
    } else {
        let title = popup_model
            .titleOfSelectedItem()
            .map(|s| s.to_string())
            .unwrap_or_default();
        if title == "— seleccionar modelo —" {
            String::new()
        } else {
            title
        }
    };

    let translate_enabled = chk_translate.state() == NSControlStateValueOn;

    let translate_dest = popup_dest
        .titleOfSelectedItem()
        .map(|s| {
            if s.to_string() == "Español" {
                "es".to_string()
            } else {
                "en".to_string()
            }
        })
        .unwrap_or_else(|| "es".to_string());

    Some(SettingsValues {
        language,
        grammar_enabled,
        grammar_model,
        translate_enabled,
        translate_dest,
        azure_mai_enabled,
        azure_mai_key,
        azure_mai_region,
        azure_mai_model,
    })
}
