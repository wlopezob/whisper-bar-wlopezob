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
    pub language: String,
    pub grammar_enabled: bool,
    pub grammar_model: String,
    pub grammar_prompt_es: String,
    pub grammar_prompt_en: String,
    pub translate_enabled: bool,
    pub translate_dest: String,
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

// ── API pública ───────────────────────────────────────────────────────────────

/// Muestra el panel modal de configuración.
/// Retorna `None` si el usuario pulsa Cancelar o cierra la ventana.
/// Debe llamarse desde el hilo principal.
pub fn show_settings_modal(
    current: &SettingsValues,
    available_models: &[String],
) -> Option<SettingsValues> {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);

    // ── Panel ──────────────────────────────────────────────────────────────
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        rect(0.0, 0.0, 420.0, 520.0),
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

    // ── TRANSCRIPCIÓN ──────────────────────────────────────────────────────
    cv.addSubview(&section_header("TRANSCRIPCIÓN", 20.0, 470.0, mtm));
    cv.addSubview(&label("Idioma:", 20.0, 445.0, 60.0, mtm));

    let seg_lang = NSSegmentedControl::initWithFrame(
        NSSegmentedControl::alloc(mtm),
        rect(80.0, 440.0, 210.0, 26.0),
    );
    seg_lang.setSegmentCount(2);
    seg_lang.setLabel_forSegment(&NSString::from_str("Español"), 0);
    seg_lang.setLabel_forSegment(&NSString::from_str("English"), 1);
    seg_lang.setTrackingMode(NSSegmentSwitchTracking::SelectOne);
    seg_lang.setSelectedSegment(if current.language == "es" { 0 } else { 1 });
    cv.addSubview(&seg_lang);

    // ── MEJORA GRAMATICAL ──────────────────────────────────────────────────
    cv.addSubview(&section_header("MEJORA GRAMATICAL", 20.0, 400.0, mtm));

    let chk_grammar = unsafe { NSButton::checkboxWithTitle_target_action(
        &NSString::from_str("Activar mejora gramatical"),
        None,
        None,
        mtm,
    ) };
    chk_grammar.setFrame(rect(20.0, 373.0, 280.0, 22.0));
    chk_grammar.setState(if current.grammar_enabled { NSControlStateValueOn } else { NSControlStateValueOff });
    cv.addSubview(&chk_grammar);

    cv.addSubview(&label("Modelo:", 20.0, 343.0, 60.0, mtm));
    let popup_model = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(85.0, 340.0, 295.0, 26.0),
        false,
    );
    if available_models.is_empty() {
        popup_model.addItemWithTitle(&NSString::from_str("(sin modelos)"));
    } else {
        // Primer ítem vacío: indica que aún no se ha elegido modelo
        popup_model.addItemWithTitle(&NSString::from_str("— seleccionar modelo —"));
        for m in available_models {
            popup_model.addItemWithTitle(&NSString::from_str(m));
        }
        if !current.grammar_model.is_empty() {
            popup_model.selectItemWithTitle(&NSString::from_str(&current.grammar_model));
        }
        // Si grammar_model está vacío el placeholder queda seleccionado (primer ítem)
    }
    cv.addSubview(&popup_model);

    cv.addSubview(&label("Prompt ES:", 20.0, 313.0, 120.0, mtm));
    let scroll_prompt_es = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 248.0, 380.0, 62.0),
    );
    scroll_prompt_es.setHasVerticalScroller(true);
    scroll_prompt_es.setHasHorizontalScroller(false);
    let txt_prompt_es = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 62.0),
    );
    txt_prompt_es.setEditable(true);
    txt_prompt_es.setSelectable(true);
    txt_prompt_es.setRichText(false);
    txt_prompt_es.setString(&NSString::from_str(&current.grammar_prompt_es));
    scroll_prompt_es.setDocumentView(Some(txt_prompt_es.as_ref()));
    cv.addSubview(&scroll_prompt_es);

    cv.addSubview(&label("Prompt EN:", 20.0, 221.0, 120.0, mtm));
    let scroll_prompt_en = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        rect(20.0, 156.0, 380.0, 62.0),
    );
    scroll_prompt_en.setHasVerticalScroller(true);
    scroll_prompt_en.setHasHorizontalScroller(false);
    let txt_prompt_en = NSTextView::initWithFrame(
        NSTextView::alloc(mtm),
        rect(0.0, 0.0, 380.0, 62.0),
    );
    txt_prompt_en.setEditable(true);
    txt_prompt_en.setSelectable(true);
    txt_prompt_en.setRichText(false);
    txt_prompt_en.setString(&NSString::from_str(&current.grammar_prompt_en));
    scroll_prompt_en.setDocumentView(Some(txt_prompt_en.as_ref()));
    cv.addSubview(&scroll_prompt_en);

    // ── TRADUCCIÓN ────────────────────────────────────────────────────────
    cv.addSubview(&section_header("TRADUCCIÓN", 20.0, 118.0, mtm));

    let chk_translate = unsafe { NSButton::checkboxWithTitle_target_action(
        &NSString::from_str("Activar traducción"),
        None,
        None,
        mtm,
    ) };
    chk_translate.setFrame(rect(20.0, 91.0, 240.0, 22.0));
    chk_translate.setState(if current.translate_enabled { NSControlStateValueOn } else { NSControlStateValueOff });
    cv.addSubview(&chk_translate);

    cv.addSubview(&label("Idioma destino:", 20.0, 61.0, 110.0, mtm));
    let popup_dest = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        rect(135.0, 58.0, 140.0, 26.0),
        false,
    );
    popup_dest.addItemWithTitle(&NSString::from_str("Español"));
    popup_dest.addItemWithTitle(&NSString::from_str("English"));
    popup_dest.selectItemWithTitle(&NSString::from_str(
        if current.translate_dest == "es" { "Español" } else { "English" },
    ));
    cv.addSubview(&popup_dest);

    // ── Botones Cancelar / Aplicar ────────────────────────────────────────
    let delegate = ModalDelegate::new();

    // Fuerza que el botón nativo "X" siga el mismo flujo que Cancelar.
    if let Some(close_btn) = panel.standardWindowButton(NSWindowButton::CloseButton) {
        let delegate_obj: &AnyObject = &*delegate;
        unsafe {
            close_btn.setTarget(Some(delegate_obj));
            close_btn.setAction(Some(sel!(cancelClicked:)));
        }
    }

    let btn_cancel = unsafe { NSButton::buttonWithTitle_target_action(
        &NSString::from_str("Cancelar"),
        Some(&*delegate),
        Some(sel!(cancelClicked:)),
        mtm,
    ) };
    btn_cancel.setFrame(rect(210.0, 15.0, 90.0, 30.0));
    cv.addSubview(&btn_cancel);

    let btn_apply = unsafe { NSButton::buttonWithTitle_target_action(
        &NSString::from_str("Aplicar"),
        Some(&*delegate),
        Some(sel!(applyClicked:)),
        mtm,
    ) };
    btn_apply.setFrame(rect(310.0, 15.0, 90.0, 30.0));
    cv.addSubview(&btn_apply);

    // ── Ejecutar modal (bloquea hasta Aplicar / Cancelar / cierre) ────────
    // En apps "Accessory" (sin Dock) forzamos foco para evitar modal invisible.
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

    // Ocultar el panel explícitamente — runModalForWindow no lo hace solo
    panel.orderOut(None);

    if response != NSModalResponseOK {
        return None;
    }

    // ── Leer estado de los controles ──────────────────────────────────────
    let language = if seg_lang.selectedSegment() == 0 { "es" } else { "en" }.to_string();

    let grammar_enabled = chk_grammar.state() == NSControlStateValueOn;

    let grammar_model = if available_models.is_empty() {
        String::new()
    } else {
        let title = popup_model
            .titleOfSelectedItem()
            .map(|s| s.to_string())
            .unwrap_or_default();
        // Si el usuario dejó el placeholder, no hay modelo seleccionado
        if title == "— seleccionar modelo —" { String::new() } else { title }
    };

    let grammar_prompt_es = txt_prompt_es
        .string()
        .to_string()
        .trim()
        .to_string();

    let grammar_prompt_en = txt_prompt_en
        .string()
        .to_string()
        .trim()
        .to_string();

    let translate_enabled = chk_translate.state() == NSControlStateValueOn;

    let translate_dest = popup_dest
        .titleOfSelectedItem()
        .map(|s| if s.to_string() == "Español" { "es".to_string() } else { "en".to_string() })
        .unwrap_or_else(|| "es".to_string());

    Some(SettingsValues {
        language,
        grammar_enabled,
        grammar_model,
        grammar_prompt_es,
        grammar_prompt_en,
        translate_enabled,
        translate_dest,
    })
}
