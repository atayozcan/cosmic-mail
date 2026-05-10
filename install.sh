#!/usr/bin/env bash
# cosmic-mail installer — places the panel-applet binary, settings
# binary, and icons under $XDG_DATA_HOME (or ~/.local/share). Per-user,
# no root required.
#
# cosmic-mail is a cosmic-panel applet, NOT a daemon. The panel
# spawns the binary as needed; this installer only deposits files.
#
# Cleans up artifacts from previous installs (the pre-rename
# `tb-tray` and `mail-tray` SNI binaries, autostart entries, the old
# .desktop launcher, etc.) so the upgrade is hands-off.
#
# Usage:
#   ./install.sh             # build + install
#   ./install.sh --uninstall # remove everything this script wrote

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
cd "$SCRIPT_DIR"

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"
APPS_DIR="$DATA_DIR/applications"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"

OWNED_FILES=(
    "$BIN_DIR/cosmic-mail"
    "$BIN_DIR/cosmic-mail-settings"
    "$ICON_DIR/cosmic-mail-symbolic.svg"
    "$ICON_DIR/cosmic-mail-unread-symbolic.svg"
    "$APPS_DIR/io.github.atayozcan.CosmicMail.desktop"
    "$BIN_DIR/mail-tray"
    "$BIN_DIR/tb-tray"
    "$BIN_DIR/tb-tray-settings"
    "$ICON_DIR/mail-tray-symbolic.svg"
    "$ICON_DIR/mail-tray-unread-symbolic.svg"
    "$ICON_DIR/tb-tray-symbolic.svg"
    "$ICON_DIR/tb-tray-unread-symbolic.svg"
    "$APPS_DIR/mail-tray.desktop"
    "$APPS_DIR/tb-tray.desktop"
    "$APPS_DIR/tb-tray-settings.desktop"
    "$AUTOSTART_DIR/mail-tray.desktop"
    "$AUTOSTART_DIR/tb-tray.desktop"
)

clean_old_artifacts() {
    local removed=0
    for f in "${OWNED_FILES[@]}"; do
        if [[ -e "$f" ]]; then
            rm -f "$f" && removed=$((removed + 1))
        fi
    done
    if (( removed > 0 )); then
        echo "cosmic-mail: cleaned up $removed stale file(s)."
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

stop_old_processes() {
    for name in mail-tray tb-tray cosmic-mail; do
        if pgrep -x "$name" >/dev/null 2>&1; then
            pkill -x "$name" 2>/dev/null || true
        fi
    done
    sleep 0.2
}

uninstall() {
    echo "cosmic-mail: uninstalling..."
    stop_old_processes
    clean_old_artifacts
    refresh_caches
    echo "cosmic-mail: uninstalled. (Remove the applet from your panel via cosmic-settings.)"
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
    exit 0
fi

echo "cosmic-mail: building (cargo build --release)..."
cargo build --release

stop_old_processes

echo "cosmic-mail: cleaning previous install..."
clean_old_artifacts

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APPS_DIR"

install -m 0755 target/release/cosmic-mail "$BIN_DIR/cosmic-mail"
install -m 0755 target/release/cosmic-mail-settings "$BIN_DIR/cosmic-mail-settings"
install -m 0644 resources/icons/cosmic-mail-symbolic.svg "$ICON_DIR/cosmic-mail-symbolic.svg"
install -m 0644 resources/icons/cosmic-mail-unread-symbolic.svg "$ICON_DIR/cosmic-mail-unread-symbolic.svg"

# cosmic-panel discovers applets by their APP_ID-named .desktop file
# in $XDG_DATA_HOME/applications.
sed "s|@BIN@|$BIN_DIR/cosmic-mail|g" resources/cosmic-mail.desktop \
    > "$APPS_DIR/io.github.atayozcan.CosmicMail.desktop"
chmod 0644 "$APPS_DIR/io.github.atayozcan.CosmicMail.desktop"

refresh_caches

cat <<EOF
cosmic-mail: installed.

  Applet:   $BIN_DIR/cosmic-mail
  Settings: $BIN_DIR/cosmic-mail-settings
  Manifest: $APPS_DIR/io.github.atayozcan.CosmicMail.desktop

To attach to the panel:
  cosmic-settings → Panel → <Top|Bottom|Dock> → Add Applet → Mail

NOTE on rename: your old ~/.config/mail-tray/accounts.toml and the
keyring entries tagged 'application=mail-tray' (or 'tb-tray') are
NOT migrated. Re-add your account in the cosmic-mail settings
window. To purge old artifacts:

  rm -rf ~/.config/mail-tray ~/.config/tb-tray
  rm -rf ~/.config/cosmic/io.github.atayozcan.MailTray ~/.config/cosmic/io.github.atayozcan.TbTray
  secret-tool clear application mail-tray
  secret-tool clear application tb-tray

To uninstall: ./install.sh --uninstall
EOF
