//! Provider layer: talks to OpenAI-compatible chat servers (llama-swap, llama.cpp, Ollama, LM Studio,
//! OpenRouter). Native tool calls only; quirks are handled by `Compat` and tolerant parsing.

mod client;
mod compat;
mod repair;
mod sse;
mod stream;
mod types;

pub use client::{Error, Provider, Retry};
pub use compat::Compat;
pub use repair::repair_json;
pub use sse::SseParser;
pub use stream::{parse_response, Assembler};
pub use types::*;
