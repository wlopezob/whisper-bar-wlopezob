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

/// Proveedor de traducción por defecto
pub const TRANSLATE_DEFAULT_PROVIDER: &str = "azure";

/// Modelo Ollama para traducción
pub const TRANSLATE_OLLAMA_MODEL: &str = "gemma4:e4b";

/// Prompt por defecto para Ollama translator (usa {dest_lang} y {input_text} como placeholders)
pub const TRANSLATE_OLLAMA_DEFAULT_PROMPT: &str =
    "Eres un traductor. Analiza el texto y devuelve ÚNICAMENTE un JSON con este formato exacto:\n\
     {\"text\": \"...\", \"detected_lang\": \"es\"}\n\n\
     Reglas:\n\
     - detected_lang: idioma del texto original (\"es\" o \"en\")\n\
     - text: si detected_lang == \"{dest_lang}\", devuelve el texto original sin cambios\n\
     - text: si detected_lang != \"{dest_lang}\", traduce a {dest_lang}\n\
     - Nada más que el JSON. Sin explicaciones, sin markdown, sin backticks.\n\n\
     Texto: \"{input_text}\"";

/// Versión de la API de Azure MAI Transcribe (LLM Speech API)
pub const AZURE_MAI_API_VERSION: &str = "2025-10-15";

/// Definition JSON por defecto para MAI-Transcribe-1.5
pub const AZURE_MAI_DEFINITION: &str =
    r#"{"enhancedMode":{"enabled":true,"model":"mai-transcribe-1.5"}}"#;

/// Prompt de corrección gramatical por defecto para inglés.
/// Incluye /no_think para evitar cadenas de razonamiento en modelos thinking.
pub const GRAMMAR_PROMPT_EN: &str =
    "Fix grammar and pronunciation errors in this English text. Return ONLY the corrected text, no explanations, no extra words. /no_think";

/// Prompt de corrección gramatical por defecto para español.
/// Incluye /no_think para evitar cadenas de razonamiento en modelos thinking.
pub const GRAMMAR_PROMPT_ES: &str =
    "Corrige los errores gramaticales de este texto en español. Devuelve ÚNICAMENTE el texto corregido. /no_think";


/// Subcarpeta de audio dentro de APP_CONFIG_DIR — guarda last-tts.wav
pub const TTS_AUDIO_DIR: &str = "audio";

/// Nombre del archivo de la última respuesta TTS (repetir con ⌘⌥R)
pub const TTS_LAST_AUDIO_FILE: &str = "last-tts.wav";

/// Nombre del archivo con el último texto TTS (ver modal con ⌘⌥V)
pub const TTS_LAST_TEXT_FILE: &str = "last-tts-text.txt";

/// Voz Gemini por defecto para síntesis TTS
pub const TTS_DEFAULT_VOICE: &str = "Sulafat";

/// Velocidad de reproducción TTS (1.0 = normal, 0.85 = 85%)
pub const TTS_DEFAULT_PLAYBACK_RATE: &str = "1.0";

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
    "Your task: rewrite an AI assistant's response so it sounds natural when read aloud.\n\n\
     CRITICAL — LANGUAGE RULE (read first):\n\
     Detect the language of [Assistant response]. Output MUST be in that exact language.\n\
     If the assistant wrote in English → your output in English.\n\
     If the assistant wrote in Spanish → your output in Spanish.\n\
     NEVER translate. NEVER switch languages.\n\n\
     Rules:\n\
     1. Remove all source code. If there is code, describe in one sentence what it does.\n\
     2. Remove markdown formatting: asterisks, backticks, headers, list dashes.\n\
     3. Convert lists into flowing sentences joined with \"and\", \"also\", or \"additionally\".\n\
     4. Be concise: max 4 sentences for short responses, 8 for long ones.\n\
     5. Keep critical warnings, errors, and important next steps.\n\
     6. Direct conversational tone. No phrases like \"Here is:\" or \"In summary:\".\n\
     7. Output ONLY the ready-to-speak text. Nothing else.";

/// Rutas candidatas de CLI LLM (prioriza llama-completion; fallback llama-cli)
pub const LLAMA_CLI_CANDIDATES: &[&str] = &[
    "/opt/homebrew/bin/llama-completion", // Apple Silicon (preferido)
    "/usr/local/bin/llama-completion",    // Intel (preferido)
    "/opt/homebrew/bin/llama-cli",        // fallback legacy
    "/usr/local/bin/llama-cli",           // fallback legacy
];
