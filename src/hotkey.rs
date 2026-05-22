// src/hotkey.rs

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};

pub struct HotkeyHandler {
    manager: GlobalHotKeyManager,
    hotkey_record: HotKey,
    hotkey_replay: HotKey,
    hotkey_modal: HotKey,
    record_id: u32,
    replay_id: u32,
    modal_id: u32,
}

impl HotkeyHandler {
    /// Registra ⌘⌥W (grabar), ⌘⌥R (repetir audio) y ⌘⌥V (ver texto modal)
    pub fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Error creando hotkey manager: {}", e))?;

        let hotkey_record = HotKey::new(Some(Modifiers::META | Modifiers::ALT), Code::KeyW);
        let hotkey_replay = HotKey::new(Some(Modifiers::META | Modifiers::ALT), Code::KeyR);
        let hotkey_modal  = HotKey::new(Some(Modifiers::META | Modifiers::ALT), Code::KeyV);

        let record_id = hotkey_record.id();
        let replay_id = hotkey_replay.id();
        let modal_id  = hotkey_modal.id();

        manager
            .register(hotkey_record)
            .map_err(|e| format!("Error registrando ⌘⌥W: {}", e))?;
        manager
            .register(hotkey_replay)
            .map_err(|e| format!("Error registrando ⌘⌥R: {}", e))?;
        manager
            .register(hotkey_modal)
            .map_err(|e| format!("Error registrando ⌘⌥V: {}", e))?;

        Ok(HotkeyHandler {
            manager,
            hotkey_record,
            hotkey_replay,
            hotkey_modal,
            record_id,
            replay_id,
            modal_id,
        })
    }

    pub fn hotkey_id(&self) -> u32 {
        self.record_id
    }

    pub fn replay_hotkey_id(&self) -> u32 {
        self.replay_id
    }

    pub fn modal_hotkey_id(&self) -> u32 {
        self.modal_id
    }
}

impl Drop for HotkeyHandler {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey_record);
        let _ = self.manager.unregister(self.hotkey_replay);
        let _ = self.manager.unregister(self.hotkey_modal);
    }
}
