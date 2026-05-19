# Quickstart: Azure TTS Core

## Pre-requisitos

- `cargo` instalado (Rust edition 2024)
- Credenciales Azure MAI configuradas en la app (o manualmente en SQLite)
- `afplay` disponible (macOS 13+ — incluido en sistema base)

## Compilar el binary TTS

```bash
cargo build --bin whisper-tts
```

## Configurar via SQLite (para pruebas sin UI)

```bash
sqlite3 ~/.config/whisperwlopezob/data.db <<'SQL'
INSERT OR REPLACE INTO settings VALUES
  ('tts_enabled', 'true'),
  ('tts_voice', 'es-MX-DaliaNeural'),
  ('azure_mai_key', 'tu-api-key-aquí'),
  ('azure_mai_region', 'eastus');
SQL
```

## Prueba básica

```bash
echo "Hola, la compilación fue exitosa." | ./target/debug/whisper-tts
# → Debe reproducir la voz de Dalia en el altavoz
```

## Prueba de fallback (sin credenciales)

```bash
sqlite3 ~/.config/whisperwlopezob/data.db \
  "UPDATE settings SET value='' WHERE key='azure_mai_key';"

echo "Texto de prueba" | ./target/debug/whisper-tts
# → Debe reproducir con `say -v Samantha`, exit 0
```

## Prueba de TTS desactivado

```bash
sqlite3 ~/.config/whisperwlopezob/data.db \
  "UPDATE settings SET value='false' WHERE key='tts_enabled';"

echo "Este texto no debe sonar" | ./target/debug/whisper-tts
echo "Exit: $?"   # → Exit: 0, sin audio
```

## Prueba de limpieza de markdown

```bash
cat <<'EOF' | ./target/debug/whisper-tts
## Resultado de la compilación

**Error**: El archivo `main.rs` tiene un problema en la línea 42.

- Verifica que el módulo esté importado
- Añade `use crate::tts;` al inicio
EOF
# → Debe sonar natural, sin leer **, `, ## ni guiones
```

## Configurar via Settings UI

1. `bash bundle.sh && open /Applications/whisperwlopezob.app`
2. Clic en icono → **Configuración...**
3. Cambiar Backend a **Azure MAI**
4. En sección **LECTURA DE RESPUESTAS**: activar checkbox, confirmar voz
5. Clic **Aplicar**
6. Verificar en DB: `sqlite3 ~/.config/whisperwlopezob/data.db "SELECT key,value FROM settings WHERE key LIKE 'tts%';"`

## Ver log en tiempo real

```bash
tail -f ~/.config/whisperwlopezob/whisperwlopezob.log
# Mientras ejecutas el binary en otra terminal:
echo "Prueba de log" | ./target/debug/whisper-tts
# → Deben aparecer entradas TTS: sin borrar las entradas previas de la app
```

## Prueba de concurrencia (interrumpir y reemplazar)

```bash
# Terminal 1: texto largo para que tarde en reproducir
python3 -c "print('texto ' * 200)" | ./target/debug/whisper-tts &

# Terminal 2: segundos después, enviar nuevo texto (debe interrumpir el anterior)
sleep 2 && echo "Nuevo texto que interrumpe al anterior" | ./target/debug/whisper-tts
```
