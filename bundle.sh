#!/usr/bin/env bash
# bundle.sh — Compila e instala whisperwlopezob.app en /Applications/
set -e

APP_NAME="whisperwlopezob"
BUNDLE_ID="com.wlopezob.whisperwlopezob"
BINARY="target/release/whisper-bar-rust"
APP_DIR="/Applications/${APP_NAME}.app"
LOG_PATH="${HOME}/.config/${APP_NAME}/${APP_NAME}.log"

# Añade un Stop hook a un JSON de settings (Claude Code o Codex) sin tocar los
# demás hooks. Crea el archivo si no existe; si existe pero no es JSON válido,
# avisa y no lo modifica (sobrescribirlo borraría la config del usuario).
#   $1 = archivo  $2 = comando  $3 = campos extra del handler en JSON
register_stop_hook() {
    python3 - "$1" "$2" "$3" <<'PYEOF'
import json, os, sys

path, command, extra = sys.argv[1], sys.argv[2], json.loads(sys.argv[3])

data = {}
if os.path.isfile(path):
    try:
        with open(path) as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        print(f"  ⚠️  {path} no es JSON válido ({e}); registra el hook a mano")
        sys.exit(0)

stops = data.setdefault("hooks", {}).setdefault("Stop", [])
if any(h.get("command") == command for s in stops for h in s.get("hooks", [])):
    print(f"  → Stop hook ya registrado en {path} (sin cambios)")
    sys.exit(0)

stops.append({"hooks": [{"type": "command", "command": command, **extra}]})
os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
print(f"  → Stop hook registrado en {path}")
PYEOF
}

echo "🔨 Compilando..."
cargo build --release --bins

echo "📦 Creando bundle ${APP_NAME}.app..."

# Si el bundle no existe, crearlo con sudo y ceder ownership al usuario actual
# (solo necesario la primera vez)
if [ ! -d "${APP_DIR}" ]; then
    echo "  → Primera instalación en /Applications/ (requiere contraseña una sola vez)"
    sudo mkdir -p "${APP_DIR}"
    sudo chown -R "$(whoami)" "${APP_DIR}"
fi

# Crear estructura del bundle (sin sudo desde aquí en adelante)
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

# Matar instancia anterior si está corriendo
pkill "${APP_NAME}" 2>/dev/null && echo "  → Instancia anterior detenida" || true

# Copiar binario
cp "${BINARY}" "${APP_DIR}/Contents/MacOS/${APP_NAME}"
chmod +x "${APP_DIR}/Contents/MacOS/${APP_NAME}"

# Info.plist
cat > "${APP_DIR}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>

    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>

    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>

    <key>CFBundleVersion</key>
    <string>2.0.0</string>

    <key>CFBundleShortVersionString</key>
    <string>2.0.0</string>

    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>

    <key>CFBundlePackageType</key>
    <string>APPL</string>

    <!-- Ocultar icono del Dock — solo barra de menú -->
    <key>LSUIElement</key>
    <true/>

    <!-- Descripción para el diálogo de permiso de micrófono -->
    <key>NSMicrophoneUsageDescription</key>
    <string>${APP_NAME} necesita el micrófono para grabar audio y transcribirlo con Whisper.</string>

    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>

    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
</dict>
</plist>
EOF

# Quitar cuarentena
xattr -dr com.apple.quarantine "${APP_DIR}" 2>/dev/null || true

# ── Instalar whisper-tts en ~/.local/bin ──────────────────────────────────────
echo "📦 Instalando whisper-tts..."
mkdir -p "${HOME}/.local/bin"
cp target/release/whisper-tts "${HOME}/.local/bin/whisper-tts"
chmod +x "${HOME}/.local/bin/whisper-tts"
echo "  → ${HOME}/.local/bin/whisper-tts"

# ── Instalar Stop hook de Claude Code ────────────────────────────────────────
echo "🔗 Instalando Claude Code Stop hook..."
mkdir -p "${HOME}/.claude/hooks"
cp hooks/whisper-tts-stop.sh "${HOME}/.claude/hooks/whisper-tts-stop.sh"
chmod +x "${HOME}/.claude/hooks/whisper-tts-stop.sh"
echo "  → ${HOME}/.claude/hooks/whisper-tts-stop.sh"

# Registrar el hook en ~/.claude/settings.json si no está ya
register_stop_hook "${HOME}/.claude/settings.json" \
    "${HOME}/.claude/hooks/whisper-tts-stop.sh" '{"async": true}'

# ── Instalar Stop hook de Codex CLI ──────────────────────────────────────────
echo "🔗 Instalando Codex CLI Stop hook..."
mkdir -p "${HOME}/.codex/hooks"
cp hooks/whisper-tts-codex-stop.sh "${HOME}/.codex/hooks/whisper-tts-stop.sh"
chmod +x "${HOME}/.codex/hooks/whisper-tts-stop.sh"
echo "  → ${HOME}/.codex/hooks/whisper-tts-stop.sh"

register_stop_hook "${HOME}/.codex/hooks.json" \
    "${HOME}/.codex/hooks/whisper-tts-stop.sh" '{"timeout": 30}'

echo ""
echo "✅ Instalado en: ${APP_DIR}"
echo ""
echo "  Lanzar:    open /Applications/${APP_NAME}.app"
echo "  Relanzar:  pkill ${APP_NAME}; open /Applications/${APP_NAME}.app"
echo "  Log:       tail -f ${LOG_PATH}"
echo ""
echo "⚠️  Cada rebuild revoca Accessibility en macOS."
echo "   → System Settings → Privacy & Security → Accessibility"
echo "   → Desactiva y reactiva el toggle de ${APP_NAME}"
