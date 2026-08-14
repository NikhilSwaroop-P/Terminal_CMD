#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "=== Building TermCMD Linux Desktop Distribution Bundle ==="

echo "Step 1: Building frontend assets..."
npm run build

echo "Step 2: Building release binary..."
cargo build --release --manifest-path src-tauri/Cargo.toml

RELEASE_BIN="target/release/termcmd"
CLI_BIN="target/release/termcmd-cli"
if [ ! -f "${RELEASE_BIN}" ]; then
    echo "✗ FAIL: Release binary ${RELEASE_BIN} not found."
    exit 1
fi

BIN_SIZE_BYTES=$(stat -c%s "${RELEASE_BIN}")
BIN_SIZE_MB=$(awk "BEGIN {printf \"%.2f\", ${BIN_SIZE_BYTES} / (1024.0 * 1024.0)}")
echo "Compiled Release Binary Size: ${BIN_SIZE_MB} MB"

PKG_DIR="dist-package"
rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}/bin"
mkdir -p "${PKG_DIR}/share/applications"
mkdir -p "${PKG_DIR}/share/icons/hicolor/32x32/apps"
mkdir -p "${PKG_DIR}/share/icons/hicolor/128x128/apps"
mkdir -p "${PKG_DIR}/share/icons/hicolor/256x256/apps"
mkdir -p "${PKG_DIR}/share/icons/hicolor/512x512/apps"

mkdir -p "${PKG_DIR}/skills/termcmd"
mkdir -p "${PKG_DIR}/share/skills/termcmd"

echo "Step 3: Staging binaries, icons, skills, and desktop shortcuts..."
cp "${RELEASE_BIN}" "${PKG_DIR}/bin/termcmd"
if [ -f "${CLI_BIN}" ]; then
    cp "${CLI_BIN}" "${PKG_DIR}/bin/termcmd-cli"
fi
cp "termcmd.desktop" "${PKG_DIR}/share/applications/termcmd.desktop"

if [ -f "skills/termcmd/SKILL.md" ]; then
    cp "skills/termcmd/SKILL.md" "${PKG_DIR}/skills/termcmd/SKILL.md"
    cp "skills/termcmd/SKILL.md" "${PKG_DIR}/share/skills/termcmd/SKILL.md"
fi

cp "src-tauri/icons/32x32.png" "${PKG_DIR}/share/icons/hicolor/32x32/apps/termcmd.png"
cp "src-tauri/icons/128x128.png" "${PKG_DIR}/share/icons/hicolor/128x128/apps/termcmd.png"
cp "src-tauri/icons/128x128@2x.png" "${PKG_DIR}/share/icons/hicolor/256x256/apps/termcmd.png"
cp "src-tauri/icons/icon.png" "${PKG_DIR}/share/icons/hicolor/512x512/apps/termcmd.png"

cat <<'EOF' > "${PKG_DIR}/install.sh"
#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${PREFIX:-${HOME}/.local}"
echo "Installing TermCMD to ${PREFIX}..."
mkdir -p "${PREFIX}/bin" "${PREFIX}/share/applications" "${PREFIX}/share/icons/hicolor/512x512/apps" "${PREFIX}/share/skills/termcmd"
install -m 755 "${DIR}/bin/termcmd" "${PREFIX}/bin/termcmd"
if [ -f "${DIR}/bin/termcmd-cli" ]; then
    install -m 755 "${DIR}/bin/termcmd-cli" "${PREFIX}/bin/termcmd-cli"
    ln -sf "${PREFIX}/bin/termcmd-cli" "${PREFIX}/bin/termcli"
fi
cp -r "${DIR}/share/"* "${PREFIX}/share/"
if [ -f "${DIR}/skills/termcmd/SKILL.md" ]; then
    mkdir -p "${HOME}/.gemini/config/skills/termcmd" 2>/dev/null || true
    cp "${DIR}/skills/termcmd/SKILL.md" "${HOME}/.gemini/config/skills/termcmd/SKILL.md" 2>/dev/null || true
fi
if which update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${PREFIX}/share/applications" 2>/dev/null || true
fi
if which gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache "${PREFIX}/share/icons/hicolor" 2>/dev/null || true
fi
echo "✓ Installation complete. TermCMD is installed in ${PREFIX}/bin and registered with your application launcher."
EOF
chmod +x "${PKG_DIR}/install.sh"

TARBALL="termcmd-v0.1.0-linux-x86_64.tar.gz"
tar -czf "${TARBALL}" -C "${PKG_DIR}" .

TAR_SIZE_BYTES=$(stat -c%s "${TARBALL}")
TAR_SIZE_MB=$(awk "BEGIN {printf \"%.2f\", ${TAR_SIZE_BYTES} / (1024.0 * 1024.0)}")

echo "--------------------------------------------------------"
echo "✓ Package successfully assembled in: ${PKG_DIR}/"
echo "✓ Standalone Archive: ${TARBALL} (${TAR_SIZE_MB} MB)"
echo "--------------------------------------------------------"
