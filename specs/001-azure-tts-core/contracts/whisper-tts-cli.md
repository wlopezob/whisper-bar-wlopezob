# Contract: whisper-tts CLI

## Descripción

Binary CLI standalone que recibe texto por stdin, lo sintetiza a voz usando Azure
Speech TTS, y lo reproduce con `afplay`. Diseñado para ser invocado desde el Stop
hook de Claude Code (`async: true`).

## Interfaz

### Input

| Canal | Formato | Descripción |
|-------|---------|-------------|
| stdin | texto plano UTF-8 | Texto a sintetizar. Leído hasta EOF. |

**Precondiciones**:
- La DB `~/.config/whisperwlopezob/data.db` debe existir y ser accesible.
- Si `tts_enabled = "false"`, el binary termina sin reproducir nada.
- Si stdin está vacío (solo whitespace), el binary termina sin reproducir nada.

### Output

| Canal | Contenido |
|-------|-----------|
| stdout | Vacío siempre |
| stderr | Vacío siempre (errores van al log, no a stderr) |

### Exit Code

| Código | Significado |
|--------|-------------|
| 0 | Siempre — incluyendo errores, timeouts y texto vacío |

### Efectos

| Efecto | Descripción |
|--------|-------------|
| Audio | Reproducción via `afplay` (Azure TTS) o `say -v Samantha` (fallback) |
| Log | Entrada en `~/.config/whisperwlopezob/whisperwlopezob.log` (append) |
| `/tmp/whisper-tts.pid` | Creado al inicio, eliminado al terminar |
| `/tmp/whisper-tts-audio.mp3` | Creado durante síntesis, eliminado tras reproducción |

## Configuración (desde SQLite)

| Clave DB | Efecto |
|----------|--------|
| `tts_enabled` | "true" = ejecutar, "false" = salir inmediatamente |
| `tts_voice` | Nombre de voz Azure Neural (default: `es-MX-DaliaNeural`) |
| `azure_mai_key` | API Key de Azure Speech. Vacío → fallback a `say` |
| `azure_mai_region` | Región Azure (ej: `eastus`). Vacío → fallback a `say` |

## Comportamiento de Concurrencia

Si hay una instancia previa activa (detectada via `/tmp/whisper-tts.pid`):
1. Se envía SIGTERM al proceso anterior
2. Se espera 300ms
3. La nueva instancia procede con el nuevo texto

## Ejemplos de Uso

```bash
# Uso básico
echo "La compilación fue exitosa." | ./target/debug/whisper-tts

# Simular respuesta de Claude con markdown
printf "## Resultado\n**Error**: el archivo no existe\n" | ./target/debug/whisper-tts

# Verificar que TTS está desactivado (silencio)
sqlite3 ~/.config/whisperwlopezob/data.db "UPDATE settings SET value='false' WHERE key='tts_enabled';"
echo "Este texto no debe sonar" | ./target/debug/whisper-tts; echo "Exit: $?"

# Verificar fallback (sin key)
sqlite3 ~/.config/whisperwlopezob/data.db "UPDATE settings SET value='' WHERE key='azure_mai_key';"
echo "Texto con fallback" | ./target/debug/whisper-tts
```

## Integración Claude Code (Fase 2 — fuera de scope aquí)

```json
{
  "hooks": {
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/tts-response.sh",
        "timeout": 60,
        "async": true
      }]
    }]
  }
}
```

```bash
# ~/.claude/hooks/tts-response.sh
#!/bin/bash
INPUT=$(cat)
echo "$INPUT" | jq -r '.assistant_response // ""' | /path/to/whisper-tts
```
