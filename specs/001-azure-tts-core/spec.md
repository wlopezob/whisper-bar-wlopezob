# Feature Specification: Azure TTS Core — Síntesis de Voz para Respuestas de Claude

**Feature Branch**: `001-azure-tts-core`
**Created**: 2026-05-16
**Status**: Draft

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reproducción de texto como voz mediante CLI (Priority: P1)

El desarrollador ejecuta el binario `whisper-tts` pasando texto por stdin y escucha el
audio reproducido en el altavoz de su Mac. El binario usa las credenciales Azure
guardadas en la base de datos local para sintetizar la voz.

**Why this priority**: Es el núcleo de toda la funcionalidad TTS. Sin esto no hay Fase 2
(hook), ni Fase 3 (summarizer). Debe funcionar de forma standalone antes de integrarse
con Claude Code.

**Independent Test**: `echo "La compilación fue exitosa." | whisper-tts` reproduce audio
con la voz configurada. El binario termina con exit code 0.

**Acceptance Scenarios**:

1. **Given** las credenciales Azure están en la DB y `tts_enabled = "true"`, **When** se ejecuta `echo "Texto de prueba" | whisper-tts`, **Then** se reproduce audio en el altavoz con la voz configurada y el proceso termina con exit 0.

2. **Given** `tts_enabled = "false"` en la DB, **When** se ejecuta `echo "Cualquier texto" | whisper-tts`, **Then** no se reproduce ningún audio y el proceso termina silenciosamente con exit 0.

3. **Given** el texto contiene markdown (`**negrita**`, `` `código` ``, `# Título`), **When** se ejecuta el binario, **Then** el audio reproduce texto limpio sin leer los símbolos de formato.

---

### User Story 2 - Fallback automático cuando Azure no está disponible (Priority: P2)

Cuando las credenciales Azure están vacías, son inválidas, o la llamada HTTP falla
(timeout, red no disponible), el sistema reproduce el texto con el motor de voz
local de macOS (`say`) sin mostrar errores al usuario.

**Why this priority**: El binario será invocado por un hook de Claude Code con
ejecución asíncrona. Un crash o error visible rompería el flujo de trabajo. El
fallback garantiza que el usuario siempre escucha algo útil.

**Independent Test**: Con `azure_mai_key` vacío en la DB, `echo "Texto" | whisper-tts`
produce audio via `say` y termina con exit 0.

**Acceptance Scenarios**:

1. **Given** `azure_mai_key` está vacío en la DB, **When** se ejecuta el binario con texto, **Then** se reproduce el audio con la voz local de macOS y exit code es 0.

2. **Given** la clave Azure es inválida y la API devuelve error HTTP, **When** se ejecuta el binario, **Then** el fallback activa `say`, no se muestra ningún error en stdout/stderr, exit 0.

3. **Given** la llamada HTTP supera 10 segundos sin respuesta, **When** se ejecuta el binario, **Then** el timeout corta la llamada, activa `say`, y el binario termina en ≤ 11 segundos con exit 0.

---

### User Story 3 - Configuración de TTS desde la ventana de Settings (Priority: P3)

El usuario activa/desactiva la lectura de respuestas y elige la voz Azure desde la
ventana nativa de Configuración de la app, dentro de la sección Azure MAI. Los
cambios se guardan en la base de datos local y el binario los lee en la siguiente
invocación.

**Why this priority**: Sin la UI el usuario debe editar SQLite manualmente. La UI es
necesaria para el uso diario pero no bloquea la validación técnica del TTS (P1 y P2
son independientes).

**Independent Test**: Abrir Configuración → cambiar a Backend "Azure MAI" → activar
"Leer respuestas de Claude" → cambiar voz → Aplicar → verificar en la DB que
`tts_enabled = "true"` y `tts_voice` tiene el valor ingresado.

**Acceptance Scenarios**:

1. **Given** el usuario selecciona backend "Azure MAI" en Configuración, **When** aparece la sección "LECTURA DE RESPUESTAS", **Then** se muestran el checkbox "Leer respuestas de Claude" y el campo "Voz:" con el valor guardado en la DB (default: "es-MX-DaliaNeural").

2. **Given** el usuario activa el checkbox y escribe una voz distinta, luego pulsa Aplicar, **When** se cierra la ventana, **Then** la DB contiene `tts_enabled = "true"` y `tts_voice = <voz ingresada>`.

3. **Given** el usuario cambia el backend a "Whisper local", **When** la sección cambia, **Then** los controles de TTS se ocultan junto con los demás campos Azure.

4. **Given** el usuario pulsa Cancelar o cierra con la X, **When** la ventana se cierra, **Then** los valores de TTS en la DB no cambian.

---

### Edge Cases

- ¿Qué pasa si stdin está vacío? → el binario termina con exit 0 sin llamar a Azure ni `say`.
- ¿Qué pasa si el campo "Voz:" en la UI se deja en blanco al guardar? → se usa el default `es-MX-DaliaNeural`.
- ¿Qué pasa con textos muy largos (>5000 caracteres)? → se trunca a 5000 caracteres antes de enviar a Azure para evitar rechazos de la API.
- ¿Qué pasa si `afplay` no puede reproducir el archivo? → el proceso termina igualmente con exit 0; el audio simplemente no suena.
- ¿Qué pasa si ya hay una instancia de `whisper-tts` reproduciendo audio cuando llega una nueva invocación? → la nueva instancia DEBE interrumpir la reproducción activa y comenzar con el nuevo texto (interrumpir y reemplazar). La instancia anterior se termina antes de iniciar la síntesis nueva.

## Clarifications

### Session 2026-05-16

- Q: ¿Dónde y cómo se registran las actividades TTS? → A: Usar el log existente (`~/.config/whisperwlopezob/whisperwlopezob.log`) en modo append, sin truncar el archivo; registrar los eventos: síntesis iniciada, éxito, fallback activado, omisión por `tts_enabled=false`, y error con motivo.
- Q: ¿Qué hace el binario cuando ya hay una instancia reproduciendo audio? → A: Interrumpir y reemplazar — detener la reproducción activa e iniciar inmediatamente con el nuevo texto.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: El sistema DEBE sintetizar texto recibido por stdin usando la API REST de Azure Speech TTS cuando las credenciales están configuradas y `tts_enabled = "true"`.
- **FR-002**: El sistema DEBE limpiar marcadores de markdown del texto antes de sintetizarlo (`**`, `*`, `` ` ``, `#` al inicio de línea, guiones de lista al inicio de línea).
- **FR-003**: El sistema DEBE aplicar un timeout de 10 segundos a la llamada HTTP a Azure Speech.
- **FR-004**: El sistema DEBE reproducir el audio resultante usando `afplay` en macOS.
- **FR-005**: El sistema DEBE caer silenciosamente al motor de voz local (`say -v Samantha`) si Azure no está disponible, las credenciales están vacías, o la llamada HTTP falla.
- **FR-006**: El sistema DEBE terminar siempre con exit code 0, independientemente del resultado.
- **FR-007**: El sistema DEBE leer la configuración TTS (`tts_enabled`, `tts_voice`, `azure_mai_key`, `azure_mai_region`) desde la base de datos SQLite en `~/.config/whisperwlopezob/data.db`.
- **FR-008**: El sistema DEBE omitir toda acción si `tts_enabled = "false"` en la DB.
- **FR-009**: El sistema DEBE omitir toda acción si stdin está vacío.
- **FR-010**: El sistema DEBE truncar el texto a 5000 caracteres antes de enviarlo a Azure.
- **FR-011**: La ventana de Configuración DEBE mostrar el bloque "LECTURA DE RESPUESTAS" únicamente cuando el backend seleccionado es "Azure MAI".
- **FR-012**: La ventana de Configuración DEBE persistir `tts_enabled` y `tts_voice` en SQLite al pulsar Aplicar.
- **FR-013**: El valor por defecto de la voz DEBE ser `es-MX-DaliaNeural`.
- **FR-015**: Al iniciarse, el binario DEBE verificar si existe otra instancia activa de reproducción TTS (mediante un archivo PID en `/tmp/whisper-tts.pid`). Si existe y el proceso sigue activo, DEBE terminarlo antes de proceder. Al finalizar, DEBE eliminar el archivo PID.
- **FR-014**: El binario DEBE registrar los siguientes eventos en el log existente (`whisperwlopezob.log`) en modo append (sin truncar): síntesis iniciada (voz y longitud del texto), síntesis exitosa, fallback a `say` activado (con motivo), omisión por `tts_enabled = "false"`, y error de síntesis (con mensaje). El binario NUNCA DEBE truncar el archivo de log al inicializarse.

### Key Entities

- **TTS Config**: Representa la configuración de síntesis de voz del usuario. Atributos: `tts_enabled` (activado/desactivado), `tts_voice` (nombre de la voz Azure Neural). Persiste en la tabla `settings` de SQLite junto con el resto de la configuración de la app.
- **Audio temporal**: Archivo `.mp3` generado en `/tmp/` durante la síntesis. Se elimina tras la reproducción. No persiste entre ejecuciones.
- **PID de control**: Archivo `/tmp/whisper-tts.pid` que contiene el PID del proceso activo. Permite a una nueva instancia detectar y terminar la reproducción anterior (comportamiento interrumpir-y-reemplazar). Se crea al iniciar la reproducción y se elimina al terminar.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: El tiempo desde que el binario recibe el texto hasta que comienza la reproducción de audio es inferior a 3 segundos en condiciones normales de red.
- **SC-002**: El binario termina con exit code 0 en el 100% de los casos, incluyendo errores, timeouts y texto vacío.
- **SC-003**: El texto reproducido es inteligible y no contiene símbolos de markdown (verificable escuchando la salida con texto que incluya `**`, `` ` `` y `#`).
- **SC-004**: Los cambios de configuración realizados en la ventana de Settings se reflejan en la siguiente invocación del binario sin reiniciar la app.
- **SC-005**: Cuando Azure falla, el fallback a voz local completa la reproducción en menos de 2 segundos adicionales tras el timeout.
- **SC-006**: Cada invocación del binario deja al menos una entrada en `whisperwlopezob.log` sin eliminar entradas previas de la app principal (verificable con `tail -f` mientras se usa la app).

## Assumptions

- Las credenciales Azure (`azure_mai_key`, `azure_mai_region`) ya pueden estar guardadas en SQLite si el usuario configuró Azure MAI Transcribe previamente; si no, el binario activa el fallback.
- `afplay` y `say` están disponibles en macOS 13+ como parte del sistema base.
- El binario `whisper-tts` se invoca directamente desde `./target/debug/whisper-tts` durante desarrollo y se incluirá en el bundle de la app en fases posteriores.
- El campo "Voz:" de la UI acepta cualquier string libre; la validación de que sea una voz Azure Neural válida queda fuera del scope de esta fase.
- La API REST de Azure Speech TTS v1 es compatible con las credenciales MAI ya usadas para transcripción en `azure_transcriber.rs`.
- El formato de audio `audio-16khz-128kbitrate-mono-mp3` es reproducible por `afplay` en macOS sin configuración adicional.
- El scope de esta fase es exclusivamente el binario CLI y la configuración UI/DB. La integración con el Stop hook de Claude Code es Fase 2.
- El binario usa una inicialización de log en modo append (sin truncar) distinta a la de la app principal, para no borrar el historial de la sesión activa.
