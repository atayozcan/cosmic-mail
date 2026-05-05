# tb-tray

A minimal, Wayland-native (StatusNotifierItem) IMAP mail notifier for Linux.

Originally built for [COSMIC](https://github.com/pop-os/cosmic-epoch) but works
on any DE that consumes the `org.kde.StatusNotifierItem` D-Bus protocol (KDE,
Sway with `waybar`, Hyprland, etc).

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
- Tray icon switches between `mail-read-symbolic` and `mail-unread-symbolic`
- Tray menu: Open mail client / Edit Config / Open Config Folder / Quit
- `--configure` interactive setup (writes `~/.config/tb-tray/config.toml`
  with mode 0600)
- ~2 MB binary, ~15 MB RSS at runtime

## Install

```sh
git clone https://github.com/atayozcan/tb-tray
cd tb-tray
cargo build --release
install -m 0755 target/release/tb-tray ~/.local/bin/tb-tray
```

## Configure

Either:

```sh
tb-tray --configure
```

…which prompts interactively for server / port / username / password / folder.

Or hand-write `~/.config/tb-tray/config.toml`:

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

Always `chmod 600` the config — passwords are stored plaintext. A future
version may move them to `secret-service` / libsecret.

## Run

```sh
tb-tray &
```

Or as an autostart entry at `~/.config/autostart/tb-tray.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=tb-tray
Exec=/home/USER/.local/bin/tb-tray
Icon=mail-unread-symbolic
StartupNotify=false
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Delay=8
```

## License

MIT — see `LICENSE`.
