// src/defaults.rs
// Valores por defecto de la aplicación — fuente única de verdad

/// Directorio base de la aplicación (relativo a $HOME)
pub const APP_CONFIG_DIR: &str = ".config/whisperwlopezob";

/// Idioma de transcripción por defecto
pub const LANGUAGE: &str = "es";

/// Duración mínima de grabación en segundos (grabaciones más cortas se descartan)
pub const MIN_RECORDING_DURATION: f64 = 0.5;

/// Carpeta de modelos Whisper dentro de APP_CONFIG_DIR
pub const WHISPER_MODELS_DIR: &str = "whisper-models";

/// Prioridad de modelos Whisper (mayor a menor calidad)
pub const MODEL_PRIORITY: &[&str] = &[
    "ggml-large-v3.bin",
    "ggml-large-v2.bin",
    "ggml-medium.bin",
    "ggml-small.bin",
    "ggml-base.bin",
    "ggml-tiny.bin",
];

/// Carpeta de modelos LLM (.gguf) dentro de APP_CONFIG_DIR
pub const LLM_MODELS_DIR: &str = "llm";

/// Rutas candidatas de whisper-cli (en orden de preferencia)
pub const WHISPER_CLI_CANDIDATES: &[&str] = &[
    "/opt/homebrew/bin/whisper-cli", // Apple Silicon
    "/usr/local/bin/whisper-cli",    // Intel
    "/usr/bin/whisper-cli",
];

/// Idioma destino de traducción por defecto
pub const TRANSLATE_DEST_LANG: &str = "es";

/// Versión de la API de Azure MAI Transcribe
pub const AZURE_MAI_API_VERSION: &str = "2024-11-15";

/// Prompt de corrección gramatical por defecto para inglés.
/// Incluye /no_think para evitar cadenas de razonamiento en modelos thinking.
pub const GRAMMAR_PROMPT_EN: &str =
    "Fix grammar and pronunciation errors in this English text. Return ONLY the corrected text, no explanations, no extra words. /no_think";

/// Prompt de corrección gramatical por defecto para español.
/// Incluye /no_think para evitar cadenas de razonamiento en modelos thinking.
pub const GRAMMAR_PROMPT_ES: &str =
    "Corrige los errores gramaticales de este texto en español. Devuelve ÚNICAMENTE el texto corregido. /no_think";


/// Rutas candidatas de CLI LLM (prioriza llama-completion; fallback llama-cli)
pub const LLAMA_CLI_CANDIDATES: &[&str] = &[
    "/opt/homebrew/bin/llama-completion", // Apple Silicon (preferido)
    "/usr/local/bin/llama-completion",    // Intel (preferido)
    "/opt/homebrew/bin/llama-cli",        // fallback legacy
    "/usr/local/bin/llama-cli",           // fallback legacy
];
