//! Keyring integration (secrets storage).

use thiserror::Error;

const SERVICE: &str = "io.xenvmanager.em";
const ATTR_SERVICE: &str = "em_service";
const ATTR_KEY: &str = "em_key";

#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("keyring error")]
    Backend,
}

pub fn init_default_credential_builder() {}

pub fn is_available() -> bool {
    connect_secret_service().is_ok()
}

pub fn set_secret(key: &str, value: &str) -> Result<(), KeyringError> {
    use std::collections::HashMap;

    let ss = connect_secret_service()?;
    let collection = ss
        .get_default_collection()
        .map_err(|_| KeyringError::Backend)?;

    let attributes: HashMap<&str, &str> = HashMap::from([(ATTR_SERVICE, SERVICE), (ATTR_KEY, key)]);
    collection
        .create_item(key, attributes, value.as_bytes(), true, "text/plain")
        .map_err(|_| KeyringError::Backend)?;
    Ok(())
}

pub fn get_secret(key: &str) -> Result<Option<String>, KeyringError> {
    use std::collections::HashMap;

    let ss = connect_secret_service()?;
    let search = ss
        .search_items(HashMap::from([(ATTR_SERVICE, SERVICE), (ATTR_KEY, key)]))
        .map_err(|_| KeyringError::Backend)?;

    if let Some(item) = search.unlocked.first() {
        return decode_secret_to_string(item.get_secret().map_err(|_| KeyringError::Backend)?);
    }

    if let Some(item) = search.locked.first() {
        item.ensure_unlocked().map_err(|_| KeyringError::Backend)?;
        return decode_secret_to_string(item.get_secret().map_err(|_| KeyringError::Backend)?);
    }

    Ok(None)
}

pub fn delete_secret(key: &str) -> Result<(), KeyringError> {
    use dbus_secret_service::Error;
    use std::collections::HashMap;

    let ss = connect_secret_service()?;
    let search = ss
        .search_items(HashMap::from([(ATTR_SERVICE, SERVICE), (ATTR_KEY, key)]))
        .map_err(|_| KeyringError::Backend)?;

    for item in search.unlocked.iter().chain(search.locked.iter()) {
        let _ = item.ensure_unlocked();
        match item.delete() {
            Ok(()) => {}
            Err(Error::NoResult) => {}
            Err(_) => return Err(KeyringError::Backend),
        }
    }

    Ok(())
}

fn decode_secret_to_string(bytes: Vec<u8>) -> Result<Option<String>, KeyringError> {
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| KeyringError::Backend)
}

fn connect_secret_service() -> Result<dbus_secret_service::SecretService, KeyringError> {
    use dbus_secret_service::EncryptionType;

    let encryption = EncryptionType::Dh;
    let timeout = keyring_prompt_timeout_seconds();
    match timeout {
        Some(secs) => {
            dbus_secret_service::SecretService::connect_with_max_prompt_timeout(encryption, secs)
                .map_err(|_| KeyringError::Backend)
        }
        None => dbus_secret_service::SecretService::connect(encryption)
            .map_err(|_| KeyringError::Backend),
    }
}

fn keyring_prompt_timeout_seconds() -> Option<u64> {
    if std::env::var_os("CI").is_some() {
        return Some(0);
    }

    let v = std::env::var("EM_KEYRING_PROMPT_TIMEOUT_SECS").ok()?;
    let secs: u64 = v.parse().ok()?;
    Some(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_surface_compiles() {
        let _set: fn(&str, &str) -> Result<(), KeyringError> = set_secret;
        let _get: fn(&str) -> Result<Option<String>, KeyringError> = get_secret;
        let _del: fn(&str) -> Result<(), KeyringError> = delete_secret;
    }
}
