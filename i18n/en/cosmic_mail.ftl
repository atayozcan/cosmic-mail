# Popover (panel-applet click target)
popup-no-accounts = No accounts yet — open Settings to add one.
popup-no-unread = No unread mail.
popup-total-unread = { $count } unread
popup-account-empty = { $account }: 0 unread
popup-account-unread = { $account }: { $count } unread
popup-account-error = { $account }: { $error }

# Footer
settings = Settings…

# Notifications
notify-summary = New mail
notify-body-one = 1 new message in { $account }
    ({ $total } unread total)
notify-body-many = { $delta } new messages in { $account }
    ({ $total } unread total)
notify-action-open = Open

# Settings window
settings-window-title = cosmic-mail Settings
settings-section-general = General
settings-mail-client = Mail-client launch command
settings-mail-client-placeholder = xdg-email
settings-interval = Poll interval (seconds)

# Account section
account-fallback-title = Account #{ $index }
account-remove = Remove
account-display-name = Display name
account-display-name-placeholder = Personal
account-server = JMAP server URL
account-server-placeholder = https://api.fastmail.com
account-username = Username
account-username-placeholder = you@example.com
account-password = Password
account-password-placeholder = ••••••
account-add = Add account

# Footer
settings-save = Save
settings-saving = Saving…
settings-saved = Saved.
settings-error = Error: { $error }
settings-error-settings = settings: { $error }
settings-error-accounts = accounts.toml: { $error }
settings-error-secrets = secret-service: { $error }
