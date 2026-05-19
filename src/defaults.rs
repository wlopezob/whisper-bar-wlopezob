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

/// Versión de la API de Azure MAI Transcribe (LLM Speech API)
pub const AZURE_MAI_API_VERSION: &str = "2025-10-15";

/// Definition JSON por defecto para MAI-Transcribe-1
pub const AZURE_MAI_DEFINITION: &str =
    r#"{"enhancedMode":{"enabled":true,"model":"mai-transcribe-1"}}"#;

/// Prompt de corrección gramatical por defecto para inglés.
/// Incluye /no_think para evitar cadenas de razonamiento en modelos thinking.
pub const GRAMMAR_PROMPT_EN: &str =
    "Fix grammar and pronunciation errors in this English text. Return ONLY the corrected text, no explanations, no extra words. /no_think";

/// Prompt de corrección gramatical por defecto para español.
/// Incluye /no_think para evitar cadenas de razonamiento en modelos thinking.
pub const GRAMMAR_PROMPT_ES: &str =
    "Corrige los errores gramaticales de este texto en español. Devuelve ÚNICAMENTE el texto corregido. /no_think";


/// Voz Gemini por defecto para síntesis TTS
pub const TTS_DEFAULT_VOICE: &str = "Sulafat";

/// Descripción de escena por defecto para el director's note de Gemini TTS
pub const TTS_DEFAULT_SCENE: &str =
    "A highly natural female AI assistant speaking fluent Latin American Spanish \
     and English naturally. She sounds warm, intelligent, calm, conversational, \
     and human-like. Maintain smooth pacing, realistic pauses, subtle emotional \
     nuance, and clear pronunciation in both Spanish and English. Never sound \
     robotic, overly excited, or exaggerated.";

/// Contexto de muestra por defecto para el director's note de Gemini TTS
pub const TTS_DEFAULT_SAMPLE_CONTEXT: &str =
    "The assistant is having a real-time voice conversation with a user. \
     Responses should feel fluid, natural, concise, friendly, and emotionally \
     subtle in both English and Spanish. Maintain conversational rhythm with \
     realistic pauses and natural transitions between languages when needed.";

/// Modelo Gemini para formatear respuestas antes de TTS
pub const GEMINI_FORMATTER_MODEL: &str = "gemini-3.1-flash-lite";

/// Prompt por defecto para el formateador TTS (convierte respuestas AI a prosa natural)
pub const FORMATTER_DEFAULT_PROMPT: &str =
    "Tu tarea es adaptar la respuesta de un asistente de IA para que sea natural al escucharla en voz alta.\n\n\
     Reglas:\n\
     1. Elimina todo el código fuente. Si hay código, menciona en una frase qué hace.\n\
     2. Quita el formato markdown: asteriscos, backticks, encabezados, guiones de lista.\n\
     3. Convierte listas en oraciones fluidas conectadas con \"y\", \"además\" o \"también\".\n\
     4. Sé conciso: máximo 4 oraciones para respuestas cortas, 8 para respuestas largas.\n\
     5. Conserva advertencias críticas, errores y próximos pasos importantes.\n\
     6. Usa el idioma predominante en la respuesta (español o inglés).\n\
     7. Tono conversacional directo, sin frases como \"Aquí está:\" o \"En resumen:\".\n\
     8. Responde ÚNICAMENTE con el texto listo para ser leído. Nada más.";

/// Rutas candidatas de CLI LLM (prioriza llama-completion; fallback llama-cli)
pub const LLAMA_CLI_CANDIDATES: &[&str] = &[
    "/opt/homebrew/bin/llama-completion", // Apple Silicon (preferido)
    "/usr/local/bin/llama-completion",    // Intel (preferido)
    "/opt/homebrew/bin/llama-cli",        // fallback legacy
    "/usr/local/bin/llama-cli",           // fallback legacy
];
