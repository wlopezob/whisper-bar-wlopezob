// src/hotkey.rs

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};

pub struct HotkeyHandler {
    manager: GlobalHotKeyManager,
    hotkey_record: HotKey,
    hotkey_replay: HotKey,
    record_id: u32,
    replay_id: u32,
}

impl HotkeyHandler {
    /// Registra ⌘⌥W (grabar/transcribir) y ⌘⌥R (repetir última respuesta TTS)
    pub fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Error creando hotkey manager: {}", e))?;

        let hotkey_record = HotKey::new(Some(Modifiers::META | Modifiers::ALT), Code::KeyW);
        let hotkey_replay = HotKey::new(Some(Modifiers::META | Modifiers::ALT), Code::KeyR);

        let record_id = hotkey_record.id();
        let replay_id = hotkey_replay.id();

        manager
            .register(hotkey_record)
            .map_err(|e| format!("Error registrando ⌘⌥W: {}", e))?;
        manager
            .register(hotkey_replay)
            .map_err(|e| format!("Error registrando ⌘⌥R: {}", e))?;

        Ok(HotkeyHandler {
            manager,
            hotkey_record,
            hotkey_replay,
            record_id,
            replay_id,
        })
    }

    pub fn hotkey_id(&self) -> u32 {
        self.record_id
    }

    pub fn replay_hotkey_id(&self) -> u32 {
        self.replay_id
    }
}

impl Drop for HotkeyHandler {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey_record);
        let _ = self.manager.unregister(self.hotkey_replay);
    }
}
