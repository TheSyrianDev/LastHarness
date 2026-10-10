# Spike 4 (throwaway): can SQLite (FTS5 + vectors as blobs) be the memory store? Python stdlib + numpy.
import os, random, sqlite3, sys, time, subprocess, threading, tempfile
import numpy as np
D = 768
EN = "memory project decision user prefers short answers database storage trip flight allergy salary rent hobby backup drive laptop model server prompt tool call rule skill folder password health finance birthday garden recipe meeting invoice shoes dentist router keyboard coffee".split()
AR = "ذاكرة مشروع قرار مستخدم يفضل إجابات قصيرة قاعدة بيانات تخزين رحلة طيران حساسية راتب إيجار هواية نسخة احتياطية قرص حاسوب نموذج خادم أداة قاعدة مهارة مجلد كلمة سر صحة مال عيد ميلاد حديقة وصفة اجتماع فاتورة حذاء طبيب جهاز لوحة قهوة".split()
SCHEMA = """
pragma journal_mode=wal; pragma synchronous=normal;
create table pages(id integer primary key, owner text not null, scope text not null, layer text not null, privacy text not null,
  text text not null, created integer not null, archived integer not null default 0);
create virtual table pages_fts using fts5(text, content='pages', content_rowid='id', tokenize='unicode61 remove_diacritics 2');
create table embeddings(page_id integer primary key references pages(id), model text not null, vec blob not null);
create table history(seq integer primary key, at integer, page_id integer, op text, old_text text, new_text text);  -- append-only
create trigger pages_ai after insert on pages begin
  insert into pages_fts(rowid, text) values (new.id, new.text);
  insert into history(at, page_id, op, new_text) values (strftime('%s','now'), new.id, 'insert', new.text); end;
create trigger pages_au after update of text on pages begin
  insert into pages_fts(pages_fts, rowid, text) values ('delete', old.id, old.text);
  insert into pages_fts(rowid, text) values (new.id, new.text);
  insert into history(at, page_id, op, old_text, new_text) values (strftime('%s','now'), new.id, 'update', old.text, new.text); end;
create trigger history_no_update before update on history begin select raise(abort, 'history is append-only'); end;
create trigger history_no_delete before delete on history begin select raise(abort, 'history is append-only'); end;
"""
def text(rng):
    w = EN if rng.random() < .5 else AR
    return " ".join(rng.choice(w) for _ in range(rng.randint(25, 60)))
def build(path, n, rng, rs):
    if os.path.exists(path): os.remove(path)
    c = sqlite3.connect(path); c.executescript(SCHEMA)
    t0 = time.time()
    with c:
        c.executemany("insert into pages(owner,scope,layer,privacy,text,created) values ('me','personal','facts','normal',?,?)", ((text(rng), i) for i in range(n)))
    t_pages = time.time() - t0
    t0 = time.time()
    with c:
        v = rs.standard_normal((n, D), dtype=np.float32); v /= np.linalg.norm(v, axis=1, keepdims=True)
        c.executemany("insert into embeddings values (?, 'bench', ?)", ((i + 1, v[i].tobytes()) for i in range(n)))
    t_vec = time.time() - t0
    return c, v, t_pages, t_vec
def main():
    rng = random.Random(1); rs = np.random.default_rng(1)
    d = tempfile.mkdtemp(prefix="lh-store-", dir=os.path.dirname(os.path.abspath(__file__)))
    print(f"sqlite {sqlite3.sqlite_version}, numpy {np.__version__}, dim {D}")
    for n in (1_000, 10_000, 100_000):
        path = f"{d}/m{n}.db"
        c, v, tp, tv = build(path, n, rng, rs)
        size = os.path.getsize(path) / 1048576
        # keyword (FTS5), English and Arabic
        qs = ["database", "رحلة", "birthday", "حساسية", "folder", "قرص"]
        ts = []
        for q in qs * 10:
            t = time.time(); c.execute("select rowid from pages_fts where pages_fts match ? order by rank limit 10", (q,)).fetchall(); ts.append(time.time() - t)
        fts_ms = sum(ts) / len(ts) * 1000
        # vectors: cold load of all vectors from SQLite, then scan
        t = time.time(); rows = c.execute("select vec from embeddings order by page_id").fetchall()
        m = np.frombuffer(b"".join(r[0] for r in rows), dtype=np.float32).reshape(-1, D); load_s = time.time() - t
        q = v[rs.integers(n)]; ts = []
        for _ in range(30):
            t = time.time(); top = np.argpartition(-(m @ q), 10)[:10]; ts.append(time.time() - t)
        scan_ms = sum(ts) / len(ts) * 1000
        # update through the trigger path, then check history and FTS agree
        c.execute("update pages set text='zzupdated unique token' where id=5"); c.commit()
        hit = c.execute("select rowid from pages_fts where pages_fts match 'zzupdated'").fetchall()
        hist = c.execute("select count(*) from history where page_id=5").fetchone()[0]
        try: c.execute("delete from history where seq=1"); blocked = False
        except sqlite3.DatabaseError: blocked = True
        print(f"N={n:>7}: insert pages {tp:5.2f}s, vectors {tv:5.2f}s | file {size:7.1f} MB | FTS5 query {fts_ms:5.2f} ms | vector load {load_s:5.2f}s ({m.nbytes/1048576:.0f} MB in RAM), scan {scan_ms:6.2f} ms | edit seen by FTS {bool(hit)}, history rows {hist}, history delete blocked {blocked}")
        c.close()
    # concurrent: reader latency while a writer commits continuously (WAL)
    path = f"{d}/m10000.db"
    stop = False; wcount = [0]
    def writer():
        w = sqlite3.connect(path)
        while not stop:
            with w: w.execute("insert into pages(owner,scope,layer,privacy,text,created) values ('me','personal','facts','normal',?,0)", (text(rng),))
            wcount[0] += 1
        w.close()
    th = threading.Thread(target=writer); th.start()
    r = sqlite3.connect(path); ts = []
    t_end = time.time() + 5
    while time.time() < t_end:
        t = time.time(); r.execute("select rowid from pages_fts where pages_fts match 'database' order by rank limit 10").fetchall(); ts.append(time.time() - t)
    stop = True; th.join()
    ts.sort()
    print(f"WAL: {wcount[0]} commits in 5 s while reading; reader median {ts[len(ts)//2]*1000:.2f} ms, worst {ts[-1]*1000:.1f} ms over {len(ts)} reads")
    # crash safety: kill -9 a writer mid-flight, then integrity check
    code = f"import sqlite3,sys\nc=sqlite3.connect({path!r})\nwhile True:\n  with c: c.execute(\"insert into pages(owner,scope,layer,privacy,text,created) values ('me','personal','facts','normal','crash test text',0)\")\n"
    p = subprocess.Popen([sys.executable, "-I", "-c", code]); time.sleep(2); p.kill(); p.wait()
    c = sqlite3.connect(path)
    ok = c.execute("pragma integrity_check").fetchone()[0]
    c.execute("insert into pages_fts(pages_fts) values ('integrity-check')")
    print(f"after kill -9 mid-write: integrity_check = {ok}, FTS integrity-check passed, rows = {c.execute('select count(*) from pages').fetchone()[0]}, history rows = {c.execute('select count(*) from history').fetchone()[0]}")
    import shutil; shutil.rmtree(d)
main()
