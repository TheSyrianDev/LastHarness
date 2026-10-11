//! Web fetch and web search. Both leave the machine, so both ask first (see `perm.rs`).
//!
//! Fetching is guarded against reaching things the user did not mean to expose: the host is
//! resolved first and refused if any address is on this machine or a private network, the connection
//! then uses exactly those addresses (no second lookup an attacker could change), and every redirect
//! is checked again. Search uses a SearXNG instance the user configured.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use serde_json::Value;

use crate::ToolError;

#[derive(Clone, Debug, Default)]
pub struct WebConfig {
    /// Base URL of a SearXNG instance, for `web_search`. `None` means search is not set up.
    pub searxng_url: Option<String>,
    /// Allow fetching local and private addresses. Off by default; tests turn it on.
    pub allow_private: bool,
    /// Use the proxy named in the environment. Off by default: a proxy looks the name up itself, so the
    /// private-address check could not see where it really goes.
    pub use_proxy: bool,
}

const MAX_BODY: usize = 1024 * 1024;
const MAX_TEXT: usize = 20_000;
const MAX_REDIRECTS: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(20);

fn fail(m: impl Into<String>) -> ToolError {
    ToolError::Failed(m.into())
}

pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => {
            let o = a.octets();
            !(a.is_loopback() || a.is_private() || a.is_link_local() || a.is_unspecified() || a.is_broadcast() || a.is_multicast() || a.is_documentation() || (o[0] == 100 && (64..128).contains(&o[1])) || o[0] == 0 || o[0] >= 240
                || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24 protocol assignments
                || (o[0] == 198 && (o[1] == 18 || o[1] == 19))) // benchmarking
        }
        IpAddr::V6(a) => {
            if let Some(v4) = a.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let s = a.segments();
            // addresses that carry an IPv4 address inside: NAT64 (64:ff9b::/96), 6to4 (2002::/16) and the old IPv4-compatible form
            let embedded = |hi: u16, lo: u16| std::net::Ipv4Addr::new((hi >> 8) as u8, hi as u8, (lo >> 8) as u8, lo as u8);
            if s[0] == 0x64 && s[1] == 0xff9b && s[2..6].iter().all(|x| *x == 0) {
                return is_public(IpAddr::V4(embedded(s[6], s[7])));
            }
            if s[0] == 0x2002 {
                return is_public(IpAddr::V4(embedded(s[1], s[2])));
            }
            if s[..6].iter().all(|x| *x == 0) {
                return false; // ::a.b.c.d, deprecated, and ::1 / :: themselves
            }
            !(a.is_loopback() || a.is_unspecified() || a.is_multicast() || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80 || (s[0] & 0xffc0) == 0xfec0 /* site-local */ || (s[0] == 0x2001 && s[1] == 0xdb8) /* documentation */ || (s[0] == 0x2001 && s[1] == 0) /* Teredo */)
        }
    }
}

/// What the agent sends out in an address or a search must not carry a key, a card or account number, or
/// an identity number. (Ordinary words about health or money are the user's own to ask about.)
fn outbound_ok(text: &str, words_too: bool) -> Result<(), ToolError> {
    let c = lh_memory::detect(text);
    // in an address a word like "passwd" is just part of a page name; only key shapes count there
    let credentials = if words_too { c.reason.starts_with("credentials") } else { c.reason.starts_with("credentials: looks like a key") };
    if c.privacy == lh_memory::Privacy::Sensitive && (credentials || c.reason.starts_with("finance: card") || c.reason.starts_with("identity document number")) {
        return Err(fail(format!("not sent: it looks like it holds a secret or an account number ({}). Nothing was sent.", c.reason)));
    }
    Ok(())
}

pub async fn fetch(cfg: &WebConfig, url: &str) -> Result<String, ToolError> {
    let mut url = reqwest::Url::parse(url).map_err(|e| fail(format!("not a valid address: {e}")))?;
    for _ in 0..=MAX_REDIRECTS {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(fail("only http and https addresses can be fetched"));
        }
        outbound_ok(url.as_str(), false)?;
        let host = url.host_str().ok_or_else(|| fail("the address has no host"))?.to_string();
        let port = url.port_or_known_default().unwrap_or(80);
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port)).await.map_err(|e| fail(format!("cannot find {host}: {e}")))?.collect();
        if addrs.is_empty() {
            return Err(fail(format!("cannot find {host}")));
        }
        if !cfg.allow_private && addrs.iter().any(|a| !is_public(a.ip())) {
            return Err(fail(format!("{host} is on this machine or a private network. Fetching such addresses is blocked.")));
        }
        let mut builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(TIMEOUT);
        if !cfg.use_proxy {
            builder = builder.no_proxy();
        }
        let client = builder
            .resolve_to_addrs(&host, &addrs)
            .user_agent("LastHarness/0 (agent fetch)")
            .build()
            .map_err(|e| fail(e.to_string()))?;
        let mut resp = client.get(url.clone()).send().await.map_err(|e| fail(format!("request failed: {e}")))?;
        if resp.status().is_redirection() {
            let next = resp.headers().get(reqwest::header::LOCATION).and_then(|v| v.to_str().ok()).ok_or_else(|| fail("redirect without a location"))?;
            url = url.join(next).map_err(|e| fail(format!("bad redirect: {e}")))?;
            continue;
        }
        let status = resp.status();
        let ctype = resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
        let mut body = Vec::new();
        let mut cut = false;
        while let Some(chunk) = resp.chunk().await.map_err(|e| fail(format!("download failed: {e}")))? {
            body.extend_from_slice(&chunk);
            if body.len() > MAX_BODY {
                body.truncate(MAX_BODY);
                cut = true;
                break;
            }
        }
        let textual = ctype.is_empty() || ctype.starts_with("text/") || ctype.contains("json") || ctype.contains("xml") || ctype.contains("javascript");
        if !textual {
            return Ok(format!("HTTP {status}, {ctype}, {} bytes{}. This is not text, so it is not shown.", body.len(), if cut { " or more" } else { "" }));
        }
        let raw = String::from_utf8_lossy(&body);
        let mut text = if ctype.contains("html") { html_to_text(&raw) } else { raw.into_owned() };
        if text.chars().count() > MAX_TEXT {
            text = text.chars().take(MAX_TEXT).collect::<String>() + "\n[cut]";
        } else if cut {
            text.push_str("\n[download cut at 1 MB]");
        }
        return Ok(format!("HTTP {status} {ctype}\n\n{text}"));
    }
    Err(fail("too many redirects"))
}

pub async fn search(cfg: &WebConfig, query: &str, max: usize) -> Result<String, ToolError> {
    outbound_ok(query, true)?;
    let base = cfg.searxng_url.as_deref().ok_or_else(|| fail("web search is not set up. Set a SearXNG address in the settings."))?;
    let mut url = reqwest::Url::parse(&format!("{}/search", base.trim_end_matches('/'))).map_err(|e| fail(format!("the SearXNG address is not valid: {e}")))?;
    url.query_pairs_mut().append_pair("q", query).append_pair("format", "json");
    let v: Value = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| fail(e.to_string()))?
        .get(url)
        .send()
        .await
        .map_err(|e| fail(format!("the search server cannot be reached: {e}")))?
        .error_for_status()
        .map_err(|e| fail(format!("the search server refused the request: {e}")))?
        .json()
        .await
        .map_err(|_| fail("the search server did not answer in JSON. SearXNG needs `json` in its allowed formats."))?;
    let rows = v["results"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return Ok("No results.".into());
    }
    Ok(rows
        .iter()
        .take(max)
        .enumerate()
        .map(|(i, r)| format!("{}. {}\n   {}\n   {}", i + 1, r["title"].as_str().unwrap_or(""), r["url"].as_str().unwrap_or(""), r["content"].as_str().unwrap_or("").trim()))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Plain text from HTML: drops scripts, styles and tags, keeps paragraph breaks, decodes common
/// entities. Not a browser; good enough to read an article.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let lower = html.to_lowercase();
    let (mut i, bytes) = (0, html.as_bytes());
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // skip whole script / style / head blocks
            let skip = ["script", "style", "head", "noscript", "svg"].iter().find(|t| lower[i..].starts_with(&format!("<{t}")) && lower[i + 1 + t.len()..].starts_with(['>', ' ', '\n', '\t', '/']));
            if let Some(t) = skip {
                match lower[i..].find(&format!("</{t}")) {
                    Some(j) => i += lower[i + j..].find('>').map_or(j, |k| j + k) + 1,
                    None => break,
                }
                continue;
            }
            let end = html[i..].find('>').map_or(html.len(), |j| i + j + 1);
            let tag = lower[i + 1..end.saturating_sub(1).max(i + 1)].trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
            if matches!(tag, "p" | "div" | "br" | "li" | "tr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "section" | "article" | "ul" | "ol" | "table" | "pre" | "blockquote") {
                out.push('\n');
            }
            i = end;
        } else {
            let next = html[i..].find('<').map_or(html.len(), |j| i + j);
            out.push_str(&html[i..next]);
            i = next;
        }
    }
    let decoded = out.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&");
    // one line per block, no blank lines: fewer tokens for the model
    decoded.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Serves canned raw HTTP replies for any request.
    async fn serve(replies: Vec<String>) -> String {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        tokio::spawn(async move {
            let mut n = 0;
            loop {
                let (mut s, _) = l.accept().await.unwrap();
                let r = replies[n.min(replies.len() - 1)].clone();
                n += 1;
                tokio::spawn(async move {
                    let mut b = vec![0u8; 8192];
                    let _ = s.read(&mut b).await;
                    let _ = s.write_all(r.as_bytes()).await;
                    let _ = s.shutdown().await;
                });
            }
        });
        url
    }
    fn http(status: &str, ctype: &str, body: &str) -> String {
        format!("HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
    }
    fn open() -> WebConfig {
        WebConfig { allow_private: true, ..Default::default() }
    }

    #[test]
    fn public_and_private_addresses() {
        for a in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(is_public(a.parse().unwrap()), "{a}");
        }
        for a in ["127.0.0.1", "10.1.2.3", "192.168.0.9", "172.16.5.5", "169.254.169.254", "100.64.0.1", "0.0.0.0", "::1", "fe80::1", "fd00::1", "::ffff:127.0.0.1", "224.0.0.1"] {
            assert!(!is_public(a.parse().unwrap()), "{a}");
        }
    }

    #[test]
    fn tricky_addresses_are_not_public() {
        for a in ["64:ff9b::7f00:1", "64:ff9b::a00:1", "2002:7f00:1::", "2002:c0a8:1::1", "::127.0.0.1", "::10.0.0.1", "fec0::1", "2001:db8::1", "2001::1", "192.0.0.8", "198.18.0.1", "198.19.255.255", "::ffff:10.0.0.1", "::ffff:169.254.169.254", "255.255.255.255"] {
            assert!(!is_public(a.parse().unwrap()), "{a}");
        }
        for a in ["64:ff9b::808:808", "2002:0808:0808::1", "198.20.0.1"] {
            assert!(is_public(a.parse().unwrap()), "{a}");
        }
    }

    #[tokio::test]
    async fn odd_spellings_of_local_addresses_are_blocked() {
        // the address parser turns these into 127.0.0.1 or similar before anything is resolved
        for u in ["http://2130706433/", "http://0x7f.1/", "http://0177.0.0.1/", "http://127.1/", "http://[::1]/", "http://[::ffff:127.0.0.1]/", "http://[::ffff:7f00:1]/", "http://0/", "http://localhost./", "http://LOCALHOST/", "http://169.254.169.254/latest/meta-data/", "http://user:pw@127.0.0.1/", "http://127.0.0.1.nip.io@127.0.0.1/"] {
            let e = fetch(&WebConfig::default(), u).await.unwrap_err().to_string();
            assert!(e.contains("blocked") || e.contains("cannot find"), "{u}: {e}");
        }
    }

    #[tokio::test]
    async fn secrets_and_account_numbers_are_not_sent_in_addresses_or_searches() {
        for u in ["https://example.com/?key=sk-abcdefghijklmnopqrstuvwx", "https://example.com/?card=4111111111111111", "https://example.com/?iban=DE89370400440532013000"] {
            assert!(fetch(&WebConfig::default(), u).await.unwrap_err().to_string().contains("Nothing was sent"), "{u}");
        }
        let cfg = WebConfig { searxng_url: Some("http://127.0.0.1:9".into()), allow_private: true, use_proxy: false };
        assert!(search(&cfg, "my password is hunter2", 3).await.unwrap_err().to_string().contains("Nothing was sent"));
        // an ordinary question about health is the user's own to ask
        assert!(!search(&cfg, "diabetes symptoms", 3).await.unwrap_err().to_string().contains("Nothing was sent"));
    }

    #[test]
    fn html_becomes_readable_text() {
        let h = "<html><head><title>T</title><style>p{}</style></head><body><h1>Hello &amp; welcome</h1><script>evil()</script><p>First   para.</p><p>Second<br>line</p><ul><li>a</li><li>b</li></ul></body></html>";
        assert_eq!(html_to_text(h), "Hello & welcome\nFirst para.\nSecond\nline\na\nb");
    }

    #[tokio::test]
    async fn private_addresses_are_blocked_by_default() {
        let url = serve(vec![http("200 OK", "text/plain", "secret")]).await;
        let e = fetch(&WebConfig::default(), &url).await.unwrap_err().to_string();
        assert!(e.contains("blocked"), "{e}");
        assert!(fetch(&WebConfig::default(), "http://localhost:1/").await.unwrap_err().to_string().contains("blocked"));
        assert!(fetch(&WebConfig::default(), "file:///etc/passwd").await.unwrap_err().to_string().contains("only http"));
    }

    #[tokio::test]
    async fn fetches_text_follows_redirects_and_stops_on_bad_ones() {
        let url = serve(vec![http("200 OK", "text/html; charset=utf-8", "<p>Hi there</p>")]).await;
        let t = fetch(&open(), &url).await.unwrap();
        assert!(t.starts_with("HTTP 200 OK text/html") && t.ends_with("Hi there"), "{t}");

        let target = serve(vec![http("200 OK", "text/plain", "landed")]).await;
        let hop = serve(vec![format!("HTTP/1.1 302 Found\r\nLocation: {target}/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")]).await;
        assert!(fetch(&open(), &hop).await.unwrap().ends_with("landed"));

        let looping = serve(vec!["HTTP/1.1 302 Found\r\nLocation: /again\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()]).await;
        assert!(fetch(&open(), &looping).await.unwrap_err().to_string().contains("too many redirects"));
    }

    #[tokio::test]
    async fn binary_and_huge_bodies_are_not_dumped() {
        let bin = serve(vec![http("200 OK", "image/png", "\u{1}\u{2}\u{3}")]).await;
        assert!(fetch(&open(), &bin).await.unwrap().contains("not text"));
        let big = serve(vec![http("200 OK", "text/plain", &"a".repeat(2 * MAX_BODY))]).await;
        let t = fetch(&open(), &big).await.unwrap();
        assert!(t.chars().count() < MAX_TEXT + 100 && t.ends_with("[cut]"));
    }

    #[tokio::test]
    async fn search_uses_searxng_and_explains_problems() {
        let body = r#"{"results":[{"title":"Rust","url":"https://rust-lang.org","content":"A language."},{"title":"Two","url":"https://two.example","content":""}]}"#;
        let url = serve(vec![http("200 OK", "application/json", body)]).await;
        let cfg = WebConfig { searxng_url: Some(url.clone()), ..Default::default() };
        let r = search(&cfg, "rust", 5).await.unwrap();
        assert!(r.starts_with("1. Rust\n   https://rust-lang.org\n   A language.\n2. Two"), "{r}");
        assert_eq!(search(&cfg, "x", 1).await.unwrap().lines().count(), 3);
        assert!(search(&WebConfig::default(), "x", 1).await.unwrap_err().to_string().contains("not set up"));
        let html = serve(vec![http("200 OK", "text/html", "<html>no json</html>")]).await;
        assert!(search(&WebConfig { searxng_url: Some(html), ..Default::default() }, "x", 1).await.unwrap_err().to_string().contains("JSON"));
        let empty = serve(vec![http("200 OK", "application/json", r#"{"results":[]}"#)]).await;
        assert_eq!(search(&WebConfig { searxng_url: Some(empty), ..Default::default() }, "x", 1).await.unwrap(), "No results.");
    }
}
