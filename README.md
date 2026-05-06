# tb-tray

A minimal, Wayland-native (StatusNotifierItem) IMAP mail notifier for Linux,
with a libcosmic settings GUI.

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
- libcosmic settings GUI for accounts, mail client, poll interval,
  autostart toggle — reached from the tray icon's menu, or shown
  automatically on first run
- Autostart toggle writes/removes `~/.config/autostart/tb-tray.desktop`
- Single binary, single launcher

## Install

```sh
git clone https://github.com/atayozcan/tb-tray
cd tb-tray
./install.sh
```

That installs:

| Path | What |
| --- | --- |
| `~/.local/bin/tb-tray` | the binary |
| `~/.local/share/icons/hicolor/scalable/apps/tb-tray{,-unread}-symbolic.svg` | tray + app icon |
| `~/.local/share/applications/tb-tray.desktop` | app-menu launcher |

Per-user, no root needed. The launcher's `Exec=` is templated with the
absolute binary path at install time so it keeps working even when your
desktop session's PATH doesn't include `~/.local/bin`. The script cleans
up artifacts from earlier installs (older colored icons, the obsolete
second binary, etc.) before laying down the new files, and (re)starts
the daemon so the new version is live immediately.

To uninstall:

```sh
./uninstall.sh
```

### Build deps (Arch)

`pkexec pacman -S --needed rust pkgconf libxkbcommon wayland mesa vulkan-icd-loader fontconfig freetype2`

### Manual build

```sh
cargo build --release
# binary lands at target/release/tb-tray
```

## Configure

Run `tb-tray` (or click the launcher). On first run, with no config, the
settings window opens automatically — add an account, set the mail
client, toggle autostart, hit *Save*. Subsequent runs start the tray
daemon. Reach settings later from the tray icon's *Settings…* menu.

If you'd rather hand-edit `~/.config/tb-tray/config.toml`:

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

The settings GUI writes the file with mode `0600` — passwords are stored
plaintext. A future version may move them to `secret-service` / libsecret.

## License

MIT — see `LICENSE`.
