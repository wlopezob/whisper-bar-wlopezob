// src/db.rs

use crate::defaults;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Ruta de la base de datos: ~/.config/whisperwlopezob/data.db
fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(defaults::APP_CONFIG_DIR)
        .join("data.db")
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Abre (o crea) la base de datos e inicializa el schema
    pub fn open() -> Result<Arc<Self>> {
        let path = db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        log::info!("DB: {:?}", path);

        Ok(Arc::new(Db {
            conn: Mutex::new(conn),
        }))
    }

    /// Lee un valor de settings; devuelve `default` si la clave no existe
    pub fn get(&self, key: &str, default: &str) -> String {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| default.to_string())
    }

    /// Inserta o actualiza un valor en settings
    pub fn set(&self, key: &str, value: &str) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        ) {
            log::error!("DB: error guardando '{}': {}", key, e);
        }
    }
}
