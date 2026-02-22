// src/hotkey.rs

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};

pub struct HotkeyHandler {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    hotkey_id: u32,
}

impl HotkeyHandler {
    /// Registra el hotkey ⌘⌥W (Cmd+Option+W) para iniciar/detener grabación
    /// Requiere permiso de Accesibilidad en System Settings
    pub fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Error creando hotkey manager: {}", e))?;

        // ⌘⌥W — Cmd+Option+W para grabar / detener
        let hotkey = HotKey::new(
            Some(Modifiers::META | Modifiers::ALT),
            Code::KeyW,
        );

        let hotkey_id = hotkey.id();

        manager
            .register(hotkey)
            .map_err(|e| format!("Error registrando ⌘⌥W: {}", e))?;

        Ok(HotkeyHandler {
            manager,
            hotkey,
            hotkey_id,
        })
    }

    /// ID del hotkey registrado — usar para comparar eventos en el event loop
    pub fn hotkey_id(&self) -> u32 {
        self.hotkey_id
    }
}

impl Drop for HotkeyHandler {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey);
    }
}
