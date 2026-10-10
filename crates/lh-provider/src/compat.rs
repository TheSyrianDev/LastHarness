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
}

impl Default for Compat {
    fn default() -> Self {
        Self {
            max_tokens_field: "max_tokens",
            stream_usage: true,
            reasoning_fields: &["reasoning_content", "reasoning", "reasoning_text"],
        }
    }
}

impl Compat {
    pub fn detect(base_url: &str) -> Self {
        let mut c = Self::default();
        if base_url.contains("api.openai.com") {
            c.max_tokens_field = "max_completion_tokens";
        }
        c
    }
}
