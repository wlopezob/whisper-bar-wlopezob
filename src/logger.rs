// src/logger.rs

use crate::defaults;
use simplelog::{
    ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::OnceLock;

static LOG_FILE_PATH: OnceLock<String> = OnceLock::new();

/// Ruta del archivo de log: ~/.config/whisperwlopezob/whisperwlopezob.log
pub fn log_path() -> &'static str {
    LOG_FILE_PATH.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/{}/whisperwlopezob.log", home, defaults::APP_CONFIG_DIR)
    })
}

/// Writer que abre (o crea) el archivo en cada escritura.
/// Si el archivo fue eliminado mientras la app corría, se recrea automáticamente.
struct ReopeningWriter;

impl Write for ReopeningWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path())?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Inicializa el logger: escribe a consola Y al archivo de log simultáneamente.
/// Crea el directorio si no existe y trunca el log al iniciar cada sesión.
pub fn init() {
    // Asegurar que el directorio existe
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config_dir = format!("{}/{}", home, defaults::APP_CONFIG_DIR);
    std::fs::create_dir_all(&config_dir).ok();

    // Truncar el log al iniciar (log fresco por sesión)
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path())
        .expect("No se pudo crear el archivo de log");

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::Debug, Config::default(), ReopeningWriter),
    ])
    .expect("No se pudo inicializar el logger");

    log::info!("=== whisperwlopezob iniciado ===");
    log::info!("Log en: {}", log_path());
}
