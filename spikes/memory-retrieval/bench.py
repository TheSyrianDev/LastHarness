# Spike 3 (throwaway): keyword (BM25) vs embedding vs hybrid (RRF) retrieval, English + Arabic.
# Embeddings come from llama-server --embedding on CPU. Stdlib only.
import json, math, os, re, subprocess, sys, time, glob, random, urllib.request, unicodedata
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
P = os.path.expanduser("~/AI/engines/_prebuilt")
SERVER = P + "/mainline-b11541/llama-b11541/llama-server"  # prebuilt CUDA build, run here with -ngl 0 (CPU)
LIBS = P + "/mainline-b11541/llama-b11541:" + P + "/mainline-b11538/cudart-llama-b11538-bin-ubuntu-cuda-13.4-x64"
PORT = 8791

# ---- gold items: (id, English memory, Arabic memory) ----
ITEMS = {
 "dog":   ("User cried for an hour after their dog died last spring.", "بكى المستخدم لمدة ساعة كاملة بعد موت كلبه في الربيع الماضي."),
 "style": ("Omar prefers short answers, no bullet lists, and plain words.", "يفضل عمر الإجابات القصيرة بدون قوائم وبكلمات بسيطة."),
 "db":    ("Decided on 3 March: the project uses SQLite for storage because it needs no server.", "تقرر في ٣ مارس أن المشروع يستخدم SQLite للتخزين لأنه لا يحتاج إلى خادم."),
 "allergy":("The user is allergic to penicillin and carries an epinephrine pen.", "المستخدم لديه حساسية من البنسلين ويحمل حقنة الإبينفرين."),
 "money": ("Salary is 4200 per month after tax and rent is 1100.", "الراتب ٤٢٠٠ شهريا بعد الضريبة والإيجار ١١٠٠."),
 "trip":  ("Flight to Berlin on 14 June, seat 22A, with Lufthansa.", "رحلة إلى برلين يوم ١٤ يونيو على مقعد ٢٢أ مع الخطوط اللوفتهانزا."),
 "gpu":   ("The laptop has 24 GB of RAM and an RTX 3050 with 4 GB of VRAM.", "الحاسوب المحمول فيه ذاكرة ٢٤ جيجابايت وبطاقة RTX 3050 بذاكرة ٤ جيجابايت."),
 "hobby": ("The user wants to learn Arabic calligraphy in the evenings.", "يريد المستخدم تعلم الخط العربي في المساء."),
 "backup":("Decision: no public hosting until release; backups go to the Seagate drive.", "قرار: لا نشر علني حتى الإصدار، والنسخ الاحتياطية على قرص سيجيت."),
 "mom":   ("Mother's birthday is 9 October and she likes jasmine tea.", "عيد ميلاد الأم في ٩ أكتوبر وهي تحب شاي الياسمين."),
}
# queries: (kind, lang, text, gold id)
QUERIES = [
 ("paraphrase","en","when was the user sad about a pet","dog"),
 ("paraphrase","en","how does the user like replies to look","style"),
 ("paraphrase","en","which database did we pick and why","db"),
 ("paraphrase","en","any problem with medicines","allergy"),
 ("paraphrase","en","how much money does the user earn","money"),
 ("paraphrase","en","upcoming trip to Germany","trip"),
 ("paraphrase","en","what graphics card does the user have","gpu"),
 ("paraphrase","en","an artistic hobby the user wants","hobby"),
 ("paraphrase","en","where is a safe copy of the code kept","backup"),
 ("paraphrase","en","gift ideas for a family member","mom"),
 ("lexical","en","SQLite","db"),("lexical","en","Berlin","trip"),("lexical","en","Seagate","backup"),
 ("lexical","en","penicillin","allergy"),("lexical","en","jasmine","mom"),("lexical","en","RTX 3050","gpu"),
 ("paraphrase","ar","متى كان المستخدم حزينا بسبب حيوان أليف","dog"),
 ("paraphrase","ar","كيف يحب المستخدم شكل الردود","style"),
 ("paraphrase","ar","أي نظام تخزين اخترنا ولماذا","db"),
 ("paraphrase","ar","هل توجد مشكلة مع الأدوية","allergy"),
 ("paraphrase","ar","كم يكسب المستخدم من المال","money"),
 ("paraphrase","ar","سفر قادم إلى ألمانيا","trip"),
 ("paraphrase","ar","ما بطاقة الشاشة لدى المستخدم","gpu"),
 ("paraphrase","ar","هواية فنية يريدها المستخدم","hobby"),
 ("paraphrase","ar","أين نحفظ نسخة آمنة من الشيفرة","backup"),
 ("paraphrase","ar","أفكار هدايا لأحد أفراد العائلة","mom"),
 ("lexical","ar","SQLite","db"),("lexical","ar","برلين","trip"),("lexical","ar","سيجيت","backup"),
 ("lexical","ar","البنسلين","allergy"),("lexical","ar","الياسمين","mom"),("lexical","ar","الإيجار","money"),
 # cross-language: query in one language, memory in the other
 ("cross","en>ar","the user cried when the pet died","dog"),
 ("cross","en>ar","which database did we pick","db"),
 ("cross","ar>en","رحلة إلى ألمانيا","trip"),
 ("cross","ar>en","عيد ميلاد الأم","mom"),
]
FILL_EN = ["The meeting with the landlord was moved to Thursday afternoon.","Weather in Cairo was hot and dusty all week.","User asked for a recipe for lentil soup with lemon.","The build failed because a crate version was too old.","Plan to repaint the bedroom walls light green.","Remember to renew the car insurance before November.","The router needs a firmware update.","User likes to read science fiction before sleeping.","Football match on Saturday at seven with friends.","The dentist appointment is next Tuesday at ten.","Bought new running shoes, size 43.","The printer in the office jams on thick paper.","Discussed a refactor of the settings screen.","User listens to podcasts about history while walking.","Keyboard shortcuts for the terminal were rearranged.","Coffee beans from the local roaster taste better than the supermarket ones.","The garden tomatoes need watering every second day.","Notes about the Rust borrow checker and lifetimes.","The cat refuses to eat the new brand of food.","Library books are due on the 20th.","Tutorial on setting up a VPN at home.","The phone battery drains fast after the update.","User wants to repaint the bicycle blue.","Invoice number 5521 was paid by bank transfer.","Meeting notes: ship the prototype before the holidays.","The monitor flickers when the laptop wakes up.","Recipe: roast chicken with garlic and potatoes.","Dinner with cousins on Friday at their place.","Ideas for a birthday party with a football theme.","Reading list: three books on distributed systems."]
FILL_AR = ["تم تأجيل الاجتماع مع صاحب البيت إلى ظهر الخميس.","كان الطقس في القاهرة حارا ومغبرا طوال الأسبوع.","طلب المستخدم وصفة شوربة العدس بالليمون.","فشل البناء لأن إصدار المكتبة قديم جدا.","نية طلاء جدران غرفة النوم باللون الأخضر الفاتح.","تذكير بتجديد تأمين السيارة قبل نوفمبر.","الراوتر يحتاج إلى تحديث البرنامج الثابت.","يحب المستخدم قراءة الخيال العلمي قبل النوم.","مباراة كرة القدم يوم السبت الساعة السابعة مع الأصدقاء.","موعد طبيب الأسنان يوم الثلاثاء القادم في العاشرة.","اشترى المستخدم حذاء جري جديدا مقاس ٤٣.","الطابعة في المكتب تتعطل مع الورق السميك.","ناقشنا إعادة تنظيم شاشة الإعدادات.","يستمع المستخدم إلى بودكاست عن التاريخ أثناء المشي.","تغيرت اختصارات لوحة المفاتيح في الطرفية.","قهوة المحمصة المحلية ألذ من قهوة السوبرماركت.","طماطم الحديقة تحتاج إلى سقاية كل يومين.","ملاحظات عن مدقق الاستعارة في لغة رست.","القطة ترفض الطعام الجديد.","كتب المكتبة يجب إرجاعها يوم العشرين.","شرح إعداد شبكة افتراضية خاصة في البيت.","بطارية الهاتف تنفد بسرعة بعد التحديث.","يريد المستخدم دهان الدراجة باللون الأزرق.","تم دفع الفاتورة رقم ٥٥٢١ بتحويل بنكي.","ملاحظات الاجتماع: إطلاق النموذج الأولي قبل العطلة.","الشاشة ترتجف عند استيقاظ الحاسوب.","وصفة: دجاج مشوي بالثوم والبطاطس.","عشاء مع أبناء العم يوم الجمعة في بيتهم.","أفكار لحفلة عيد ميلاد بطابع كرة القدم.","قائمة قراءة: ثلاثة كتب عن الأنظمة الموزعة."]

def real_english(n=150, seed=1):
    """Distractors: real text from this machine's own Claude Code transcripts. Stays local, never printed."""
    out = []
    for f in glob.glob(os.path.expanduser("~/.claude/projects/-home-omar-AI*/*.jsonl")):
        try:
            for line in open(f):
                d = json.loads(line)
                m = d.get("message", {})
                c = m.get("content")
                if isinstance(c, str): texts = [c]
                elif isinstance(c, list): texts = [x.get("text", "") for x in c if isinstance(x, dict) and x.get("type") == "text"]
                else: continue
                for t in texts:
                    t = re.sub(r"\s+", " ", t).strip()
                    if 80 <= len(t) <= 400 and t.isascii() and "<" not in t: out.append(t)
        except Exception: pass
    random.Random(seed).shuffle(out)
    return out[:n]

# ---- keyword search: BM25 with light normalisation ----
AR_DIAC = re.compile("[ً-ٰٟـ]")
def norm(s):
    s = unicodedata.normalize("NFKC", s).lower()
    s = AR_DIAC.sub("", s)
    s = re.sub("[أإآ]", "ا", s).replace("ى", "ي").replace("ة", "ه")
    s = s.translate(str.maketrans("٠١٢٣٤٥٦٧٨٩", "0123456789"))
    return s
STOP = set("the a an of to in on for and or is are was were be do does did what which who when where how why that this it its with as at by from my me i you your we our they their user users any about has have had there than then so if not no can will would should".split()) | set("في من على الى عن مع هل ما ماذا كيف متى اين لماذا هذا هذه ذلك التي الذي كان كانت يكون هو هي نحن انا انت او ان لا".split())
def toks(s):
    out = []
    for w in re.findall(r"\w+", norm(s)):
        if w in STOP: continue
        if re.match(r"[؀-ۿ]", w):
            for p in ("وال", "بال", "كال", "فال", "لل", "ال", "و"):
                if w.startswith(p) and len(w) - len(p) >= 3: w = w[len(p):]; break
        out.append(w)
    return out
class BM25:
    def __init__(s, docs, k1=1.5, b=0.75):
        s.t = [Counter(toks(d)) for d in docs]; s.len = [sum(c.values()) for c in s.t]
        s.avg = sum(s.len) / len(docs); s.k1, s.b = k1, b; s.n = len(docs)
        s.df = Counter(w for c in s.t for w in c)
    def scores(s, q):
        r = [0.0] * s.n
        for w in set(toks(q)):
            if w not in s.df: continue
            idf = math.log(1 + (s.n - s.df[w] + .5) / (s.df[w] + .5))
            for i, c in enumerate(s.t):
                f = c.get(w, 0)
                if f: r[i] += idf * f * (s.k1 + 1) / (f + s.k1 * (1 - s.b + s.b * s.len[i] / s.avg))
        return r

# ---- embedding server ----
MODELS = {
 "embeddinggemma-2":  ("embeddinggemma-2-Q8_0.gguf",  "task: search result | query: ", "title: none | text: "),
 "embeddinggemma-1":  ("embeddinggemma-300M-Q8_0.gguf","task: search result | query: ", "title: none | text: "),
 "bge-m3":            ("bge-m3-Q8_0.gguf",            "", ""),
 "multilingual-e5-small": ("e5b/multilingual-e5-small-Q8_0.gguf", "query: ", "passage: "),
}
def pss(pid):
    for l in open(f"/proc/{pid}/smaps_rollup"):
        if l.startswith("Pss:"): return int(l.split()[1]) // 1024
def embed(texts):
    body = json.dumps({"input": texts}).encode()
    r = urllib.request.Request(f"http://127.0.0.1:{PORT}/v1/embeddings", body, {"Content-Type": "application/json"})
    d = json.load(urllib.request.urlopen(r, timeout=300))
    vs = []
    for x in sorted(d["data"], key=lambda x: x["index"]):
        v = x["embedding"]; n = math.sqrt(sum(a * a for a in v)) or 1
        vs.append([a / n for a in v])
    return vs
def cos(a, b): return sum(x * y for x, y in zip(a, b))

def rrf(*rankings, k=60):
    sc = Counter()
    for r in rankings:
        for pos, i in enumerate(r): sc[i] += 1 / (k + pos + 1)
    return [i for i, _ in sc.most_common()]
def rank(scores): return sorted(range(len(scores)), key=lambda i: -scores[i])
def rank_kw(scores): return [i for i in rank(scores) if scores[i] > 0]  # keyword list: only real matches

def main(names):
    docs = []  # (id or None, lang, text)
    for k, (en, ar) in ITEMS.items(): docs += [(k, "en", en), (k, "ar", ar)]
    docs += [(None, "en", t) for t in FILL_EN] + [(None, "ar", t) for t in FILL_AR]
    real = real_english()
    docs += [(None, "en", t) for t in real]
    print(f"corpus: {len(docs)} docs ({sum(1 for d in docs if d[0])} gold, {len(FILL_EN)+len(FILL_AR)} written fillers, {len(real)} real English chunks)")
    texts = [d[2] for d in docs]
    bm = BM25(texts)
    def pool(q):  # candidate indices allowed for this query, and gold indices
        kind, lang = q[0], q[1]
        want = {"en": "en", "ar": "ar", "en>ar": "ar", "ar>en": "en"}[lang]
        idx = [i for i, d in enumerate(docs) if d[1] == want]
        gold = {i for i in idx if docs[i][0] == q[3]}
        return idx, gold
    def score(method_rank_fn, label, res):
        for grp in ("lexical-en", "paraphrase-en", "lexical-ar", "paraphrase-ar", "cross"):
            hits = rr = n = 0
            for q in QUERIES:
                g = "cross" if q[0] == "cross" else f"{q[0]}-{q[1]}"
                if g != grp: continue
                idx, gold = pool(q)
                ranked = [i for i in method_rank_fn(q) if i in set(idx)]
                n += 1
                top = ranked[:5]
                hits += bool(gold & set(top))
                for pos, i in enumerate(ranked):
                    if i in gold: rr += 1 / (pos + 1); break
            res.setdefault(label, {})[grp] = (hits, n, rr / n)
    results = {"keyword (BM25)": {}}
    score(lambda q: rank(bm.scores(q[2])), "keyword (BM25)", results)
    perf = {}
    for name in names:
        fn, qp, dp = MODELS[name]
        path = os.path.join(HERE, "models", fn)
        t0 = time.time()
        p = subprocess.Popen([SERVER, "-m", path, "--embedding", "--port", str(PORT), "-ngl", "0", "-c", "2048", "-ub", "2048", "-np", "1", "-t", "6"], stdout=subprocess.DEVNULL, stderr=open(f"/tmp/ls-{name}.log", "w"), env={**os.environ, "LD_LIBRARY_PATH": LIBS})
        try:
            while True:
                try:
                    urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2); break
                except Exception:
                    if p.poll() is not None: raise SystemExit(f"{name}: server died, see /tmp/ls-{name}.log")
                    time.sleep(0.2)
            load = time.time() - t0
            embed(["warm up"])
            pss_idle = pss(p.pid)
            t1 = time.time(); dv = []
            for i in range(0, len(texts), 16): dv += embed([dp + t for t in texts[i:i + 16]])
            doc_s = time.time() - t1
            t2 = time.time(); qv = {}
            for q in QUERIES: qv[q[2]] = embed([qp + q[2]])[0]
            q_ms = (time.time() - t2) / len(QUERIES) * 1000
            pss_busy = pss(p.pid)
            dim = len(dv[0])
            for label, fnrank in (
                (f"{name} (embedding only)", lambda q: rank([cos(qv[q[2]], d) for d in dv])),
                (f"{name} (hybrid: BM25 + embedding)", lambda q: rrf(rank_kw(bm.scores(q[2])), rank([cos(qv[q[2]], d) for d in dv]))),
            ): score(fnrank, label, results)
            perf[name] = dict(file_mb=os.path.getsize(path)//1048576, dim=dim, load_s=round(load,1), ram_mb_after_load=pss_idle, ram_mb_after_embedding=pss_busy, docs_per_s=round(len(texts)/doc_s,1), query_ms=round(q_ms,1))
        finally:
            p.terminate(); p.wait()
    print("\nTop-5 hit rate (and MRR) per query group:")
    grps = ("lexical-en", "paraphrase-en", "lexical-ar", "paraphrase-ar", "cross")
    print(f"{'method':46}" + "".join(f"{g:>18}" for g in grps))
    for label, r in results.items():
        print(f"{label:46}" + "".join(f"{f'{r[g][0]}/{r[g][1]} mrr={r[g][2]:.2f}':>18}" for g in grps))
    print("\nPerformance (CPU, 6 threads, Q8):"); [print(k, v) for k, v in perf.items()]
    json.dump({"results": results, "perf": perf}, open(os.path.join(HERE, "results.json"), "w"), ensure_ascii=False, indent=1)
if __name__ == "__main__": main(sys.argv[1:] or list(MODELS))
