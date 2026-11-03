/// Per-provider quirks. Auto-detected from the base URL, overridable by the caller.
/// Values come from spike 1 (llama-swap) and from how Pi, Codex and OpenCode handle other servers.
#[derive(Clone, Debug, PartialEq)]
pub struct Compat {
    /// Name of the token-limit field: `max_tokens` (most servers) or `max_completion_tokens` (OpenAI).
    pub max_tokens_field: &'static str,
    /// Ask for a usage chunk at the end of a stream.
    pub stream_usage: bool,
    /// Field names that may carry thinking text in a delta or message, in order of preference.
    pub reasoning_fields: &'static [&'static str],
    /// The server accepts `top_k`, `min_p` and `repeat_penalty` (llama.cpp and most local servers do;
    /// the OpenAI API rejects them).
    pub extended_sampling: bool,
}

impl Default for Compat {
    fn default() -> Self {
        Self {
            max_tokens_field: "max_tokens",
            stream_usage: true,
            reasoning_fields: &["reasoning_content", "reasoning", "reasoning_text"],
            extended_sampling: true,
        }
    }
}

impl Compat {
    pub fn detect(base_url: &str) -> Self {
        let mut c = Self::default();
        if base_url.contains("api.openai.com") {
            c.max_tokens_field = "max_completion_tokens";
            c.extended_sampling = false;
        }
        c
    }
}

/// Does this base URL point at this machine or a private network? Such endpoints cost no money
/// unless a price says otherwise. Names other than `localhost` are treated as remote.
pub fn is_local_url(url: &str) -> bool {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(a)) => a.is_loopback() || a.is_private() || a.is_link_local(),
        Ok(std::net::IpAddr::V6(a)) => a.is_loopback() || (a.segments()[0] & 0xfe00) == 0xfc00,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_and_remote_endpoints() {
        for u in ["http://localhost:9292/v1", "http://127.0.0.1:1234/v1", "http://[::1]:8080/v1", "http://192.168.1.20:11434/v1", "http://10.0.0.5/v1", "http://user:pw@localhost/v1"] {
            assert!(is_local_url(u), "{u}");
        }
        for u in ["https://api.openai.com/v1", "https://openrouter.ai/api/v1", "http://8.8.8.8/v1", "http://localhost.evil.com/v1", "http://172.32.0.1/v1"] {
            assert!(!is_local_url(u), "{u}");
        }
    }
}
