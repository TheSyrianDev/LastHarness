// LastHarness browser UI. Plain and unstyled on purpose: structure and behaviour only. Every piece of
// text from the server goes in with textContent, never as HTML.
(() => {
'use strict';
const $ = (s, r = document) => r.querySelector(s);
const enc = new TextEncoder();
const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
const gate = $('#gate');
const statusEl = $('#status');
const say = (t) => { statusEl.textContent = t || ''; };

// h('button', {onclick: fn, type: 'button'}, 'Text', child...)
function h(tag, attrs, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v === false || v == null) continue;
    if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (v === true) e.setAttribute(k, '');
    else e.setAttribute(k, v);
  }
  for (const k of kids.flat()) if (k != null && k !== false) e.append(k.nodeType ? k : document.createTextNode(String(k)));
  return e;
}
const field = (label, input) => h('p', {}, h('label', {}, label + ' ', input));
// Pages other than the chat open in the side panel; before sign-in they fill the gate.
function show(title, ...nodes) {
  const host = signedIn ? $('#panel-body') : gate;
  if (signedIn) { $('#panel').hidden = false; $('#panel').classList.remove('dock'); $('#dock-tabs').replaceChildren(); $('#panel-title').textContent = title; }
  host.replaceChildren(h('h2', { tabindex: '-1', id: 'view-title' }, title), ...nodes);
  $('#view-title').focus();
}

// ---- the device key, kept in IndexedDB as a key the browser cannot export ----
function db() {
  return new Promise((res, rej) => {
    const r = indexedDB.open('lastharness', 1);
    r.onupgradeneeded = () => r.result.createObjectStore('kv');
    r.onsuccess = () => res(r.result);
    r.onerror = () => rej(r.error);
  });
}
async function kv(op, key, val) {
  const d = await db();
  return new Promise((res, rej) => {
    const t = d.transaction('kv', 'readwrite').objectStore('kv');
    const q = op === 'get' ? t.get(key) : op === 'put' ? t.put(val, key) : t.delete(key);
    q.onsuccess = () => res(q.result);
    q.onerror = () => rej(q.error);
  });
}

async function signIn(device) {
  const c = await fetch('/auth/challenge', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ device_id: device.id }) });
  if (!c.ok) return false;
  const { nonce } = await c.json();
  const sig = await crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, device.privateKey, enc.encode(nonce));
  const v = await fetch('/auth/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ device_id: device.id, nonce, signature: hex(sig) }) });
  return v.ok;
}

let device = null;
let signedIn = false;
async function api(path, opts = {}, retry = true) {
  const o = { credentials: 'same-origin', ...opts, headers: { ...(opts.body ? { 'content-type': 'application/json' } : {}), ...(opts.headers || {}) } };
  if (opts.body && typeof opts.body !== 'string') o.body = JSON.stringify(opts.body);
  const r = await fetch(path, o);
  if (r.status === 401 && retry && device && (await signIn(device))) return api(path, opts, false);
  if (r.status === 401) { signedIn = false; await start(); throw new Error('Signed out'); }
  return r;
}
async function json(path, opts) {
  const r = await api(path, opts);
  const text = await r.text();
  let v = {};
  try { v = JSON.parse(text); } catch { /* not JSON */ }
  if (!r.ok) throw new Error(v.error || text.slice(0, 200) || `The core answered ${r.status}`);
  return v;
}
// Server-sent events over fetch (POST turns, GET runs). Calls onEvent({type, ...}) for each.
async function stream(path, opts, onEvent) {
  const r = await api(path, opts);
  if (!r.ok) { const v = await r.json().catch(() => ({})); throw new Error(v.error || `The core answered ${r.status}`); }
  const rd = r.body.getReader();
  const dec = new TextDecoder();
  let buf = '';
  for (;;) {
    const { value, done } = await rd.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    let i;
    while ((i = buf.indexOf('\n\n')) >= 0) {
      const block = buf.slice(0, i); buf = buf.slice(i + 2);
      const data = block.split('\n').filter((l) => l.startsWith('data:')).map((l) => l.slice(5).trim()).join('\n');
      if (data) { try { onEvent(JSON.parse(data)); } catch { /* keep-alive or partial */ } }
    }
  }
}

// ---- pairing ----
function randomPin() {
  const a = new Uint32Array(1);
  let n;
  do { crypto.getRandomValues(a); n = a[0]; } while (n >= 4294000000); // no modulo bias
  return String(n % 1000000).padStart(6, '0');
}
async function pairView() {
  $('#app').hidden = true;
  const name = h('input', { type: 'text', value: navigator.userAgent.includes('Firefox') ? 'Firefox' : 'Browser', autocomplete: 'off' });
  const out = h('div', { 'aria-live': 'polite' });
  const go = h('button', { type: 'button' }, 'Pair this browser');
  go.addEventListener('click', async () => {
    go.disabled = true;
    try {
      const kp = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign', 'verify']);
      const jwk = await crypto.subtle.exportKey('jwk', kp.publicKey);
      const pub = { kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y };
      const pin = randomPin();
      const mac = await crypto.subtle.importKey('raw', enc.encode(pin), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
      const proof = hex(await crypto.subtle.sign('HMAC', mac, enc.encode(`${pub.x}.${pub.y}`)));
      const r = await fetch('/pair/start', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name: name.value.trim() || 'Browser', public_key: pub, proof }) });
      const s = await r.json();
      if (!r.ok) throw new Error(s.error || 'Could not start pairing');
      out.replaceChildren(
        h('p', {}, 'Your PIN is:'), h('p', {}, h('strong', { id: 'pin' }, pin.slice(0, 3) + ' ' + pin.slice(3))),
        h('p', {}, 'On the machine that runs LastHarness, type: ', h('code', {}, `lh devices accept ${pin}`)),
        h('p', {}, 'The terminal also shows the certificate fingerprint. Compare it with the one in this browser\'s certificate details (the page you clicked through at the warning) before you accept.'),
        h('p', {}, 'Waiting for you to accept it there. The request lasts 5 minutes.'));
      $('#pin').focus?.();
      for (let i = 0; i < 150; i++) {
        await new Promise((ok) => setTimeout(ok, 2000));
        const st = await (await fetch(`/pair/status/${s.id}`)).json();
        if (st.state === 'paired') {
          device = { id: st.device_id, privateKey: kp.privateKey, name: name.value };
          await kv('put', 'device', device);
          if (await signIn(device)) return start();
          throw new Error('Paired, but signing in failed');
        }
        if (st.state === 'expired') throw new Error('The request ran out of time. Pair again.');
      }
      throw new Error('The request ran out of time. Pair again.');
    } catch (e) { out.replaceChildren(h('p', { role: 'alert' }, e.message)); go.disabled = false; }
  });
  show('Pair this browser',
    h('p', {}, 'This browser is not paired with the LastHarness on this machine yet. Pairing gives it its own key, so it never needs a password. You can remove it later under Settings.'),
    field('A name for this browser', name), go, out);
}

// ---- themes (desktop app only: the browser keeps the default look) ----
const THEME_VARS = { background: '--bg', surface: '--surface', sunken: '--sunken', text: '--text', muted: '--muted', accent: '--accent', accent_text: '--accent-text', button: '--button', button_hover: '--button-hover', border: '--border', danger: '--danger', bubble: '--bubble', warn: '--warn', good: '--good', focus_ring: '--focus' };
let themes = [];
let curTheme = null;
// The Glass look tinted around up to three picked colours, as in GPUI (lh-gui theme.rs tint_glass): the first leads the
// accent and the top of the background, the last the bottom, the middle one the panels. The tint backs off until every
// readability check passes, so it never makes words faint.
const GLASS_COLS = 24, GLASS_ROWS = 6;
const rgbOf = (hex) => { let x = String(hex || '').replace('#', ''); if (x.length === 3) x = [...x].map((c) => c + c).join(''); if (x.length === 6) x += 'ff'; const n = (i) => parseInt(x.slice(i, i + 2), 16); return x.length === 8 && !isNaN(n(0)) ? { r: n(0), g: n(2), b: n(4), a: n(6) } : null; };
const hexOf = (c) => '#' + [c.r, c.g, c.b].map((v) => v.toString(16).padStart(2, '0')).join('') + (c.a === 255 ? '' : c.a.toString(16).padStart(2, '0'));
const mixC = (a, b, t) => { const k = Math.min(1, Math.max(0, t)); const m = (x, y) => Math.round(x + (y - x) * k); return { r: m(a.r, b.r), g: m(a.g, b.g), b: m(a.b, b.b), a: m(a.a, b.a) }; };
const overC = (fg, bg) => ({ ...mixC({ ...bg, a: 255 }, { ...fg, a: 255 }, fg.a / 255), a: 255 });
const lumC = (c) => { const f = (v) => { const x = v / 255; return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b); };
const contrastC = (a, b) => { const x = lumC(a), y = lumC(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };
function glassField(col, row) {
  const hh = col / GLASS_COLS, l = 0.85 - 0.55 * row / (GLASS_ROWS - 1), sat = 0.8;
  const q = l < 0.5 ? l * (1 + sat) : l + sat - l * sat, p = 2 * l - q;
  const ch = (t) => { t = ((t % 1) + 1) % 1; const v = t < 1 / 6 ? p + (q - p) * 6 * t : t < 0.5 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p; return Math.round(v * 255); };
  return hexOf({ r: ch(hh + 1 / 3), g: ch(hh), b: ch(hh - 1 / 3), a: 255 });
}
function readable(c, dark) {
  const bg = overC(c.background, dark ? { r: 0, g: 0, b: 0, a: 255 } : { r: 255, g: 255, b: 255, a: 255 });
  const [surface, sunken, button, accent] = [c.surface, c.sunken || c.surface, c.button || c.surface, c.accent].map((x) => overC(x, bg));
  const checks = [[c.text, surface, 4.5], [c.text, bg, 4.5], [c.text, sunken, 4.5], [c.text, button, 4.5], [c.text, overC(c.bubble || c.surface, bg), 4.5], [c.muted, surface, 4.5], [c.muted, bg, 4.5], [c.muted, sunken, 4.5],
    [c.accent_text || c.text, accent, 4.5], [c.danger || c.text, surface, 4.5], [c.warn || c.text, surface, 4.5], [c.good || c.text, surface, 4.5], [accent, surface, 3], [c.focus_ring || c.accent, bg, 3]];
  return checks.every(([fg, back, min]) => contrastC(overC(fg, back), back) >= min);
}
function glassTinted(t) {
  if (!t || t.name !== 'Glass') return t;
  const picks = jsonPref('glass-colours', []).map(rgbOf).filter(Boolean).slice(0, 3);
  if (!picks.length || !t.colors) return t;
  const C = {}; for (const [k, v] of Object.entries(t.colors)) { const c = rgbOf(v); if (c) C[k] = c; }
  const first = picks[0], last = picks[picks.length - 1], mid = picks[1] || first, dark = t.mode === 'dark';
  const black = { r: 0, g: 0, b: 0, a: 255 }, white = { r: 255, g: 255, b: 255, a: 255 };
  const towards = (c, p, k) => ({ ...mixC(c, { ...p, a: c.a }, k), a: c.a });
  let s = Math.min(1, Math.max(0, +(pref('glass-opacity') ?? 0.5) || 0));
  if (pref('glass-opacity') === null) s = 0.5;
  for (;;) {
    const c = { ...C };
    c.background = towards(C.background, mixC(first, last, 0.5), s * 0.45);
    c.surface = towards(C.surface, mid, s * 0.25); c.button = towards(C.button, mid, s * 0.3); c.button_hover = towards(C.button_hover, mid, s * 0.35);
    const bg = overC(c.background, dark ? black : white), surface = overC(c.surface, bg);
    let accent = { ...first, a: 255 };
    for (let i = 0; i < 20 && contrastC(accent, surface) < 3.2; i++) accent = mixC(accent, dark ? white : black, 0.12);
    c.accent = accent; c.focus_ring = accent;
    const ink10 = mixC(black, accent, 0.1);
    c.accent_text = contrastC(white, accent) >= contrastC(ink10, accent) ? white : ink10;
    const ink = dark ? white : black;
    for (const k of ['danger', 'warn', 'good', 'muted']) { if (!c[k]) continue; for (let i = 0; i < 12 && !(contrastC(c[k], surface) >= 4.7 && contrastC(c[k], bg) >= 4.7); i++) c[k] = mixC(c[k], ink, 0.08); }
    if (readable(c, dark) || s <= 0) {
      if (s <= 0 && !readable(c, dark)) return t;
      const out = { ...t, colors: { ...t.colors } }; for (const [k, v] of Object.entries(c)) out.colors[k] = hexOf(v);
      const b = t.background; if (b?.kind === 'gradient') { const f = rgbOf(b.from), to = rgbOf(b.to); out.background = { ...b, from: f ? hexOf(towards(f, first, s * 0.6)) : b.from, to: to ? hexOf(towards(to, last, s * 0.6)) : b.to }; }
      return out;
    }
    s = Math.max(0, s - 0.1);
  }
}
function applyTheme(t) {
  curTheme = t; t = glassTinted(t); if (typeof fontInstalled === 'function') applyLayout();
  const st = document.documentElement.style;
  for (const [k, v] of Object.entries(THEME_VARS)) if (t?.colors?.[k]) st.setProperty(v, t.colors[k]);
  if (t?.shape?.roundness != null) { st.setProperty('--radius', `${t.shape.roundness}px`); st.setProperty('--inner', `${Math.max(3, t.shape.roundness - 8)}px`); }
  // motion as the desktop app has it: a press answers in 140 ms and a panel moves in 220 ms, easing out; a look with
  // duration 0 turns both off
  const still = t?.animation?.duration_ms === 0;
  st.setProperty('--dur', still ? '0ms' : '140ms'); st.setProperty('--dur-panel', still ? '0ms' : '220ms');
  st.colorScheme = t?.mode === 'dark' ? 'dark' : 'light';
  // the window background: a gradient where the look has one, and frosted panels for the glass looks
  const b = t?.background;
  document.body.style.background = b?.kind === 'gradient' ? `linear-gradient(${(b.angle || 0) + 90}deg, ${b.from}, ${b.to})` : '';
  const glass = t?.material?.panels && t.material.panels !== 'none';
  document.documentElement.toggleAttribute('data-glass', !!glass);
  st.setProperty('--glass-blur', glass ? `${(t.material.blur || 6) * 3}px` : '0px');
  // the Opacity choice: how much of the backdrop shows through the frosted panels
  const op = pref('glass-opacity') === null ? 0.5 : Math.min(1, Math.max(0, +pref('glass-opacity') || 0));
  st.setProperty('--glass-fill', `${Math.round(88 - op * 50)}%`);
}
// when storage is off (a private window), choices last until the page closes
const memPrefs = new Map();
const pref = (k, v) => { try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch { if (v === undefined) return memPrefs.get(k) ?? null; memPrefs.set(k, String(v)); } };
// The core stamps each build into the page. On a new build, stored keys that are not a choice the person made are
// dropped, so an old default never hides a new one. The device key lives in IndexedDB and is never touched here.
const CHOSEN = ['look', 'look-mode', 'chat-width', 'side-width', 'font-latin', 'font-arabic', 'fonts-added', 'width-presets', 'pages-off', 'direction', 'name', 'stops', 'stop-count', 'stop-titles', 'model', 'folded', 'settings-page', 'pinned', 'settle-days', 'glass-colours', 'glass-opacity'];
function freshBuild() {
  const id = document.querySelector('meta[name="lh-build"]')?.content;
  if (!id) return;
  try {
    const was = localStorage.getItem('build');
    if (was === id) return;
    // builds before the stamp saved the model and the theme on their own, so those are not taken as choices
    const keep = was ? CHOSEN : CHOSEN.filter((k) => k !== 'model');
    for (const k of Object.keys(localStorage)) if (!keep.includes(k) && !k.startsWith('font-data-')) localStorage.removeItem(k);
    localStorage.setItem('build', id);
  } catch { /* storage may be off */ }
}
freshBuild();
async function loadThemes() {
  // the desktop app's proxy adds the person's own looks; a plain browser gets the built-in ones from the core
  let r = await fetch('/desktop.json', { credentials: 'same-origin' }).catch(() => null);
  if (!r?.ok) r = await fetch('/themes.json', { credentials: 'same-origin' }).catch(() => null);
  if (!r?.ok) return;
  const d = await r.json();
  themes = d.themes || [];
  if (d.name) userName = d.name;
  applyTheme(themes.find((t) => `${t.name}/${t.mode}` === themeKey()) || themes.find((t) => t.name === lookFamily()) || themes[0]);
  greet();
}
function applyLayout() {
  const st = document.documentElement.style;
  const w = pref('chat-width') || '860';
  st.setProperty('--col', w === 'full' ? '100%' : `${w}px`);
  st.setProperty('--side', `${+pref('side-width') || 260}px`);
  // order: the font chosen in Settings, then the look's own font, then the default
  const tf = curTheme?.fonts || {};
  const first = (v) => [].concat(v || []).find((x) => fontInstalled(x)) || '';
  const f = [pref('font-latin') || first(tf.latin), pref('font-arabic') || first(tf.arabic)].filter(Boolean).map((x) => `"${x.replace(/"/g, '')}"`);
  const mono = first(tf.mono); if (mono) st.setProperty('--mono', `"${mono}", ui-monospace, monospace`); else st.removeProperty('--mono');
  if (f.length) st.setProperty('--font', `${f.join(', ')}, system-ui, sans-serif`); else st.removeProperty('--font');
  const d = pref('direction') || 'auto';
  document.documentElement.dir = d === 'auto' ? (/^ar\b/.test(navigator.language) ? 'rtl' : 'ltr') : d;
}
// as in GPUI (lh-gui appearance.rs): the look is a family, and light or dark is its own choice: 'system' follows
// the desktop, 'light' and 'dark' are explicit. Picking a family never touches the mode.
const sysDark = () => matchMedia('(prefers-color-scheme: dark)').matches;
function lookFamily() { return pref('look') || (pref('theme') || '').split('/')[0] || 'Calm'; }
function lookMode() { return pref('look-mode') || ((pref('theme') || '').split('/')[1] || 'system'); }
function themeKey() { const m = lookMode(); return `${lookFamily()}/${m === 'system' ? (sysDark() ? 'dark' : 'light') : m}`; }
function setLook(family, mode) { if (family) pref('look', family); if (mode) pref('look-mode', mode); pref('theme', ''); const k = themeKey(); applyTheme(themes.find((t) => `${t.name}/${t.mode}` === k) || themes.find((t) => t.name === lookFamily())); }
matchMedia('(prefers-color-scheme: dark)').addEventListener?.('change', () => { if (lookMode() === 'system' && themes.length) setLook(); });
function setTheme(key) { const [f, m] = key.split('/'); setLook(f, m); }

// ---- answers as Markdown (built as elements, never as HTML from the model) ----
function inline(text) {
  const out = [];
  const re = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\*[^*\s][^*]*\*)|(\[[^\]]+\]\([^)\s]+\))/g;
  let last = 0; let m;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const t = m[0];
    if (m[1]) out.push(h('code', {}, t.slice(1, -1)));
    else if (m[2]) out.push(h('strong', {}, t.slice(2, -2)));
    else if (m[3]) out.push(h('em', {}, t.slice(1, -1)));
    else {
      const [, label, href] = t.match(/^\[([^\]]+)\]\(([^)\s]+)\)$/);
      out.push(/^https?:\/\//.test(href) ? h('a', { href, target: '_blank', rel: 'noopener noreferrer' }, label) : label);
    }
    last = m.index + t.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}
function markdown(src) {
  const lines = src.replace(/\r/g, '').split('\n');
  const out = [];
  let i = 0;
  const cells = (l) => l.trim().replace(/^\||\|$/g, '').split('|').map((c) => c.trim());
  while (i < lines.length) {
    const l = lines[i];
    if (/^```/.test(l)) {
      const lang = l.slice(3).trim(); const body = [];
      for (i++; i < lines.length && !/^```/.test(lines[i]); i++) body.push(lines[i]);
      i++;
      out.push(h('pre', { class: 'code' }, h('code', { 'data-lang': lang || null }, body.join('\n'))));
    } else if (/^#{1,6}\s/.test(l)) {
      const n = l.match(/^#+/)[0].length;
      out.push(h(`h${Math.min(n + 2, 6)}`, {}, inline(l.replace(/^#+\s*/, '')))); i++;
    } else if (/^\s*\|.*\|\s*$/.test(l) && /^\s*\|?\s*:?-{3,}/.test(lines[i + 1] || '')) {
      const head = cells(l); const rows = [];
      for (i += 2; i < lines.length && /^\s*\|.*\|\s*$/.test(lines[i]); i++) rows.push(cells(lines[i]));
      out.push(tableEl({ columns: head, rows }, inline));
    } else if (/^\s*([-*+]|\d+[.)])\s/.test(l)) {
      const ordered = /^\s*\d/.test(l); const items = [];
      for (; i < lines.length && /^\s*([-*+]|\d+[.)])\s/.test(lines[i]); i++) items.push(h('li', {}, inline(lines[i].replace(/^\s*([-*+]|\d+[.)])\s+/, ''))));
      out.push(h(ordered ? 'ol' : 'ul', {}, items));
    } else if (/^>\s?/.test(l)) {
      const q = [];
      for (; i < lines.length && /^>\s?/.test(lines[i]); i++) q.push(lines[i].replace(/^>\s?/, ''));
      out.push(h('blockquote', {}, inline(q.join(' '))));
    } else if (/^\s*([-*_])\s*\1\s*\1[\s\-*_]*$/.test(l)) { out.push(h('hr', {})); i++; }
    else if (!l.trim()) i++;
    else {
      const p = [];
      for (; i < lines.length && lines[i].trim() && !/^(```|#{1,6}\s|>|\s*([-*+]|\d+[.)])\s|\s*\|)/.test(lines[i]); i++) p.push(lines[i]);
      if (!p.length) p.push(lines[i++]);
      out.push(h('p', {}, inline(p.join('\n'))));
    }
  }
  return out;
}
function renderAnswer(body, text) { body.dataset.raw = text; body.classList.add('md'); body.replaceChildren(...markdown(text)); }

// ---- generative answers: tables, charts and small apps from the `show` tool ----
function tableEl(t, fmt = (x) => x) {
  // like GPUI's table_view: muted small header, rows with every second one shaded, number columns on the right,
  // the caption under the table
  const num = t.columns.map((_, c) => t.rows.length > 0 && t.rows.every((r) => typeof r[c] === 'number' || r[c] == null));
  const short = (v) => Math.abs(v) >= 1e6 ? `${+(v / 1e6).toFixed(1)}M` : Math.abs(v) >= 1e4 ? `${+(v / 1e3).toFixed(1)}k` : `${+v.toFixed(2)}`;
  const cell = (tag, v, c) => h(tag, { scope: tag === 'th' ? 'col' : null, class: num[c] ? 'num' : null }, fmt(v == null ? '' : tag === 'td' && num[c] ? short(v) : String(v)));
  return h('div', { class: 'answer-table' },
    h('div', { class: 'answer-table-scroll', tabindex: '0' },
      h('table', {},
        h('thead', {}, h('tr', {}, t.columns.map((x, c) => cell('th', x, c)))),
        h('tbody', {}, t.rows.map((r) => h('tr', {}, r.map((v, c) => cell('td', v, c))))))),
    t.caption ? h('div', { class: 'answer-caption' }, t.caption) : null);
}
const SVG = 'http://www.w3.org/2000/svg';
function s(tag, attrs, ...kids) { const e = document.createElementNS(SVG, tag); for (const [k, v] of Object.entries(attrs || {})) e.setAttribute(k, v); e.append(...kids); return e; }
const SERIES = ['var(--accent)', '#d9822b', '#2e9e6a', '#b5479b', '#7a6ad8', '#c0392b', '#3a9bb5', '#8a8a3a', '#d65a7a', '#4a7a3a', '#9a6a3a', '#5a5a8a'];
function chartEl(c) {
  const W = 640, H = 300, L = 48, B = 36, T = 12, R = 12;
  const svg = s('svg', { viewBox: `0 0 ${W} ${H}`, role: 'img', 'aria-label': c.caption || 'Chart', class: 'chart' });
  const fmt = (n) => Math.abs(n) >= 1e6 ? `${+(n / 1e6).toFixed(1)}M` : Math.abs(n) >= 1e3 ? `${+(n / 1e3).toFixed(1)}k` : `${+n.toFixed(2)}`;
  const legend = h('div', { class: 'legend' }, c.series.map((x, i) => h('span', {}, h('i', { class: `sw s${i}` }), x.name || `Series ${i + 1}`)));
  if (c.type === 'pie') {
    const vals = c.series[0].values.map((v) => Math.max(0, v)); const sum = vals.reduce((a, b) => a + b, 0) || 1;
    let a0 = -Math.PI / 2; const cx = W / 2, cy = H / 2, r = H / 2 - 16;
    vals.forEach((v, i) => {
      const a1 = a0 + (v / sum) * Math.PI * 2; const big = a1 - a0 > Math.PI ? 1 : 0;
      const p = (a) => `${cx + r * Math.cos(a)} ${cy + r * Math.sin(a)}`;
      const d = vals.length === 1 || v === sum ? `M ${cx - r} ${cy} a ${r} ${r} 0 1 0 ${2 * r} 0 a ${r} ${r} 0 1 0 ${-2 * r} 0` : `M ${cx} ${cy} L ${p(a0)} A ${r} ${r} 0 ${big} 1 ${p(a1)} Z`;
      svg.append(s('path', { d, fill: SERIES[i % SERIES.length], stroke: 'var(--surface)', 'stroke-width': 2 }, s('title', {}, `${c.labels[i]}: ${v} (${Math.round((v / sum) * 100)}%)`)));
      a0 = a1;
    });
    legend.replaceChildren(...c.labels.map((l, i) => h('span', {}, h('i', { class: `sw s${i % SERIES.length}` }), l)));
    return h('figure', { class: 'answer-chart' }, svg, legend, c.caption ? h('figcaption', {}, c.caption) : null);
  }
  // like GPUI's chart_view: a 200px plot, a 44px value axis, bars at most 48px wide and topped round,
  // one series of bars shows its biggest value in the accent and the rest in a quiet neutral
  const all = c.series.flatMap((x) => x.values);
  const lo0 = Math.min(0, ...all), hi0 = Math.max(0, ...all);
  const raw = Math.max(hi0 - lo0, 1e-9) / 4; const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((x) => x >= raw) || 10 * mag;
  const lo = Math.floor(lo0 / step) * step, hi = Math.ceil(hi0 / step) * step;
  const lvl = (v) => Math.min(1, Math.max(0, (v - lo) / Math.max(hi - lo, 1e-9)));
  const ticks = []; for (let k = 0; k <= Math.min(10, Math.round((hi - lo) / step)); k++) ticks.push(lo + step * k);
  const n = Math.max(1, c.labels.length); const zero = lvl(0);
  const color = (si) => SERIES[si % SERIES.length];
  const area = h('div', { class: 'chart-area', role: 'img', 'aria-label': c.caption || 'Chart' });
  for (const t of ticks) area.append(h('i', { class: 'chart-grid', style: `top:${(1 - lvl(t)) * 100}%` }));
  if (c.type === 'bar') {
    const k = c.series.length || 1; const one = k === 1;
    const lead = one ? c.series[0].values.reduce((m, v, i, a) => (v > a[m] ? i : m), 0) : -1;
    for (let i = 0; i < n; i++) {
      const g = h('div', { class: 'chart-group', style: `width:min(70%, ${48 * k}px)` });
      c.series.forEach((ser, si) => {
        const l = lvl(ser.values[i] ?? 0); const top = Math.max(l, zero), bot = Math.min(l, zero);
        g.append(h('div', { class: 'chart-bar', title: `${ser.name ? ser.name + ', ' : ''}${c.labels[i]}: ${ser.values[i]}`, style: `top:${(1 - top) * 100}%;bottom:${bot * 100}%;left:calc(${(si / k) * 100}% + 1px);width:calc(${100 / k}% - 2px);background:${one && i !== lead ? 'var(--muted)' : color(si)}` }));
      });
      area.append(h('div', { class: 'chart-slot' }, g));
    }
  } else {
    const svg = s('svg', { viewBox: '0 0 100 100', preserveAspectRatio: 'none', class: 'chart-lines' });
    c.series.forEach((ser, si) => {
      const pts = ser.values.map((v, i) => [(100 * (i + 0.5)) / n, 100 * (1 - lvl(v))]);
      svg.append(s('polyline', { points: pts.map((q) => q.join(',')).join(' '), fill: 'none', stroke: color(si), 'stroke-width': 2, 'vector-effect': 'non-scaling-stroke' }));
      pts.forEach((q, i) => svg.append(s('line', { x1: q[0], y1: q[1], x2: q[0], y2: q[1], stroke: color(si), 'stroke-width': 6, 'stroke-linecap': 'round', 'vector-effect': 'non-scaling-stroke' }, s('title', {}, `${ser.name ? ser.name + ', ' : ''}${c.labels[i]}: ${ser.values[i]}`))));
    });
    area.append(svg);
  }
  const axis = h('div', { class: 'chart-axis' }, ticks.map((t) => h('span', { style: `top:${(1 - lvl(t)) * 100}%` }, fmt(t))));
  const every = Math.max(1, Math.floor(c.labels.length / 12));
  const labels = h('div', { class: 'chart-labels' }, c.labels.map((l, i) => h('span', {}, i % every === 0 ? String(l) : '')));
  return h('figure', { class: 'answer-chart' }, h('div', { class: 'chart-plot' }, axis, area), labels, c.series.length > 1 ? legend : null, c.caption ? h('figcaption', {}, c.caption) : null);
}
// The app's page gets the theme's colours and some base styles, so a plain page already fits the look.
function appCss() {
  const cs = getComputedStyle(document.documentElement);
  const v = Object.values(THEME_VARS).concat('--radius').map((k) => `${k}:${cs.getPropertyValue(k).trim()}`).join(';');
  return `:root{${v};color-scheme:${cs.colorScheme || 'light'}}*{box-sizing:border-box}html,body{margin:0;background:transparent;color:var(--text);font:15px/1.5 ${cs.fontFamily}}` +
    'button,input,select,textarea{font:inherit;color:inherit;background:var(--sunken);border:1px solid var(--border);border-radius:var(--inner);padding:6px 10px}' +
    'button{background:var(--button);cursor:pointer}button:hover{background:var(--button-hover)}input[type=range]{padding:0;accent-color:var(--accent)}' +
    'table{border-collapse:collapse}th,td{border-top:1px solid var(--border);padding:6px 10px;text-align:start}a{color:var(--accent)}svg text{fill:currentColor}';
}
function appEl(title, html) {
  const frame = h('iframe', { sandbox: 'allow-scripts', src: '/sandbox.html', title: title || 'App', class: 'answer-app', loading: 'eager' });
  const box = h('figure', { class: 'answer-app-box' }, h('figcaption', {}, title || 'App'), frame);
  const onMsg = (e) => {
    if (e.source !== frame.contentWindow || e.origin !== 'null' || !e.data) return;
    const d = e.data;
    if (d.lh === 'ready') frame.contentWindow.postMessage({ lh: 'load', html, css: appCss() }, '*');
    else if (d.lh === 'height' && Number.isFinite(d.h)) frame.style.height = `${Math.min(Math.max(d.h, 60), 2000)}px`;
    else if (d.lh === 'follow_up' && typeof d.text === 'string' && d.text.trim()) {
      const text = d.text.trim().slice(0, 2000);
      const card = h('div', { class: 'card follow' }, h('p', {}, 'The app suggests asking:'), h('blockquote', {}, text),
        h('div', { class: 'actions' },
          h('button', { type: 'button', class: 'btn primary', onclick: () => { card.remove(); const i = $('#chat-input'); i.value = text; $('#composer').requestSubmit(); } }, 'Send'),
          h('button', { type: 'button', class: 'btn', onclick: () => card.remove() }, 'Dismiss')));
      box.after(card);
    }
  };
  addEventListener('message', onMsg);
  return box;
}
// table, chart and app go into the conversation; everything else opens beside it
function showInline(ev) {
  // a card in the conversation with the title and "Open beside", like GPUI's shown_view
  const cardOf = (body, beside) => h('figure', { class: 'answer-block', role: 'group', 'aria-label': ev.title || '' },
    h('div', { class: 'ab-head' }, h('span', { class: 'ab-title' }, ev.title || ''), h('button', { type: 'button', class: 'seg ghost', onclick: () => beside() }, 'Open beside')), body);
  try {
    if (ev.kind === 'table') { const mk = () => tableEl(JSON.parse(ev.content)); return cardOf(mk(), () => show(ev.title || 'Table', mk())); }
    if (ev.kind === 'chart') { const mk = () => chartEl(JSON.parse(ev.content)); return cardOf(mk(), () => show(ev.title || 'Chart', mk())); }
    if (ev.kind === 'app') {
      if (pref('pages-off') === 'true') return cardOf(h('div', { class: 'ab-off' }, 'Web pages inside the app are off. Turn them on in Settings, Appearance.'), () => showContent(ev));
      return cardOf(appEl(ev.title, ev.content), () => show(ev.title || 'App', appEl(ev.title, ev.content)));
    }
  } catch (e) { return h('p', { class: 'msg error' }, `Could not draw ${ev.title}: ${e.message}`); }
  return null;
}

// ---- model picker ----
// Like /pick: one row per model, its settings inside it. A llama-swap name is "<model>-<context tier>[-<template>]",
// and a mode is asked by the alias "<name>:<mode>". Search box, arrows, Enter; the choice shows on the button.
// A choice in the list takes effect at once, as in the desktop app.
const TIERS = ['low', 'balanced', 'native', 'max'];
const MODE_WORD = { think: 'Thinking', code: 'Coding', instruct: 'No thinking', auto: 'Auto', low: 'Reasoning low', medium: 'Reasoning medium', high: 'Reasoning high' };
function displayName(name) {
  let n = String(name);
  n = n.replace(/[-_.](APEX|A3B)(?![A-Za-z0-9])/gi, '');
  n = n.replace(/[-_.]?[0-9]+(\.[0-9]+)?B(?![A-Za-z0-9])/gi, '');
  n = n.replace(/(^|[-_])[0-9]+(?=$|[-_])/g, '$1');
  n = n.replace(/[-_ ]+/g, '-').replace(/-+/g, '-').replace(/^-+|-+$/g, '');
  n = n.replace(/[-_]([0-9]+(?:\.[0-9]+)*|X\d+(?:\.[0-9]+)*)(?=$|[-_])/g, ' $1');
  return n.replace(/^([A-Za-z]+)(\d+\.\d+)/, '$1 $2') || name;
}
function parseModel(id) {
  const [name, mode = ''] = id.split(/:(?=[^:]*$)/);
  const m = name.match(/^(.*?)-(low|balanced|native|max)(?:-(stock|vision))?$/);
  return m ? { base: m[1], tier: m[2], tpl: m[3] || 'sharp', mode, name } : { base: name, tier: '', tpl: 'sharp', mode, name };
}
let modelGroups = []; // [{base, label, size, members: [{id, tier, tpl, modes, loaded}]}]
let modelChoice = { id: '', mode: '' };
function groupModels(details) {
  const g = new Map();
  for (const d of details) {
    const p = parseModel(d.id);
    if (!g.has(p.base)) g.set(p.base, { base: p.base, label: displayName(p.base), size: (p.base.match(/(\d+(?:\.\d+)?)B(?![a-z])/i) || [])[1] || '', members: [] });
    g.get(p.base).members.push({ id: d.id, tier: p.tier, tpl: p.tpl, modes: d.modes || [], dflt: d.default_mode || '', loaded: !!d.loaded });
  }
  return [...g.values()].sort((a, b) => a.label.localeCompare(b.label));
}
const defaultEntry = (g) => g.members.find((m) => m.tier === 'balanced' && m.tpl === 'sharp') || g.members.find((m) => m.tpl === 'sharp') || g.members[0];

// The model slider (tiers.rs): 2 to 7 stops, 5 by default, each with a title and one model. A stop set by hand
// keeps its model; the rest are filled from the models on hand, smallest to largest.
const STOP_TITLES = ['Low', 'Medium', 'High', 'Extra', 'Max'];
const STOP_DEFAULTS = ['Sage-A1.5-35B-A3B', '', '', '', ''];
const jsonPref = (k, d) => { try { return JSON.parse(pref(k) || '') || d; } catch { return d; } };
const stopCount = () => { const n = +pref('stop-count') || 0; return n === 0 ? 5 : Math.min(7, Math.max(2, n)); };
const stopTitle = (i) => (jsonPref('stop-titles', [])[i] || '').trim() || (i < 5 ? STOP_TITLES[i] : `Stop ${i + 1}`);
const sizeOf = (label) => { const v = parseFloat(String(label).trim().replace(/[Bb]$/, '')); return Number.isFinite(v) ? v : 8; };
function stops() {
  const n = stopCount(); const set = jsonPref('stops', []);
  const onHand = (s) => modelGroups.some((g) => g.base === s);
  const out = Array(n).fill(''); const taken = [];
  for (let i = 0; i < n; i++) { const s = set[i]; if (s && onHand(s) && !taken.includes(s)) { out[i] = s; taken.push(s); } }
  for (let i = 0; i < n; i++) { const d = STOP_DEFAULTS[i]; if (d && onHand(d) && !out[i] && !taken.includes(d)) { out[i] = d; taken.push(d); } }
  const free = modelGroups.filter((g) => !taken.includes(g.base) && !/gpt-oss/i.test(g.base)).sort((a, b) => sizeOf(a.size) - sizeOf(b.size) || (a.base < b.base ? -1 : a.base > b.base ? 1 : 0));
  const empty = out.map((s, i) => (s ? -1 : i)).filter((i) => i >= 0);
  const k = empty.length; const m = free.length;
  if (m >= k) empty.forEach((i, j) => { const at = k === 1 ? Math.floor((m - 1) / 2) : Math.floor((j * (m - 1) * 2 + (k - 1)) / (2 * (k - 1))); out[i] = free[at].base; });
  else empty.slice(0, m).forEach((i, j) => { out[i] = free[j].base; });
  return out;
}
function setStop(i, base) {
  const n = stopCount(); const set = jsonPref('stops', []);
  while (set.length < n) set.push('');
  set.length = n;
  set[i] = set[i] === base ? '' : base;
  pref('stops', JSON.stringify(set));
}
function choiceLabel() {
  if (!modelChoice.id) return 'Pick a model';
  const p = parseModel(modelChoice.id);
  const bits = [displayName(p.base)];
  if (p.tier && p.tier !== 'balanced') bits.push(p.tier);
  if (p.tpl !== 'sharp') bits.push(p.tpl);
  if (modelChoice.mode) bits.push(MODE_WORD[modelChoice.mode] || modelChoice.mode);
  return bits.join(' · ');
}
// the button shows the stop's title before the model, like "High · Qwen 3.6"
function buttonLabel() {
  const at = stops().indexOf(parseModel(modelChoice.id).base);
  return at >= 0 && modelChoice.id ? `${stopTitle(at)} · ${choiceLabel()}` : choiceLabel();
}
// the id sent to the core: the recipe, plus ":mode" when a mode other than the default is chosen
const currentModel = () => modelChoice.id ? modelChoice.id + (modelChoice.mode ? `:${modelChoice.mode}` : '') : '';
// save is false when the app picks or redraws on its own: only a choice the person makes is stored
function setChoice(id, mode, save = true) {
  modelChoice = { id, mode: mode || '' };
  if (save) pref('model', currentModel());
  $('#model-btn-label').textContent = buttonLabel();
}
async function loadModels() {
  let v = {};
  try { v = await json('/v1/models'); } catch (e) { say(e.message); }
  const details = v.details || (v.models || []).map((id) => ({ id }));
  modelGroups = groupModels(details);
  const saved = pref('model') || '';
  const p = parseModel(saved);
  const known = details.some((d) => d.id === p.name);
  if (known) setChoice(p.name, p.mode, false);
  else {
    const g = modelGroups.find((x) => /qwen/i.test(x.base)) || modelGroups[0];
    const m = g && defaultEntry(g);
    setChoice(m ? m.id : '', '', false);
  }
}
// A row of stops with a thumb that glides between them; used by the model slider and the context length slider.
function stopSlider(cls, labels, at, choose, aria) {
  const n = labels.length;
  const el = h('div', { class: `stops ${cls}`, role: 'radiogroup', 'aria-label': aria, style: `--n:${n}` },
    h('div', { class: 'stops-line' }),
    labels.map((l, i) => h('button', { type: 'button', class: 'stop', role: 'radio', 'aria-checked': String(i === at), 'aria-label': l.aria || l.text, style: `--i:${i}`, onclick: () => choose(i) },
      h('span', { class: 'stop-dot' }), h('span', { class: 'stop-name' }, l.text))),
    at >= 0 ? h('div', { class: 'stops-thumb', style: `--i:${at}` }) : null);
  el.addEventListener('keydown', (e) => {
    const d = { ArrowRight: 1, ArrowLeft: -1, ArrowUp: 1, ArrowDown: -1 }[e.key];
    if (!d) return;
    e.preventDefault(); const rtl = getComputedStyle(el).direction === 'rtl';
    const to = Math.min(n - 1, Math.max(0, (at < 0 ? 0 : at) + (rtl && e.key.startsWith('Arrow') && (e.key === 'ArrowRight' || e.key === 'ArrowLeft') ? -d : d)));
    if (to !== at) choose(to);
  });
  return el;
}
function closeSlider() { $('#slider-card')?.remove(); $('#model-btn').classList.remove('on'); removeEventListener('pointerdown', outsideSlider); }
function outsideSlider(e) { if (!e.target.closest('#slider-card, #model-btn')) closeSlider(); }
// Springs from motion.dev (motion.js, window.Motion): critically damped, so nothing overshoots. Overlays
// grow from where they open and the CSS entrance stays as the fallback. Reduced motion or a theme with no
// animation gets a short fade instead.
const springs = window.Motion?.animate;
if (springs) document.documentElement.classList.add('mo');
function arrive(el, from = { y: 6, scale: 0.97 }) {
  if (!springs || !el) return;
  const still = getComputedStyle(document.documentElement).getPropertyValue('--dur').trim() === '0ms';
  if (still) return;
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) { springs(el, { opacity: [0, 1] }, { duration: 0.15 }); return; }
  const k = Object.fromEntries(Object.entries(from).map(([a, v]) => [a, [v, a === 'scale' ? 1 : 0]]));
  springs(el, { opacity: [0, 1], ...k }, { type: 'spring', bounce: 0, duration: 0.32 });
}
function openSlider() {
  const st = stops(); const n = st.length;
  const at = st.indexOf(parseModel(modelChoice.id).base);
  const head = h('button', { type: 'button', class: 'slider-name', 'aria-label': 'Pick a model yourself', onclick: () => { closeSlider(); openPicker(); } },
    h('span', {}, buttonLabel()), svgIcon('M9 6l6 6-6 6', { 'aria-hidden': 'true' }));
  const choose = (i) => {
    const g = modelGroups.find((x) => x.base === st[i]); if (!g) return;
    setChoice(defaultEntry(g).id, ''); closeSlider(); openSlider();
  };
  const labels = st.map((b, i) => ({ text: stopTitle(i), aria: `${stopTitle(i)}: ${b ? displayName(b) : ''}` }));
  const card = h('div', { class: 'slider-card', id: 'slider-card', role: 'dialog', 'aria-label': 'Model slider' }, head,
    h('div', { class: 'slider-well' }, stopSlider('model-stops', labels, at, choose, 'Model slider')));
  $('#model-btn').after(card); $('#model-btn').classList.add('on'); arrive(card, { y: 6, scale: 0.97 });
  card.querySelector('.stop[aria-checked="true"]')?.focus({ preventScroll: true });
  setTimeout(() => addEventListener('pointerdown', outsideSlider), 0);
}
// The model button opens the slider when any stop holds a model, else the list.
function modelButton() {
  if ($('#slider-card')) return closeSlider();
  closePicker();
  if (stops().some(Boolean)) openSlider(); else openPicker();
}
const svgIcon = (d, attrs = {}) => { const ns = 'http://www.w3.org/2000/svg'; const v = document.createElementNS(ns, 'svg'); v.setAttribute('viewBox', '0 0 24 24'); for (const [k, x] of Object.entries(attrs)) v.setAttribute(k, x); const p = document.createElementNS(ns, 'path'); p.setAttribute('d', d); v.append(p); return v; };
function openPicker() {
  const btn = $('#model-btn');
  if ($('#picker')) { closePicker(); return; }
  closeSlider();
  const q = h('input', { type: 'search', placeholder: 'Search models', 'aria-label': 'Search models', autocomplete: 'off' });
  const list = h('ul', { role: 'listbox', 'aria-label': 'Models' });
  const detail = h('div', { class: 'pick-detail' });
  const pop = h('div', { class: 'picker', id: 'picker', role: 'dialog', 'aria-label': 'Choose a model' }, h('div', { class: 'pick-search' }, q), h('div', { class: 'pick-body' }, list, detail));
  let rows = []; let active = 0;
  let openBase = parseModel(modelChoice.id).base;
  const chip = (label, on, onclick) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(on), onclick }, label);
  const section = (title, ...kids) => h('div', { class: 'pick-row' }, h('span', { class: 'pick-label' }, title), ...kids);
  function showDetail(g) {
    openBase = g.base;
    const cur = parseModel(modelChoice.id);
    const here = cur.base === g.base;
    const sel = (here && g.members.find((m) => m.id === cur.name)) || defaultEntry(g);
    const mode = here ? modelChoice.mode : '';
    const tpls = [...new Set(g.members.map((m) => m.tpl))];
    const tiers = TIERS.filter((t) => g.members.some((m) => m.tier === t && m.tpl === sel.tpl));
    const pick = (tier, tpl, md) => {
      const m = g.members.find((x) => x.tier === tier && x.tpl === tpl) || g.members.find((x) => x.tpl === tpl) || sel;
      setChoice(m.id, md); showDetail(g); render();
    };
    const kids = [h('div', { class: 'pick-title' }, h('span', {}, g.label), g.size ? h('span', { class: 'muted' }, `${g.size}B`) : null)];
    if (tiers.length > 1) kids.push(section('Context length', stopSlider('ctx-stops', tiers.map((t) => ({ text: t })), here ? tiers.indexOf(sel.tier) : -1, (i) => pick(tiers[i], sel.tpl, mode), 'Context length')));
    if (tpls.length > 1) kids.push(section('Template', h('div', { class: 'chips' }, tpls.map((t) => chip(t, here && sel.tpl === t, () => pick(sel.tier, t, mode))))));
    const onOff = sel.modes.includes('instruct') && sel.modes.some((m) => m === 'think' || m === 'code');
    if (onOff) {
      const effective = mode || sel.dflt;
      const on = effective !== 'instruct';
      let to = on ? 'instruct' : (sel.modes.find((m) => m !== 'instruct') || '');
      if (to === sel.dflt) to = '';
      const sw = h('input', { type: 'checkbox', class: 'switch', 'aria-label': 'Thinking' }); sw.checked = on;
      sw.onchange = () => pick(sel.tier, sel.tpl, to);
      kids.push(h('label', { class: 'set-row pick-switch' }, sw, 'Thinking'));
    } else if (sel.modes.length) {
      kids.push(section('Mode', h('div', { class: 'chips' },
        chip(sel.dflt ? `${MODE_WORD[sel.dflt] || sel.dflt} (default)` : 'Default', here && !mode, () => pick(sel.tier, sel.tpl, '')),
        sel.modes.filter((m) => m !== sel.dflt).map((m) => chip(MODE_WORD[m] || m, here && mode === m, () => pick(sel.tier, sel.tpl, m))))));
    }
    const st = stops();
    kids.push(section('Its stop on the slider', h('div', { class: 'chips' }, st.map((_, i) => chip(stopTitle(i), st.indexOf(g.base) === i, () => { setStop(i, g.base); setChoice(modelChoice.id, modelChoice.mode, false); showDetail(g); })))));
    detail.replaceChildren(...kids);
  }
  function render() {
    const words = q.value.toLowerCase().split(/\s+/).filter(Boolean);
    const curBase = parseModel(modelChoice.id).base;
    rows = modelGroups.filter((g) => words.every((w) => `${g.label} ${g.base}`.toLowerCase().includes(w)));
    active = Math.min(active, Math.max(0, rows.length - 1));
    list.replaceChildren(...rows.map((g, i) => {
      const loaded = g.members.some((m) => m.loaded);
      return h('li', { role: 'option', 'aria-selected': String(i === active), class: g.base === curBase ? 'open' : null, onclick: () => { active = i; const m = defaultEntry(g); if (g.base !== curBase) setChoice(m.id, ''); showDetail(g); render(); }, onmouseenter: () => { active = i; mark(); } },
        h('span', { class: 'dot', 'data-on': String(loaded), title: loaded ? 'Loaded' : null }),
        h('span', { class: 'name' }, g.label), g.size ? h('span', { class: 'muted' }, `${g.size}B`) : null,
        g.base === curBase ? svgIcon('M5 12.5l5 4.5 9-10', { class: 'check', 'aria-label': 'current' }) : null);
    }));
    if (!rows.length) list.append(h('li', { class: 'empty' }, 'No model matches.'));
  }
  const mark = () => list.querySelectorAll('[role=option]').forEach((li, i) => li.setAttribute('aria-selected', String(i === active)));
  q.addEventListener('input', () => { active = 0; render(); });
  pop.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); closePicker(); btn.focus(); }
    else if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && !e.target.closest('.stops')) { e.preventDefault(); active = (active + (e.key === 'ArrowDown' ? 1 : -1) + rows.length) % Math.max(rows.length, 1); mark(); showDetail(rows[active]); list.querySelectorAll('[role=option]')[active]?.scrollIntoView({ block: 'nearest' }); }
    else if (e.key === 'Enter' && e.target === q && rows[active]) { e.preventDefault(); list.querySelectorAll('[role=option]')[active]?.click(); closePicker(); btn.focus(); }
  });
  btn.after(pop); btn.setAttribute('aria-expanded', 'true'); arrive(pop, { y: pop.closest('.composer') ? 6 : -6, scale: 0.97 });
  render();
  const g0 = modelGroups.find((g) => g.base === openBase) || modelGroups[0];
  if (g0) { active = Math.max(0, rows.indexOf(g0)); showDetail(g0); mark(); list.querySelector('.open')?.scrollIntoView({ block: 'nearest' }); }
  q.focus();
  setTimeout(() => addEventListener('pointerdown', outside), 0);
}
function outside(e) { if (!e.target.closest('#picker, #model-btn')) closePicker(); }
function closePicker() { $('#picker')?.remove(); $('#model-btn').setAttribute('aria-expanded', 'false'); removeEventListener('pointerdown', outside); }

// ---- chat ----
// One reading column. Answers have no bubble; your messages do. Thinking and tool calls fold into one quiet
// line before the answer. The view never scrolls by itself while text streams in, only when you send.
let userName = '';
let chat = { id: null, stored: null, busy: false, ctl: null, mode: 'chat', incognito: false };
const log = () => $('#log');
// the hello for the time of day, with the name from Settings (each word capitalised), as the desktop app has it
function greet() {
  const name = (userName || '').trim().split(/\s+/).filter(Boolean).map((w) => w[0].toUpperCase() + w.slice(1)).join(' ');
  const hr = new Date().getHours();
  const hello = hr >= 5 && hr <= 11 ? 'Good morning' : hr >= 12 && hr <= 17 ? 'Good afternoon' : 'Good evening';
  $('#greeting').textContent = name ? `${hello}, ${name}` : hello;
  $('#greeting-sub').textContent = chat.incognito ? 'Private chat: nothing is remembered or stored' : 'Everything stays on this computer';
}
// a new chat is a plain Chat, an Assistant (home folder) or a Project (one folder); Private is for plain chats
const KIND_HINT = { chat: 'Plain conversation, no tools', assistant: 'Works from your home folder and asks before changing anything', project: 'Works inside one folder and asks before changing anything' };
function setMode(m) {
  chat.mode = m;
  const i = ['chat', 'assistant', 'project'].indexOf(m);
  for (const b of document.querySelectorAll('#modes button')) b.setAttribute('aria-pressed', String(b.dataset.mode === m));
  $('#modes').style.setProperty('--at', i);
  $('#kind-hint').textContent = m === 'chat' && chat.incognito ? 'Nothing is remembered or stored' : KIND_HINT[m];
  $('#project-dir').hidden = m !== 'project';
  $('#private-btn').hidden = m !== 'chat';
  if (m !== 'chat' && chat.incognito) setPrivate(false);
}
function setPrivate(on) {
  chat.incognito = on;
  $('#private-btn').setAttribute('aria-pressed', String(on));
  $('#composer').classList.toggle('private', on);
  setMode(chat.mode); greet();
}
function setEmpty(on) { $('#app').classList.toggle('empty', on); }
function message(role, text, opts = {}) {
  const body = h('div', { class: 'body' }, text || '');
  const el = h('div', { class: `msg ${role}${opts.reveal ? ' reveal' : ''}` }, body);
  if (role === 'user' || role === 'assistant') {
    el.append(h('button', { type: 'button', class: 'copy', onclick: async (e) => { await navigator.clipboard?.writeText(body.dataset.raw || body.textContent).catch(() => {}); e.target.textContent = 'Copied'; setTimeout(() => { e.target.textContent = 'Copy'; }, 1200); } }, 'Copy'));
  }
  log().append(el);
  return body;
}
// transcript.rs: thinking and tool calls fold into one quiet line, "Worked for 14 s, 3 steps", that opens to the list
const dur = (t) => t < 60 ? `${t} s` : t % 60 === 0 ? `${t / 60} min` : `${Math.floor(t / 60)} min ${t % 60} s`;
const stepsWord = (n) => `${n} step${n === 1 ? '' : 's'}`;
// GPUI's step_words: a tool call as a short line, "Read ~/garden/notes.md"
const VERBS = { read_file: 'Read', write_file: 'Wrote', edit_file: 'Edited', list_dir: 'Listed', list_files: 'Listed', run_command: 'Ran', bash: 'Ran', shell: 'Ran', web_fetch: 'Opened', fetch: 'Opened', web_search: 'Searched for', search: 'Searched for', recall: 'Looked up', remember: 'Saved' };
function stepWords(name, args) {
  let a = {}; try { a = JSON.parse(args || '{}') || {}; } catch { /* not json */ }
  const t = ['path', 'file', 'command', 'url', 'query', 'name', 'text'].map((k) => a[k]).find((v) => typeof v === 'string');
  const w = name in VERBS ? VERBS[name] : name.replace(/_/g, ' ').replace(/^./, (c) => c.toUpperCase());
  const s = t ? `${w} ${t.replace(/\s+/g, ' ')}` : w;
  return s.length > 120 ? s.slice(0, 119) + '…' : s;
}
// the side panel with a chat open, as GPUI's right_panel: a row of tabs with counts, widen and close, then Work
let chatSteps = [];
function dockOpen() { return !$('#panel').hidden && $('#panel').classList.contains('dock'); }
function showWork() {
  $('#panel').hidden = false; $('#panel').classList.add('dock'); $('#panel-title').textContent = '';
  $('#panel-toggle').setAttribute('aria-pressed', 'true');
  const n = chatSteps.length;
  $('#dock-tabs').replaceChildren(h('button', { type: 'button', role: 'tab', 'aria-selected': 'true', class: 'dock-tab' }, 'Work', n ? h('span', { class: 'n' }, String(n)) : null));
  const body = [];
  if (n) {
    body.push(h('div', { class: 'work-sec' }, 'Steps in this chat'));
    for (const st of [...chatSteps].reverse()) {
      const args = h('div', { class: 'work-args', hidden: true }, st.args);
      body.push(h('button', { type: 'button', class: 'work-step', 'aria-label': st.words, onclick: () => { args.hidden = !args.hidden; } },
        h('span', { class: st.done ? 'dot done' : 'dot' }), h('span', { class: 'w' }, st.words), args));
    }
  } else body.push(h('p', { class: 'note' }, 'Nothing is being done for this chat. Tool steps and department work show up here.'));
  $('#panel-body').replaceChildren(...body);
}
function hideDock() { $('#panel').hidden = true; $('#panel').classList.remove('dock'); $('#panel-toggle').setAttribute('aria-pressed', 'false'); }
function steps() {
  const list = h('div', { class: 'step-list', role: 'list', 'aria-label': 'Steps', hidden: true });
  const chev = svgIcon('M9 6l6 6-6 6', { class: 'chev', 'aria-hidden': 'true' });
  const txt = h('span', {}, 'Working…');
  const btn = h('button', { type: 'button', 'aria-expanded': 'false' }, chev, txt);
  const box = h('div', { class: 'steps' }, btn, list);
  const t0 = Date.now();
  let n = 0; let think = null; let tokens = 0; let thought = false;
  btn.addEventListener('click', () => { const open = list.hidden; list.hidden = !open; btn.setAttribute('aria-expanded', String(open)); box.toggleAttribute('data-open', open); });
  const label = (done) => {
    const secs = Math.round((Date.now() - t0) / 1000);
    if (!done && think) { txt.textContent = `Thinking… ${tokens.toLocaleString('en')} token${tokens === 1 ? '' : 's'}, ${dur(secs)}`; return; }
    // GPUI: a group that thought says how long it worked; tools alone only count their steps
    txt.textContent = done && !thought ? stepsWord(n) : done ? (secs === 0 ? `Worked for under 1 s, ${stepsWord(n)}` : `Worked for ${dur(secs)}, ${stepsWord(n)}`) : `Working… ${stepsWord(n)}`;
  };
  return {
    box,
    thinking(text) {
      thought = true;
      if (!think) { n++; const body = h('div', { class: 'think-text' }); think = body; list.append(h('div', { class: 'step', role: 'listitem' }, h('div', { class: 'step-name' }, 'Thinking'), body)); }
      think.textContent += text || ''; tokens++; think.scrollTop = think.scrollHeight; label(false);
    },
    add(name, args) {
      n++; think = null;
      const result = h('div', { class: 'step-result' }, 'running…');
      const open = h('button', { type: 'button', class: 'seg ghost', onclick: () => show(name, h('pre', { class: 'body' }, result.textContent)) }, 'Open in panel');
      const more = h('div', { class: 'step-more', hidden: true }, result, h('div', {}, open));
      const head = h('button', { type: 'button', class: 'step-head', 'aria-expanded': 'false', onclick: () => { more.hidden = !more.hidden; head.setAttribute('aria-expanded', String(!more.hidden)); } }, `${name} ${String(args || '').slice(0, 90)}`.trim());
      list.append(h('div', { class: 'step', role: 'listitem' }, head, more));
      label(false);
      const st = { words: stepWords(name, args), args: String(args || ''), done: false }; chatSteps.push(st); if (dockOpen()) showWork();
      return { result(t) { result.textContent = String(t).slice(0, 4000); st.done = true; if (dockOpen()) showWork(); } };
    },
    done() { think = null; if (n) label(true); else box.remove(); },
  };
}
function ago(t) {
  if (!t) return '';
  const d = Math.max(0, Date.now() / 1000 - (t > 1e12 ? t / 1000 : t));
  return d < 60 ? 'now' : d < 3600 ? `${Math.round(d / 60)}m` : d < 86400 ? `${Math.round(d / 3600)}h` : `${Math.round(d / 86400)}d`;
}
// The chat list in sections, as in GPUI (lh-gui sidebar.rs): Needs you, Pinned, Active, one per project folder, Earlier.
// A chat settles into Earlier after three days without a word; Earlier shows eight until asked for more.
let earlierAll = false;
const pinnedChats = () => jsonPref('pinned', []);
function togglePin(id) { const p = pinnedChats(); const i = p.indexOf(id); if (i < 0) p.push(id); else p.splice(i, 1); pref('pinned', JSON.stringify(p)); loadChats(); }
function chatSections(list) {
  const secs = (t) => (t > 1e12 ? t / 1000 : t) || 0;
  const now = Date.now() / 1000, settle = now - (+pref('settle-days') || 3) * 86400;
  const needs = [], pins = [], active = [], earlier = [], projects = new Map();
  for (const c of [...list].sort((a, b) => secs(b.last ?? b.updated) - secs(a.last ?? a.updated))) {
    const last = secs(c.last ?? c.updated);
    if (c.id === chat.stored && $('#asks')?.children.length) needs.push(c);
    else if (pinnedChats().includes(c.id)) pins.push(c);
    else if (c.project) { const f = c.project.replace(/\/+$/, ''); if (!projects.has(f)) projects.set(f, []); projects.get(f).push(c); }
    else if (c.live || !last || last >= settle) active.push(c);
    else earlier.push(c);
  }
  const shown = earlierAll ? earlier : earlier.slice(0, 8);
  const out = [[`Needs you · ${needs.length}`, needs, '', true], ['Pinned', pins], ['Active', active],
    ...[...projects].map(([f, cs]) => [f.split('/').pop() || f, cs]),
    [`Earlier · ${earlier.length}`, shown, earlier.length > 8 ? (earlierAll ? 'Show less' : `Show ${earlier.length - 8} more`) : '']];
  return out.filter(([, cs]) => cs.length);
}
async function loadChats() {
  const ul = $('#chat-list');
  let list = [];
  try { list = (await json('/v1/chats')).chats || []; } catch { /* chats may be off */ }
  chatCache = list;
  const rowFor = (c) => {
    const li = h('li', {});
    const row = () => li.replaceChildren(
      h('button', { type: 'button', class: 'open-chat', 'aria-current': String(c.id === chat.stored), onclick: () => openStored(c) }, h('span', { class: 't' }, (pinnedChats().includes(c.id) ? '★ ' : '') + (c.title || 'Untitled')), h('span', { class: 'ago' }, ago(c.last ?? c.updated))),
      h('button', { type: 'button', class: 'pin', 'aria-label': pinnedChats().includes(c.id) ? 'Unpin' : 'Pin', title: pinnedChats().includes(c.id) ? 'Unpin' : 'Pin', onclick: () => togglePin(c.id) }, pinnedChats().includes(c.id) ? '★' : '☆'),
      h('button', { type: 'button', class: 'del', 'aria-label': `Delete ${c.title || 'Untitled'}`, title: 'Delete', onclick: () => ask() }, h('svg', { viewBox: '0 0 24 24', 'aria-hidden': 'true' })));
    const ask = () => {
      const yes = h('button', { type: 'button', class: 'btn danger', onclick: async () => {
        try { await json(`/v1/chats/${c.id}`, { method: 'DELETE' }); if (chat.stored === c.id) newChat(); else loadChats(); } catch (e) { say(e.message); row(); }
      } }, 'Delete');
      li.replaceChildren(h('div', { class: 'confirm', role: 'group', 'aria-label': 'Delete this chat?' }, h('span', {}, `Delete “${(c.title || 'Untitled').slice(0, 40)}”?`), h('span', { class: 'actions' }, yes, h('button', { type: 'button', class: 'btn', onclick: row }, 'Keep'))));
      yes.focus();
    };
    row();
    return li;
  };
  ul.replaceChildren(...chatSections(list).flatMap(([title, rows, more, danger]) => [
    h('li', { class: 'side-sec' + (danger ? ' danger' : ''), role: 'presentation' }, title), ...rows.map(rowFor),
    ...(more ? [h('li', { role: 'presentation' }, h('button', { type: 'button', class: 'earlier-more', onclick: () => { earlierAll = !earlierAll; loadChats(); } }, more))] : [])]));
  if (!list.length) ul.replaceChildren(h('li', { class: 'side-empty' }, 'No chats yet. What you write stays on this computer.'));
  // the trash icon (drawn here: svg children made with h() would be in the wrong namespace)
  for (const svg of ul.querySelectorAll('.del svg')) svg.replaceWith(s('svg', { viewBox: '0 0 24 24', 'aria-hidden': 'true' }, s('path', { d: 'M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12' })));
  if (list.length && !userName) greet();
}
function newChat() {
  chat.ctl?.abort();
  chat = { id: null, stored: null, busy: false, ctl: null, mode: 'chat', incognito: false };
  setPrivate(false); greet();
  log().replaceChildren(); $('#asks').replaceChildren(); chatSteps = []; if (dockOpen()) hideDock();
  $('#chat-title').textContent = 'New chat'; $('#chat-meta').textContent = '';
  setEmpty(true); loadChats(); $('#chat-input').focus();
  if (location.hash && location.hash !== '#/chat') location.hash = '#/chat';
}
async function openStored(c) {
  newChat(); setEmpty(false);
  chat.stored = c.id; $('#chat-title').textContent = c.title || 'Untitled';
  try {
    const v = await json(`/v1/chats/${c.id}`);
    for (const m of v.history || []) if (m.role === 'user') message('user', m.content); else if (m.role === 'assistant' && m.content) renderAnswer(message('assistant', ''), m.content);
    if (v.context_tokens) $('#chat-meta').textContent = `${v.context_tokens.toLocaleString()} tokens`;
    $('#scroll').scrollTop = $('#scroll').scrollHeight;
  } catch (e) { message('error', e.message); }
  loadChats();
}
// A stored chat is read again by the core before it continues; it may warn first (size, cost, cloud).
async function resume(confirm) {
  const v = await json(`/v1/chats/${chat.stored}/resume`, { method: 'POST', body: { confirm, model: currentModel() } });
  if (v.needs_confirmation) {
    return new Promise((ok) => {
      const card = h('div', { class: 'card', role: 'group', 'aria-label': 'Continue this chat?' }, h('p', {}, v.warning?.message || 'The core will read this chat again before it can continue.'),
        h('div', { class: 'actions' },
          h('button', { type: 'button', class: 'btn primary', onclick: async () => { card.remove(); ok(await resume(true)); } }, 'Continue'),
          h('button', { type: 'button', class: 'btn', onclick: () => { card.remove(); ok(false); } }, 'Cancel')));
      $('#asks').append(card);
    });
  }
  chat.id = v.session || chat.stored;
  return true;
}
async function sendMessage(text) {
  if (!chat.id) {
    if (chat.stored) { if (!(await resume(false))) return false; }
    else {
      const body = { model: currentModel(), incognito: !!chat.incognito };
      if (chat.mode === 'project') { const dir = $('#project-dir').value.trim(); if (!dir) { say('Choose the project folder first.'); return false; } body.project_dir = dir; } else body.mode = chat.mode;
      const v = await json('/v1/sessions', { method: 'POST', body });
      chat.id = v.id;
    }
  }
  setEmpty(false);
  message('user', text, { reveal: true });
  $('#scroll').scrollTop = $('#scroll').scrollHeight; // only on send
  let reply = null; let work = null;
  const tools = {};
  const answer = () => { if (!reply) { work?.done(); reply = message('assistant', '', { reveal: true }); } return reply; };
  const step = (t, a) => { if (reply) { reply = null; work = null; } if (!work) { work = steps(); log().append(work.box); } return work.add(t, a); };
  chat.ctl = new AbortController();
  try {
    await stream(`/v1/sessions/${chat.id}/turns`, { method: 'POST', body: { text }, signal: chat.ctl.signal }, (ev) => {
      if (ev.type === 'text') { const b = answer(); b.dataset.raw = (b.dataset.raw || '') + ev.text; if (!b.pending) { b.pending = true; requestAnimationFrame(() => { b.pending = false; renderAnswer(b, b.dataset.raw); }); } }
      else if (ev.type === 'reasoning') { if (reply) { reply = null; work = null; } if (!work) { work = steps(); log().append(work.box); } work.thinking(ev.text); }
      else if (ev.type === 'tool_start') tools[ev.id] = step(ev.name, typeof ev.arguments === 'string' ? ev.arguments : JSON.stringify(ev.arguments || ''));
      else if (ev.type === 'tool_result') { if (ev.text != null) tools[ev.id]?.result(ev.text); }
      else if (ev.type === 'usage') $('#chat-meta').textContent = `${(ev.prompt_tokens + ev.completion_tokens).toLocaleString()} tokens`;
      else if (ev.type === 'notice') say(ev.message);
      else if (ev.type === 'refused' || ev.type === 'error') message('error', ev.message);
      else if (ev.type === 'step_limit') message('note', 'It used all its steps without finishing.');
      else if (ev.type === 'show') { const el = showInline(ev); if (el) { reply = null; work?.done(); work = null; log().append(el); } else showContent(ev); }
      else if (ev.type === 'ask') question($('#asks'), `/v1/sessions/${chat.id}/answers/${ev.id}`, `${ev.worker || 'The assistant'} wants to use ${ev.tool}: ${ev.what}`, ev.reason);
    });
  } catch (e) { if (e.name !== 'AbortError') message('error', e.message); }
  work?.done();
  return true;
}
// What the assistant opens beside the chat (the `show` tool).
function showContent(ev) {
  const body = ev.kind === 'image' ? h('p', {}, ev.path || ev.content) : h('pre', { class: 'body' }, ev.content || '');
  show(ev.title || 'Opened', body);
}
function chatSetup() {
  const input = $('#chat-input'); const send = $('#send');
  const grow = () => { input.style.height = 'auto'; input.style.height = `${input.scrollHeight}px`; };
  input.addEventListener('input', grow);
  input.addEventListener('keydown', (e) => { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); $('#composer').requestSubmit(); } });
  $('#composer').addEventListener('submit', async (e) => {
    e.preventDefault();
    const text = input.value.trim();
    if (!text || chat.busy) return;
    chat.busy = true; send.disabled = true;
    input.value = ''; grow();
    try { if (!(await sendMessage(text))) input.value = text; } catch (err) { message('error', err.message); input.value = text; }
    chat.busy = false; send.disabled = false; $('#asks').replaceChildren(); input.focus(); loadChats();
  });
  $('#new-chat').addEventListener('click', newChat);
  for (const b of document.querySelectorAll('#modes button')) b.addEventListener('click', () => setMode(b.dataset.mode));
  $('#private-btn').addEventListener('click', () => setPrivate(!chat.incognito));
  for (const b of document.querySelectorAll('#chips button')) b.addEventListener('click', () => { input.value = b.dataset.text; grow(); input.focus(); });
  $('#panel-close').addEventListener('click', () => { const d = dockOpen(); hideDock(); if (!d) location.hash = '#/chat'; });
  $('#panel-toggle').addEventListener('click', () => (dockOpen() ? hideDock() : showWork()));
  $('#panel-widen').addEventListener('click', () => $('#panel').classList.toggle('wide'));
  $('#side-toggle').addEventListener('click', () => { const f = $('#app').classList.toggle('folded'); pref('folded', f ? '1' : ''); });
  if (pref('folded')) $('#app').classList.add('folded');
  // folded, the mark is the open button, as in the desktop app
  $('#side-logo').addEventListener('click', () => { if ($('#app').classList.contains('folded')) $('#side-toggle').click(); });
  for (const b of document.querySelectorAll('[data-foot]')) b.addEventListener('click', () => { const g = b.dataset.foot; location.hash = location.hash === `#/${g}` ? '#/chat' : `#/${g}`; });
  $('#side-search').addEventListener('click', openPalette);
  addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); }
    if (e.ctrlKey && e.key.toLowerCase() === 'b') { e.preventDefault(); $('#side-toggle').click(); }
    if (e.ctrlKey && e.key.toLowerCase() === 'n') { e.preventDefault(); newChat(); }
  });
  $('#model-btn').addEventListener('click', modelButton);
  greet(); loadModels(); newChat();
}
// The command palette (Ctrl+K), as in GPUI (lh-gui palette.rs): every place and setting by typing a few letters.
let chatCache = [];
function paletteEntries() {
  const e = (group, title, hint, run) => ({ group, title, hint, run });
  const cur = 'current';
  const v = [e('Do', 'New chat', 'Ctrl+N', newChat), e('Do', 'Settings', 'Ctrl+Shift+S', () => { location.hash = '#/settings'; }),
    e('Do', 'Show or hide the chat list', 'Ctrl+B', () => $('#side-toggle').click()), e('Tab', 'Open Work', 'Ctrl+Shift+W', showWork)];
  for (const c of chatCache) v.push(e('Chat', c.title || 'Untitled', c.id === chat.stored ? cur : '', () => openStored(c)));
  for (const g of modelGroups || []) { const m = defaultEntry(g); if (m) v.push(e('Model', g.label, g.members.some((x) => x.id === modelChoice.id) ? cur : '', () => setChoice(m.id, ''))); }
  for (const f of [...new Set(themes.map((t) => t.name))]) v.push(e('Look', f, f === lookFamily() ? cur : '', () => setLook(f)));
  const dark = (curTheme?.mode || '') === 'dark';
  v.push(e('Mode', dark ? 'Switch to light' : 'Switch to dark', 'Ctrl+T', () => setLook(null, dark ? 'light' : 'dark')));
  for (const [m, t] of [['light', 'Light'], ['dark', 'Dark'], ['system', 'Follow the system']]) v.push(e('Mode', t, m === lookMode() ? cur : '', () => setLook(null, m)));
  const dir = pref('direction') || 'auto';
  for (const [d, t] of [['auto', 'Follow the desktop language'], ['ltr', 'Left to right'], ['rtl', 'Right to left']]) v.push(e('Direction', t, d === dir ? cur : 'Ctrl+Shift+R', () => { pref('direction', d); applyLayout(); }));
  const w = pref('chat-width') || '860';
  for (const x of ['700', '900', '1100', 'full']) v.push(e('Width', `Chat width: ${x === 'full' ? 'Full' : `${x} px`}`, x === w ? cur : '', () => { pref('chat-width', x); applyLayout(); }));
  return v;
}
// every word must appear in the title or the group; the start of the title counts most, then the start of a word
function paletteFilter(entries, query) {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return entries;
  const scored = [];
  entries.forEach((en, i) => {
    const t = en.title.toLowerCase(), g = en.group.toLowerCase(); let score = 0;
    for (const w of words) {
      if (t.startsWith(w)) score += 0; else if (t.split(/[^\p{L}\p{N}]+/u).some((x) => x.startsWith(w))) score += 1;
      else if (t.includes(w)) score += 2; else if (g.includes(w)) score += 3; else return;
    }
    scored.push([score, i]);
  });
  return scored.sort((a, b) => a[0] - b[0] || a[1] - b[1]).map(([, i]) => entries[i]);
}
function openPalette() {
  if ($('#palette')) return closePalette();
  const all = paletteEntries();
  let rows = all, sel = 0;
  const input = h('input', { type: 'text', placeholder: 'Type a command, a chat, a model or a look', 'aria-label': 'Type a command, a chat, a model or a look' });
  const list = h('div', { id: 'palette-list', role: 'listbox', 'aria-label': 'Commands' });
  const run = (en) => { closePalette(); en.run(); };
  const draw = () => {
    if (!rows.length) return list.replaceChildren(h('div', { class: 'palette-none' }, 'Nothing matches what you typed.'));
    const start = Math.max(0, Math.min(sel - 8, rows.length - 9));
    list.replaceChildren(...rows.slice(start, start + 9).map((en, k) => h('div', { class: 'palette-row', role: 'option', 'aria-selected': String(start + k === sel), 'aria-label': `${en.group}: ${en.title}`, onclick: () => run(en) },
      h('span', { class: 'g' }, en.group), h('span', { class: 't' }, en.title), h('span', { class: 'k' }, en.hint))));
  };
  input.addEventListener('input', () => { rows = paletteFilter(all, input.value); sel = 0; draw(); });
  const veil = h('div', { id: 'palette', role: 'dialog', 'aria-label': 'Command palette', onmousedown: (e) => { if (e.target === veil) closePalette(); } }, h('div', { class: 'palette-box' }, input, list));
  veil.addEventListener('keydown', (e) => {
    const n = rows.length, k = e.key;
    if (k === 'ArrowDown' || (e.ctrlKey && k === 'n')) sel = n ? (sel + 1) % n : 0;
    else if (k === 'ArrowUp' || (e.ctrlKey && k === 'p')) sel = n ? (sel + n - 1) % n : 0;
    else if (k === 'Escape') return closePalette();
    else if (k === 'Enter') { if (rows[sel]) run(rows[sel]); return e.preventDefault(); }
    else return;
    e.preventDefault(); draw();
  });
  document.body.append(veil); draw(); input.focus();
}
function closePalette() { if ($('#palette')) { $('#palette').remove(); $('#chat-input')?.focus(); } }
function question(host, answerPath, what, reason) {
  const allow = h('button', { type: 'button' }, 'Allow');
  const deny = h('button', { type: 'button' }, 'Do not allow');
  const box = h('div', { role: 'alertdialog', 'aria-label': 'Permission needed' }, h('p', {}, what), reason ? h('p', {}, reason) : null, h('p', {}, allow, ' ', deny));
  const reply = (yes) => async () => { box.remove(); await api(answerPath, { method: 'POST', body: { answer: yes ? 'yes' : 'no' } }).catch(() => {}); };
  allow.addEventListener('click', reply(true)); deny.addEventListener('click', reply(false));
  host.append(box); allow.focus();
}

// ---- memory ----
async function memoryView() {
  const q = h('input', { type: 'search', id: 'q' });
  const list = h('ul', {});
  const detail = h('div', { 'aria-live': 'polite' });
  async function load() {
    list.replaceChildren();
    try {
      const v = q.value.trim() ? await json(`/v1/memory/search?q=${encodeURIComponent(q.value.trim())}`) : await json('/v1/memory/pages');
      const items = (v.results || v.hits || v.pages || []).map((x) => x.page || x);
      if (!items.length) list.append(h('li', {}, 'Nothing found.'));
      for (const p of items) list.append(h('li', {}, h('button', { type: 'button', onclick: () => open(p.id) }, p.title), ` (${p.layer}${p.privacy === 'sensitive' ? ', sensitive' : ''})`));
    } catch (e) { list.append(h('li', { role: 'alert' }, e.message)); }
  }
  async function open(id) {
    const v = await json(`/v1/memory/pages/${id}`);
    const p = v.page;
    const title = h('input', { type: 'text', value: p.title, id: 'pt' });
    const body = h('textarea', { rows: '10', id: 'pb' }, p.body);
    detail.replaceChildren(
      h('h3', {}, p.title),
      h('p', {}, `Layer ${p.layer}, version ${p.version}, ${p.privacy}${p.archived ? ', archived' : ''}`),
      h('p', {}, h('label', { for: 'pt' }, 'Title'), ' ', title), h('p', {}, h('label', { for: 'pb' }, 'Text')), h('p', {}, body),
      h('p', {},
        h('button', { type: 'button', onclick: async () => { await json(`/v1/memory/pages/${id}`, { method: 'PATCH', body: { title: title.value, text: body.value } }); say('Saved.'); load(); } }, 'Save'), ' ',
        h('button', { type: 'button', onclick: async () => { await json(`/v1/memory/pages/${id}/archive`, { method: 'POST', body: {} }); say('Archived.'); detail.replaceChildren(); load(); } }, 'Archive'), ' ',
        h('button', { type: 'button', onclick: async () => { if (confirm('Delete this note for good, including its history?')) { await api(`/v1/memory/pages/${id}`, { method: 'DELETE' }); say('Deleted.'); detail.replaceChildren(); load(); } } }, 'Delete')));
    detail.querySelector('h3').tabIndex = -1; detail.querySelector('h3').focus();
  }
  show('Memory',
    h('form', { role: 'search', onsubmit: (e) => { e.preventDefault(); load(); } }, h('label', { for: 'q' }, 'Search your notes '), q, ' ', h('button', { type: 'submit' }, 'Search')),
    list, detail);
  load();
}

// ---- review ----
async function reviewView() {
  const mem = h('ul', {}), learn = h('ul', {}), rules = h('ul', {});
  async function load() {
    for (const [ul, path, kind] of [[mem, '/v1/memory/proposals', 'memory'], [learn, '/v1/learn/proposals', 'learn']]) {
      ul.replaceChildren();
      try {
        const v = await json(path);
        if (!v.proposals.length) ul.append(h('li', {}, 'Nothing waiting.'));
        for (const p of v.proposals) {
          const text = kind === 'memory' ? `${p.kind}: ${p.title}: ${p.text}` : `${p.kind}: ${p.payload.text || p.payload.added || p.payload.name || JSON.stringify(p.payload)}`;
          const why = (p.evidence || [])[0];
          ul.append(h('li', {}, h('p', {}, text, p.privacy === 'sensitive' ? ' [sensitive]' : ''), why ? h('p', {}, `because: "${why}"`) : null,
            h('button', { type: 'button', onclick: async () => { await json(`${path}/${p.id}/accept`, { method: 'POST', body: {} }); load(); } }, 'Accept'), ' ',
            h('button', { type: 'button', onclick: async () => { await json(`${path}/${p.id}/reject`, { method: 'POST', body: {} }); load(); } }, 'Drop')));
        }
      } catch (e) { ul.append(h('li', { role: 'alert' }, e.message)); }
    }
    rules.replaceChildren();
    try {
      const v = await json('/v1/learn/rules?all=true');
      if (!v.rules.length) rules.append(h('li', {}, 'No rules yet.'));
      for (const r of v.rules) rules.append(h('li', {}, `${r.text}${r.active ? '' : ' [off]'}${r.privacy === 'sensitive' ? ' [sensitive]' : ''} `,
        h('button', { type: 'button', onclick: async () => { await json(`/v1/learn/rules/${r.id}/status`, { method: 'POST', body: { active: !r.active } }); load(); } }, r.active ? 'Switch off' : 'Switch on')));
    } catch (e) { rules.append(h('li', { role: 'alert' }, e.message)); }
  }
  show('Review', h('h3', {}, 'Notes to remember'), mem, h('h3', {}, 'What it has learned'), learn, h('h3', {}, 'Rules in force'), rules);
  load();
}

// ---- departments ----
// The Departments settings page keeps its open forms here between redraws, as GPUI keeps them in its state.
const teamUi = { role: null, dept: null, roleHist: null, deptHist: null, typeEdit: null, typeDel: null, notes: {} };
let deptPick = '';
const AUTONOMY = [['plan-first', 'Ask me to approve the plan first'], ['check-ins', 'Also pause at milestones'], ['fire-and-forget', 'Do not ask me anything']];
const UNAVAILABLE = [['stop', 'Stop and tell me'], ['fallback', 'Use another model']];
// The tools a role may list, the core's own list, and the most a role's instructions may hold.
const KNOWN_TOOLS = ['read_file', 'write_file', 'list_dir', 'run_command', 'web_fetch', 'web_search', 'generate_image', 'recall', 'open_note', 'remember', 'forget', 'use_skill', 'read_skill_file'];
const INSTRUCTIONS_MAX = 600;
const validName = (n) => n.length > 0 && n.length <= 40 && /^[a-z0-9]+(-[a-z0-9]+)*$/.test(n);
const quote = (s) => '"' + s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n') + '"';
async function teamView() {
  const depts = h('select', { id: 'dept' });
  const goal = h('textarea', { rows: '3', id: 'goal' });
  const log = h('div', { role: 'log', 'aria-live': 'polite', 'aria-label': 'Progress' });
  const asks = h('div', {});
  const workers = h('ul', { 'aria-label': 'Workers' });
  const away = h('p', {});
  const err = h('p', { role: 'alert' });
  try { (await json('/v1/team/departments')).departments.filter((d) => !d.error).forEach((d) => depts.append(h('option', { value: d.name }, `${d.name}: ${d.description || ''}`))); if (deptPick) { depts.value = deptPick; deptPick = ''; } } catch (e) { err.textContent = e.message; }
  const line = (t) => log.append(h('p', {}, t));
  async function follow(id) {
    await stream(`/v1/team/runs/${id}/events`, {}, (ev) => {
      const s = (k) => ev[k] ?? '';
      switch (ev.type) {
        case 'planning': line(`${s('dept')} is planning…`); break;
        case 'plan': line('Plan: ' + ev.steps.map((x) => `${x.id}. ${x.title} (${x.assignee})`).join('; ')); break;
        case 'plan_needed': {
          const yes = h('button', { type: 'button' }, 'Run this plan'), no = h('button', { type: 'button' }, 'Do not run it');
          const box = h('div', { role: 'alertdialog', 'aria-label': 'Approve the plan' }, h('p', {}, 'Approve this plan?'), h('ol', {}, ev.steps.map((x) => h('li', {}, `${x.title} (${x.assignee})`))), h('p', {}, yes, ' ', no));
          yes.addEventListener('click', async () => { box.remove(); await api(`/v1/team/runs/${id}/answers/${ev.id}`, { method: 'POST', body: { decision: 'approve' } }); });
          no.addEventListener('click', async () => { box.remove(); await api(`/v1/team/runs/${id}/answers/${ev.id}`, { method: 'POST', body: { decision: 'reject', note: 'not approved' } }); });
          asks.append(box); yes.focus(); break;
        }
        case 'checkin_needed': {
          const yes = h('button', { type: 'button' }, 'Go on'), no = h('button', { type: 'button' }, 'Stop here');
          const box = h('div', { role: 'alertdialog', 'aria-label': 'Milestone reached' }, h('p', {}, `Milestone ${s('milestone')} is done.`), h('p', {}, (ev.summaries || []).join('; ')), h('p', {}, yes, ' ', no));
          for (const [b, d] of [[yes, 'continue'], [no, 'stop']]) b.addEventListener('click', async () => { box.remove(); await api(`/v1/team/runs/${id}/answers/${ev.id}`, { method: 'POST', body: { decision: d } }); });
          asks.append(box); yes.focus(); break;
        }
        case 'ask': question(asks, `/v1/team/runs/${id}/answers/${ev.id}`, `${s('worker')} wants to use ${s('tool')}: ${s('what')}`, s('reason')); break;
        case 'step_started': line(`[${s('step')}] ${s('worker')} starts "${s('title')}"`); break;
        case 'step_checked': line(`[${s('step')}] check: ${s('verdict')}`); break;
        case 'step_retry': line(`[${s('step')}] ${s('reason')}; trying again`); break;
        case 'step_done': line(`[${s('step')}] done: ${s('summary')}`); break;
        case 'step_failed': line(`[${s('step')}] failed: ${s('report')}`); break;
        case 'step_parked': line(`[${s('step')}] set aside for you: ${s('why')}`); break;
        case 'step_blocked': line(`[${s('step')}] not run`); break;
        case 'away_denied': line(`Said no while you were away: ${s('worker')} asked to ${s('what')}`); break;
        case 'model_note': case 'plan_decided': if (ev.note) line(ev.note); break;
        case 'outcome': line(ev.status === 'done' ? `Result: ${ev.result}` : `Stopped (${ev.status}): ${ev.report || ev.note || ''}`); break;
        default: break;
      }
    });
  }
  const form = h('form', { onsubmit: async (e) => {
    e.preventDefault(); err.textContent = '';
    try { const r = await json('/v1/team/runs', { method: 'POST', body: { department: depts.value, goal: goal.value.trim() } }); log.replaceChildren(); follow(r.id).catch((x) => { err.textContent = x.message; }); } catch (x) { err.textContent = x.message; }
  } }, field('Department', depts), h('p', {}, h('label', { for: 'goal' }, 'Goal')), h('p', {}, goal), h('p', {}, h('button', { type: 'submit' }, 'Give it the goal')), err);
  async function refresh() {
    try {
      const w = (await json('/v1/team/workers')).workers || [];
      workers.replaceChildren(...(w.length ? w.map((x) => h('li', {}, `${x.role} in ${x.dept}: ${x.step} [${x.status}] on ${x.model} `,
        ...['pause', 'resume', 'stop'].map((a) => h('button', { type: 'button', onclick: async () => { await api(`/v1/team/workers/${x.id}/${a}`, { method: 'POST' }); refresh(); } }, a[0].toUpperCase() + a.slice(1))))) : [h('li', {}, 'Nobody is working right now.')]));
      const a = await json('/v1/team/away');
      away.replaceChildren(`Away mode is ${a.away ? 'on' : 'off'}. `, h('button', { type: 'button', onclick: async () => { await json('/v1/team/away', { method: 'POST', body: { on: !a.away } }); refresh(); } }, a.away ? 'I am back' : 'I am going away'),
        ...(a.decisions || []).map((d) => h('p', {}, `Decided alone in ${d.dept}: ${d.decision}`)), ...(a.parked || []).map((p) => h('p', {}, `Set aside for you: ${p}`)));
    } catch (e) { workers.replaceChildren(h('li', { role: 'alert' }, e.message)); }
  }
  show('Departments', form, asks, log, h('h3', {}, 'Who is working'), workers, h('h3', {}, 'Away mode'), away);
  refresh();
}

// ---- settings ----
// ---- settings, as in the desktop app (crates/lh-gui/src/setting_names.rs): a window with its pages listed ----
// Fonts. A browser cannot list the fonts on the computer, so a family counts as installed when text in it measures
// differently from the fallback. Added fonts are kept in this browser and loaded with FontFace.
let fontOpen = ''; let fontNote = '';
const FONT_CANDIDATES = ['Adwaita Sans', 'Inter', 'Noto Sans', 'Noto Serif', 'DejaVu Sans', 'DejaVu Serif', 'Liberation Sans', 'Liberation Serif', 'Cantarell', 'Ubuntu', 'Roboto', 'Open Sans', 'Source Sans 3', 'IBM Plex Sans', 'Fira Sans', 'Lato', 'Arial', 'Helvetica', 'Segoe UI', 'Georgia', 'Times New Roman', 'Verdana', 'SF Pro Text', 'Noto Naskh Arabic', 'Noto Sans Arabic', 'Noto Kufi Arabic', 'Amiri', 'Cairo', 'Tajawal', 'Vazirmatn'];
const ARABIC_FIRST = /arab|naskh|kufi|amiri|cairo|tajawal|vazir/i;
const fontProbe = document.createElement('canvas').getContext('2d');
function fontInstalled(name) {
  if (!name) return false;
  return ['monospace', 'serif'].some((base) => { fontProbe.font = `40px ${base}`; const a = fontProbe.measureText('mmmwwwlli أبجد 0123').width; fontProbe.font = `40px "${name}", ${base}`; return fontProbe.measureText('mmmwwwlli أبجد 0123').width !== a; });
}
function addedFonts() { try { return JSON.parse(pref('fonts-added') || '[]'); } catch { return []; } }
function installedFonts(slot) {
  const names = [...new Set([...addedFonts().map((a) => a.family), ...FONT_CANDIDATES.filter(fontInstalled)])].sort((a, b) => a.localeCompare(b));
  return slot === 'arabic' ? [...names.filter((n) => ARABIC_FIRST.test(n)), ...names.filter((n) => !ARABIC_FIRST.test(n))] : names;
}
async function loadAddedFonts() { for (const a of addedFonts()) { try { const f = new FontFace(a.family, `url(${pref(`font-data-${a.family}`)})`); await f.load(); document.fonts.add(f); } catch { /* a broken font is skipped */ } } }
async function addFont(file) {
  if (!/\.(ttf|otf|ttc)$/i.test(file.name)) return /\.woff2?$/i.test(file.name) ? 'Web fonts (.woff, .woff2) are not supported. Use a .ttf, .otf or .ttc file.' : 'That file type is not a font. Use a .ttf, .otf or .ttc file.';
  if (file.size > 3e6) return 'That font is too big to keep in the browser (over 3 MB).';
  const url = await new Promise((ok) => { const r = new FileReader(); r.onload = () => ok(r.result); r.readAsDataURL(file); });
  const family = file.name.replace(/\.[^.]+$/, '').replace(/[-_](Regular|Book|Roman)$/i, '').replace(/[-_]/g, ' ');
  try { const f = new FontFace(family, `url(${url})`); await f.load(); document.fonts.add(f); } catch { return 'The file is not a font the browser can read.'; }
  try { localStorage.setItem(`font-data-${family}`, url); } catch { return 'There is no room left in the browser to keep that font.'; }
  pref('fonts-added', JSON.stringify([...addedFonts().filter((a) => a.family !== family), { family, file: file.name }]));
  return `Added ${family}.`;
}
function removeFont(family) { pref('fonts-added', JSON.stringify(addedFonts().filter((a) => a.family !== family))); try { localStorage.removeItem(`font-data-${family}`); } catch { /* storage may be off */ } if (pref('font-latin') === family) pref('font-latin', ''); if (pref('font-arabic') === family) pref('font-arabic', ''); applyLayout(); }
// the outline icon of each Settings page, from the desktop app (chrome.rs category_icon), on its 24 by 24 grid
const PAGE_ICON = {"appearance": "M4 12a8 8 0 1 0 16 0a8 8 0 1 0 -16 0M12 4v16", "models": "M7 7h10v10H7zM10 4v3M14 4v3M10 17v3M14 17v3M4 10h3M4 14h3M17 10h3M17 14h3", "chats": "M4 5h16v11H9l-5 4z", "memory": "M5 4h11l3 3v13H5zM8 11h8M8 15h8", "learning": "M6 10a6 6 0 1 0 12 0a6 6 0 1 0 -12 0M9.5 16v3h5v-3M10.5 21h3", "departments": "M10 5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M3 19a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M17 19a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M12 7v5l-6 5M12 12l6 5", "permissions": "M12 3.5l7 2.5v6l-7 8.5L5 12V6z", "spending": "M4 12a8 8 0 1 0 16 0a8 8 0 1 0 -16 0M12 7v10M9.5 9.5h5M9.5 14.5h5", "web": "M4 12a8 8 0 1 0 16 0a8 8 0 1 0 -16 0M8.5 12a3.5 8 0 1 0 7.0 0a3.5 8 0 1 0 -7.0 0M4 12h16", "tools": "M8 10h8v4l-4 4-4-4zM10 5v5M14 5v5M12 18v3", "channels": "M4 11l16-7-6 16-3-7zM11 13l9-9", "data": "M5 6a7 2.5 0 1 0 14 0a7 2.5 0 1 0 -14 0M5 6v12M19 6v12M5 18a7 2.5 0 0 0 14 0M5 12a7 2.5 0 0 0 14 0", "activity": "M3 12h5l2-6 4 12 2-6h5", "hooks": "M12 5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0M14 7v8l-1.5 3.5-3 1-2.5-2-.5-4M6.5 13.5l3 2", "schedules": "M4 12a8 8 0 1 0 16 0a8 8 0 1 0 -16 0M12 7v5l3.5 2", "updates": "M12 4v10M8 10l4 4 4-4M4 15v4h16v-4", "other": "M4.8 12a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0 -2.4 0M10.8 12a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0 -2.4 0M16.8 12a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0 -2.4 0"};
const pageIcon = (id) => s('svg', { viewBox: '0 0 24 24', 'aria-hidden': 'true' }, s('path', { d: PAGE_ICON[id] || '' }));
const SET_PAGES = [["appearance", "Appearance", "How the window looks and how wide the chat is"], ["models", "Models and keys", "Where your models come from, and the key to reach them"], ["chats", "Chats and speed", "How chats are kept, shortened and resumed"], ["memory", "Memory", "What the app remembers about you, and how much of it goes with a message"], ["learning", "Learning", "Rules, how I talk to you, and skills, with a way back for each change"], ["departments", "Departments", "Departments, their roles and model types, and how they work on a big goal"], ["permissions", "Permissions", "What agents may do without asking. Take care here."], ["spending", "Spending", "What happens when a model service costs money"], ["web", "Web and phone", "Web search, and reaching the app from a browser or a phone"], ["tools", "Tools and MCP", "Servers that give the assistant more tools, and folders that may bring their own"], ["channels", "Channels", "Talk to the assistant from Telegram, Matrix, Signal or Slack. All are off until you switch them on."], ["data", "Your data", "Take your memory, rules and roles with you, and start with the computer"], ["activity", "Activity", "What the assistant did, newest first, and what it cost"], ["hooks", "Hooks", "Your own commands, run before or after the assistant uses a tool"], ["schedules", "Scheduled tasks", "Tasks the assistant starts on its own, on a timetable"], ["updates", "Updates and reports", "Updates and crash reports"], ["other", "More", "Settings that have no page of their own yet"]];
const SET_INFO = [["provider.url", "models", "Model server address", "Where your models are served from, for example http://localhost:9292/v1.", "Text", "Not set", false], ["provider.name", "models", "Name of the model service", "Told to the model so it knows what serves it. Leave it empty to use the name the server gives.", "Text", "Use the server's own name", false], ["provider.key", "models", "Key for the model service", "Points to the stored secret that holds your key. The key itself is never written in the settings file.", "Text", "None", false], ["media.image_model", "models", "Image model", "The model that makes images. Leave it empty to turn image making off.", "Text", "Off", false], ["memory.enabled", "memory", "Remember between chats", "Keeps notes about you and your work so later chats can use them.", "OnOff", null, false], ["memory.trust", "memory", "Where memory may be sent", "Sensitive notes never leave this computer. This sets how far the model service may see the rest.", "Words", [["auto", "Decide for me"], ["local", "This computer only"], ["own", "My own cloud account"], ["any", "Any cloud service"]], false], ["memory.recall_items", "memory", "Notes added to a message", "The most notes memory adds to one message you send.", "Count", ["note", "notes"], false], ["memory.recall_chars", "memory", "Memory added to a message, in characters", "The most text, all notes together, that memory adds to one message you send.", "Count", ["character", "characters"], false], ["learning.mode", "learning", "Learning from your habits", "How the app picks up your habits. It can suggest and wait for you, apply changes quietly, or not learn.", "Words", [["propose", "Suggest and wait"], ["silent", "Apply quietly"], ["off", "Off"]], false], ["learning.threshold", "learning", "Chats before it counts as a habit", "How many chats a suggestion must come up in before the app treats it as a habit.", "Count", ["chat", "chats"], false], ["team.autonomy", "departments", "How much departments ask you", "How often a department checks with you while it works on a big goal.", "Words", [["plan-first", "Show a plan first"], ["check-ins", "Check in as it goes"], ["fire-and-forget", "Just do it"]], false], ["team.parallel", "departments", "Workers at the same time", "How many workers a department may run at once.", "Count", ["worker", "workers"], false], ["team.attempts", "departments", "Tries for each step", "How many times a step is tried before the failure is passed up to you.", "Count", ["try", "tries"], false], ["team.unavailable", "departments", "When a model is missing", "Stop and tell you, or try the backup models listed for that kind of job.", "Words", [["stop", "Stop and tell me"], ["fallback", "Try a backup model"]], false], ["web.listen", "web", "Address for the browser and phone", "Lets you use LastHarness in a browser over HTTPS, for example at 127.0.0.1:7843. Every device must be paired first. Leave it empty to keep this off.", "Text", "Off", true], ["web.tailscale", "web", "Reach it from your phone over Tailscale", "Opens the browser page on this computer's Tailscale address only, so a phone on your tailnet can use it. Sign the phone in under Phone, after a restart.", "OnOff", null, true], ["tools.codemode", "tools", "Let the model write small scripts", "The model can run a short script that calls its other tools and keeps only what matters from long results. Every call inside is checked like any other. Off by default: it adds one tool to every request.", "OnOff", null, false], ["tools.sandbox", "tools", "Keep commands in the chat's folder", "Commands the model runs can change files only in the chat's folder and a private temporary folder. They can still read the rest, and network access is set under Permissions. Needs bubblewrap.", "OnOff", null, true], ["web.use_proxy", "web", "Use the system proxy for web pages", "A proxy looks up names itself, so the check against private addresses cannot see where a request goes. This is off unless you turn it on.", "OnOff", null, true], ["web.searxng", "web", "Web search server", "The address of a SearXNG server used for web search. Leave it empty to turn search off.", "Text", "Off", false], ["spending.mode", "spending", "What happens as costs add up", "Only record what you spend, stop at your limits, or ask before a call that looks expensive.", "Words", [["none", "Only record"], ["caps", "Stop at my limits"], ["estimate", "Ask before costly calls"]], true], ["spending.approve_over_usd", "spending", "Ask before a call over", "When asking is on, you are asked before one call that is expected to cost more than this.", "Money", null, true], ["spending.per_request_cap_usd", "spending", "Most for one call", "Refuses any single call that would cost more than this.", "Money", null, true], ["spending.daily_cap_usd", "spending", "Most per day", "Refuses further calls once a day's spending passes this.", "Money", null, true], ["spending.monthly_cap_usd", "spending", "Most per month", "Refuses further calls once a month's spending passes this.", "Money", null, true], ["permissions.mode", "permissions", "When to ask permission", "Reads are free, writes ask once, and risky things always ask. Or ask for everything.", "Words", [["tiered", "Ask for risky things"], ["ask-all", "Ask for everything"]], true], ["permissions.bypass", "permissions", "Skip all permission questions", "Turns every permission question off. You accept the risk, and the project is not responsible for the outcome.", "OnOff", null, true], ["permissions.show_diff", "permissions", "Show the changes of a file write", "When a write is allowed without asking, the chat shows what changed. Choose where that happens.", "Words", [["always", "Always"], ["untrusted", "In folders I have not trusted"], ["trusted", "In trusted folders"], ["never", "Never"]], false], ["tools.sandbox_network", "permissions", "Network for sandboxed commands", "Whether commands in the sandbox may use the network. Ask each run, allow only the sites listed below, always allow, or let a small model decide and ask you when it cannot.", "Words", [["ask", "Ask each time"], ["allowlist", "Only listed sites"], ["on", "Always"], ["auto", "Let a model decide"]], true], ["tools.sandbox_network_allow", "permissions", "Sites sandboxed commands may reach", "Used when only listed sites are allowed. Separate them with commas, for example github.com, crates.io. A site covers its subdomains.", "Text", "None", false], ["tools.sandbox_network_model", "permissions", "Model that decides network access", "A small, fast model that decides each run when a model decides. Leave it empty and you are asked instead.", "Text", "Not set, so you are asked", false], ["permissions.sudo", "permissions", "Allow sudo", "Lets agents run commands with sudo. It still asks like any other command.", "OnOff", null, true], ["tree.summarize", "chats", "Summary when you leave a branch", "When you move to another branch of a chat, a short summary of the branch you left can be written where you come back.", "Words", [["ask", "Ask each time"], ["always", "Always"], ["never", "Never"]], false], ["kvcache.enabled", "chats", "Saved chat memory for faster resume", "Saves the local model's work on a chat so a stored chat goes on quickly instead of being read again. It needs the folder below, and is never used for private chats or cloud models.", "OnOff", null, false], ["kvcache.dir", "chats", "Folder for saved chat memory", "The folder your local model server saves into. With llama-swap, it is the folder that holds the per-model ones. Leave it empty to turn saving off.", "Text", "Not set, so saving is off", false], ["kvcache.max_gb", "chats", "Most disk for saved chat memory", "The most disk the saved chat memory may use in total. The oldest is removed first.", "Gb", null, false], ["memory.check_model", "memory", "Model that checks notes", "While the app is idle, a model on this computer looks for notes that disagree and suggests archiving the older one. You accept or reject each suggestion.", "Text", "Not set, so notes are not checked", false], ["chats.title_model", "chats", "Model that names chats", "A small, fast model that gives a chat a short name after the first reply. Leave it empty to use the first words of your first message. Private chats are never named by a model.", "Text", "Not set, so the first words are used", false], ["core.idle_after", "chats", "Sleep after being idle for", "How long the core waits without any activity before it goes idle. Never means it stays awake.", "Duration", null, false], ["core.compact_at", "chats", "Shorten a long chat at", "Long conversations are shortened at about this size. Automatic means three quarters of what the model can hold. Never means no shortening.", "Tokens", null, false], ["reports.crash", "updates", "Crash reports", "Anonymous reports when something breaks. You can send none, see exactly what would be sent and decide, or send them automatically.", "Words", [["off", "Never"], ["ask", "Ask first"], ["auto", "Send automatically"]], false], ["updates.enabled", "updates", "Check for updates", "Looks for signed updates.", "OnOff", null, false]];
function shownValue(kind, arg, raw) {
  raw = String(raw ?? '');
  if (kind === 'OnOff') return raw === 'true' ? 'On' : raw === 'false' ? 'Off' : raw;
  if (kind === 'Words') return (arg.find((w) => w[0] === raw) || [raw, raw])[1];
  return raw;
}
let setPage = pref('settings-page') || 'appearance';
async function settingsView() {
  setPage = pref('settings-page') || setPage;
  $('#panel').hidden = true;
  $('.settings-veil')?.remove();
  let core = [];
  let err = '';
  try { core = (await json('/v1/settings')).settings || []; } catch (e) { err = e.message; }
  const byKey = Object.fromEntries(core.map((s) => [s.key, s]));
  const q = h('input', { type: 'search', placeholder: 'Search the settings', 'aria-label': 'Search the settings' });
  const nav = h('nav', { class: 'settings-nav', 'aria-label': 'Settings pages' }, q);
  const page = h('div', { class: 'settings-page' });
  const close = () => { veil.remove(); if (location.hash === '#/settings') location.hash = '#/chat'; };
  const veil = h('div', { class: 'settings-veil', onclick: (e) => { if (e.target === veil) close(); } },
    h('div', { class: 'settings-win', role: 'dialog', 'aria-label': 'Settings' },
      h('div', { class: 'settings-top' }, h('h2', {}, 'Settings'), h('span', { class: 'kbd' }, 'Esc'), h('button', { type: 'button', class: 'ghost', onclick: close }, 'Back to chat')),
      h('div', { class: 'settings-main' }, nav, page)));
  veil.addEventListener('keydown', (e) => { if (e.key === 'Escape') close(); });
  queueMicrotask(() => { arrive(veil, {}); arrive(veil.firstChild, { y: 10, scale: 0.985 }); });
  const pages = () => {
    const words = q.value.toLowerCase().split(/\s+/).filter(Boolean);
    const hit = (id) => SET_INFO.some((i) => i[1] === id && words.every((w) => `${i[2]} ${i[3]} ${i[0]}`.toLowerCase().includes(w)));
    const list = SET_PAGES.filter(([id, title]) => !words.length || words.every((w) => title.toLowerCase().includes(w)) || hit(id));
    nav.replaceChildren(q, ...list.map(([id, title]) => h('button', { type: 'button', class: 'nav-row', 'aria-pressed': String(id === setPage), onclick: () => { setPage = id; pref('settings-page', id); pages(); draw(); } }, pageIcon(id), h('span', {}, title))));
  };
  const item = (i) => {
    const [key, , title, about, kind, arg, careful] = i;
    const s = byKey[key];
    const v = s ? s.value : '';
    let input;
    const save = async (val) => { try { const r = await json('/v1/settings', { method: 'PUT', body: { [key]: String(val) } }); if (s) s.value = val; say(r.restart_needed?.length ? 'Saved. Applies at the next start.' : 'Saved.'); } catch (e) { say(e.message); } };
    if (kind === 'OnOff') { input = h('input', { type: 'checkbox', class: 'switch', 'aria-label': title }); input.checked = String(v) === 'true'; input.onchange = () => save(input.checked); }
    else if (kind === 'Words') input = h('span', { class: 'set-row', role: 'radiogroup', 'aria-label': title }, arg.map(([w, label]) => h('button', { type: 'button', class: 'seg', 'aria-pressed': String(String(v) === w), onclick: (e) => { for (const b of e.target.parentNode.children) b.setAttribute('aria-pressed', String(b === e.target)); save(w); } }, label)));
    else { input = h('input', { type: ['Count', 'Gb', 'Duration'].includes(kind) ? 'number' : 'text', value: v ?? '', placeholder: kind === 'Text' ? arg : '', 'aria-label': title }); input.onchange = () => save(input.value); }
    // one card per setting, as GPUI's setting_card: the title with its control beside it, the help below; a text box goes under the help
    const wide = kind === 'Text';
    return h('div', { class: 'set-item' }, h('div', { class: 'set-head' }, h('div', { class: 'title' }, title), wide ? null : input), h('p', { class: 'about' }, about), wide ? input : null,
      careful ? h('p', { class: 'about' }, 'Take care with this one.') : null);
  };
  const appearance = () => {
    const fam = lookFamily(); const following = lookMode() === 'system';
    const cur = `${fam}/`;
    const names = [...new Set(themes.map((t) => t.name))];
    const pick = (name) => { setLook(name, null); draw(); };
    const dark = h('input', { type: 'checkbox', class: 'switch', 'aria-label': 'Dark mode' }); dark.checked = themeKey().endsWith('/dark'); dark.disabled = following;
    dark.onchange = () => { setLook(null, dark.checked ? 'dark' : 'light'); draw(); };
    const sys = h('input', { type: 'checkbox', class: 'switch', 'aria-label': 'Match the desktop' }); sys.checked = following;
    sys.onchange = () => { setLook(null, sys.checked ? 'system' : (sysDark() ? 'dark' : 'light')); draw(); };
    const col = () => pref('chat-width') || '860';
    const setCol = (v) => { pref('chat-width', v); applyLayout(); draw(); };
    const width = h('input', { type: 'range', min: 480, max: 1410, step: 10, value: col() === 'full' ? 1410 : col(), 'aria-label': 'Chat width' });
    const num = h('input', { type: 'number', min: 480, step: 10, value: col() === 'full' ? '' : col(), 'aria-label': 'Chat width in pixels' });
    width.onchange = () => setCol(+width.value >= 1410 ? 'full' : width.value);
    width.oninput = () => { num.value = width.value; };
    num.onchange = () => setCol(String(Math.max(480, Math.round(+num.value / 10) * 10)));
    const presets = JSON.parse(pref('width-presets') || '[]');
    const label = (w) => (w === 'full' ? 'Full' : `${w} px`);
    const chip = (w, text) => h('button', { type: 'button', class: col() === w ? 'seg primary' : 'seg', 'aria-pressed': String(col() === w), onclick: () => setCol(w) }, text);
    const pname = h('input', { type: 'text', placeholder: 'Name for this width', 'aria-label': 'Preset name' });
    const savePreset = () => { const n = pname.value.trim(); if (!n || presets.length >= 12) return; presets.push({ name: n.slice(0, 40), width: col() }); pref('width-presets', JSON.stringify(presets)); draw(); };
    // a button with the font in use that opens a short scrolling list of installed fonts, as the desktop app has it
    const fontSel = (slot, title) => {
      const t = themes.find((x) => `${x.name}/${x.mode}` === themeKey()) || themes.find((x) => x.name === lookFamily());
      const tf = (t?.fonts?.[slot === 'latin' ? 'latin' : 'arabic']) || (slot === 'latin' ? t?.fonts?.ui : null);
      const themePick = [].concat(tf || []).find((f) => fontInstalled(f));
      const dflt = themePick ? `Theme default (${themePick})` : 'Default';
      const chosen = pref(`font-${slot}`) || '';
      const shown = chosen || (slot === 'arabic' && !themePick ? (['Noto Naskh Arabic', 'Noto Sans Arabic'].find(fontInstalled) || 'System default') : dflt);
      const open = fontOpen === slot;
      const toggle = h('button', { type: 'button', class: 'btn font-btn', 'aria-expanded': String(open), onclick: () => { fontOpen = open ? '' : slot; draw(); } }, `${title}: ${shown}`);
      if (!open) return h('div', { class: 'font-pick' }, toggle);
      const names = installedFonts(slot);
      const list = h('div', { class: 'font-list', role: 'list', 'aria-label': title }, ['', ...names].map((n) => h('button', { type: 'button', class: n === chosen ? 'btn primary' : 'btn ghost', onclick: () => { pref(`font-${slot}`, n); fontOpen = ''; applyLayout(); draw(); } }, n || dflt)));
      return h('div', { class: 'font-pick' }, toggle, list);
    };
    const added = addedFonts();
    const fontFile = h('input', { type: 'file', accept: '.ttf,.otf,.ttc', hidden: true });
    fontFile.onchange = async () => { const f = fontFile.files[0]; if (f) { fontNote = await addFont(f); draw(); } };
    const fontAdd = h('div', { class: 'font-pick' }, h('button', { type: 'button', class: 'btn', onclick: () => fontFile.click() }, 'Add font…'), fontFile,
      fontNote ? h('p', { class: 'note' }, fontNote) : null,
      added.map((a) => h('div', { class: 'set-row' }, h('span', { class: 'note' }, a.file), h('button', { type: 'button', class: 'btn ghost', onclick: () => { removeFont(a.family); draw(); } }, 'Remove'))));
    const pages = h('input', { type: 'checkbox', class: 'switch', 'aria-label': 'Show web pages inside the app' }); pages.checked = pref('pages-off') !== 'true';
    pages.onchange = () => pref('pages-off', String(!pages.checked));
    const DIRS = { auto: 'Direction: follow the desktop language', ltr: 'Direction: left to right', rtl: 'Direction: right to left (mirrored)' };
    const dir = pref('direction') || 'auto';
    const side = (d) => { pref('side-width', String(Math.min(420, Math.max(180, (+pref('side-width') || 260) + d)))); applyLayout(); };
    // the Glass look's colours: up to three dots on a colour field and how strongly they show (lh-gui panels.rs)
    const glassBlock = () => {
      const picks = jsonPref('glass-colours', []);
      const save = (v) => { pref('glass-colours', JSON.stringify(v)); setLook(); draw(); };
      const toggle = (hex) => { const v = [...picks]; const i = v.indexOf(hex); if (i >= 0) v.splice(i, 1); else { if (v.length >= 3) v.length = 2; v.push(hex); } save(v); };
      const field = h('div', { class: 'glass-field', role: 'group', 'aria-label': 'Glass colours' });
      for (let r = 0; r < GLASS_ROWS; r++) for (let c = 0; c < GLASS_COLS; c++) { const hex = glassField(c, r); field.append(h('button', { type: 'button', class: 'glass-cell', style: `background:${hex}`, 'aria-label': hex, 'aria-pressed': String(picks.includes(hex)), onclick: () => toggle(hex) })); }
      const op = pref('glass-opacity') === null ? 0.5 : +pref('glass-opacity');
      return h('div', { class: 'glass-block' },
        h('div', { class: 'set-row' }, h('span', { class: 'lbl' }, 'Glass colours (up to three)'), picks.map((c) => h('span', { class: 'glass-swatch', style: `background:${c}` })), h('button', { type: 'button', class: 'seg ghost', onclick: () => save([]) }, 'No colours')),
        field,
        h('div', { class: 'set-row' }, h('span', { class: 'lbl' }, 'Opacity'), [0.2, 0.4, 0.6, 0.8, 1].map((v) => h('button', { type: 'button', class: Math.abs(op - v) < 0.05 ? 'seg primary' : 'seg', 'aria-pressed': String(Math.abs(op - v) < 0.05), onclick: () => { pref('glass-opacity', String(v)); setLook(); draw(); } }, `${Math.round(v * 100)}%`))),
        h('p', { class: 'about' }, 'Words stay readable: the tint softens itself when it has to.'));
    };
    const name = h('input', { type: 'text', placeholder: 'Your name', value: pref('name') || '' });
    name.onchange = () => { pref('name', name.value); userName = name.value; greet(); };
    return h('div', { class: 'set-card' },
      h('label', { class: 'set-row' }, dark, 'Dark mode', following ? h('span', { class: 'note' }, sysDark() ? 'Following the desktop: dark now' : 'Following the desktop: light now') : null), h('label', { class: 'set-row' }, sys, 'Match the desktop'),
      h('div', { class: 'set-row' }, h('span', { class: 'lbl' }, 'Look'), names.map((n) => h('button', { type: 'button', class: 'seg', 'aria-pressed': String(cur.startsWith(n + '/')), onclick: () => pick(n) }, n))),
      fam === 'Glass' ? glassBlock() : null,
      h('div', { class: 'set-row', role: 'group', 'aria-label': 'Chat width' }, h('span', { class: 'lbl' }, 'Chat width'), width, num, h('span', { class: 'lbl' }, col() === 'full' ? 'Full' : 'px')),
      h('div', { class: 'set-row' }, h('span', { class: 'lbl' }, 'Presets'), ['700', '900', '1100', 'full'].map((w) => chip(w, label(w))), presets.map((p, i) => h('span', { class: 'set-row' }, chip(p.width, `${p.name} · ${label(p.width)}`), h('button', { type: 'button', class: 'seg danger', 'aria-label': `Delete ${p.name}`, onclick: () => { presets.splice(i, 1); pref('width-presets', JSON.stringify(presets)); draw(); } }, '×')))),
      h('div', { class: 'set-row' }, pname, h('button', { type: 'button', class: 'seg', onclick: savePreset }, 'Save this width')),
      fontSel('latin', 'Latin font'), fontSel('arabic', 'Arabic font'), fontAdd,
      h('div', {}, h('div', { class: 'title' }, 'Your name on the new-chat screen'), name, h('p', { class: 'about' }, 'Leave it empty to use your name from the system.')),
      h('div', {}, h('label', { class: 'set-row' }, pages, 'Show web pages inside the app'), h('p', { class: 'about' }, 'No page can load anything from the internet or from this computer. Small apps run their scripts with no network. Pages over 2 MB show as a card instead.')),
      h('div', { class: 'set-row' },
        h('button', { type: 'button', class: 'seg', onclick: () => { pref('direction', { auto: 'ltr', ltr: 'rtl', rtl: 'auto' }[dir]); applyLayout(); draw(); } }, DIRS[dir]),
        h('button', { type: 'button', class: 'seg ghost', onclick: () => side(-30) }, 'Chat list narrower'),
        h('button', { type: 'button', class: 'seg ghost', onclick: () => side(30) }, 'Chat list wider')),
      h('p', { class: 'about' }, 'The Omarchy look, and your own looks from the themes folder, show here when the page is opened from the desktop app.'));
  };
  // Web and phone, as the desktop app has it: the phone link, a pairing window, a PIN, then the paired devices.
  // A paired browser may not pair others (the core refuses it), so those parts show only where the core allows them.
  async function devices() {
    const out = [];
    const tailscale = "Turn on \"Reach it from your phone over Tailscale\" above, then restart the app. A sign-in link for your phone appears here.";
    const ph = await json('/v1/devices/phone').catch(() => null);
    out.push(sub('Phone'));
    if (!ph) out.push(muted(tailscale));
    else if (!ph.link) out.push(muted(ph.enabled === true ? 'The core is not listening on an address a phone can reach. Check that Tailscale is running on this computer, then restart the app.' : tailscale));
    else out.push(muted("Scan this with your phone's camera. The phone must be signed in to Tailscale too. Its browser warns about the certificate once: check the fingerprint below, then continue. The phone stays signed in and shows up under Paired devices."),
      h('p', { class: 'about' }, h('code', {}, ph.link)),
      row(btn('Copy link', () => navigator.clipboard?.writeText(ph.link).then(() => say('Copied.'), () => say('The browser would not copy it.'))), sure('Make a new link', 'Yes, replace the link', () => act('POST', '/v1/devices/phone/rotate'), 'seg')),
      ph.fingerprint ? muted(`Certificate fingerprint: ${ph.fingerprint}`) : null,
      muted('Keep the link private: anyone with it on your tailnet can sign in. Make a new link if it was shared by mistake.'));
    out.push(sub('Paired devices'), muted('A browser or phone can use this app over the address above only after you pair it here. Open pairing for a few minutes, then on the other device open the address and press Pair. It shows a PIN: type that PIN below.'));
    const pend = await json('/v1/devices/pending').catch(() => null);
    if (pend) {
      out.push(row(...[5, 10].map((m) => btn(`Open pairing for ${m} minutes`, () => act('POST', '/v1/devices/open', { minutes: m })))));
      const [pin, pinF] = fieldIn('PIN shown on the other device', '123456');
      const accept = () => { const p = pin.value.replace(/[\s-]/g, ''); if (p.length < 4 || !/^[A-Za-z0-9]+$/.test(p)) return say('Type the PIN the other device shows.'); act('POST', '/v1/devices/accept', { pin: p }); };
      out.push(card(pinF, row(btn('Pair this device', accept, 'seg primary'))));
      if (pend.fingerprint) out.push(muted(`This computer's certificate fingerprint, to compare with what the other device shows: ${pend.fingerprint}`));
      for (const w of pend.pending || []) out.push(card(sub(`Waiting to pair: ${w.name}`), muted(`Its key starts ${String(w.key_fingerprint || '').slice(0, 23)}. Waiting ${w.waiting_secs || 0} seconds.`)));
    } else out.push(muted('Pairing a new device is done on the computer itself: in the desktop app, or with lh devices.'));
    try {
      const list = ((await json('/v1/devices')).devices || []).filter((d) => d.revoked !== true);
      if (!list.length) out.push(muted('No device is paired.'));
      for (const d of list) out.push(h('div', { class: 'set-card', role: 'group', 'aria-label': `Device ${d.name}` },
        h('div', { class: 'set-row' }, h('div', { class: 'grow' }, d.name), sure('Remove', 'Yes, remove it', () => act('DELETE', `/v1/devices/${d.id}`))),
        muted(d.last_seen > 0 ? `Last used ${stamp(d.last_seen)} (UTC)` : 'Not used yet')));
    } catch (e) { out.push(muted(`The device list could not be read: ${e.message}`)); }
    out.push(row(btn('Refresh', () => draw(), 'seg ghost')));
    return out.filter(Boolean);
  }
  // ---- the pages GPUI draws itself, from the same core calls ----
  const muted = (t) => h('p', { class: 'about' }, t);
  const sub = (t) => h('div', { class: 'title' }, t);
  const card = (...k) => h('div', { class: 'set-card' }, ...k);
  const row = (...k) => h('div', { class: 'set-row' }, ...k);
  const btn = (label, fn, cls = 'seg') => h('button', { type: 'button', class: cls, onclick: fn }, label);
  // a button that asks once more before it acts, like GPUI's confirm buttons
  const sure = (label, ask, fn, cls = 'seg danger') => { const b = btn(label, () => { if (b.dataset.armed) return fn(); b.dataset.armed = '1'; b.textContent = ask; }, cls); return b; };
  const act = async (method, path, body, note) => { try { const r = await json(path, { method, body }); say(note ? note(r) : 'Done.'); } catch (e) { say(e.message); } draw(); };
  const fetchOr = async (path, what) => { try { return [await json(path), null]; } catch (e) { return [null, muted(`${what} could not be read: ${e.message}`)]; } };
  const fieldIn = (label, ph = '') => { const i = h('input', { type: 'text', placeholder: ph, 'aria-label': label }); return [i, h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, label), i)]; };
  const stamp = (t) => { const d = new Date(t * 1000); return isNaN(d) ? '' : d.toISOString().slice(0, 16).replace('T', ' '); };
  const ACT_KINDS = [['', 'Everything'], ['tool_call', 'Tools used'], ['permission', 'Permission answers'], ['request_sent', 'Sent to a model service'], ['setting_changed', 'Settings changed'], ['undo', 'Undone'], ['mcp_server', 'MCP servers'], ['folder_trust', 'Folder trust'], ['device', 'Devices'], ['channel', 'Channels']];
  const kindLabel = (k) => (ACT_KINDS.find((x) => x[0] === k) || [k, k.replace(/_/g, ' ')])[1];
  const actSummary = (k, d = {}) => {
    const short = (x, n) => (String(x).replace(/\s+/g, ' ').length > n ? String(x).replace(/\s+/g, ' ').slice(0, n) + '…' : String(x).replace(/\s+/g, ' '));
    if (k === 'tool_call') return d.incognito ? `Used ${d.tool || 'a tool'} in a private chat` : `Used ${d.tool || 'a tool'}${d.result ? ': ' + short(d.result, 80) : ''}`;
    if (k === 'permission') return `${d.tool || 'A tool'} ${(d.outcome || '').replace(/_/g, ' ')}`;
    if (k === 'request_sent') return `Sent a request to ${d.provider || ''} (${d.model || ''})${d.cost_micro_usd != null ? `, cost $${(d.cost_micro_usd / 1e6).toFixed(4)}` : ''}`;
    if (k === 'setting_changed') return `Changed ${d.key || d.setting || 'a setting'}${d.value ? ` to ${d.value}` : ''}`;
    if (k === 'undo') return `Undid the change to ${d.path || 'a file'}`;
    if (k === 'mcp_server') return `MCP server: ${d.op || ''}`;
    if (k === 'folder_trust') return `Folder decision: ${d.decision || ''}`;
    if (k === 'device') return `Device: ${d.op || ''}`;
    if (k === 'schedule_ran') return `Scheduled task "${d.name || ''}" ran: ${d.result || ''}`;
    if (k === 'schedule_added') return `Added the scheduled task "${d.name || ''}"`;
    if (k === 'schedule_changed_files') { const f = d.files || []; return `Scheduled task "${d.name || ''}" changed ${f.length === 1 ? '1 file' : `${f.length} files`}: ${short(f.join(', '), 160)}`; }
    if (k === 'hooks_problem') return `hooks.toml could not be read: ${d.error || ''}`;
    return kindLabel(k);
  };
  const calls = (n) => (n === 1 ? '1 call' : `${n || 0} calls`);
  let actKind = '';
  const extra = {
    // the model slider block GPUI puts above the model service settings
    async models() {
      const n = stopCount(); const saved = jsonPref('stop-titles', []);
      const setCount = (k) => { pref('stop-count', String(Math.min(7, Math.max(2, k)))); const t = jsonPref('stops', []); t.length = Math.min(t.length, stopCount()); pref('stops', JSON.stringify(t)); setChoice(modelChoice.id, modelChoice.mode, false); draw(); };
      const boxes = Array.from({ length: n }, (_, i) => h('input', { type: 'text', placeholder: i < 5 ? STOP_TITLES[i] : `Stop ${i + 1}`, value: saved[i] || '', 'aria-label': `Title of stop ${i + 1}` }));
      const saveTitles = () => { pref('stop-titles', JSON.stringify(boxes.map((b) => b.value.trim()))); setChoice(modelChoice.id, modelChoice.mode, false); say('Saved.'); };
      const rank = () => {
        const names = modelGroups.map((g) => `${g.label} (${g.size ? g.size + 'B' : 'size unknown'})`);
        const titles = Array.from({ length: n }, (_, i) => stopTitle(i));
        const text = `Look up public benchmark results for these local models and rank them from fastest to smartest: ${names.join(', ')}. Then propose which model goes on each of the ${n} stops of my model slider (${titles.join(', ')}), from first to last, with one line on why for each. Do not change any settings or files.`;
        location.hash = '#/chat'; newChat(); $('#chat-input').value = text; $('#composer').requestSubmit();
      };
      return [h('div', { class: 'slider-block' }, h('div', { class: 'sub-sm' }, 'Model slider'), muted('The stops run from the lowest to the highest. Choose how many (2 to 7) and what each is called. The assistant can look up benchmarks and suggest which model goes where; nothing changes until you set it. Set a stop yourself in the model list.'),
        row(btn('Fewer', () => setCount(n - 1)), h('span', {}, `${n} stops`), btn('More', () => setCount(n + 1))),
        h('div', { class: 'stop-titles' }, boxes), row(btn('Save stop titles', saveTitles)), row(btn('Rank my models from benchmarks', rank)))];
    },
    async activity() {
      const [sp, spErr] = await fetchOr('/v1/spend', 'The spending summary');
      const spend = card(sub('Spending'), spErr || [
        muted('Counted from the ledger of model calls, by UTC day and month. Models on this computer cost nothing.'),
        h('div', {}, `Today: ${sp.today?.text || ''} (${calls(sp.today?.calls)})`), h('div', {}, `This month: ${sp.this_month?.text || ''} (${calls(sp.this_month?.calls)})`),
        sp.unpriced_calls ? muted(`${calls(sp.unpriced_calls)} this month had no known price, so their cost is not counted. Add a price in the spend settings.`) : null,
        (sp.providers || []).length ? [muted('This month, by service:'), sp.providers.map((x) => h('div', {}, `${x.provider}: ${x.text} (${calls(x.calls)})`))] : muted('Nothing recorded this month.')]);
      const [v, err] = await fetchOr(`/v1/activity?limit=100${actKind ? '&kind=' + actKind : ''}`, 'The activity log');
      const list = (v?.entries || []).map((e) => h('div', { class: 'set-entry', role: 'listitem' },
        h('div', { class: 'grow' }, h('div', {}, actSummary(e.kind, e.detail)), h('div', { class: 'about' }, `${stamp(e.at)} · ${kindLabel(e.kind)} · ${e.actor || ''}`)),
        e.undoable ? btn('Undo', () => act('POST', `/v1/activity/${e.seq}/undo`, undefined, (r) => (r.result === 'refused' ? `Not undone. ${r.message || ''}` : r.message || 'Undone.'))) : null));
      return [spend, sub('What was done'), muted('Every tool the assistant used, every permission you answered, and every request that left this computer. It is only ever added to. Undo is offered for file changes that are still as the assistant left them.'),
        row(ACT_KINDS.map(([k, l]) => btn(l, () => { actKind = k; draw(); }, actKind === k ? 'seg primary' : 'seg ghost'))),
        err || (list.length ? h('div', { role: 'list', class: 'set-list' }, list) : muted('Nothing here yet.')), row(btn('Refresh', () => draw(), 'seg ghost'))];
    },
    async tools() {
      const [sv, sErr] = await fetchOr('/v1/mcp/servers', 'The server list');
      const servers = (sv?.servers || []).map((x) => card(
        h('div', { class: 'title' }, `${x.name} · ${x.approved ? 'Approved' : 'Waiting for your approval'}`),
        muted(`${x.kind === 'http' ? 'Link:' : 'Runs:'} ${x.runs || ''}`),
        x.error ? h('p', { class: 'about bad' }, `Cannot start: ${x.error}`) : (x.tools || []).length ? h('ul', {}, x.tools.map((t) => h('li', {}, h('b', {}, t.name), t.description ? ` · ${t.description}` : ''))) : muted('It offers no tools.'),
        row(x.approved ? null : btn('Approve and start', () => act('POST', `/v1/mcp/servers/${encodeURIComponent(x.name)}/approve`), 'seg primary'),
          sure('Remove', 'Yes, remove it', () => act('DELETE', `/v1/mcp/servers/${encodeURIComponent(x.name)}`)))));
      let link = false;
      const [nm, nmF] = fieldIn('Name', 'files');
      const [cmd, cmdF] = fieldIn('Command (the program and what follows it)', 'npx -y @modelcontextprotocol/server-filesystem /home/me/docs');
      const [env, envF] = fieldIn('Name of the environment variable that holds the key (leave empty if none)', 'MY_SERVER_KEY');
      envF.hidden = true;
      const kinds = row(btn('A program on this computer', (e) => setKind(false, e)), btn('A link', (e) => setKind(true, e)));
      const setKind = (l, e) => { link = l; for (const b of kinds.children) b.setAttribute('aria-pressed', String(b === e?.target)); cmdF.firstChild.textContent = l ? 'Link' : 'Command (the program and what follows it)'; cmd.placeholder = l ? 'https://mcp.example.org/mcp' : 'npx -y @modelcontextprotocol/server-filesystem /home/me/docs'; envF.hidden = !l; for (const b of kinds.children) b.classList.toggle('primary', b.getAttribute('aria-pressed') === 'true'); preview(); envF.hidden = !l; };
      kinds.firstChild.setAttribute('aria-pressed', 'true'); kinds.firstChild.classList.add('primary');
      // what will run, shown exactly, before it is added
      const willRun = h('p', { class: 'will-run' });
      const preview = () => { const t = cmd.value.trim().replace(/\s+/g, ' '); willRun.textContent = t ? `It will ${link ? 'connect to' : 'run:'} ${t}` : ''; };
      cmd.addEventListener('input', preview);
      const add = () => {
        const name = nm.value.trim();
        if (!/^[A-Za-z0-9_-]+$/.test(name)) return say('Give it a name with letters, digits, - or _.');
        let transport;
        if (link) { const u = cmd.value.trim(); if (!/^https?:\/\//.test(u)) return say('A link starts with http:// or https://.'); transport = { kind: 'http', url: u, bearer_env: env.value.trim() || null }; }
        else { const w = (cmd.value.match(/"[^"]*"|'[^']*'|\S+/g) || []).map((x) => x.replace(/^["']|["']$/g, '')); if (!w.length) return say('Type the command that starts the server.'); transport = { kind: 'stdio', command: w[0], args: w.slice(1) }; }
        act('POST', '/v1/mcp/servers', { name, transport });
      };
      const form = h('div', {});
      const closed = row(btn('Add a server', () => form.replaceChildren(opened)));
      const opened = card(sub('Add an MCP server'), kinds, nmF, cmdF, envF, willRun, row(btn('Add (it will not run yet)', add, 'seg primary'), btn('Cancel', () => form.replaceChildren(closed), 'seg ghost')));
      form.append(closed);
      const [tv, tErr] = await fetchOr('/v1/trust', 'The folder list');
      const folders = (tv?.folders || []).map((f) => card(h('div', { class: 'title' }, `${f.folder} · ${f.decision}`), f.exists === false ? muted('This folder is not there any more.') : null,
        (f.mcp || []).map((m) => muted(`It lists ${m.name}: ${m.runs}`)), row(sure('Revoke', 'Yes, forget this folder', () => act('POST', '/v1/trust', { folder: f.folder, decision: 'forget' })))));
      return [sub('MCP servers'), muted('An MCP server gives the assistant more tools. Nothing runs until you approve it, and you see the exact command or link first.'),
        sErr || (servers.length ? servers : muted('No MCP servers yet.')),
        form,
        sub('Folders that bring their own tools'), muted('A project folder can list servers of its own. They load only after you trust that folder. When you open a chat in such a folder you are asked; your answers are listed here.'),
        tErr || (folders.length ? folders : muted('No folder has a decision yet.')), row(btn('Refresh', () => draw(), 'seg ghost'))];
    },
    async schedules() {
      const [v, err] = await fetchOr('/v1/schedules', 'The task list');
      const tasks = (v?.schedules || []).map((t) => card(h('div', { class: 'title' }, t.name || t.prompt), muted(`${t.when} · ${t.model}${t.enabled === false ? ' · off' : ''}`), t.last_result ? muted(`Last time: ${t.last_result}`) : null,
        row(btn(t.enabled === false ? 'Turn on' : 'Turn off', () => act('POST', `/v1/schedules/${t.id}/enabled`, { enabled: t.enabled === false })), sure('Delete', 'Delete this task', () => act('DELETE', `/v1/schedules/${t.id}`)))));
      const [fv] = await fetchOr('/v1/activity?limit=5&kind=schedule_changed_files', 'Files');
      const files = (fv?.entries || []).map((e) => muted(`${stamp(e.at)} · ${actSummary(e.kind, e.detail)}`));
      const [w, wF] = fieldIn('When: every 30m, every 2h, every 1d or daily 08:00', 'every 2h');
      const [t, tF] = fieldIn('What to do', 'Summarise new files in my notes folder');
      const [m, mF] = fieldIn('Model', 'Qwen3.6-35B-A3B-balanced');
      const [f, fF] = fieldIn('Folder (empty for your home folder)', '');
      const add = () => { if (!w.value.trim()) return say('Write when: every 30m, every 2h, every 1d or daily 08:00.'); if (!t.value.trim()) return say('Write what the task should do.'); if (!m.value.trim()) return say('Choose a model for the task.'); act('POST', '/v1/schedules', { name: '', prompt: t.value.trim(), folder: f.value.trim() || null, model: m.value.trim(), when: w.value.trim() }); };
      return [muted('Tasks that start a chat on their own at set times. Nobody is there to answer, so every permission question is answered no.'),
        err || (tasks.length ? tasks : muted('No tasks yet.')), files.length ? [sub('Files changed by tasks'), files] : null,
        sub('Add a task'), wF, tF, mF, fF, row(btn('Add the task', add, 'seg primary'), btn('Refresh', () => draw(), 'seg ghost'))];
    },
    async learning() {
      const [pv] = await fetchOr('/v1/learn/proposals', 'Suggestions');
      const props = (pv?.proposals || []).filter((x) => x.status === 'pending' || !x.status).map((x) => card(h('div', { class: 'title' }, `It would say: ${x.payload?.text || x.payload?.name || x.kind}`),
        (x.evidence || []).length ? muted(`Because you said: "${[].concat(x.evidence).join('", "')}"`) : null,
        row(btn('Accept', () => act('POST', `/v1/learn/proposals/${x.id}/accept`, {}, () => 'Done. It is part of what the assistant knows now.'), 'seg primary'), btn('Reject', () => act('POST', `/v1/learn/proposals/${x.id}/reject`, {}, () => 'Dropped.')))));
      const [rv, rErr] = await fetchOr('/v1/learn/rules?all=true', 'The rules');
      const [nr, nrF] = fieldIn('New rule', 'Write a rule, like: Always give the source for a fact');
      const addRule = () => { if (!nr.value.trim()) return say('Write the rule first.'); act('POST', '/v1/learn/rules', { text: nr.value.trim() }, () => 'Rule added.'); };
      const rules = (rv?.rules || []).map((r) => card(h('div', { class: 'title' }, r.text || r.id), muted([r.topic, r.privacy === 'sensitive' ? 'sensitive, stays on this computer' : '', r.active ? '' : 'off'].filter(Boolean).join(' · ')),
        row(btn(r.active ? 'Turn off' : 'Turn on', () => act('POST', `/v1/learn/rules/${r.id}/status`, { active: !r.active })),
          r.version > 1 ? btn('Undo last change', () => act('POST', `/v1/learn/rules/${r.id}/rollback`, { version: r.version - 1 }, () => `Went back to version ${r.version - 1}.`)) : null,
          sure('Delete', 'Delete this rule and its history for good?', () => act('DELETE', `/v1/learn/rules/${r.id}`, undefined, () => 'Deleted for good.')))));
      const [sv] = await fetchOr('/v1/learn/style', 'How I talk to you');
      const st = h('textarea', { rows: 3, placeholder: 'For example: Short answers. Plain words. No filler.', 'aria-label': 'Style' }); const stF = h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Style'), st);
      st.value = sv?.style?.text || '';
      const [kv2] = await fetchOr('/v1/learn/skills', 'Skills');
      const skills = (kv2?.skills || []).map((k) => card(h('div', { class: 'title' }, k.name), muted(k.error ? `This skill is broken: ${k.error}` : (k.description || '')),
        row(btn('Undo last change', async () => { const v = (await json(`/v1/learn/skills/${encodeURIComponent(k.name)}`)).history || []; const prev = v.map((x) => x.version).sort((a, b) => b - a)[1]; if (prev) await act('POST', `/v1/learn/skills/${encodeURIComponent(k.name)}/rollback`, { version: prev }); else say('There is no earlier version.'); }),
          sure('Delete', 'Delete this skill and its history for good?', () => act('DELETE', `/v1/learn/skills/${encodeURIComponent(k.name)}`)))));
      return [props.length ? [sub('Waiting for you'), props] : null,
        sub('Rules'), muted('Things the assistant always keeps to. Switch one off to stop it without losing it.'), h('div', { class: 'set-inline' }, nr, btn('Add rule', addRule, 'seg primary')), rErr || (rules.length ? rules : muted('No rules yet.')),
        sub('How I talk to you'), muted('A short note on tone and length. It goes with every chat, so keep it short.'), stF, row(btn('Save', () => act('PUT', '/v1/learn/style', { text: st.value }, () => 'Saved. The old text stays in the history.'), 'seg primary'),
          sv?.style?.version > 1 ? btn('Undo last change', () => act('POST', '/v1/learn/style/rollback', { version: sv.style.version - 1 }, () => `Went back to the text of version ${sv.style.version - 1}.`)) : null),
        sub('Skills'), muted('Ready-made know-how the assistant reads when a task needs it.'), skills.length ? skills : muted('No skills yet.')];
    },
    // Departments as the desktop app draws it: team settings, away mode, and forms for departments, roles and model types
    async departments() {
      const enc = encodeURIComponent, T = teamUi;
      const [sv, sErr] = await fetchOr('/v1/team/settings', 'Team settings');
      const [av, aErr] = await fetchOr('/v1/team/away', 'Away mode');
      const [dv, dErr] = await fetchOr('/v1/team/departments', 'The departments');
      const [rv, rErr] = await fetchOr('/v1/team/roles', 'The roles');
      const [tv, tErr] = await fetchOr('/v1/team/types', 'The model types');
      let mv = null; try { mv = await json('/v1/models'); } catch { /* no model list: the pickers stay empty */ }
      const models = (mv?.details || (mv?.models || []).map((id) => ({ id }))).map((d) => d.id);
      const types = tv?.types || [], roleList = rv?.roles || [], deptList = dv?.departments || [];
      const roleNames = roleList.filter((r) => !r.error).map((r) => r.name);
      const head = (title, label, fn) => h('div', { class: 'set-row' }, h('div', { class: 'grow' }, sub(title)), btn(label, fn, 'seg ghost'));
      const dd = (label, shown, opts, value, fn) => { const s = h('select', { 'aria-label': label }); if (!opts.some(([, v]) => v === value)) s.append(h('option', { value, disabled: true }, shown)); for (const [l, v] of opts) s.append(h('option', { value: v }, l)); s.value = value; s.onchange = () => fn(s.value); return s; };
      const bound = (label, obj, key, ph = '') => { const [i, F] = fieldIn(label, ph); i.value = obj[key] ?? ''; i.oninput = () => { obj[key] = i.value; }; return F; };
      const area = (label, obj, key, ph, rows, onIn) => { const t = h('textarea', { rows, placeholder: ph, 'aria-label': label }); t.value = obj[key] ?? ''; t.oninput = () => { obj[key] = t.value; onIn?.(); }; return h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, label), t); };
      const chips = (all, on, fn) => h('div', { class: 'set-row wrap' }, all.map((m) => btn(on.includes(m) ? `✓ ${m}` : m, () => { fn(m); draw(); }, on.includes(m) ? 'seg primary' : 'seg')));
      const toggleIn = (list, m) => { const p = list.indexOf(m); if (p >= 0) list.splice(p, 1); else list.push(m); };
      const note = (k) => (T.notes[k] ? muted(T.notes[k]) : null);
      const history = (kind, name, open, set) => {
        if (!open) return null;
        const slot = h('div', { class: 'set-card', role: 'list' }, muted('Loading the history…'));
        json(`/v1/team/${kind}/${enc(name)}/history`).then((v) => {
          const list = (v.versions || []).slice().sort((a, b) => b.version - a.version);
          slot.replaceChildren(...list.map((x, n) => row(h('div', { class: 'grow' }, `Version ${x.version}, ${x.op}${x.note ? ': ' + x.note : ''}${n === 0 ? ' (now)' : ''}`),
            n ? btn('Go back to this', () => act('POST', `/v1/team/${kind}/${enc(name)}/rollback`, { version: x.version }, () => `Went back to version ${x.version}.`), 'seg ghost') : null)));
        }).catch((e) => slot.replaceChildren(muted(e.message)));
        return slot;
      };
      // team settings
      let settings = sErr;
      if (sv) {
        const save = (field, v) => act('PUT', '/v1/team/settings', { [field]: v }, () => 'Saved.');
        const lim = sv.limits || {};
        const step = (field, value, max) => h('div', { class: 'set-row' }, btn('−', () => save(field, Math.max(1, value - 1)), 'seg ghost'), h('div', { class: 'step-num' }, String(value)), btn('+', () => save(field, Math.min(max, value + 1)), 'seg ghost'));
        const line = (label, c) => h('div', { class: 'set-row wrap' }, h('div', { class: 'grow' }, label), c);
        const words = (tbl, k) => (tbl.find((x) => x[0] === k) || [k, k])[1];
        settings = h('div', { class: 'set-card', role: 'group', 'aria-label': 'Team settings' },
          line('Workers at the same time', step('parallel', sv.parallel ?? 1, lim.parallel_max ?? 16)),
          line('Tries for each step', step('attempts', sv.attempts ?? 3, lim.attempts_max ?? 10)),
          line('How much departments ask you', dd('How much departments ask you', words(AUTONOMY, sv.autonomy), AUTONOMY.map(([k, w]) => [w, k]), sv.autonomy || '', (v) => save('autonomy', v))),
          line('When a model is missing', dd('When a model is missing', words(UNAVAILABLE, sv.unavailable), UNAVAILABLE.map(([k, w]) => [w, k]), sv.unavailable || '', (v) => save('unavailable', v))));
      }
      // away mode
      let away = aErr;
      if (av) {
        const sw = h('input', { type: 'checkbox', class: 'switch', 'aria-label': 'Away mode' }); sw.checked = !!av.away;
        sw.onchange = () => act('POST', '/v1/team/away', { on: sw.checked }, () => (sw.checked ? 'Away mode is on.' : 'Away mode is off.'));
        away = h('div', { class: 'set-card', role: 'group', 'aria-label': 'Away mode' }, h('label', { class: 'set-row' }, sw, h('span', {}, 'Away mode')),
          muted('While you are away, departments keep working but say no to anything that needs permission, and set those steps aside for you.'),
          (av.decisions || []).length ? [muted('Decided without you:'), ...av.decisions.map((d) => h('div', { class: 'small' }, `${d.dept || ''}: ${d.decision || ''}`))] : null,
          (av.parked || []).filter((p) => typeof p === 'string').length ? [muted('Set aside for you:'), ...av.parked.filter((p) => typeof p === 'string').map((p) => h('div', { class: 'small' }, p))] : null);
      }
      // departments
      const deptForm = (d) => ({ name: d.name || '', description: d.description || '', lead: d.lead || '', members: [...(d.members || [])], parallel: d.parallel != null ? String(d.parallel) : '', attempts: d.attempts != null ? String(d.attempts) : '', instructions: d.instructions || '' });
      const deptCard = (d) => {
        const c = h('div', { class: 'set-card', role: 'group', 'aria-label': d.name }, h('div', {}, d.name));
        const del = sure('Delete', 'Delete this department?', () => act('DELETE', `/v1/team/departments/${enc(d.name)}`, undefined, () => 'Deleted.'));
        if (d.error) { c.append(h('p', { class: 'about danger' }, `This department file has a problem: ${d.error}`), row(del)); return c; }
        if (d.description) c.append(muted(d.description));
        c.append(muted(`Lead: ${d.lead || ''}.   Members: ${(d.members || []).join(', ')}.`));
        const open = T.deptHist === d.name;
        c.append(row(btn('Edit', () => { T.dept = [false, deptForm(d)]; delete T.notes.deptform; draw(); }),
          btn(open ? 'Hide history' : 'History', () => { T.deptHist = open ? null : d.name; draw(); }, 'seg ghost'),
          btn('Open page', () => { deptPick = d.name; close(); location.hash = '#/team'; }, 'seg ghost'), del), history('departments', d.name, open));
        return c;
      };
      const deptEditor = () => {
        const [isNew, f] = T.dept;
        const others = deptList.map((d) => d.name).filter((n) => n && n !== f.name).map((n) => `dept:${n}`);
        const save = () => {
          const name = f.name.trim(), ok = (v, max) => v.trim() === '' || (/^\d+$/.test(v.trim()) && +v >= 1 && +v <= max);
          const e = !validName(name) ? 'The name can use lower case letters, digits and single hyphens, like web.' : !f.lead.trim() ? 'Pick a lead. The lead plans the work and checks it.' : !f.members.length ? 'Pick at least one member.'
            : !ok(f.parallel, 16) ? 'Workers at once must be a whole number from 1 to 16, or empty.' : !ok(f.attempts, 10) ? 'Attempts per step must be a whole number from 1 to 10, or empty.' : '';
          if (e) { T.notes.deptform = e; return draw(); }
          let t = `---\nname: ${name}\n`;
          if (f.description.trim()) t += `description: ${quote(f.description.trim())}\n`;
          t += `lead: ${f.lead}\nmembers: ${f.members.join(' ')}\n`;
          if (f.parallel.trim()) t += `parallel: ${+f.parallel}\n`;
          if (f.attempts.trim()) t += `attempts: ${+f.attempts}\n`;
          T.dept = null; act('PUT', `/v1/team/departments/${enc(name)}`, { text: `${t}---\n\n${f.instructions.trim()}\n` }, () => 'Saved.');
        };
        return h('div', { class: 'set-card editing', role: 'group', 'aria-label': isNew ? 'New department' : 'Edit department' }, h('div', {}, isNew ? 'New department' : `Edit ${f.name}`),
          isNew ? bound('Name', f, 'name', 'web (lower case letters, digits, hyphens)') : null, bound('What it is for', f, 'description', 'One line'),
          h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Lead'), dd('Lead', f.lead || 'Pick a lead', roleNames.map((r) => [r, r]), f.lead, (v) => { f.lead = v; })),
          muted('The lead plans the work, hands out steps and checks the results.'),
          h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Members'), chips([...roleNames, ...others], f.members, (m) => toggleIn(f.members, m))),
          bound('Workers at once for this department (empty: use the team setting)', f, 'parallel'), bound('Tries per step for this department (empty: use the team setting)', f, 'attempts'),
          area('Instructions for the whole department', f, 'instructions', 'Optional', 3),
          row(btn('Save', save, 'seg primary'), btn('Cancel', () => { T.dept = null; draw(); }, 'seg ghost')), note('deptform'));
      };
      // roles
      const roleForm = (r) => { const s = r.sampling || {}, n = (v) => (v == null ? '' : String(v)); return { name: r.name || '', description: r.description || '', kind: r.target?.kind || 'type', target: r.target?.value || '', tools: [...(r.tools || [])], memory: r.memory || '', temperature: n(s.temperature), top_p: n(s.top_p), top_k: n(s.top_k), min_p: n(s.min_p), repeat_penalty: n(s.repeat_penalty), max_steps: r.max_steps != null ? String(r.max_steps) : '', thinking: r.thinking === true ? 'on' : r.thinking === false ? 'off' : '', instructions: r.instructions || '' }; };
      const roleCard = (r) => {
        const c = h('div', { class: 'set-card', role: 'group', 'aria-label': r.name }, h('div', {}, r.name));
        if (r.error) c.append(h('p', { class: 'about danger' }, `This role file has a problem: ${r.error}`));
        else { const f = roleForm(r); c.append(muted(f.description), muted(`Runs on ${f.kind === 'model' ? `the model ${f.target}` : `the ${f.target} model type`}. Tools: ${f.tools.length ? f.tools.join(', ') : 'none'}.`)); }
        const open = T.roleHist === r.name;
        c.append(row(r.error ? null : btn('Edit', () => { T.role = [false, roleForm(r)]; delete T.notes.roleform; draw(); }),
          btn(open ? 'Hide history' : 'History', () => { T.roleHist = open ? null : r.name; draw(); }, 'seg ghost'),
          sure('Delete', 'Delete this role?', () => act('DELETE', `/v1/team/roles/${enc(r.name)}`, undefined, () => 'Deleted.'))), history('roles', r.name, open));
        return c;
      };
      const roleEditor = () => {
        const [isNew, f] = T.role;
        const count = h('div', { class: 'small' });
        const recount = () => { const n = [...f.instructions.trim()].length; count.textContent = `${n} of ${INSTRUCTIONS_MAX} characters`; count.classList.toggle('danger', n > INSTRUCTIONS_MAX); };
        recount();
        const kindBtn = (label, k) => btn(label, () => { f.kind = k; draw(); }, (k === 'model') === (f.kind === 'model') ? 'seg primary' : 'seg');
        let target;
        if (f.kind === 'model') {
          const [typed, typedF] = fieldIn('or type its name', ''); typed.value = f.target; typed.oninput = () => { f.target = typed.value; };
          target = [dd('Model', f.target || 'Pick a model', models.map((m) => [m, m]), f.target, (v) => { f.target = v; typed.value = v; }), typedF];
        } else target = dd('Model type', f.target || 'Pick a type', types.map((t) => [t.name, t.name]), f.target, (v) => { f.target = v; });
        const samp = h('div', { class: 'set-row wrap' }, [['temperature', 'Temperature'], ['top_p', 'Top P'], ['top_k', 'Top K'], ['min_p', 'Min P'], ['repeat_penalty', 'Repeat penalty']].map(([k, l]) => h('div', { class: 'samp' }, bound(l, f, k))));
        const save = () => {
          const name = f.name.trim(), ins = f.instructions.trim(), num = (v) => v.trim() === '' || !isNaN(parseFloat(v)) && isFinite(v.trim());
          const steps = f.max_steps.trim();
          const e = !validName(name) ? 'The name can use lower case letters, digits and single hyphens, like web-designer.' : !f.description.trim() ? 'Add a one-line description. The lead reads it when it decides who gets a step.'
            : !f.target.trim() ? (f.kind === 'model' ? 'Say which model this role uses.' : 'Pick a model type for this role.') : !ins ? 'Write the instructions for this role.'
            : [...ins].length > INSTRUCTIONS_MAX ? `The instructions are ${[...ins].length} characters; the most is ${INSTRUCTIONS_MAX}. They go with every step the role does, so keep them to a short role line.`
            : [['temperature', 'Temperature'], ['top_p', 'Top P'], ['min_p', 'Min P'], ['repeat_penalty', 'Repeat penalty']].map(([k, l]) => (num(f[k]) ? '' : `${l} must be a number, like 0.7, or empty.`)).find(Boolean)
            || (!/^\d+$/.test(steps) || +steps < 1 || +steps > 100 ? 'Steps per task must be a whole number from 1 to 100.' : f.top_k.trim() && !/^\d+$/.test(f.top_k.trim()) ? 'Top K must be a whole number, or empty.' : '');
          if (e) { T.notes.roleform = e; return draw(); }
          let t = `---\nname: ${name}\ndescription: ${quote(f.description.trim())}\n`;
          t += f.kind === 'model' ? `model: ${quote(f.target.trim())}\n` : `type: ${f.target.trim().toLowerCase()}\n`;
          if (f.tools.length) t += `tools: ${f.tools.join(' ')}\n`;
          t += `memory: ${f.memory.trim() || 'none'}\n`;
          for (const k of ['temperature', 'top_p', 'top_k', 'min_p', 'repeat_penalty']) if (f[k].trim()) t += `${k}: ${k === 'top_k' ? parseInt(f[k], 10) : parseFloat(f[k])}\n`;
          t += `max_steps: ${+steps}\n`;
          if (f.thinking === 'on' || f.thinking === 'off') t += `thinking: ${f.thinking}\n`;
          T.role = null; act('PUT', `/v1/team/roles/${enc(name)}`, { text: `${t}---\n\n${ins}\n` }, () => 'Saved.');
        };
        return h('div', { class: 'set-card editing', role: 'group', 'aria-label': isNew ? 'New role' : 'Edit role' }, h('div', {}, isNew ? 'New role' : `Edit ${f.name}`),
          isNew ? bound('Name', f, 'name', 'coder (lower case letters, digits, hyphens)') : null, bound('What it does, in one line', f, 'description', 'The lead reads this when it chooses who gets a step'),
          h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Model'), row(kindBtn('A model type', 'type'), kindBtn('One exact model', 'model')), target),
          h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Tools it may use'), chips(KNOWN_TOOLS, f.tools, (t) => toggleIn(f.tools, t))),
          bound('Memory it may see (none, all, or preferences facts projects)', f, 'memory', 'none'),
          h('div', { class: 'small' }, 'Sampling (leave empty to let the model decide)'), samp,
          bound('Most model calls in one step (1 to 100)', f, 'max_steps', '12'),
          h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Thinking'), dd('Thinking', '', [['Let the model decide', ''], ['Answers without thinking', 'off'], ['Thinks first', 'on']], f.thinking, (v) => { f.thinking = v; })),
          area('Instructions', f, 'instructions', 'Who the role is and how it works. Keep it short: it goes with every step.', 4, recount), count,
          row(btn('Save', save, 'seg primary'), btn('Cancel', () => { T.role = null; draw(); }, 'seg ghost')), note('roleform'));
      };
      // model types
      const typeCard = (t) => h('div', { class: 'set-card', role: 'group', 'aria-label': t.name }, h('div', { class: 'set-row' }, h('div', {}, t.name), t.stock ? muted('built in') : null),
        muted(t.model ? `Uses ${t.model}${(t.fallbacks || []).length ? ', then ' + t.fallbacks.join(', ') : ''}` : 'No model chosen yet.'),
        row(btn('Edit', () => { T.typeEdit = { name: t.name, model: t.model || '', fallbacks: (t.fallbacks || []).join(', ') }; delete T.notes.typeform; draw(); }), btn('Delete', () => startDelete(t.name), 'seg ghost')));
      const startDelete = (name) => {
        const affected = roleList.filter((r) => !r.error && r.target?.kind === 'type' && r.target?.value === name).map((r) => r.name);
        const other = types.find((t) => t.name !== name);
        T.typeDel = { name, affected, homes: Object.fromEntries(other ? affected.map((r) => [r, `type:${other.name}`]) : []) }; delete T.notes.types; draw();
      };
      const typeEditor = () => {
        const f = T.typeEdit, isNew = !f.name;
        const save = async () => {
          const model = f.model.trim() || null, fallbacks = f.fallbacks.split(',').map((s) => s.trim()).filter(Boolean);
          if (isNew) {
            const name = (f.newName || '').trim().toLowerCase();
            if (!validName(name)) { T.notes.typeform = 'The name can use lower case letters, digits and single hyphens, like cheap.'; return draw(); }
            T.typeEdit = null;
            try { await json('/v1/team/types', { method: 'POST', body: { name, model } }); if (fallbacks.length) await json(`/v1/team/types/${enc(name)}`, { method: 'PUT', body: { fallbacks } }); say('Added.'); } catch (e) { say(e.message); }
            return draw();
          }
          T.typeEdit = null; act('PUT', `/v1/team/types/${enc(f.name)}`, { model, fallbacks }, () => 'Saved.');
        };
        return h('div', { class: 'set-card editing', role: 'group', 'aria-label': 'Model type' }, h('div', {}, isNew ? 'New model type' : `Edit ${f.name}`),
          isNew ? bound('Name', f, 'newName', 'cheap (lower case letters, digits, hyphens)') : null,
          h('div', { class: 'set-field' }, h('div', { class: 'lbl' }, 'Model'), dd('Model', f.model, [['No model yet', ''], ...models.map((m) => [m, m])], f.model, (v) => { f.model = v; })),
          bound('Models to try, in order, when the first is missing (separate with commas)', f, 'fallbacks'),
          row(btn('Save', save, 'seg primary'), btn('Cancel', () => { T.typeEdit = null; draw(); }, 'seg ghost')), note('typeform'));
      };
      const typeDelete = () => {
        const d = T.typeDel;
        const opts = [...types.filter((t) => t.name !== d.name).map((t) => [`Type ${t.name}`, `type:${t.name}`]), ...models.map((m) => [`Model ${m}`, `model:${m}`])];
        const go = () => {
          const reassign = {};
          for (const r of d.affected) {
            const [k, v] = (d.homes[r] || '').split(/:(.*)/s);
            if (!v || !v.trim()) { T.notes.types = `Choose where ${r} goes first.`; return draw(); }
            reassign[r] = k === 'type' ? { type: v } : { model: v.trim() };
          }
          T.typeDel = null; act('DELETE', `/v1/team/types/${enc(d.name)}`, { reassign }, () => 'Deleted.');
        };
        return h('div', { class: 'set-card deleting', role: 'group', 'aria-label': 'Delete a model type' }, h('div', {}, `Delete the type ${d.name}?`),
          muted(d.affected.length ? 'These roles use it. Choose where each one goes before it is deleted.' : 'No role uses it.'),
          d.affected.map((r) => h('div', { class: 'set-row wrap' }, h('div', { class: 'home-name' }, r), dd(`Where ${r} goes`, 'Choose…', opts, d.homes[r] || '', (v) => { d.homes[r] = v; }))),
          row(btn('Delete the type', go, 'seg danger-fill'), btn('Cancel', () => { T.typeDel = null; draw(); }, 'seg ghost')));
      };
      return [
        sub('Team settings'), settings, away,
        head('Departments', 'New department', () => { T.dept = [true, deptForm({})]; delete T.notes.deptform; draw(); }), dErr || (deptList.length ? deptList.map(deptCard) : muted('No departments yet')), T.dept ? deptEditor() : null,
        head('Roles', 'New role', () => { T.role = [true, { ...roleForm({}), kind: 'type', target: 'mid-tier', memory: 'none', max_steps: '12' }]; delete T.notes.roleform; draw(); }),
        muted('A role is a job a worker can do: its instructions, the tools it may use and the kind of model it runs on.'), rErr || roleList.map(roleCard), T.role ? roleEditor() : null,
        head('Model types', 'New type', () => { T.typeEdit = { name: '', newName: '', model: '', fallbacks: '' }; delete T.notes.typeform; draw(); }),
        muted('A model type is a kind of job for a model, like frontier or mid-tier. Roles name a type, and the type names the model, so one change here moves every role that uses it.'),
        tErr || types.map(typeCard), T.typeEdit ? typeEditor() : null, T.typeDel ? typeDelete() : null, note('types')];
    },
    // Channels as the desktop app has them: each channel off until its warning is read, then a form to add or edit one
    async channels() {
      const out = [muted("Messages on a channel pass through that service's servers. Every channel is off until you have read its warning and switched it on, and only the people you list are answered.")];
      const [v, err] = await fetchOr('/v1/channels', 'The channel list');
      if (err) return [...out, err];
      const kinds = (v.kinds || []).map((k) => ({ kind: k.kind, warning: k.warning || '', fields: [...(k.required || []).map((f) => ({ ...f, required: true })), ...(k.optional || []).map((f) => ({ ...f, required: false }))] }));
      const list = v.channels || [];
      const formBox = h('div', {});
      const closed = row(btn('Add a channel', () => openForm(null)));
      const openForm = (edit) => {
        let kind = edit ? edit.kind : kinds[0]?.kind; let assistant = edit?.mode === 'assistant';
        const draw1 = () => {
          const k = kinds.find((x) => x.kind === kind);
          if (!k) return formBox.replaceChildren(muted('The core did not list any kinds of channel.'));
          const [nm, nmF] = fieldIn('Name', 'family-chat'); const [al, alF] = fieldIn('Who may talk to it (ids, separated by commas)', '123456789, 987654321'); const [md, mdF] = fieldIn('Model that answers', 'a model name from your list');
          if (edit) { nm.value = edit.name; nm.disabled = true; al.value = (edit.allow || []).join(', '); md.value = edit.model || ''; }
          const fs = k.fields.map((f) => { const [i, F] = fieldIn(`${f.name}${f.required ? '' : ' (optional)'}: ${f.help || ''}`, f.name.includes('token') ? 'env:NAME' : ''); i.value = edit?.settings?.[f.name] || ''; return [f, i, F]; });
          const pick = (label, on, fn) => btn(label, fn, on ? 'seg primary' : 'seg');
          const save = () => {
            const name = nm.value.trim();
            if (!/^[A-Za-z0-9_-]+$/.test(name)) return say('Give the channel a name with letters, digits, - or _.');
            const settings = {};
            for (const [f, i] of fs) {
              const val = i.value.trim();
              if (f.required && !val) return say(`${f.name} needs a value.`);
              if (f.name.includes('token') && val && !/^(env|secret):/.test(val)) return say(`${f.name} is written as env:NAME or secret:NAME, naming where the key is kept. Never the key itself.`);
              if (val) settings[f.name] = val;
            }
            const body = { allow: al.value.split(',').map((x) => x.trim()).filter(Boolean), model: md.value.trim(), mode: assistant ? 'assistant' : 'chat', settings };
            if (edit) act('PUT', `/v1/channels/${encodeURIComponent(edit.name)}`, body); else act('POST', '/v1/channels', { name, kind, ...body });
          };
          formBox.replaceChildren(card(sub(edit ? `Edit ${edit.name}` : 'Add a channel'),
            edit ? null : row(...kinds.map((x) => pick(x.kind, x.kind === kind, () => { kind = x.kind; draw1(); }))),
            nmF, alF, mdF, row(pick('Chat only', !assistant, () => { assistant = false; draw1(); }), pick('With tools, asking in the chat', assistant, () => { assistant = true; draw1(); })),
            ...fs.map((x) => x[2]), muted('Keys are never typed here. Write where the key is kept, as env:NAME (an environment variable) or secret:NAME.'),
            row(btn(edit ? 'Save' : 'Add (it starts off)', save, 'seg primary'), btn('Cancel', () => formBox.replaceChildren(closed), 'seg ghost'))));
        };
        draw1();
      };
      formBox.append(closed);
      if (!list.length) out.push(muted('No channels yet.'));
      for (const c of list) {
        const st = c.status || {};
        const state = !c.enabled ? 'Off' : st.running ? `On and running. Answered ${st.handled || 0}, ignored ${st.ignored || 0}.` : `On, but not running${st.last_error ? `: ${st.last_error}` : ''}`;
        const warn = h('div', { class: 'ch-warn', hidden: true }, (kinds.find((k) => k.kind === c.kind) || {}).warning || '');
        const n = encodeURIComponent(c.name);
        const buttons = row();
        const onBtn = btn('Switch on…', () => {
          if (!(c.allow || []).length) return say('List at least one person who may talk to it.');
          if (!(c.model || '').trim()) return say('Choose the model that answers.');
          warn.hidden = false; onBtn.replaceWith(btn('I have read this. Switch it on', () => act('POST', `/v1/channels/${n}/enable`, { acknowledge: true }), 'seg danger'), btn('Leave it off', () => draw(), 'seg ghost'));
        });
        buttons.append(c.enabled ? btn('Switch off', () => act('POST', `/v1/channels/${n}/disable`)) : onBtn, btn('Edit', () => openForm(c), 'seg ghost'), sure('Remove', 'Yes, remove it', () => act('DELETE', `/v1/channels/${n}`)));
        out.push(h('div', { class: 'set-card', role: 'group', 'aria-label': `Channel ${c.name}` },
          h('div', { class: 'set-row' }, h('div', { class: 'grow' }, `${c.name}  (${c.kind})`), h('span', { class: c.enabled ? 'ch-on' : 'ch-off' }, c.enabled ? 'On' : 'Off')),
          muted(state), muted(`Answers: ${c.mode === 'assistant' ? 'with tools, asking in the chat' : 'chat only'}. Model: ${c.model || 'none chosen'}. May talk to it: ${(c.allow || []).length ? c.allow.join(', ') : 'nobody yet'}.`),
          warn, buttons));
      }
      return [...out, formBox, row(btn('Refresh', () => draw(), 'seg ghost'))];
    },
    // Your data, as the desktop app has it: save a copy to a folder, and look inside a saved copy before bringing it in
    async data() {
      const [ex, exF] = fieldIn('Folder for the copy', '/path/to/a/new/folder'); exF.firstChild.remove();
      const save = async () => {
        if (!ex.value.trim()) return say('Choose a folder for the copy first.');
        try {
          const v = await json('/v1/export', { method: 'POST', body: { dir: ex.value.trim() } });
          const counts = Object.entries(v.counts || {}).map(([k, n]) => `${k}: ${n}`).join(', ');
          say(`Saved to ${v.dir || ''}.${counts ? ` ${counts}.` : ''}${v.warning ? ` ${v.warning}` : ''}`);
        } catch (e) { say(e.message); }
      };
      const [im, imF] = fieldIn('Folder that holds a saved copy', '/path/to/a/saved/copy'); imF.firstChild.remove();
      const found = h('div', {});
      const replace = new Set(); let replaceStyle = false;
      const look = async () => {
        if (!im.value.trim()) return say('Choose the folder that holds a saved copy first.');
        let p; try { p = await json('/v1/import', { method: 'POST', body: { dir: im.value.trim(), apply: false, overwrite: [], replace_style: false } }); } catch (e) { return say(e.message); }
        replace.clear(); replaceStyle = false;
        const named = (a) => (a || []).length ? a.map((i) => (i.exists ? `${i.name} (you have one)` : i.name)).join(', ') : 'none';
        const lines = [`Rules: ${p.rules?.in_folder || 0} in the folder, ${p.rules?.new || 0} of them new to you.`, `Writing style profile: ${p.style === true ? 'yes' : 'no'}.`, `Memory: ${p.memory_files === true ? 'yes' : 'no'}.`, `Roles: ${named(p.roles)}.`, `Departments: ${named(p.departments)}.`, ...(p.problems || []).map((x) => `Cannot use ${x}.`)];
        const clashes = [...(p.roles || []), ...(p.departments || [])].filter((n) => n.exists).map((n) => n.name);
        const has = (p.rules?.in_folder || 0) > 0 || p.style === true || p.memory_files === true || (p.roles || []).length || (p.departments || []).length;
        const pick = (name) => { const b = btn(`Keep mine: ${name}`, () => { if (replace.has(name)) replace.delete(name); else replace.add(name); const on = replace.has(name); b.textContent = `${on ? 'Replace' : 'Keep mine:'} ${name}`; b.classList.toggle('primary', on); }); return b; };
        const styleBtn = btn('Keep my writing style', () => { replaceStyle = !replaceStyle; styleBtn.textContent = replaceStyle ? 'Replace my writing style' : 'Keep my writing style'; styleBtn.classList.toggle('primary', replaceStyle); });
        const apply = async () => {
          try {
            const r = await json('/v1/import', { method: 'POST', body: { dir: im.value.trim(), apply: true, overwrite: [...replace].sort(), replace_style: replaceStyle } });
            say(Object.entries(r.done || {}).map(([k, v]) => `${k}: ${Array.isArray(v) ? v.join('; ') : JSON.stringify(v)}`).join('. ') || 'Done.');
          } catch (e) { say(e.message); }
        };
        found.replaceChildren(h('div', { class: 'import-sum' }, h('div', {}, 'In that folder:'), ...lines.map((l) => h('div', {}, l))),
          clashes.length || p.style === true ? [muted('You already have some of these. Pick any you want the copy to replace:'), row(...clashes.map(pick), p.style === true ? styleBtn : null)] : null,
          has ? row(btn('Bring it in', apply, 'seg primary')) : null);
      };
      return [card(sub('Save a copy of your data'), muted('Writes your memory, rules, writing style, roles and departments to a new folder. The folder must be new or empty. The copy is not encrypted, so keep it somewhere private.'), exF, row(btn('Save a copy', save, 'seg primary'))),
        card(sub('Bring in a saved copy'), muted('First you see what is in the folder. Nothing changes until you press Bring it in. Your own roles, departments and writing style stay unless you choose to replace them.'), imF, row(btn('Look inside', look)), found)];
    },
    async hooks() {
      const [v, err] = await fetchOr('/v1/hooks', 'hooks.toml');
      const intro = muted(`Commands run before or after a tool call, kept in ${v?.file || 'hooks.toml'}. A command before a tool that fails stops the tool. New chats use the changes.`);
      if (err) return [intro, err];
      if (v.error) return [intro, muted(`hooks.toml: ${v.error}`)];
      const out = [intro];
      for (const when of ['before', 'after']) {
        out.push(sub(when === 'after' ? 'After a tool' : 'Before a tool'));
        const list = v[when] || [];
        if (!list.length) out.push(muted('None.'));
        list.forEach((k, i) => out.push(card(h('div', { class: 'mono' }, k.run), muted(`${(k.tools || []).length ? k.tools.join(', ') : 'every tool'} · stops after ${k.timeout_secs} s`))));
      }
      out.push(muted('Hooks run programs, so they are added and removed on the computer: in the desktop app, or in the file.'));
      return out;
    },
  };
  async function draw() {
    const [id, title, blurb] = SET_PAGES.find((p) => p[0] === setPage) || SET_PAGES[0];
    const known = new Set(SET_INFO.map((i) => i[0]));
    const items = id === 'other' ? core.filter((s) => !known.has(s.key)).map((s) => [s.key, 'other', s.key, s.help, s.kind.type === 'bool' ? 'OnOff' : s.kind.type === 'choice' ? 'Words' : 'Text', s.kind.type === 'choice' ? s.kind.options.map((o) => [o, o]) : '', false]) : SET_INFO.filter((i) => i[1] === id);
    const body = id === 'appearance' ? [appearance()] : items.length ? items.map(item) : extra[id] ? [] : [h('p', { class: 'about' }, 'Nothing to set here from the browser yet.')];
    // Models and keys puts its slider block first, as GPUI does
    if (extra[id]) { const more = (await extra[id]()).flat(2).filter(Boolean); if (id === 'models') body.unshift(...more); else body.push(...more); }
    if (id === 'web') body.push(...await devices());
    page.replaceChildren(...(err ? [h('div', { class: 'banner' }, err)] : []), h('h3', {}, title), h('p', { class: 'blurb' }, blurb), ...body);
  }
  document.body.append(veil);
  pages(); await draw(); q.focus();
}

// ---- routing ----
const views = { memory: memoryView, review: reviewView, team: teamView, settings: settingsView };
function route() {
  if (!signedIn) return;
  const name = location.hash.replace(/^#\/?/, '') || 'chat';
  say('');
  for (const b of document.querySelectorAll('[data-foot]')) b.setAttribute('aria-pressed', String(b.dataset.foot === name));
  if (name === 'chat' || !views[name]) { $('#panel').hidden = true; return; }
  views[name]().catch((e) => say(e.message));
}
function enter() {
  signedIn = true; gate.replaceChildren(); $('#app').hidden = false;
  applyLayout(); loadAddedFonts(); loadThemes(); chatSetup(); route();
}
async function start() {
  // the desktop app's proxy signs every request itself: never pair there, wait for the core instead
  if ((await fetch('/desktop.json', { credentials: 'same-origin' }).catch(() => null))?.ok) {
    for (;;) {
      const r = await fetch('/v1/health', { credentials: 'same-origin' }).catch(() => null);
      if (r?.ok) return enter();
      const why = r?.status === 401 ? 'The core refused this app\'s token. Restart lhd, then this window.' : 'Waiting for the core (lhd) to start…';
      gate.replaceChildren(h('p', { role: 'status' }, why));
      await new Promise((ok) => setTimeout(ok, 2000));
    }
  }
  // a phone opened the pairing link (#login=<token>): trade the token for a session cookie once, then forget it
  const login = /^#login=([0-9a-f]{16,128})$/.exec(location.hash);
  if (login) {
    history.replaceState(null, '', location.pathname);
    const name = /iPhone|iPad/.test(navigator.userAgent) ? 'iPhone' : /Android/.test(navigator.userAgent) ? 'Android phone' : 'Phone';
    const r = await fetch('/auth/token', { method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ token: login[1], name }) }).catch(() => null);
    if (r?.ok) return enter();
    gate.replaceChildren(h('p', { role: 'alert' }, r?.status === 429 ? 'Too many wrong tries. Wait a minute, then open the link again.' : 'That sign-in link is not valid any more. Make a new one on the machine: Settings, Phone, or lh devices phone.'));
    return;
  }
  // a session from an earlier sign-in (the cookie) still counts
  if ((await fetch('/v1/status', { credentials: 'same-origin' }).catch(() => null))?.ok) return enter();
  device = await kv('get', 'device').catch(() => null);
  if (device && (await signIn(device))) return enter();
  if (device) { await kv('del', 'device').catch(() => {}); device = null; }
  pairView();
}
addEventListener('hashchange', route);
if (!window.crypto?.subtle) gate.append(h('p', { role: 'alert' }, 'This browser cannot do the cryptography needed here. Open the address with https://.'));
else start();
})();
