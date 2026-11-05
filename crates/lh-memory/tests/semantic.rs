use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use lh_memory::embed::*;
use lh_memory::*;
use lh_store::Store;

/// A stand-in embedder: words from the same group of near-synonyms land on the same axis.
struct Fake {
    trust: Trust,
}
const GROUPS: [&[&str]; 4] = [&["sad", "cry", "tear", "grief", "upset"], &["pet", "dog", "cat", "puppy"], &["money", "salary", "income", "earn", "pay"], &["trip", "flight", "travel", "journey"]];
fn vec_of(text: &str) -> Vec<f32> {
    let mut v = [0f32; 12];
    for w in text::tokens(text) {
        match GROUPS.iter().position(|g| g.contains(&w.as_str())) {
            Some(i) => v[i] += 1.0,
            None => v[4 + (w.bytes().map(|b| b as usize).sum::<usize>() % 8)] += 0.2,
        }
    }
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    v.iter().map(|x| x / n).collect()
}
impl Embedder for Fake {
    fn model_id(&self) -> &str {
        "fake"
    }
    fn trust(&self) -> Trust {
        self.trust
    }
    async fn embed_documents(&self, t: &[String]) -> std::result::Result<Vec<Vec<f32>>, EmbedError> {
        Ok(t.iter().map(|x| vec_of(x)).collect())
    }
    async fn embed_query(&self, t: &str) -> std::result::Result<Vec<f32>, EmbedError> {
        Ok(vec_of(t))
    }
}

fn mem() -> (tempfile::TempDir, Memory) {
    let d = tempfile::tempdir().unwrap();
    let m = Memory::open(Arc::new(Store::open_memory().unwrap()), d.path().join("m")).unwrap();
    (d, m)
}
fn add(m: &Memory, title: &str, body: &str, privacy: Privacy) -> Page {
    let mut n = NewPage::new(Layer::Facts, title, body);
    n.privacy = privacy;
    m.create(n, By::User).unwrap()
}

#[tokio::test]
async fn meaning_search_finds_what_keywords_miss_and_follows_edits() {
    let (_d, m) = mem();
    let dog = add(&m, "Dog died", "The user had a good cry after their dog died.", Privacy::Normal);
    add(&m, "Berlin", "Flight to Berlin in June.", Privacy::Normal);
    let e = Fake { trust: Trust::Local };
    assert!(m.search("sadness about a pet", State::Live, 5).unwrap().is_empty(), "keywords alone miss this");
    assert_eq!(m.index_pending(&e).await.unwrap(), 2);
    assert_eq!(m.index_pending(&e).await.unwrap(), 0, "nothing to do the second time");
    let hits = m.semantic_search(&e, "sad about a pet", 3).await.unwrap();
    assert_eq!(hits[0].0, dog.id);

    // an edit makes the old vector stale; it is not used until the page is embedded again
    m.update(&dog.id, Edit { body: Some("The user now owns a travel journey blog.".into()), ..Default::default() }, By::User).unwrap();
    m.release_vectors();
    assert!(m.semantic_search(&e, "sad about a pet", 3).await.unwrap().iter().all(|h| h.0 != dog.id));
    assert_eq!(m.index_pending(&e).await.unwrap(), 1);
    assert!(!m.semantic_search(&e, "trip abroad", 3).await.unwrap().is_empty());

    // archived pages leave the vector search
    m.archive(&dog.id, By::User, "").unwrap();
    assert!(m.semantic_search(&e, "travel journey", 5).await.unwrap().iter().all(|h| h.0 != dog.id));
}

#[tokio::test]
async fn recall_combines_both_searches_and_still_gates_by_trust() {
    let (_d, m) = mem();
    let dog = add(&m, "Dog died", "The user had a good cry after their dog died.", Privacy::Normal);
    let med = add(&m, "Medication", "Takes metformin; the upset stomach is a side effect.", Privacy::Sensitive);
    add(&m, "Berlin", "Flight to Berlin in June.", Privacy::Normal);
    let e = Fake { trust: Trust::Local };
    m.index_pending(&e).await.unwrap();
    let dest = Destination { trust: Trust::AnyCloud, project: None };
    let sem = m.semantic_search(&e, "feeling sad and upset", 5).await.unwrap();
    assert!(sem.iter().any(|h| h.0 == med.id), "the semantic search itself can see it");
    let r = m.recall("feeling sad and upset", &dest, &RecallConfig::default(), &sem, &HashSet::new()).unwrap();
    assert_eq!(r.notes[0].page.id, dog.id);
    assert!(r.notes.iter().all(|n| n.page.id != med.id) && r.withheld >= 1);
}

#[tokio::test]
async fn a_non_local_embedder_never_sees_sensitive_pages() {
    let (_d, m) = mem();
    add(&m, "Open", "Likes chess.", Privacy::Normal);
    add(&m, "Secret", "Takes insulin daily.", Privacy::Sensitive);
    assert_eq!(m.index_pending(&Fake { trust: Trust::AnyCloud }).await.unwrap(), 1);
    assert_eq!(m.index_pending(&Fake { trust: Trust::Local }).await.unwrap(), 1, "a local embedder catches up on the rest");
}

#[tokio::test]
async fn idle_release_frees_the_vectors_and_search_reloads_them() {
    let (_d, m) = mem();
    add(&m, "Berlin", "Flight to Berlin in June.", Privacy::Normal);
    let e = Fake { trust: Trust::Local };
    m.index_pending(&e).await.unwrap();
    assert!(!m.vectors_loaded());
    m.semantic_search(&e, "trip", 3).await.unwrap();
    assert!(m.vectors_loaded());
    m.release_vectors();
    assert!(!m.vectors_loaded());
    assert_eq!(m.semantic_search(&e, "trip", 3).await.unwrap().len(), 1);
}

#[tokio::test]
async fn cloud_embedding_servers_are_refused_unless_allowed() {
    assert!(matches!(HttpEmbedder::new("https://api.example.com/v1", "m", None, Prefixes::default(), false), Err(EmbedError::Refused(_))));
    let e = HttpEmbedder::new("https://api.example.com/v1", "m", None, Prefixes::default(), true).unwrap();
    assert_eq!(e.trust(), Trust::AnyCloud);
    assert_eq!(HttpEmbedder::new("http://127.0.0.1:8080/v1", "m", None, Prefixes::default(), false).unwrap().trust(), Trust::Local);
}

// ---- the real model, when it is on this machine ----

fn real_setup() -> Option<ManagedConfig> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let engine = home.join("AI/engines/_prebuilt/mainline-b11541/llama-b11541");
    let libs = format!("{}:{}", engine.display(), home.join("AI/engines/_prebuilt/mainline-b11538/cudart-llama-b11538-bin-ubuntu-cuda-13.4-x64").display());
    let model = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spikes/memory-retrieval/models/embeddinggemma-2-Q8_0.gguf");
    (engine.join("llama-server").exists() && model.exists()).then(|| ManagedConfig {
        server: engine.join("llama-server"),
        library_path: Some(libs),
        model_file: model,
        model_name: "embeddinggemma-2".into(),
        prefixes: Prefixes::embedding_gemma(),
        threads: 6,
    })
}

const GOLD: [(&str, &str, &str, &str, &str); 8] = [
    ("dog", "User cried for an hour after their dog died last spring.", "بكى المستخدم لمدة ساعة كاملة بعد موت كلبه في الربيع الماضي.", "when was the user sad about a pet", "متى كان المستخدم حزينا بسبب حيوان أليف"),
    ("style", "Omar prefers short answers, no bullet lists, and plain words.", "يفضل عمر الإجابات القصيرة بدون قوائم وبكلمات بسيطة.", "how does the user like replies to look", "كيف يحب المستخدم شكل الردود"),
    ("db", "Decided on 3 March: the project uses SQLite for storage because it needs no server.", "تقرر في ٣ مارس أن المشروع يستخدم SQLite للتخزين لأنه لا يحتاج إلى خادم.", "which database did we pick and why", "أي نظام تخزين اخترنا ولماذا"),
    ("money", "Salary is 4200 per month after tax and rent is 1100.", "الراتب ٤٢٠٠ شهريا بعد الضريبة والإيجار ١١٠٠.", "how much money does the user earn", "كم يكسب المستخدم من المال"),
    ("trip", "Flight to Berlin on 14 June, seat 22A, with Lufthansa.", "رحلة إلى برلين يوم ١٤ يونيو على مقعد ٢٢أ مع الخطوط اللوفتهانزا.", "upcoming trip to Germany", "سفر قادم إلى ألمانيا"),
    ("gpu", "The laptop has 24 GB of RAM and an RTX 3050 with 4 GB of VRAM.", "الحاسوب المحمول فيه ذاكرة ٢٤ جيجابايت وبطاقة RTX 3050 بذاكرة ٤ جيجابايت.", "what graphics card does the user have", "ما بطاقة الشاشة لدى المستخدم"),
    ("hobby", "The user wants to learn Arabic calligraphy in the evenings.", "يريد المستخدم تعلم الخط العربي في المساء.", "an artistic hobby the user wants", "هواية فنية يريدها المستخدم"),
    ("mom", "Mother's birthday is 9 October and she likes jasmine tea.", "عيد ميلاد الأم في ٩ أكتوبر وهي تحب شاي الياسمين.", "gift ideas for a family member", "أفكار هدايا لأحد أفراد العائلة"),
];
const FILLERS: [&str; 10] = [
    "The meeting with the landlord was moved to Thursday afternoon.",
    "User asked for a recipe for lentil soup with lemon.",
    "The build failed because a crate version was too old.",
    "Plan to repaint the bedroom walls light green.",
    "The router needs a firmware update.",
    "Football match on Saturday at seven with friends.",
    "The printer in the office jams on thick paper.",
    "Notes about the Rust borrow checker and lifetimes.",
    "Library books are due on the 20th.",
    "The phone battery drains fast after the update.",
];

/// Needs the EmbeddingGemma 2 file from spike 3 and the prebuilt engine; skipped otherwise.
#[tokio::test]
async fn real_embeddinggemma_beats_keywords_on_paraphrases_in_english_and_arabic() {
    let Some(cfg) = real_setup() else { return eprintln!("skipped: embedding model or engine not on this machine") };
    let (_d, m) = mem();
    let mut by_key = std::collections::HashMap::new();
    for (k, en, ar, _, _) in GOLD {
        by_key.insert(k, (add(&m, &format!("{k} en"), en, Privacy::Normal).id, add(&m, &format!("{k} ar"), ar, Privacy::Normal).id));
    }
    for (i, f) in FILLERS.iter().enumerate() {
        add(&m, &format!("filler {i}"), f, Privacy::Normal);
    }
    let e = ManagedEmbedder::new(cfg);
    assert!(!e.is_running().await, "nothing is started until it is needed");
    let started = std::time::Instant::now();
    assert_eq!(m.index_pending(&e).await.unwrap(), 16 + FILLERS.len());
    eprintln!("embedded {} pages in {:.1} s including model start", 16 + FILLERS.len(), started.elapsed().as_secs_f64());
    assert!(e.is_running().await);

    let dest = Destination { trust: Trust::Local, project: None };
    let mut table = vec![];
    for (kw, sw) in [(1.0, 0.0), (0.35, 1.0), (0.0, 1.0)] {
        let cfg = RecallConfig { keyword_weight: kw, semantic_weight: sw, max_items: 5, ..Default::default() };
        let (mut en_hits, mut ar_hits) = (0, 0);
        for (k, _, _, qe, qa) in GOLD {
            for (q, is_ar) in [(qe, false), (qa, true)] {
                let sem = if sw > 0.0 { m.semantic_search(&e, q, 10).await.unwrap() } else { vec![] };
                let r = m.recall(q, &dest, &cfg, &sem, &HashSet::new()).unwrap();
                let want = if is_ar { &by_key[k].1 } else { &by_key[k].0 };
                let hit = r.notes.iter().any(|n| &n.page.id == want);
                if hit { if is_ar { ar_hits += 1 } else { en_hits += 1 } }
            }
        }
        table.push(((kw, sw), en_hits, ar_hits));
        eprintln!("keyword weight {kw:.2}, semantic weight {sw:.2}: English {en_hits}/8, Arabic {ar_hits}/8 in the top 5");
    }
    let (keyword_only, hybrid) = (table[0], table[1]);
    assert!(hybrid.1 + hybrid.2 > keyword_only.1 + keyword_only.2 + 4, "meaning search must clearly help: {table:?}");
    assert!(hybrid.1 >= 7 && hybrid.2 >= 7, "{table:?}");

    // the model is stopped when the core goes idle
    e.release().await;
    assert!(!e.is_running().await);
    m.release_vectors();
}
