# mail-tray

A minimal, Wayland-native (StatusNotifierItem) **JMAP** mail notifier
for Linux, with a libcosmic settings GUI.

Originally built for [COSMIC](https://github.com/pop-os/cosmic-epoch) but the
tray daemon works on any DE that consumes the `org.kde.StatusNotifierItem`
D-Bus protocol (KDE, Sway with `waybar`, Hyprland, etc).

It does **one** thing: poll JMAP for unread count, fire a desktop
notification when new mail arrives, show the unread total in the
tray. Click the tray icon → menu opens with Settings… and Quit; the
notification's "Open" action launches your configured mail client.

It does **not** touch X11.

> **Renamed in v0.7.** Earlier versions (0.1–0.6) shipped as
> `tb-tray` ("Thunderbird tray") because the original target was
> Thunderbird specifically. v0.7 dropped the Thunderbird-flavored
> defaults in favor of generic `xdg-email`, and the project was
> renamed to `mail-tray`. There's no automatic migration — re-add
> your account in the settings window.

Sibling project to
[`cosmic-caffeine`](https://github.com/atayozcan/cosmic-caffeine) and
[`cosmic-clip`](https://github.com/atayozcan/cosmic-clip); shares the
[`cosmic-tray-app`](https://github.com/atayozcan/cosmic-tray-app)
helper crate for paths, autostart, and the single-binary
`--settings`-re-exec pattern.

## Why JMAP

JMAP (RFC 8620) is the modern, HTTP-based replacement for IMAP — far
nicer to write a client against (no stateful connection, no FETCH/STORE
quirks, native push via Server-Sent Events) and dramatically less code
to maintain. The trade-off is server support: not everyone runs it.
Servers known to speak JMAP:

- [Stalwart Mail Server](https://stalw.art/) (self-hostable;
  recommended for personal mail)
- [Fastmail](https://www.fastmail.com/)
- [Topicbox](https://www.topicbox.com/)

If your provider only speaks IMAP, mail-tray ≥ 0.5 won't work. Stick
with `tb-tray` 0.4 (the last IMAP release): clone the old name and
`git checkout v0.4.0`.

## Features

- StatusNotifierItem tray icon (Wayland-native, no X11 deps)
- Multi-account JMAP polling (Inbox discovered automatically by role)
- Desktop notification on unread-count delta (no spam on first poll)
- Symbolic monochrome tray icon (recolored by your COSMIC / GTK theme)
  with read / unread variants
- **Left-click opens the menu** (Settings… / Quit) — via SNI
  ItemIsMenu, no double-click needed
- libcosmic settings GUI for accounts, mail client, poll interval,
  autostart toggle — reached from the tray icon's menu, or shown
  automatically on first run
- Autostart toggle writes/removes `~/.config/autostart/mail-tray.desktop`
- Single binary, single launcher

## Install

```sh
git clone https://github.com/atayozcan/mail-tray
git clone https://github.com/atayozcan/cosmic-tray-app  # sibling lib (path dep)
cd mail-tray
./install.sh
```

That installs:

| Path | What |
| --- | --- |
| `~/.local/bin/mail-tray` | the binary |
| `~/.local/share/icons/hicolor/scalable/apps/mail-tray{,-unread}-symbolic.svg` | tray + app icon |
| `~/.local/share/applications/mail-tray.desktop` | app-menu launcher |

Per-user, no root needed. The launcher's `Exec=` is templated with
the absolute binary path at install time so it keeps working even
when your desktop session's PATH doesn't include `~/.local/bin`. The
script cleans up artifacts from earlier installs (including any
pre-rename `tb-tray` files) before laying down the new ones, and
(re)starts the daemon so the new version is live immediately.

To uninstall:

```sh
./uninstall.sh
```

### Build deps (Arch)

`pkexec pacman -S --needed rust pkgconf libxkbcommon wayland mesa vulkan-icd-loader fontconfig freetype2`

### Manual build

```sh
cargo build --release
# binary lands at target/release/mail-tray
```

## Configure

Run `mail-tray` (or click the launcher). On first run, with no
accounts file, the settings window opens automatically — add an
account, set the mail-client launch command (default `xdg-email`),
toggle autostart, hit *Save*. Subsequent runs start the tray daemon.

The tray icon's left-click opens the menu; pick *Settings…* to come
back to the GUI.

## Storage

Configuration is split across three backends so plaintext mail
passwords never touch a world-readable file.

### Non-secret settings → `cosmic_config`

`~/.config/cosmic/io.github.atayozcan.MailTray/v1/<field>` — one
RON-encoded file per field.

| Field | Type | Default |
| --- | --- | --- |
| `mail_client` | string | `xdg-email` |
| `interval_secs` | u64 | `60` |

Changes from the settings GUI propagate to the running daemon
without a restart — the daemon re-reads these on each tick.

### Account metadata → `~/.config/mail-tray/accounts.toml` at mode 0600

```toml
# mail-tray accounts — passwords live in the freedesktop
# Secret Service (gnome-keyring, kwallet, KeePassXC, …).
# This file is mode 0600 because the username/server pair
# is the lookup key into that store.

[[account]]
name     = "Personal"
server   = "https://mail.example.com"   # or just "mail.example.com"
username = "you"

# add more [[account]] blocks as needed
```

### Account passwords → freedesktop Secret Service

Passwords live in your desktop's keyring (gnome-keyring, kwallet,
KeePassXC, anything that implements `org.freedesktop.secrets`).
Items are tagged with `application = "mail-tray"`, `username = <…>`,
`server = <…>`. Inspect with `secret-tool`:

```sh
secret-tool search application mail-tray
```

The settings GUI reads/writes through this same API, and the daemon
fetches the password fresh on each JMAP reconnect — so a password
rotation from any keyring tool propagates to mail-tray on the next
reconnect, no daemon restart needed.

Stalwart accepts the same username/password pair you'd use for IMAP
login; mail-tray sends them as HTTP Basic auth.

**Heads up:** if your system has no Secret Service daemon running,
mail-tray can't log in. On Arch: `pkexec pacman -S --needed
gnome-keyring` (or kwallet, or use KeePassXC's secret-service
provider).

## Older versions

- v0.4 was the last IMAP release. If your server doesn't speak JMAP,
  stay on it: `git checkout v0.4.0 && ./install.sh`.
- v0.6-and-earlier was named `tb-tray`. v0.7 renamed the binary,
  package, APP_ID, and on-disk paths. **There's no automatic
  migration**; re-add your account in settings. To purge the old
  keyring entries: `secret-tool clear application tb-tray`. The old
  `~/.config/tb-tray/` and the old `~/.config/cosmic/io.github.atayozcan.TbTray/`
  directories can be removed manually.

## License

MIT — see `LICENSE`.
