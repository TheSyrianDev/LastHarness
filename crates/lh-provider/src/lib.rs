//! Provider layer: talks to OpenAI-compatible chat servers (llama-swap, llama.cpp, Ollama, LM Studio,
//! OpenRouter). Native tool calls only; quirks are handled by `Compat` and tolerant parsing.

mod anthropic;
mod attach;
mod client;
mod compat;
mod images;
mod ollama;
mod presets;
mod probe;
mod repair;
mod sse;
mod stream;
mod types;

pub use attach::{estimate_tokens, kind_of, prepare, AttachError, AttachKind, Attachment, Limits, Prepared, Support};
pub use client::{Error, Provider, Retry};
pub use compat::{is_local_url, Compat};
pub use images::{sniff, GeneratedImage};
pub use presets::{find as find_preset, ApiKind, Preset, PRESETS};
pub use probe::Capabilities;
pub use repair::repair_json;
pub use sse::SseParser;
pub use stream::{parse_response, Assembler};
pub use types::*;
