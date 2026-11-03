//! Capability probing: what a model supports, learned from the server instead of guessed.
//! Verified against llama-swap (`/upstream/{model}/props`) and llama.cpp (`/props`). Servers that
//! answer neither give "unknown" for every field, which callers must treat as "assume nothing".

use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Capabilities {
    /// Context window in tokens.
    pub context_tokens: Option<u64>,
    /// The chat template supports tool calls.
    pub tools: Option<bool>,
    pub vision: Option<bool>,
    /// The template understands a reasoning-effort setting.
    pub reasoning_effort: Option<bool>,
    /// The model can think before answering (Ollama reports this).
    pub thinking: Option<bool>,
    /// Number of parameters, when the server says (llama.cpp reports `n_params`).
    pub parameters: Option<u64>,
    /// Which endpoint answered, for the log. Empty when nothing did.
    pub source: String,
}

impl Capabilities {
    pub fn from_props(v: &Value, source: &str) -> Option<Self> {
        let n_ctx = v["default_generation_settings"]["n_ctx"].as_u64().or_else(|| v["n_ctx"].as_u64());
        let caps = &v["chat_template_caps"];
        if n_ctx.is_none() && caps.is_null() {
            return None; // some other JSON; not a props reply
        }
        Some(Self {
            context_tokens: n_ctx,
            tools: caps["supports_tools"].as_bool(),
            vision: v["modalities"]["vision"].as_bool(),
            reasoning_effort: caps["supports_reasoning_effort"].as_bool(),
            thinking: None,
            parameters: None,
            source: source.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_a_llama_cpp_props_reply() {
        let v = json!({"default_generation_settings":{"n_ctx":32768},"chat_template_caps":{"supports_tools":true,"supports_reasoning_effort":false},"modalities":{"vision":false}});
        let c = Capabilities::from_props(&v, "props").unwrap();
        assert_eq!((c.context_tokens, c.tools, c.vision, c.reasoning_effort), (Some(32768), Some(true), Some(false), Some(false)));
    }

    #[test]
    fn other_json_is_not_a_props_reply() {
        assert_eq!(Capabilities::from_props(&json!({"error":"not found"}), "x"), None);
    }
}
