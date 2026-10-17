//! Web and phone: pairing a browser or a phone with this computer, and the devices already paired.

use serde_json::Value;

use super::{s, stamp};

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub last_seen: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Waiting {
    pub name: String,
    /// The start of its key, so you can tell it is the device in your hand.
    pub key: String,
    pub secs: u64,
}

/// Paired devices that have not been removed.
pub fn devices(v: &Value) -> Vec<Device> {
    v["devices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["revoked"] != true)
        .map(|d| Device { id: s(d, "id"), name: s(d, "name"), last_seen: d["last_seen"].as_i64().unwrap_or(0) })
        .collect()
}

pub fn waiting(v: &Value) -> Vec<Waiting> {
    v["pending"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| Waiting { name: s(p, "name"), key: s(p, "key_fingerprint").chars().take(23).collect(), secs: p["waiting_secs"].as_u64().unwrap_or(0) })
        .collect()
}

pub fn fingerprint(v: &Value) -> Option<String> {
    v["fingerprint"].as_str().filter(|f| !f.is_empty()).map(String::from)
}

pub fn last_seen_words(at: i64) -> String {
    if at <= 0 {
        "Not used yet".into()
    } else {
        format!("Last used {} (UTC)", stamp(at))
    }
}

/// A PIN is typed as it is shown: digits, perhaps with a space or dash between groups.
pub fn clean_pin(raw: &str) -> Result<String, String> {
    let p: String = raw.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if p.len() < 4 || !p.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Type the PIN the other device shows.".into());
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn removed_devices_are_not_listed() {
        let v = json!({"devices": [{"id": "a", "name": "Phone", "last_seen": 0, "revoked": false}, {"id": "b", "name": "Old", "revoked": true}]});
        let d = devices(&v);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].name, "Phone");
        assert_eq!(last_seen_words(0), "Not used yet");
        assert!(last_seen_words(1_791_590_400).contains("2026-10-10"));
    }

    #[test]
    fn waiting_devices_show_a_short_key() {
        let v = json!({"pending": [{"name": "Laptop", "key_fingerprint": "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55", "waiting_secs": 12}], "fingerprint": "XX"});
        let w = waiting(&v);
        assert_eq!(w[0].key.len(), 23);
        assert_eq!(w[0].secs, 12);
        assert_eq!(fingerprint(&v).as_deref(), Some("XX"));
        assert_eq!(fingerprint(&json!({})), None);
    }

    #[test]
    fn a_pin_loses_its_spaces() {
        assert_eq!(clean_pin(" 123 456 ").unwrap(), "123456");
        assert_eq!(clean_pin("12-34-56").unwrap(), "123456");
        assert!(clean_pin("12").is_err());
        assert!(clean_pin("12 3!").is_err());
    }
}
