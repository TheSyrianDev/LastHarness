// Spike 2 (throwaway): minimal chat TUI to measure RAM. Streams from llama-swap, keeps full history.
use crossterm::{event::{self, Event, KeyCode}, execute, terminal::*};
use futures_util::StreamExt;
use ratatui::{prelude::*, widgets::*};
use std::{io, time::Duration};
use tokio::sync::mpsc;

const URL: &str = "http://localhost:9292/v1/chat/completions";

enum Ev { Delta(String), Done }

#[tokio::main]
async fn main() -> io::Result<()> {
    let model = std::env::args().nth(1).unwrap_or("LFM2.5-2.6B-fast".into());
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let (tx, mut rx) = mpsc::unbounded_channel::<Ev>();
    let mut history: Vec<(String, String)> = vec![]; // (role, text)
    let mut input = String::new();
    let client = reqwest::Client::new();
    loop {
        term.draw(|f| {
            let a = f.area();
            let parts = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(a);
            let text: Vec<Line> = history.iter().flat_map(|(r, t)| {
                let mut v = vec![Line::from(Span::styled(format!("{r}:"), Style::new().bold()))];
                v.extend(t.lines().map(|l| Line::from(l.to_string())));
                v
            }).collect();
            let total = text.len() as u16;
            let scroll = total.saturating_sub(parts[0].height);
            f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }).scroll((scroll, 0)), parts[0]);
            f.render_widget(Paragraph::new(input.as_str()).block(Block::bordered().title("you")), parts[1]);
        })?;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Ev::Delta(s) => if let Some(l) = history.last_mut() { l.1.push_str(&s) },
                Ev::Done => {}
            }
        }
        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Esc => break,
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace => { input.pop(); }
                    KeyCode::Enter => {
                        let q = std::mem::take(&mut input);
                        history.push(("you".into(), q));
                        let msgs: Vec<_> = history.iter().map(|(r, t)| serde_json::json!({"role": if r == "you" {"user"} else {"assistant"}, "content": t})).collect();
                        history.push(("ai".into(), String::new()));
                        let (tx, client, model) = (tx.clone(), client.clone(), model.clone());
                        tokio::spawn(async move {
                            let body = serde_json::json!({"model": model, "stream": true, "max_tokens": 400, "messages": msgs});
                            if let Ok(resp) = client.post(URL).json(&body).send().await {
                                let mut s = resp.bytes_stream();
                                let mut buf = String::new();
                                while let Some(Ok(b)) = s.next().await {
                                    buf.push_str(&String::from_utf8_lossy(&b));
                                    while let Some(i) = buf.find("\n\n") {
                                        let line: String = buf.drain(..i + 2).collect();
                                        if let Some(j) = line.trim().strip_prefix("data: ") {
                                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(j) {
                                                let d = &v["choices"][0]["delta"];
                                                if let Some(t) = d["content"].as_str().or(d["reasoning_content"].as_str()) { let _ = tx.send(Ev::Delta(t.into())); }
                                            }
                                        }
                                    }
                                }
                            }
                            let _ = tx.send(Ev::Done);
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
