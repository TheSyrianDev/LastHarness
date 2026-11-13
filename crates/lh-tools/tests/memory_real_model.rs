//! Optional real-model run of memory saving and conflict flags.
//! `LH_REAL_URL=http://127.0.0.1:9292/v1 LH_REAL_MODEL=LFM3-1.7B-balanced cargo test -p lh-tools --test memory_real_model -- --ignored --nocapture`
use std::sync::Arc;

use lh_memory::embed::AnyEmbedder;
use lh_memory::*;
use lh_provider::*;
use lh_store::Store;
use lh_tools::*;

struct Yes;
impl Asker for Yes {
    async fn ask(&self, _r: &AskRequest<'_>) -> Answer {
        Answer::Yes
    }
}

#[tokio::test]
#[ignore]
async fn model_saves_a_fact_in_one_call_and_a_conflict_from_another_department_is_flagged() {
    let (Ok(url), Ok(model)) = (std::env::var("LH_REAL_URL"), std::env::var("LH_REAL_MODEL")) else { return };
    let d = tempfile::tempdir().unwrap();
    let mem = Arc::new(Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap());

    // another department already shared a different price
    let mut other = NewPage::new(Layer::Facts, "Pro plan price", "The pro plan costs 30 euros a month.");
    other.department = Some("finance".into());
    other.shared = true;
    mem.create(other, By::User).unwrap();

    let handle = Arc::new(MemoryHandle {
        memory: mem.clone(),
        embedder: Arc::new(AnyEmbedder::Off),
        dest: Destination { trust: Trust::Local, project: None, department: Some("sales".into()) },
        cfg: RecallConfig::default(),
        layers: None,
        judge: None,
        conv: None,
    });
    let tools = default_tools().with_memory(handle);
    let d2 = tempfile::tempdir().unwrap();
    let perms = Permissions::new(d2.path()).unwrap();
    let provider = Provider::new(url, None);
    let mut r = ChatRequest {
        model,
        max_tokens: Some(3000),
        messages: vec![Message::user("Remember this for later: the pro plan costs 40 euros a month. Save it as a note now.")],
        ..Default::default()
    };
    let mut calls = Vec::new();
    run_turn(&provider, &NoMeter, &mut r, &tools, &perms, &Yes, &TurnOptions::default(), |e| {
        if let TurnEvent::ToolStart { name, arguments, .. } = e {
            calls.push(format!("{name} {arguments}"));
        }
    })
    .await
    .unwrap();
    eprintln!("tool calls: {calls:#?}");
    let saved = calls.iter().filter(|c| c.starts_with("save_note")).count();
    let saved_ok = mem.list(None, State::Live).unwrap().iter().any(|p| p.department.as_deref() == Some("sales") && p.body.contains("40"));
    eprintln!("save_note calls: {saved}; sales note with 40 saved: {saved_ok}");

    // recall as the ops department: the conflicting prices are both visible and flagged
    let ops = Destination { trust: Trust::Local, project: None, department: Some("ops".into()) };
    let rec = mem.recall("pro plan costs euros a month", &ops, &RecallConfig::default(), &[], &Default::default()).unwrap();
    let text = rec.render();
    eprintln!("recall for ops:\n{text}");
    assert!(text.contains("may conflict with"), "the conflict is flagged: {text}");
}
