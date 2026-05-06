# tb-tray

A minimal, Wayland-native (StatusNotifierItem) IMAP mail notifier for Linux,
plus a libcosmic settings GUI.

Originally built for [COSMIC](https://github.com/pop-os/cosmic-epoch) but the
tray daemon works on any DE that consumes the `org.kde.StatusNotifierItem`
D-Bus protocol (KDE, Sway with `waybar`, Hyprland, etc).

It does **one** thing: poll IMAP, fire a desktop notification when new mail
arrives, show the unread total in the tray. Click the tray icon → launch your
mail client.

It does **not**:

- Embed Thunderbird / control its window state
- Keep Thunderbird running in the background
- Talk to the Thunderbird API
- Touch X11

If you want a real Thunderbird tray-on-Linux experience, this isn't it — see
[birdtray](https://github.com/gyunaev/birdtray) (which assumes X11 and breaks
on Wayland in subtle ways, hence this project).

## Why this exists

Birdtray's "minimize on close" depends on X11 window manipulation that
Wayland refuses by design. Most Thunderbird tray add-ons targeted the legacy
XUL API and don't load in current ESR/release. The pragmatic alternative is to
sidestep Thunderbird entirely: poll IMAP yourself, show notifications yourself,
launch Thunderbird only when the user actually wants to read mail.

## Features

- StatusNotifierItem tray icon (Wayland-native, no X11 deps)
- Multi-account IMAP polling over TLS (`UID SEARCH UNSEEN`)
- Desktop notification on unread-count delta (no spam on first poll)
- Symbolic monochrome tray icon (recolored by your COSMIC / GTK theme)
  with read / unread variants
- Tray menu: Open Mail / Settings… / Quit
- **libcosmic settings GUI** — `tb-tray --settings` (also reachable from
  the tray menu and the app launcher) edits accounts, mail client, poll
  interval, autostart toggle without touching TOML
- **Autostart toggle** — writes/removes `~/.config/autostart/tb-tray.desktop`
- App-menu launcher with symbolic icon (installed to `~/.local/share`)
- Headless `--configure` CLI fallback for SSH / minimal setups
- Single binary; tray, settings GUI, and CLI all live in `tb-tray`

## Install

The provided `install.sh` builds and places everything under `~/.local`:

```sh
git clone https://github.com/atayozcan/tb-tray
cd tb-tray
./install.sh
```

That installs:

| Path | What |
| --- | --- |
| `~/.local/bin/tb-tray` | single binary (daemon + GUI + CLI) |
| `~/.local/share/icons/hicolor/scalable/apps/tb-tray*-symbolic.svg` | symbolic tray + app icons |
| `~/.local/share/applications/tb-tray*.desktop` | app-menu launchers (absolute Exec= paths) |

The launcher `.desktop` files are templated with the absolute binary path
at install time, so they keep working even when your desktop session's
PATH doesn't include `~/.local/bin`.

`./install.sh` cleans up any artifacts from earlier installs (older
colored icons, a separate `tb-tray-settings` binary, etc.) before
laying down the new files.

Run `./install.sh --uninstall` to remove every file the installer wrote
(including any autostart entry).

### Build deps (Arch)

`pkexec pacman -S --needed rust pkgconf libxkbcommon wayland mesa vulkan-icd-loader fontconfig freetype2`

### Manual build

```sh
cargo build --release
# binary lands at target/release/tb-tray
```

## Configure

**GUI (recommended):** launch *tb-tray Settings* from the app menu, or run
`tb-tray --settings`, or pick *Settings…* from the tray menu. Edit
accounts, toggle autostart, hit *Save*.

**CLI:** `tb-tray --configure` prompts interactively for one account.

**Hand-edit:** `~/.config/tb-tray/config.toml`:

```toml
mail_client    = "/usr/bin/thunderbird"
interval_secs  = 60

[[account]]
name     = "Personal"
server   = "mail.example.com"
port     = 993
username = "you@example.com"
password = "..."
folder   = "INBOX"

# add more [[account]] blocks as needed
```

Either tool writes the file with mode `0600` — passwords are stored
plaintext. A future version may move them to `secret-service` / libsecret.

## Run

```sh
tb-tray &
```

Or enable the *Start tb-tray on login* toggle in settings (writes
`~/.config/autostart/tb-tray.desktop`).

## License

MIT — see `LICENSE`.
