//! Replays the raw captures from spike 1 (7 models on llama-swap) through the parser. No network.
use lh_provider::*;

const MODELS: [&str; 7] = [
    "LFM2.5-2.6B-fast",
    "MiniCPM5-2B-max",
    "Spark-X2.5-4B-fast",
    "Ornith-1.5-35B-A3B-NanoPlus-fast",
    "Qwen3.6-35B-A3B-NanoPlus-fast",
    "Tiel-Coder-35B-fast",
    "SignOfFour-fast",
];

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn replay(sse: &str, chunk: usize) -> (Response, Vec<Event>) {
    let (mut p, mut a) = (SseParser::new(), Assembler::new(Compat::default()));
    let mut events = vec![];
    for piece in sse.as_bytes().chunks(chunk) {
        for d in p.push(piece) {
            events.extend(a.feed(&d));
        }
    }
    (a.finish(), events)
}

#[test]
fn streamed_tool_call_on_every_model() {
    for m in MODELS {
        let sse = fixture(&format!("{m}.toolstream.sse"));
        // whole file at once, and cut into awkward 7-byte pieces: same answer
        for size in [sse.len(), 7] {
            let (r, ev) = replay(&sse, size);
            assert_eq!(r.tool_calls.len(), 1, "{m}");
            let c = &r.tool_calls[0];
            assert_eq!(c.name, "get_weather", "{m}");
            assert_eq!(c.parsed().unwrap()["city"], "Paris", "{m}");
            assert!(!c.repaired && !c.id.is_empty(), "{m}");
            assert_eq!(r.finish, Some(FinishReason::ToolCalls), "{m}");
            assert!(r.usage.is_some(), "{m}: usage chunk with empty choices");
            assert!(ev.iter().any(|e| matches!(e, Event::Reasoning(_))), "{m}: thinking is in reasoning_content");
            assert_eq!(r.text, "", "{m}: no text on a tool turn");
        }
    }
}

#[test]
fn parallel_calls_non_streamed_on_every_model() {
    for m in MODELS {
        let v: serde_json::Value = serde_json::from_str(&fixture(&format!("{m}.parallel.json"))).unwrap();
        let r = parse_response(&v, &Compat::default());
        let names: Vec<_> = r.tool_calls.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["get_weather", "get_time"], "{m}");
        assert!(r.tool_calls.iter().all(|c| c.parsed().unwrap()["city"] == "Tokyo"), "{m}");
        assert_ne!(r.tool_calls[0].id, r.tool_calls[1].id, "{m}");
        assert_eq!(r.finish, Some(FinishReason::ToolCalls), "{m}");
    }
}

#[test]
fn plain_stream_keeps_text_and_thinking_apart() {
    let (r, _) = replay(&fixture("LFM2.5-2.6B-fast.chatstream.sse"), 64);
    assert_eq!(r.text, "Hello there, how are you?");
    assert!(r.reasoning.len() > 200);
    assert_eq!(r.finish, Some(FinishReason::Stop));
    assert_eq!(r.usage.unwrap().completion_tokens, 341);
}

#[test]
fn cut_off_arguments_are_repaired_and_flagged() {
    let mut a = Assembler::new(Compat::default());
    a.feed(r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"x","function":{"name":"get_weather","arguments":"{\"city\":\"Par"}}]}}]}"#);
    let r = a.finish();
    let c = &r.tool_calls[0];
    assert!(c.repaired);
    assert_eq!(c.parsed().unwrap()["city"], "Par");
}

#[test]
fn length_finish_with_empty_text_is_visible() {
    let mut a = Assembler::new(Compat::default());
    a.feed(r#"{"choices":[{"delta":{"reasoning_content":"thinking..."}}]}"#);
    a.feed(r#"{"choices":[{"delta":{},"finish_reason":"length"}]}"#);
    let r = a.finish();
    assert_eq!(r.finish, Some(FinishReason::Length));
    assert!(r.text.is_empty() && !r.reasoning.is_empty());
}
