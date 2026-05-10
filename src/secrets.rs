//! Account-password storage via the freedesktop Secret Service
//! (gnome-keyring, kwallet, KeePassXC, etc.) over D-Bus.
//!
//! Items are tagged with `application = "cosmic-mail"` so an
//! attribute-search can find them without enumerating the whole
//! keyring.
//!
//! All operations are async (zbus-driven).

use secret_service::{EncryptionType, SecretService};
use std::collections::HashMap;

const APP_TAG: &str = "cosmic-mail";

fn attrs<'a>(username: &'a str, server: &'a str) -> HashMap<&'static str, &'a str> {
    HashMap::from([
        ("application", APP_TAG),
        ("username", username),
        ("server", server),
    ])
}

/// Store (or replace) a password for `(username, server)`. Always
/// targets the user's default collection.
pub async fn store(username: &str, server: &str, password: &str) -> Result<(), String> {
    let ss = SecretService::connect(EncryptionType::Dh)
        .await
        .map_err(|e| format!("secret-service connect: {e}"))?;
    let collection = ss
        .get_default_collection()
        .await
        .map_err(|e| format!("secret-service collection: {e}"))?;
    let label = format!("cosmic-mail: {username}@{server}");
    collection
        .create_item(
            &label,
            attrs(username, server),
            password.as_bytes(),
            true, // replace if an item with the same attrs exists
            "text/plain",
        )
        .await
        .map_err(|e| format!("secret-service create_item: {e}"))?;
    Ok(())
}

/// Fetch the password for `(username, server)`. Returns Err with a
/// descriptive message if no entry exists or the daemon isn't running.
pub async fn fetch(username: &str, server: &str) -> Result<String, String> {
    let ss = SecretService::connect(EncryptionType::Dh)
        .await
        .map_err(|e| format!("secret-service connect: {e}"))?;
    let results = ss
        .search_items(attrs(username, server))
        .await
        .map_err(|e| format!("secret-service search: {e}"))?;
    let item = results
        .unlocked
        .first()
        .or_else(|| results.locked.first())
        .ok_or_else(|| {
            "secret-service entry not found; re-save the password in settings".to_string()
        })?;
    let bytes = item
        .get_secret()
        .await
        .map_err(|e| format!("secret-service get_secret: {e}"))?;
    String::from_utf8(bytes).map_err(|e| format!("secret-service utf8: {e}"))
}

/// Remove the entry for `(username, server)`, if present. No-op if
/// no entry matches.
pub async fn delete(username: &str, server: &str) -> Result<(), String> {
    let ss = SecretService::connect(EncryptionType::Dh)
        .await
        .map_err(|e| format!("secret-service connect: {e}"))?;
    let results = ss
        .search_items(attrs(username, server))
        .await
        .map_err(|e| format!("secret-service search: {e}"))?;
    if let Some(item) = results
        .unlocked
        .first()
        .or_else(|| results.locked.first())
    {
        item.delete()
            .await
            .map_err(|e| format!("secret-service delete: {e}"))?;
    }
    Ok(())
}

