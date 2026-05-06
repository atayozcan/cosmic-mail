#!/usr/bin/env bash
# tb-tray installer — places the binary, icons, and desktop launchers
# under $XDG_DATA_HOME (or ~/.local/share). Per-user, no root required.
#
# Cleans up artifacts from previous installs (old colored icons, the
# obsolete tb-tray-settings binary, etc.) before installing the new ones.
#
# Usage:
#   ./install.sh             # build release + install
#   ./install.sh --uninstall # remove every file this script ever wrote

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
cd "$SCRIPT_DIR"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"
APPS_DIR="$DATA_DIR/applications"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"

# Files this installer (current or previous versions) has written.
# Used by both the install cleanup and --uninstall paths.
OWNED_FILES=(
    "$BIN_DIR/tb-tray"
    "$BIN_DIR/tb-tray-settings"
    "$ICON_DIR/tb-tray.svg"
    "$ICON_DIR/tb-tray-unread.svg"
    "$ICON_DIR/tb-tray-symbolic.svg"
    "$ICON_DIR/tb-tray-unread-symbolic.svg"
    "$APPS_DIR/tb-tray.desktop"
    "$APPS_DIR/tb-tray-settings.desktop"
)

clean_old_artifacts() {
    local removed=0
    for f in "${OWNED_FILES[@]}"; do
        if [[ -e "$f" ]]; then
            rm -f "$f" && removed=$((removed + 1))
        fi
    done
    if (( removed > 0 )); then
        echo "tb-tray: cleaned up $removed stale file(s) from previous install."
    fi
}

refresh_caches() {
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi
}

uninstall() {
    echo "tb-tray: uninstalling..."
    clean_old_artifacts
    rm -f "$AUTOSTART_DIR/tb-tray.desktop"
    refresh_caches
    echo "tb-tray: uninstalled."
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
    exit 0
fi

echo "tb-tray: building (cargo build --release)..."
cargo build --release

echo "tb-tray: cleaning previous install..."
clean_old_artifacts

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APPS_DIR"

install -m 0755 target/release/tb-tray "$BIN_DIR/tb-tray"
install -m 0644 resources/icons/tb-tray-symbolic.svg "$ICON_DIR/tb-tray-symbolic.svg"
install -m 0644 resources/icons/tb-tray-unread-symbolic.svg "$ICON_DIR/tb-tray-unread-symbolic.svg"

# Substitute the absolute binary path into Exec= so launchers don't
# depend on the desktop session's PATH (which may not include
# ~/.local/bin even when the user's shell does).
sed "s|@BIN@|$BIN_DIR/tb-tray|g" resources/tb-tray.desktop \
    > "$APPS_DIR/tb-tray.desktop"
sed "s|@BIN@|$BIN_DIR/tb-tray|g" resources/tb-tray-settings.desktop \
    > "$APPS_DIR/tb-tray-settings.desktop"
chmod 0644 "$APPS_DIR/tb-tray.desktop" "$APPS_DIR/tb-tray-settings.desktop"

# If the user previously enabled autostart, rewrite that entry too —
# it likely points at the old binary path or the obsolete colored icon.
if [[ -f "$AUTOSTART_DIR/tb-tray.desktop" ]]; then
    cat > "$AUTOSTART_DIR/tb-tray.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=tb-tray
GenericName=Mail Notifier
Comment=IMAP mail notifier (Wayland-native tray)
Exec=$BIN_DIR/tb-tray
Icon=tb-tray-symbolic
Terminal=false
Categories=Network;Email;
StartupNotify=false
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Delay=8
EOF
    chmod 0644 "$AUTOSTART_DIR/tb-tray.desktop"
    echo "tb-tray: refreshed existing autostart entry."
fi

refresh_caches

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "tb-tray: warning: $BIN_DIR is not in your shell PATH (launchers will still work)." ;;
esac

cat <<EOF
tb-tray: installed.

  Binary:   $BIN_DIR/tb-tray
  Icons:    $ICON_DIR/tb-tray{,-unread}-symbolic.svg
  Launcher: $APPS_DIR/tb-tray{,-settings}.desktop

Modes:
  tb-tray              run the tray daemon
  tb-tray --settings   open the libcosmic settings GUI
  tb-tray --configure  CLI prompt for first-run setup

Try it now:
  $BIN_DIR/tb-tray --settings
EOF
