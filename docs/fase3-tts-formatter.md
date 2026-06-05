# Fase 3: Formateador LLM antes de TTS (Gemini 3.1 Flash Lite)

## ¿Qué hace esta fase?

Las respuestas de Claude Code contienen código, markdown y listas que suenan mal al leerse en voz alta. Esta fase añade un paso intermedio que **reformatea** el texto antes de sintetizarlo:

```
Claude Code responde
       ↓
Stop hook captura el texto (fase 2)
       ↓
[NUEVO] Gemini 3.1 Flash Lite reformatea a prosa natural
       ↓
Gemini TTS sintetiza el audio
       ↓
afplay reproduce
```

El formateador convierte respuestas como:

> "Updated `Cargo.toml` with `base64 = \"0.22\"`. Run `cargo build --release`."

En algo natural para voz:

> "Actualicé el archivo de dependencias para agregar soporte de base64. Ahora puedes compilar en modo release."

---

## Modelo usado

**`gemini-3.1-flash-lite`** (GA desde mayo 7, 2026)

- Más rápido que `gemini-2.5-flash` (381.9 tokens/s)
- Precio: $0.25/$1.50 por millón de tokens entrada/salida
- **Misma API key que TTS** — no requiere credenciales adicionales
- Límite de salida: 64K tokens (más que suficiente)

---

## Prompt por defecto (configurable en Settings)

```
Tu tarea es adaptar la respuesta de un asistente de IA para que sea natural al escucharla en voz alta.

Reglas:
1. Elimina todo el código fuente. Si hay código, menciona en una frase qué hace.
2. Quita el formato markdown: asteriscos, backticks, encabezados, guiones de lista.
3. Convierte listas en oraciones fluidas conectadas con "y", "además" o "también".
4. Sé conciso: máximo 4 oraciones para respuestas cortas, 8 para respuestas largas.
5. Conserva advertencias críticas, errores y próximos pasos importantes.
6. Usa el idioma predominante en la respuesta (español o inglés).
7. Tono conversacional directo, sin frases como "Aquí está:" o "En resumen:".
8. Responde ÚNICAMENTE con el texto listo para ser leído. Nada más.
```

---

## Configuración en la app

Abrir la app → **Configuración** → sección **LECTURA DE RESPUESTAS**:

| Campo | Descripción |
|---|---|
| **Formatear respuesta para voz** | Checkbox para activar/desactivar el formatter |
| **Prompt TTS:** | Textarea con el prompt del formateador (editable) |

El formatter solo se activa si:
- `Formatear respuesta para voz` está marcado
- `Clave Gemini:` tiene un valor válido

---

## Configuración directa en SQLite

```bash
# Activar formatter
sqlite3 ~/.config/whisperwlopezob/data.db \
  "INSERT OR REPLACE INTO settings VALUES
   ('tts_formatter_enabled', 'true');"

# Personalizar prompt (ejemplo más corto/agresivo)
sqlite3 ~/.config/whisperwlopezob/data.db \
  "INSERT OR REPLACE INTO settings VALUES
   ('tts_formatter_prompt', 'Convierte este texto técnico en una frase o dos en español, sin código. Solo el resultado.');"

# Ver configuración actual
sqlite3 ~/.config/whisperwlopezob/data.db \
  "SELECT key, substr(value,1,60) FROM settings WHERE key LIKE 'tts%';"
```

---

## Arquitectura del código

```
src/
├── formatter.rs              ← GeminiFormatter::format() — nueva
├── bin/
│   └── whisper-tts.rs        ← orquesta: formatter → TTS
└── defaults.rs               ← FORMATTER_DEFAULT_PROMPT, GEMINI_FORMATTER_MODEL
```

### Flujo en whisper-tts.rs

```
stdin → texto crudo
  ↓
tts_formatter_enabled && gemini_key no vacío?
  ├── sí → GeminiFormatter::format(texto, prompt) → texto formateado
  │         (si falla → usa texto original, log error)
  └── no → texto original
  ↓
tts::speak(texto_final, TtsConfig)
  ↓
GeminiProvider::synthesize (gemini-3.1-flash-tts-preview)
  ↓
afplay
```

---

## Verificación

```bash
# 1. Compilar
cargo build 2>&1 | grep "^error"

# 2. Activar formatter en DB
sqlite3 ~/.config/whisperwlopezob/data.db \
  "INSERT OR REPLACE INTO settings VALUES ('tts_formatter_enabled','true');"

# 3. Probar con texto técnico (requiere gemini_api_key configurada)
echo 'Updated \`Cargo.toml\` with \`base64 = "0.22"\`. Run \`cargo build --release --bins\`.' \
  | ./target/debug/whisper-tts

# 4. Ver log (debe mostrar "TTS formatter: ok (N chars → M chars)")
tail -20 ~/.config/whisperwlopezob/whisperwlopezob.log

# 5. Reinstalar binario release para el hook
cargo build --release --bins
cp target/release/whisper-tts ~/.local/bin/whisper-tts
```

---

## Replicar en otra máquina

El formatter se activa automáticamente al correr `bundle.sh` si la clave Gemini está configurada. Pasos completos:

```bash
# 1. Clonar e instalar (compila, instala binarios, registra hook)
git clone <repo-url> ~/project/whisper-bar-wlopezob
cd ~/project/whisper-bar-wlopezob
./bundle.sh

# 2. Configurar Gemini y activar TTS + formatter
sqlite3 ~/.config/whisperwlopezob/data.db \
  "INSERT OR REPLACE INTO settings VALUES
   ('tts_enabled',           'true'),
   ('tts_voice',             'Sulafat'),
   ('gemini_api_key',        'TU_API_KEY'),
   ('tts_formatter_enabled', 'true');"
```

---

## Personalización del prompt

El prompt default está optimizado para respuestas bilingües español/inglés. Ejemplos de prompts alternativos:

**Solo inglés, ultra-conciso:**
```
Convert this AI response to 1-2 natural spoken sentences in English. Remove all code. Just the result, nothing else.
```

**Solo español, detallado:**
```
Adapta esta respuesta técnica a 3-4 oraciones en español conversacional. Sin código, sin markdown. Solo el texto para hablar en voz alta.
```

**Preservar más información:**
```
Reformatea esta respuesta como prosa natural para voz. Elimina código fuente, convierte markdown a texto plano. Mantén todos los puntos importantes. Usa el mismo idioma.
```
