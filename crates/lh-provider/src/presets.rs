//! Provider presets: pick a known service instead of typing its address. Every address here was read from
//! Pi's provider source or the provider's own documentation (see docs/provider-presets.md). A preset only
//! fills in the address, the name and the key's usual environment variable; it never holds a key.

use crate::Compat;

/// How the server is spoken to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiKind {
    /// `/chat/completions` with native tool calls. What `Provider` speaks.
    OpenAiCompatible,
    /// Ollama's own API (also reachable through its OpenAI-compatible `/v1`).
    Ollama,
    /// Anthropic Messages API (`Provider::anthropic`). Also spoken by Anthropic-compatible services such as MiniMax.
    Anthropic,
}

impl ApiKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai-compatible",
            Self::Ollama => "ollama",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub base_url: &'static str,
    pub api: ApiKind,
    /// Usual environment variable for the key, if the service has one.
    pub key_env: Option<&'static str>,
    pub key_required: bool,
    pub local: bool,
    /// Can this core talk to it today?
    pub supported: bool,
    /// Where to create a key (cloud) or read about the server (local).
    pub help_url: Option<&'static str>,
    /// Short plain-words note shown beside the preset.
    pub note: &'static str,
}

impl Preset {
    /// The quirks the client will use for this address.
    pub fn compat(&self) -> Compat {
        Compat::detect(self.base_url)
    }
}

const fn cloud(id: &'static str, name: &'static str, base_url: &'static str, key_env: &'static str, help_url: &'static str, note: &'static str) -> Preset {
    Preset { id, name, base_url, api: ApiKind::OpenAiCompatible, key_env: Some(key_env), key_required: true, local: false, supported: true, help_url: Some(help_url), note }
}

const fn local(id: &'static str, name: &'static str, base_url: &'static str, api: ApiKind, help_url: Option<&'static str>, note: &'static str) -> Preset {
    Preset { id, name, base_url, api, key_env: None, key_required: false, local: true, supported: true, help_url, note }
}

/// All presets, local ones first.
pub static PRESETS: &[Preset] = &[
    local("llama-swap", "llama-swap / LastEngine", "http://127.0.0.1:9292/v1", ApiKind::OpenAiCompatible, None, "The default local engine."),
    local("ollama", "Ollama", "http://localhost:11434/v1", ApiKind::Ollama, Some("https://docs.ollama.com/openai"), "Local models through Ollama."),
    local("lmstudio", "LM Studio", "http://localhost:1234/v1", ApiKind::OpenAiCompatible, Some("https://lmstudio.ai/docs/developer/openai-compat"), "Start the local server in LM Studio first."),
    cloud("openai", "OpenAI", "https://api.openai.com/v1", "OPENAI_API_KEY", "https://platform.openai.com/api-keys", "Uses max_completion_tokens."),
    Preset { id: "anthropic", name: "Anthropic", base_url: "https://api.anthropic.com", api: ApiKind::Anthropic, key_env: Some("ANTHROPIC_API_KEY"), key_required: true, local: false, supported: true, help_url: Some("https://console.anthropic.com/settings/keys"), note: "Claude through the Messages API." },
    cloud("google", "Google Gemini", "https://generativelanguage.googleapis.com/v1beta/openai", "GEMINI_API_KEY", "https://aistudio.google.com/apikey", "Gemini's OpenAI-compatible endpoint."),
    cloud("openrouter", "OpenRouter", "https://openrouter.ai/api/v1", "OPENROUTER_API_KEY", "https://openrouter.ai/keys", "One key, many models."),
    cloud("groq", "Groq", "https://api.groq.com/openai/v1", "GROQ_API_KEY", "https://console.groq.com/keys", "Very fast hosted open models."),
    cloud("together", "Together AI", "https://api.together.ai/v1", "TOGETHER_API_KEY", "https://api.together.ai/settings/api-keys", "Hosted open models."),
    cloud("fireworks", "Fireworks AI", "https://api.fireworks.ai/inference/v1", "FIREWORKS_API_KEY", "https://fireworks.ai/account/api-keys", "Hosted open models."),
    cloud("deepseek", "DeepSeek", "https://api.deepseek.com", "DEEPSEEK_API_KEY", "https://platform.deepseek.com/api_keys", "DeepSeek's own API."),
    cloud("mistral", "Mistral", "https://api.mistral.ai/v1", "MISTRAL_API_KEY", "https://console.mistral.ai/api-keys", "Mistral's own API."),
    cloud("xai", "xAI", "https://api.x.ai/v1", "XAI_API_KEY", "https://console.x.ai", "Grok models."),
    cloud("cerebras", "Cerebras", "https://api.cerebras.ai/v1", "CEREBRAS_API_KEY", "https://cloud.cerebras.ai", "Very fast hosted open models."),
    cloud("moonshot", "Moonshot AI (Kimi)", "https://api.moonshot.ai/v1", "MOONSHOT_API_KEY", "https://platform.moonshot.ai", "International endpoint."),
    cloud("moonshot-cn", "Moonshot AI (Kimi, China)", "https://api.moonshot.cn/v1", "MOONSHOT_API_KEY", "https://platform.moonshot.cn", "China endpoint."),
    cloud("kimi-code", "Kimi Code (subscription)", "https://api.kimi.com/coding/v1", "KIMI_API_KEY", "https://www.kimi.com/coding", "Needs a Kimi Code plan."),
    cloud("zai", "Z.AI GLM (coding plan)", "https://api.z.ai/api/coding/paas/v4", "ZAI_API_KEY", "https://z.ai/manage-apikey/apikey-list", "The GLM coding plan endpoint."),
    cloud("zai-cn", "Z.AI GLM (coding plan, China)", "https://open.bigmodel.cn/api/coding/paas/v4", "ZAI_CODING_CN_API_KEY", "https://bigmodel.cn", "China endpoint."),
    Preset { id: "minimax", name: "MiniMax", base_url: "https://api.minimax.io/anthropic", api: ApiKind::Anthropic, key_env: Some("MINIMAX_API_KEY"), key_required: true, local: false, supported: true, help_url: Some("https://www.minimax.io"), note: "Anthropic-style API." },
    cloud("qwen-token-plan", "Qwen Token Plan", "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1", "QWEN_TOKEN_PLAN_API_KEY", "https://www.alibabacloud.com/product/modelstudio", "Alibaba token plan, Singapore."),
    cloud("nvidia", "NVIDIA NIM", "https://integrate.api.nvidia.com/v1", "NVIDIA_API_KEY", "https://build.nvidia.com", "Hosted NIM models."),
    cloud("huggingface", "Hugging Face router", "https://router.huggingface.co/v1", "HF_TOKEN", "https://huggingface.co/settings/tokens", "Inference providers through one token."),
    cloud("vercel", "Vercel AI Gateway", "https://ai-gateway.vercel.sh/v1", "AI_GATEWAY_API_KEY", "https://vercel.com/docs/ai-gateway", "One key, many providers."),
    cloud("baseten", "Baseten", "https://inference.baseten.co/v1", "BASETEN_API_KEY", "https://app.baseten.co/settings/api_keys", "Hosted models."),
    cloud("xiaomi", "Xiaomi MiMo", "https://api.xiaomimimo.com/v1", "XIAOMI_API_KEY", "https://platform.xiaomimimo.com", "Xiaomi's own API."),
];

pub fn find(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_urls_are_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for p in PRESETS {
            assert!(seen.insert(p.id), "duplicate id {}", p.id);
            assert!(!p.name.is_empty() && !p.note.is_empty(), "{}", p.id);
            let rest = p.base_url.strip_prefix("http://").or_else(|| p.base_url.strip_prefix("https://")).unwrap_or_else(|| panic!("{} url", p.id));
            assert!(!rest.is_empty() && !p.base_url.contains(' ') && !p.base_url.ends_with('/'), "{}", p.id);
        }
    }

    #[test]
    fn cloud_is_https_and_needs_a_key_local_is_local() {
        for p in PRESETS {
            assert_eq!(p.local, crate::is_local_url(p.base_url), "{}", p.id);
            if p.local {
                assert!(!p.key_required, "{}", p.id);
            } else {
                assert!(p.base_url.starts_with("https://"), "{}", p.id);
                assert!(p.key_required && p.key_env.is_some(), "{}", p.id);
            }
            if let Some(u) = p.help_url {
                assert!(u.starts_with("https://"), "{}", p.id);
            }
        }
    }

    #[test]
    fn openai_quirks_apply() {
        assert_eq!(find("openai").unwrap().compat().max_tokens_field, "max_completion_tokens");
        assert!(find("nope").is_none());
    }
}
