//! Fetching for apps in the chat. The page helper has no network; when a page asks for a picture, map tile or JSON
//! from an allowed site (`lh_pageview::ALLOWED_HOSTS`), the app fetches it here and hands the bytes back. Only
//! https, only those sites (redirects included), no cookies, a size cap, and only pictures and JSON come back.

use std::time::Duration;

use lh_pageview::{fetch_allowed, fetch_type_allowed, FETCH_MAX_BYTES};

/// What came back: status (0 when nothing usable did), content type and body.
pub type Fetched = (u16, String, Vec<u8>);

/// Fetch `url` for a page. Blocks the calling thread; call it off the UI thread.
pub fn fetch(url: &str) -> Fetched {
    let none = (0, String::new(), vec![]);
    if !fetch_allowed(url) {
        return none;
    }
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return none };
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent(concat!("LastHarness/", env!("CARGO_PKG_VERSION"), " (desktop app; small in-chat apps)"))
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::custom(|a| if a.previous().len() < 3 && fetch_allowed(a.url().as_str()) { a.follow() } else { a.stop() }))
            .build();
        let Ok(client) = client else { return none };
        let Ok(mut resp) = client.get(url).send().await else { return none };
        let ctype = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        if !resp.status().is_success() || !fetch_type_allowed(&ctype) || resp.content_length().is_some_and(|n| n as usize > FETCH_MAX_BYTES) {
            return none;
        }
        let mut body = vec![];
        while let Ok(Some(chunk)) = resp.chunk().await {
            if body.len() + chunk.len() > FETCH_MAX_BYTES {
                return none;
            }
            body.extend_from_slice(&chunk);
        }
        (resp.status().as_u16(), ctype, body)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_site_off_the_list_is_never_asked() {
        assert_eq!(super::fetch("https://example.com/x.png").0, 0);
        assert_eq!(super::fetch("http://tile.openstreetmap.org/0/0/0.png").0, 0);
    }
}
