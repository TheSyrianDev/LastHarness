//! What the engine is allowed to ask for: nothing from outside. Every request goes through [`Deny`], which counts
//! it, refuses it and answers with an empty body so the page does not wait for it. `data:` addresses (pictures and
//! styles inside the page itself) are decoded in memory and served; they never leave the page.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use blitz_traits::navigation::{NavigationOptions, NavigationProvider};
use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request};

/// The longest `data:` payload that is decoded (bigger ones are refused like any other request).
pub const MAX_DATA_BYTES: usize = 8 * 1024 * 1024;

/// A running count of what the page asked for, kept so it can be shown and tested.
#[derive(Default, Debug)]
pub struct NetStats {
    refused: AtomicUsize,
    data_served: AtomicUsize,
    last_refused: Mutex<Vec<String>>,
}

impl NetStats {
    /// Requests that were refused (every http, https, file and other address).
    pub fn refused(&self) -> usize {
        self.refused.load(Ordering::SeqCst)
    }
    /// `data:` addresses that were decoded from inside the page.
    pub fn data_served(&self) -> usize {
        self.data_served.load(Ordering::SeqCst)
    }
    /// The first few refused addresses, for the activity view and for tests.
    pub fn refused_urls(&self) -> Vec<String> {
        self.last_refused.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

pub struct Deny {
    stats: Arc<NetStats>,
}

impl Deny {
    pub fn new(stats: Arc<NetStats>) -> Deny {
        Deny { stats }
    }
}

impl NetProvider for Deny {
    fn fetch(&self, _doc_id: usize, request: Request, handler: Box<dyn NetHandler>) {
        let url = request.url.to_string();
        if request.url.scheme() == "data" {
            if let Some(bytes) = decode_data_url(&url) {
                self.stats.data_served.fetch_add(1, Ordering::SeqCst);
                handler.bytes(url, Bytes::from(bytes));
                return;
            }
        }
        self.stats.refused.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut v) = self.stats.last_refused.lock() {
            if v.len() < 50 {
                v.push(url.chars().take(200).collect());
            }
        }
        // empty, not silence: a page waits for a stylesheet that never comes, and draws nothing, if nobody answers
        handler.bytes(url, Bytes::new());
    }
}

/// A click on a link goes nowhere by itself: the viewer asks the page which address is under the pointer
/// (`PageView::link_at`) and the app decides. This only makes sure the engine has no other path to navigate.
pub struct NoNavigation;

impl NavigationProvider for NoNavigation {
    fn navigate_to(&self, _options: NavigationOptions) {}
}

/// The bytes of a `data:` address, or `None` if it is malformed or too big.
pub fn decode_data_url(url: &str) -> Option<Vec<u8>> {
    let rest = url.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    if payload.len() > MAX_DATA_BYTES * 2 {
        return None;
    }
    let bytes = if meta.trim_end().ends_with(";base64") {
        let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
        let clean = percent_encoding::percent_decode_str(&clean).decode_utf8().ok()?.to_string();
        base64::engine::general_purpose::STANDARD.decode(clean.trim_end_matches('=')).or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(clean.trim_end_matches('='))).ok()?
    } else {
        percent_encoding::percent_decode_str(payload).collect()
    };
    (bytes.len() <= MAX_DATA_BYTES).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_addresses_decode_in_both_forms_and_bad_ones_do_not() {
        assert_eq!(decode_data_url("data:text/plain;base64,aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(decode_data_url("data:text/plain;base64,aGVsbG8"), Some(b"hello".to_vec()), "missing padding is fine");
        assert_eq!(decode_data_url("data:text/plain;base64,aGVs bG8="), Some(b"hello".to_vec()), "whitespace is ignored");
        assert_eq!(decode_data_url("data:text/css,a%7Bcolor%3Ared%7D"), Some(b"a{color:red}".to_vec()));
        assert_eq!(decode_data_url("data:,plain"), Some(b"plain".to_vec()));
        for bad in ["data:", "data:text/plain", "http://x/", "data:text/plain;base64,!!!", "file:///etc/passwd"] {
            assert_eq!(decode_data_url(bad), None, "{bad}");
        }
        let huge = format!("data:text/plain,{}", "a".repeat(MAX_DATA_BYTES + 1));
        assert_eq!(decode_data_url(&huge), None, "over the cap");
    }
}
