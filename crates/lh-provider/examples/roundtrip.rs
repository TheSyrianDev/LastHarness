//! Live check: stream a reply, run one tool, send the result back. Usage: roundtrip [model] [base_url]
use lh_provider::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut a = std::env::args().skip(1);
    let model = a.next().unwrap_or("LFM2.5-2.6B-fast".into());
    let base = a.next().unwrap_or("http://localhost:9292/v1".into());
    let p = Provider::new(base, None);
    println!("models on server: {}", p.models().await?.len());

    let weather = ToolDef {
        name: "get_weather".into(),
        description: "Get current weather for a city".into(),
        parameters: json!({"type":"object","properties":{"city":{"type":"string"}},"required":["city"]}),
    };
    let mut req = ChatRequest {
        model,
        messages: vec![Message::user("What is the weather in Paris? Use the tool.")],
        tools: vec![weather],
        max_tokens: Some(2048),
        ..Default::default()
    };
    let r = p.chat_stream(&req, |e| match e {
        Event::Reasoning(_) => eprint!("."),
        Event::Text(t) => print!("{t}"),
        Event::ToolCall(c) => println!("\n[tool call] {}({})", c.name, c.arguments),
        Event::Finish(f) => println!("\n[finish] {f:?}"),
        Event::Usage(u) => println!("[usage] {u:?}"),
    })
    .await?;
    let call = r.tool_calls.first().cloned().expect("model made no tool call");
    req.messages.push(r.into_message());
    req.messages.push(Message::tool_result(&call.id, r#"{"temp_c":18,"sky":"cloudy"}"#));
    let r2 = p.chat_stream(&req, |e| if let Event::Text(t) = e { print!("{t}") }).await?;
    println!("\n[final finish] {:?}", r2.finish);
    Ok(())
}
