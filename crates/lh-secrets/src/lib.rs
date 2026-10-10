//! Secret storage for API keys and tokens. Secrets live in the OS secret store (Secret Service on
//! Linux, Keychain on macOS, Credential Manager on Windows). Config files hold only a reference
//! such as `secret:openrouter`, never the value. If no secret store is available the call fails
//! with a clear message; nothing is ever written to a plain file.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no secret store is available ({0}). On Linux, start a Secret Service such as gnome-keyring and unlock it.")]
    Unavailable(String),
    #[error("the secret store refused or failed: {0}")]
    Store(String),
}

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<String>, Error>;
    fn set(&self, name: &str, value: &str) -> Result<(), Error>;
    /// Deleting a secret that does not exist is not an error.
    fn delete(&self, name: &str) -> Result<(), Error>;
}

/// The OS secret store.
pub struct OsStore {
    service: String,
}

impl OsStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self { service: service.into() }
    }

    fn entry(&self, name: &str) -> Result<keyring::Entry, Error> {
        keyring::Entry::new(&self.service, name).map_err(map)
    }
}

fn map(e: keyring::Error) -> Error {
    match e {
        keyring::Error::NoStorageAccess(x) | keyring::Error::PlatformFailure(x) => Error::Unavailable(x.to_string()),
        other => Error::Store(other.to_string()),
    }
}

impl SecretStore for OsStore {
    fn get(&self, name: &str) -> Result<Option<String>, Error> {
        match self.entry(name)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(map(e)),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), Error> {
        self.entry(name)?.set_password(value).map_err(map)
    }

    fn delete(&self, name: &str) -> Result<(), Error> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(map(e)),
        }
    }
}

/// In-memory store for tests and for incognito use. Nothing reaches disk.
#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, name: &str) -> Result<Option<String>, Error> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }
    fn set(&self, name: &str, value: &str) -> Result<(), Error> {
        self.0.lock().unwrap().insert(name.into(), value.into());
        Ok(())
    }
    fn delete(&self, name: &str) -> Result<(), Error> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

/// Turn a config value into the real secret. `secret:NAME` is looked up in the store; anything else
/// is rejected, so a key pasted into a config file cannot be used by accident.
pub fn resolve(store: &dyn SecretStore, reference: &str) -> Result<Option<String>, Error> {
    match reference.strip_prefix("secret:") {
        Some(name) => store.get(name),
        None => Err(Error::Store("keys must be stored as a reference like `secret:NAME`, not as plain text".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_roundtrip_and_resolve() {
        let s = MemoryStore::default();
        assert_eq!(s.get("k").unwrap(), None);
        s.set("k", "sk-123").unwrap();
        assert_eq!(resolve(&s, "secret:k").unwrap().as_deref(), Some("sk-123"));
        assert!(resolve(&s, "sk-123").is_err());
        s.delete("k").unwrap();
        s.delete("k").unwrap();
        assert_eq!(s.get("k").unwrap(), None);
    }

    /// Talks to the real OS store with a throwaway entry. Skipped (not failed) when no store runs.
    #[test]
    fn os_store_roundtrip() {
        let s = OsStore::new("lastharness-test");
        let name = format!("probe-{}", std::process::id());
        match s.set(&name, "value-1") {
            Err(Error::Unavailable(m)) => return eprintln!("skipped: {m}"),
            r => r.unwrap(),
        }
        assert_eq!(s.get(&name).unwrap().as_deref(), Some("value-1"));
        s.set(&name, "value-2").unwrap();
        assert_eq!(s.get(&name).unwrap().as_deref(), Some("value-2"));
        s.delete(&name).unwrap();
        assert_eq!(s.get(&name).unwrap(), None);
    }
}
