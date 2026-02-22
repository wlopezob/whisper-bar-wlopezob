#!/usr/bin/env bash
# bundle.sh — Compila e instala whisperwlopezob.app en /Applications/
set -e

APP_NAME="whisperwlopezob"
BUNDLE_ID="com.wlopezob.whisperwlopezob"
BINARY="target/release/whisper-bar-rust"
APP_DIR="/Applications/${APP_NAME}.app"
LOG_PATH="${HOME}/.config/${APP_NAME}/${APP_NAME}.log"

echo "🔨 Compilando..."
cargo build --release

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
    <string>0.1.0</string>

    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>

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
