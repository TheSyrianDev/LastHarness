//! Provider layer: talks to OpenAI-compatible chat servers (llama-swap, llama.cpp, Ollama, LM Studio,
//! OpenRouter). Native tool calls only; quirks are handled by `Compat` and tolerant parsing.

mod client;
mod compat;
mod images;
mod ollama;
mod probe;
mod repair;
mod sse;
mod stream;
mod types;

pub use client::{Error, Provider, Retry};
pub use compat::{is_local_url, Compat};
pub use images::{sniff, GeneratedImage};
pub use probe::Capabilities;
pub use repair::repair_json;
pub use sse::SseParser;
pub use stream::{parse_response, Assembler};
pub use types::*;
