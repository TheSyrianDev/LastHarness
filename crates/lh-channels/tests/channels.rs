//! Each channel client against a mock of the real service's API.
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message as WsMsg, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use lh_channels::*;
use serde_json::{json, Value};

type Log = Arc<Mutex<Vec<String>>>;

async fn serve(app: Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    url
}

// ---- Telegram ----

#[tokio::test]
async fn telegram_returns_messages_once_moves_its_offset_and_sends() {
    #[derive(Clone)]
    struct S {
        log: Log,
        calls: Arc<Mutex<u32>>,
    }
    let s = S { log: Default::default(), calls: Default::default() };
    let app = Router::new()
        .route(
            "/botTOKEN/getUpdates",
            get(|State(s): State<S>, Query(q): Query<std::collections::HashMap<String, String>>| async move {
                s.log.lock().unwrap().push(format!("offset={}", q["offset"]));
                let n = { let mut c = s.calls.lock().unwrap(); *c += 1; *c };
                if n == 1 {
                    Json(json!({"ok": true, "result": [
                        {"update_id": 10, "message": {"text": "hello", "from": {"id": 42, "is_bot": false}, "chat": {"id": 42}}},
                        {"update_id": 11, "message": {"from": {"id": 42}, "chat": {"id": 42}, "photo": []}},
                        {"update_id": 12, "message": {"text": "from a bot", "from": {"id": 7, "is_bot": true}, "chat": {"id": 7}}},
                        {"update_id": 13, "message": {"text": "in a group", "from": {"id": 42, "is_bot": false}, "chat": {"id": -100, "type": "supergroup"}}},
                    ]}))
                } else {
                    Json(json!({"ok": true, "result": []}))
                }
            }),
        )
        .route("/botTOKEN/sendMessage", post(|State(s): State<S>, Json(b): Json<Value>| async move { s.log.lock().unwrap().push(format!("send {} {}", b["chat_id"], b["text"])); Json(json!({"ok": true})) }))
        .route("/botBAD/getUpdates", get(|| async { (axum::http::StatusCode::UNAUTHORIZED, Json(json!({"ok": false}))) }))
        .with_state(s.clone());
    let base = serve(app).await;
    let t = Telegram::new(&base, "TOKEN");
    let first = t.next().await.unwrap();
    assert_eq!(first, vec![Message { chat: "42".into(), sender: "42".into(), text: "hello".into() }], "text from people only");
    assert!(t.next().await.unwrap().is_empty());
    t.send("42", "hi back").await.unwrap();
    assert_eq!(*s.log.lock().unwrap(), ["offset=0", "offset=14", "send \"42\" \"hi back\""], "the next poll asks past the last update");
    assert!(matches!(Telegram::new(&base, "BAD").next().await, Err(Error::Auth(_))));
    // an unreachable service must not put the token (which is part of Telegram's address) into the error
    let e = Telegram::new("http://127.0.0.1:9", "123456:SECRETTOKEN").next().await.unwrap_err().to_string();
    assert!(!e.contains("SECRETTOKEN") && e.contains("not reachable"), "{e}");
    let e = Telegram::new("http://127.0.0.1:9", "123456:SECRETTOKEN").send("1", "x").await.unwrap_err().to_string();
    assert!(!e.contains("SECRETTOKEN"), "{e}");
}

// ---- Matrix ----

#[tokio::test]
async fn matrix_skips_history_its_own_messages_and_encrypted_rooms_and_joins_only_invites_from_allowed_people() {
    #[derive(Clone, Default)]
    struct S {
        log: Log,
        syncs: Arc<Mutex<u32>>,
    }
    let s = S::default();
    let app = Router::new()
        .route("/_matrix/client/v3/account/whoami", get(|| async { Json(json!({"user_id": "@bot:hs"})) }))
        .route(
            "/_matrix/client/v3/sync",
            get(|State(s): State<S>, Query(q): Query<std::collections::HashMap<String, String>>| async move {
                s.log.lock().unwrap().push(format!("sync since={:?} timeout={}", q.get("since"), q["timeout"]));
                let n = { let mut c = s.syncs.lock().unwrap(); *c += 1; *c };
                if n == 1 {
                    // history that must not be treated as a request
                    return Json(json!({"next_batch": "b1", "rooms": {"join": {"!old:hs": {"timeline": {"events": [{"type": "m.room.message", "sender": "@me:hs", "content": {"msgtype": "m.text", "body": "old"}}]}}}}}));
                }
                Json(json!({"next_batch": "b2", "rooms": {
                    "join": {"!group:hs": {"timeline": {"events": [{"type": "m.room.message", "sender": "@me:hs", "content": {"msgtype": "m.text", "body": "said in a group"}}]}}, "!r:hs": {"timeline": {"events": [
                        {"type": "m.room.message", "sender": "@me:hs", "content": {"msgtype": "m.text", "body": "hello bot"}},
                        {"type": "m.room.message", "sender": "@bot:hs", "content": {"msgtype": "m.text", "body": "my own echo"}},
                        {"type": "m.room.encrypted", "sender": "@me:hs", "content": {}},
                        {"type": "m.room.message", "sender": "@me:hs", "content": {"msgtype": "m.image", "body": "pic.png"}},
                    ]}}},
                    "invite": {
                        "!ok:hs": {"invite_state": {"events": [{"type": "m.room.member", "sender": "@Me:hs", "content": {"membership": "invite"}}]}},
                        "!stranger:hs": {"invite_state": {"events": [{"type": "m.room.member", "sender": "@evil:hs", "content": {"membership": "invite"}}]}},
                    }}}))
            }),
        )
        .route("/_matrix/client/v3/rooms/{room}/joined_members", get(|Path(r): Path<String>| async move { if r == "!r:hs" { Json(json!({"joined": {"@me:hs": {}, "@bot:hs": {}}})) } else { Json(json!({"joined": {"@me:hs": {}, "@bot:hs": {}, "@x:hs": {}}})) } }))
        .route("/_matrix/client/v3/join/{room}", post(|State(s): State<S>, Path(r): Path<String>| async move { s.log.lock().unwrap().push(format!("join {r}")); Json(json!({})) }))
        .route("/_matrix/client/v3/rooms/{room}/send/m.room.message/{txn}", put(|State(s): State<S>, h: HeaderMap, Path((r, _t)): Path<(String, String)>, Json(b): Json<Value>| async move { s.log.lock().unwrap().push(format!("send {r} {} auth={}", b["body"], h["authorization"].to_str().unwrap())); Json(json!({"event_id": "$e"})) }))
        .with_state(s.clone());
    let base = serve(app).await;
    let m = Matrix::new(&base, "tok", vec!["@me:hs".into()]);
    assert!(m.next().await.unwrap().is_empty(), "the first sync only finds where now is");
    let got = m.next().await.unwrap();
    assert_eq!(got, vec![Message { chat: "!r:hs".into(), sender: "@me:hs".into(), text: "hello bot".into() }]);
    m.send("!r:hs", "hi").await.unwrap();
    let log = s.log.lock().unwrap().clone();
    assert!(log[0].contains("since=None") && log[0].contains("timeout=0") && log[1].contains("since=Some(\"b1\")"), "{log:?}");
    assert!(log.contains(&"join !ok:hs".to_string()) && !log.iter().any(|l| l.contains("stranger")), "{log:?}");
    assert!(log.contains(&"send !r:hs \"hi\" auth=Bearer tok".to_string()), "{log:?}");
}

// ---- Signal ----

#[tokio::test]
async fn signal_reads_the_event_stream_and_sends_through_rpc() {
    let log: Log = Default::default();
    let l2 = log.clone();
    let app = Router::new()
        .route("/api/v1/events", get(|| async {
            let body = "data: {\"envelope\":{\"sourceNumber\":\"+15550001\",\"dataMessage\":{\"message\":\"hi there\"}}}\n\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"receive\",\"params\":{\"envelope\":{\"source\":\"+15550002\",\"dataMessage\":{\"message\":\"wrapped\"}}}}\n\ndata: {\"envelope\":{\"sourceNumber\":\"+15559999\",\"dataMessage\":{\"message\":\"my own\"}}}\n\ndata: {\"envelope\":{\"sourceNumber\":\"+15550004\",\"dataMessage\":{\"message\":\"in a group\",\"groupInfo\":{\"groupId\":\"g\"}}}}\n\ndata: {\"envelope\":{\"sourceNumber\":\"+15550003\",\"receiptMessage\":{}}}\n\n";
            ([("content-type", "text/event-stream")], body)
        }))
        .route("/api/v1/rpc", post(move |Json(b): Json<Value>| { let l = l2.clone(); async move {
            l.lock().unwrap().push(format!("{} {} {}", b["method"], b["params"]["recipient"], b["params"]["message"]));
            if b["params"]["message"] == "fail" { Json(json!({"jsonrpc": "2.0", "id": b["id"], "error": {"message": "not registered"}})) } else { Json(json!({"jsonrpc": "2.0", "id": b["id"], "result": {}})) }
        }}));
    let base = serve(app).await;
    let s = Signal::new(&base, "+15559999");
    let got = s.next().await.unwrap();
    assert_eq!(got.iter().map(|m| (m.sender.as_str(), m.text.as_str())).collect::<Vec<_>>(), [("+15550001", "hi there"), ("+15550002", "wrapped")], "not our own, not receipts");
    s.send("+15550001", "reply").await.unwrap();
    assert!(s.send("+15550001", "fail").await.unwrap_err().to_string().contains("not registered"));
    assert_eq!(log.lock().unwrap()[0], "\"send\" [\"+15550001\"] \"reply\"");
    // a stream that ends is reported so the caller reconnects
    assert!(s.next().await.is_err());
    // a daemon that is not there says where it looked
    let e = Signal::new("http://127.0.0.1:9", "+1").next().await.unwrap_err().to_string();
    assert!(e.contains("signal-cli is not reachable at http://127.0.0.1:9"), "{e}");
}

// ---- Slack ----

#[tokio::test]
async fn slack_socket_mode_acknowledges_events_answers_only_direct_messages_and_posts_replies() {
    #[derive(Clone)]
    struct S {
        log: Log,
        base: Arc<Mutex<String>>,
    }
    let s = S { log: Default::default(), base: Default::default() };
    async fn open(State(s): State<S>, h: HeaderMap) -> Json<Value> {
        s.log.lock().unwrap().push(format!("open auth={}", h["authorization"].to_str().unwrap()));
        let base = s.base.lock().unwrap().clone();
        Json(json!({"ok": true, "url": format!("ws://{base}/socket")}))
    }
    async fn socket(State(s): State<S>, ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(move |mut sock| async move {
            let ev = |id: &str, e: Value| json!({"type": "events_api", "envelope_id": id, "payload": {"event": e}}).to_string();
            let frames = vec![
                json!({"type": "hello"}).to_string(),
                ev("e1", json!({"type": "message", "channel_type": "channel", "user": "U1", "channel": "C1", "text": "in a public channel"})),
                ev("e2", json!({"type": "message", "channel_type": "im", "user": "U1", "channel": "D1", "text": "bot loop", "bot_id": "B1"})),
                ev("e3", json!({"type": "message", "channel_type": "im", "user": "U1", "channel": "D1", "text": "hello from slack"})),
            ];
            for f in frames {
                let _ = sock.send(WsMsg::Text(f.into())).await;
            }
            // collect the acknowledgements
            while let Some(Ok(WsMsg::Text(t))) = sock.recv().await {
                s.log.lock().unwrap().push(format!("ack {t}"));
                if t.contains("e3") {
                    break;
                }
            }
        })
    }
    async fn post_message(State(s): State<S>, h: HeaderMap, Json(b): Json<Value>) -> impl IntoResponse {
        s.log.lock().unwrap().push(format!("post {} {} auth={}", b["channel"], b["text"], h["authorization"].to_str().unwrap()));
        Json(json!({"ok": true}))
    }
    let app = Router::new().route("/apps.connections.open", post(open)).route("/socket", get(socket)).route("/chat.postMessage", post(post_message)).with_state(s.clone());
    let base = serve(app).await;
    *s.base.lock().unwrap() = base.trim_start_matches("http://").to_string();
    let sl = Slack::new(&base, "xapp-1", "xoxb-1");
    let mut got = vec![];
    for _ in 0..6 {
        got.extend(sl.next().await.unwrap());
        if !got.is_empty() {
            break;
        }
    }
    assert_eq!(got, vec![Message { chat: "D1".into(), sender: "U1".into(), text: "hello from slack".into() }], "only a human's direct message");
    sl.send("D1", "answer").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let log = s.log.lock().unwrap().clone();
    assert_eq!(log[0], "open auth=Bearer xapp-1");
    assert!(log.iter().any(|l| l.contains("ack") && l.contains("e1")) && log.iter().any(|l| l.contains("ack") && l.contains("e3")), "every event is acknowledged: {log:?}");
    assert!(log.contains(&"post \"D1\" \"answer\" auth=Bearer xoxb-1".to_string()), "{log:?}");
}

#[tokio::test]
async fn slack_with_a_bad_token_reports_an_auth_error() {
    let app = Router::new().route("/apps.connections.open", post(|| async { Json(json!({"ok": false, "error": "invalid_auth"})) }));
    let base = serve(app).await;
    assert!(matches!(Slack::new(&base, "bad", "bad").next().await, Err(Error::Auth(_))));
}

#[test]
fn building_a_channel_needs_its_settings_and_real_secret_references() {
    let mut c = ChannelConfig { name: "p".into(), kind: "telegram".into(), enabled: false, allow: vec![], mode: "chat".into(), model: "m".into(), settings: Default::default() };
    assert!(build(&c).err().unwrap().to_string().contains("needs the setting token"));
    c.settings.insert("token".into(), "pasted-token".into());
    assert!(build(&c).err().unwrap().to_string().contains("env:NAME"), "a pasted token is refused");
    c.settings.insert("token".into(), "env:LH_TEST_CH_TOKEN".into());
    std::env::set_var("LH_TEST_CH_TOKEN", "123:abc");
    assert_eq!(build(&c).unwrap().kind(), "telegram");
}
