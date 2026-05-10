#!/usr/bin/env bash
# Remove mail-tray (binary, icons, launcher, autostart entry).
# Also cleans up any leftover pre-rename `tb-tray` artifacts.
#
# Thin wrapper around install.sh's uninstall path so the file manifest
# stays in one place. Run this script to uninstall — no flags to remember.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
exec "$SCRIPT_DIR/install.sh" --uninstall
