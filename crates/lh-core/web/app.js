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
  if (signedIn) { $('#panel').hidden = false; $('#panel-title').textContent = title; }
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
const THEME_VARS = { background: '--bg', surface: '--surface', sunken: '--sunken', text: '--text', muted: '--muted', accent: '--accent', accent_text: '--accent-text', button: '--button', button_hover: '--button-hover', border: '--border', danger: '--danger' };
let themes = [];
function applyTheme(t) {
  const st = document.documentElement.style;
  for (const [k, v] of Object.entries(THEME_VARS)) if (t?.colors?.[k]) st.setProperty(v, t.colors[k]);
  if (t?.shape?.roundness != null) st.setProperty('--radius', `${t.shape.roundness}px`);
  if (t?.animation?.duration_ms != null) st.setProperty('--dur', `${t.animation.duration_ms}ms`);
  st.colorScheme = t?.mode === 'dark' ? 'dark' : 'light';
}
const pref = (k, v) => { try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch { return null; } };
async function loadThemes() {
  const r = await fetch('/desktop.json', { credentials: 'same-origin' }).catch(() => null);
  if (!r?.ok) return;
  const d = await r.json();
  themes = d.themes || [];
  if (d.name) userName = d.name;
  const want = pref('theme') || 'Calm/' + (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  applyTheme(themes.find((t) => `${t.name}/${t.mode}` === want) || themes[0]);
  greet();
}
function setTheme(key) { pref('theme', key); applyTheme(themes.find((t) => `${t.name}/${t.mode}` === key)); }

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
  const cell = (tag, v) => h(tag, { scope: tag === 'th' ? 'col' : null }, fmt(v == null ? '' : String(v)));
  return h('div', { class: 'answer-table' },
    h('div', { class: 'answer-table-scroll', tabindex: '0' },
      h('table', {}, t.caption ? h('caption', {}, t.caption) : null,
        h('thead', {}, h('tr', {}, t.columns.map((c) => cell('th', c)))),
        h('tbody', {}, t.rows.map((r) => h('tr', {}, r.map((v) => cell('td', v))))))));
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
  const all = c.series.flatMap((x) => x.values);
  const lo = Math.min(0, ...all), hi = Math.max(0, ...all) || 1;
  const y = (v) => T + (H - T - B) * (1 - (v - lo) / (hi - lo));
  const n = c.labels.length; const bw = (W - L - R) / n;
  for (let k = 0; k <= 4; k++) {
    const v = lo + ((hi - lo) * k) / 4;
    svg.append(s('line', { x1: L, x2: W - R, y1: y(v), y2: y(v), class: 'grid' }), s('text', { x: L - 6, y: y(v) + 4, 'text-anchor': 'end', class: 'axis' }, fmt(v)));
  }
  const every = Math.ceil(n / 12);
  c.labels.forEach((l, i) => { if (i % every === 0) svg.append(s('text', { x: L + bw * (i + 0.5), y: H - B + 18, 'text-anchor': 'middle', class: 'axis' }, String(l).slice(0, 14))); });
  c.series.forEach((ser, si) => {
    const col = SERIES[si % SERIES.length];
    if (c.type === 'bar') {
      const gw = (bw * 0.8) / c.series.length;
      ser.values.forEach((v, i) => svg.append(s('rect', { x: L + bw * i + bw * 0.1 + gw * si, y: Math.min(y(v), y(0)), width: Math.max(1, gw - 2), height: Math.abs(y(v) - y(0)), fill: col, rx: 2 }, s('title', {}, `${ser.name ? ser.name + ', ' : ''}${c.labels[i]}: ${v}`))));
    } else {
      svg.append(s('polyline', { points: ser.values.map((v, i) => `${L + bw * (i + 0.5)},${y(v)}`).join(' '), fill: 'none', stroke: col, 'stroke-width': 2.2, 'stroke-linejoin': 'round' }));
      ser.values.forEach((v, i) => svg.append(s('circle', { cx: L + bw * (i + 0.5), cy: y(v), r: 3, fill: col }, s('title', {}, `${ser.name ? ser.name + ', ' : ''}${c.labels[i]}: ${v}`))));
    }
  });
  return h('figure', { class: 'answer-chart' }, svg, c.series.length > 1 ? legend : null, c.caption ? h('figcaption', {}, c.caption) : null);
}
// The app's page gets the theme's colours and some base styles, so a plain page already fits the look.
function appCss() {
  const cs = getComputedStyle(document.documentElement);
  const v = Object.values(THEME_VARS).concat('--radius').map((k) => `${k}:${cs.getPropertyValue(k).trim()}`).join(';');
  return `:root{${v};color-scheme:${cs.colorScheme || 'light'}}*{box-sizing:border-box}html,body{margin:0;background:transparent;color:var(--text);font:15px/1.5 ${cs.fontFamily}}` +
    'button,input,select,textarea{font:inherit;color:inherit;background:var(--sunken);border:1px solid var(--border);border-radius:8px;padding:6px 10px}' +
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
  try {
    if (ev.kind === 'table') return h('figure', { class: 'answer-block' }, ev.title ? h('figcaption', {}, ev.title) : null, tableEl(JSON.parse(ev.content)));
    if (ev.kind === 'chart') return h('figure', { class: 'answer-block' }, ev.title ? h('figcaption', {}, ev.title) : null, chartEl(JSON.parse(ev.content)));
    if (ev.kind === 'app') return appEl(ev.title, ev.content);
  } catch (e) { return h('p', { class: 'msg error' }, `Could not draw ${ev.title}: ${e.message}`); }
  return null;
}

// ---- model picker ----
// Like /pick: one row per model, its settings inside it. A llama-swap name is "<model>-<context tier>[-<template>]",
// and a mode is asked by the alias "<name>:<mode>". Search box, arrows, Enter; the choice shows on the button.
const TIERS = ['low', 'balanced', 'native', 'max'];
const MODE_WORD = { think: 'Thinking', code: 'Coding', instruct: 'No thinking', auto: 'Auto', low: 'Reasoning low', medium: 'Reasoning medium', high: 'Reasoning high' };
const TPL_NOTE = { sharp: 'terse, steers reasoning', stock: "the model's own template", vision: 'reads images, slower' };
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
function choiceLabel() {
  if (!modelChoice.id) return 'Pick a model';
  const p = parseModel(modelChoice.id);
  const bits = [displayName(p.base)];
  if (p.tier && p.tier !== 'balanced') bits.push(p.tier);
  if (p.tpl !== 'sharp') bits.push(p.tpl);
  if (modelChoice.mode) bits.push(MODE_WORD[modelChoice.mode] || modelChoice.mode);
  return bits.join(' · ');
}
// the id sent to the core: the recipe, plus ":mode" when a mode other than the default is chosen
const currentModel = () => modelChoice.id ? modelChoice.id + (modelChoice.mode ? `:${modelChoice.mode}` : '') : '';
function setChoice(id, mode) {
  modelChoice = { id, mode: mode || '' };
  pref('model', currentModel());
  $('#model-btn-label').textContent = choiceLabel();
}
async function loadModels() {
  let v = {};
  try { v = await json('/v1/models'); } catch (e) { say(e.message); }
  const details = v.details || (v.models || []).map((id) => ({ id }));
  modelGroups = groupModels(details);
  const saved = pref('model') || '';
  const p = parseModel(saved);
  const known = details.some((d) => d.id === p.name);
  if (known) setChoice(p.name, p.mode);
  else {
    const g = modelGroups.find((x) => /qwen/i.test(x.base)) || modelGroups[0];
    const m = g && (g.members.find((x) => x.tier === 'balanced' && x.tpl === 'sharp') || g.members[0]);
    setChoice(m ? m.id : '', '');
  }
}
function openPicker() {
  const btn = $('#model-btn');
  if ($('#picker')) { closePicker(); return; }
  const q = h('input', { type: 'search', placeholder: 'Search models', 'aria-label': 'Search models', autocomplete: 'off' });
  const list = h('ul', { role: 'listbox', 'aria-label': 'Models' });
  const detail = h('div', { class: 'pick-detail' });
  const pop = h('div', { class: 'picker', id: 'picker', role: 'dialog', 'aria-label': 'Choose a model' }, h('div', { class: 'pick-search' }, q), h('div', { class: 'pick-body' }, list, detail));
  let rows = []; let active = 0;
  const cur = parseModel(modelChoice.id);
  let openBase = cur.base;
  const chip = (label, on, onclick, note) => h('button', { type: 'button', class: 'chip', 'aria-pressed': String(on), title: note || null, onclick }, label);
  function showDetail(g) {
    openBase = g.base;
    const sel = g.members.find((m) => m.id === modelChoice.id) || g.members.find((m) => m.tier === 'balanced' && m.tpl === 'sharp') || g.members[0];
    const tpls = [...new Set(g.members.map((m) => m.tpl))].sort((a, b) => (a === 'sharp' ? -1 : b === 'sharp' ? 1 : a.localeCompare(b)));
    const tiers = TIERS.filter((t) => g.members.some((m) => m.tier === t && m.tpl === sel.tpl));
    const pick = (tier, tpl, mode) => {
      const m = g.members.find((x) => x.tier === tier && x.tpl === tpl) || g.members.find((x) => x.tpl === tpl) || sel;
      setChoice(m.id, mode); showDetail(g); render();
    };
    const mode = sel.id === modelChoice.id ? modelChoice.mode : '';
    detail.replaceChildren(
      h('div', { class: 'pick-title' }, g.label, g.size ? h('span', { class: 'muted' }, ` ${g.size}B`) : null),
      tiers.length > 1 ? h('div', { class: 'pick-row' }, h('span', { class: 'pick-label' }, 'Context'), h('div', { class: 'chips' }, tiers.map((t) => chip(t, sel.tier === t && sel.id === modelChoice.id, () => pick(t, sel.tpl, mode))))) : null,
      tpls.length > 1 ? h('div', { class: 'pick-row' }, h('span', { class: 'pick-label' }, 'Template'), h('div', { class: 'chips' }, tpls.map((t) => chip(t, sel.tpl === t && sel.id === modelChoice.id, () => pick(sel.tier, t, mode), TPL_NOTE[t])))) : null,
      sel.modes.length ? h('div', { class: 'pick-row' }, h('span', { class: 'pick-label' }, 'Mode'), h('div', { class: 'chips' },
        chip(`${MODE_WORD[sel.dflt] || 'Default'} (default)`, sel.id === modelChoice.id && !modelChoice.mode, () => pick(sel.tier, sel.tpl, '')),
        sel.modes.filter((m) => m !== sel.dflt).map((m) => chip(MODE_WORD[m] || m, sel.id === modelChoice.id && modelChoice.mode === m, () => pick(sel.tier, sel.tpl, m))))) : null,
      h('div', { class: 'pick-actions' }, h('button', { type: 'button', class: 'btn primary', onclick: () => { if (sel.id !== modelChoice.id) setChoice(sel.id, ''); closePicker(); } }, 'Use this model')));
  }
  function render() {
    const words = q.value.toLowerCase().split(/\s+/).filter(Boolean);
    rows = modelGroups.filter((g) => words.every((w) => `${g.label} ${g.base}`.toLowerCase().includes(w)));
    active = Math.min(active, Math.max(0, rows.length - 1));
    list.replaceChildren(...rows.map((g, i) => {
      const isCur = g.base === cur.base;
      return h('li', { role: 'option', 'aria-selected': String(i === active), class: g.base === openBase ? 'open' : null, onclick: () => { active = i; showDetail(g); render(); }, onmouseenter: () => { active = i; mark(); } },
        h('span', { class: 'dot', 'data-on': String(g.members.some((m) => m.loaded)), title: g.members.some((m) => m.loaded) ? 'Loaded' : null }),
        h('span', { class: 'name' }, g.label), g.size ? h('span', { class: 'muted' }, `${g.size}B`) : null, isCur ? h('span', { class: 'check', 'aria-label': 'current' }, '✓') : null);
    }));
    if (!rows.length) list.append(h('li', { class: 'empty' }, 'No model matches'));
  }
  const mark = () => list.querySelectorAll('[role=option]').forEach((li, i) => li.setAttribute('aria-selected', String(i === active)));
  q.addEventListener('input', () => { active = 0; render(); });
  pop.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); closePicker(); btn.focus(); }
    else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); active = (active + (e.key === 'ArrowDown' ? 1 : -1) + rows.length) % Math.max(rows.length, 1); mark(); list.querySelectorAll('[role=option]')[active]?.scrollIntoView({ block: 'nearest' }); }
    else if (e.key === 'Enter' && e.target === q && rows[active]) {
      e.preventDefault();
      const g = rows[active];
      if (openBase === g.base) { detail.querySelector('.pick-actions button')?.click(); } else { showDetail(g); render(); }
    }
  });
  btn.after(pop); btn.setAttribute('aria-expanded', 'true');
  render();
  const g0 = modelGroups.find((g) => g.base === cur.base) || modelGroups[0];
  if (g0) { active = Math.max(0, rows.indexOf(g0)); showDetail(g0); render(); list.querySelector('.open')?.scrollIntoView({ block: 'nearest' }); }
  q.focus();
  setTimeout(() => addEventListener('pointerdown', outside), 0);
}
function outside(e) { if (!e.target.closest('#picker, #model-btn')) closePicker(); }
function closePicker() { $('#picker')?.remove(); $('#model-btn').setAttribute('aria-expanded', 'false'); removeEventListener('pointerdown', outside); }

// ---- chat ----
// One reading column. Answers have no bubble; your messages do. Thinking and tool calls fold into one quiet
// line before the answer. The view never scrolls by itself while text streams in, only when you send.
let userName = '';
let chat = { id: null, stored: null, busy: false, ctl: null };
const log = () => $('#log');
function greet() { $('#greeting').textContent = userName ? `${userName} is back!` : 'What would you like to explore?'; }
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
function steps() {
  const list = h('ol', { hidden: true });
  const btn = h('button', { type: 'button', 'aria-expanded': 'false' }, 'Working…');
  const box = h('div', { class: 'steps' }, btn, list);
  const t0 = Date.now();
  let n = 0;
  btn.addEventListener('click', () => { const open = list.hidden; list.hidden = !open; btn.setAttribute('aria-expanded', String(open)); box.toggleAttribute('data-open', open); });
  const label = (done) => { btn.textContent = done ? `Worked for ${Math.max(1, Math.round((Date.now() - t0) / 1000))} s, ${n} step${n === 1 ? '' : 's'}` : `Working… ${n} step${n === 1 ? '' : 's'}`; };
  return {
    box,
    add(text) { n++; const li = h('li', {}, text); list.append(li); label(false); return li; },
    done() { if (n) label(true); else box.remove(); },
  };
}
async function loadChats() {
  const ul = $('#chat-list');
  let list = [];
  try { list = (await json('/v1/chats')).chats || []; } catch { /* chats may be off */ }
  ul.replaceChildren(...list.map((c) => {
    const li = h('li', {});
    const row = () => li.replaceChildren(
      h('button', { type: 'button', class: 'open-chat', 'aria-current': String(c.id === chat.stored), onclick: () => openStored(c) }, c.title || 'Untitled'),
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
  }));
  // the trash icon (drawn here: svg children made with h() would be in the wrong namespace)
  for (const svg of ul.querySelectorAll('.del svg')) svg.replaceWith(s('svg', { viewBox: '0 0 24 24', 'aria-hidden': 'true' }, s('path', { d: 'M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12' })));
  if (list.length && !userName) greet();
}
function newChat() {
  chat.ctl?.abort();
  chat = { id: null, stored: null, busy: false, ctl: null };
  log().replaceChildren(); $('#asks').replaceChildren();
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
      const v = await json('/v1/sessions', { method: 'POST', body: { model: currentModel(), mode: 'assistant' } });
      chat.id = v.id;
    }
  }
  setEmpty(false);
  message('user', text, { reveal: true });
  $('#scroll').scrollTop = $('#scroll').scrollHeight; // only on send
  let reply = null; let work = null;
  const tools = {};
  const answer = () => { if (!reply) { work?.done(); reply = message('assistant', '', { reveal: true }); } return reply; };
  const step = (t) => { if (reply) { reply = null; work = null; } if (!work) { work = steps(); log().append(work.box); } return work.add(t); };
  chat.ctl = new AbortController();
  try {
    await stream(`/v1/sessions/${chat.id}/turns`, { method: 'POST', body: { text }, signal: chat.ctl.signal }, (ev) => {
      if (ev.type === 'text') { const b = answer(); b.dataset.raw = (b.dataset.raw || '') + ev.text; if (!b.pending) { b.pending = true; requestAnimationFrame(() => { b.pending = false; renderAnswer(b, b.dataset.raw); }); } }
      else if (ev.type === 'reasoning') { if (!tools.thinking) tools.thinking = step('Thinking'); }
      else if (ev.type === 'tool_start') { tools.thinking = null; tools[ev.id] = step(`${ev.name}`); }
      else if (ev.type === 'tool_result') { const li = tools[ev.id]; if (li && ev.text) li.append(h('pre', {}, ev.text.slice(0, 4000))); }
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
  $('#panel-close').addEventListener('click', () => { $('#panel').hidden = true; location.hash = '#/chat'; });
  for (const b of document.querySelectorAll('.rail [data-go]')) {
    b.addEventListener('click', () => {
      if (b.dataset.go === 'chats') { $('#chats').hidden = !$('#chats').hidden; b.setAttribute('aria-pressed', String(!$('#chats').hidden)); return; }
      location.hash = location.hash === `#/${b.dataset.go}` ? '#/chat' : `#/${b.dataset.go}`;
    });
  }
  addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key.toLowerCase() === 'b') { e.preventDefault(); $('.rail [data-go="chats"]').click(); }
    if (e.ctrlKey && e.key.toLowerCase() === 'n') { e.preventDefault(); newChat(); }
  });
  $('#model-btn').addEventListener('click', openPicker);
  greet(); loadModels(); newChat();
}
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
async function teamView() {
  const depts = h('select', { id: 'dept' });
  const goal = h('textarea', { rows: '3', id: 'goal' });
  const log = h('div', { role: 'log', 'aria-live': 'polite', 'aria-label': 'Progress' });
  const asks = h('div', {});
  const workers = h('ul', { 'aria-label': 'Workers' });
  const away = h('p', {});
  const err = h('p', { role: 'alert' });
  try { (await json('/v1/team/departments')).departments.filter((d) => !d.error).forEach((d) => depts.append(h('option', { value: d.name }, `${d.name}: ${d.description || ''}`))); } catch (e) { err.textContent = e.message; }
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
      const w = (await json('/v1/team/workers')).workers;
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
async function settingsView() {
  const out = h('div', {});
  const devs = h('ul', {});
  const msg = h('p', { role: 'status' });
  async function load() {
    const v = await json('/v1/settings');
    const inputs = [];
    let group = '', fs;
    out.replaceChildren();
    for (const s of v.settings) {
      if (s.group !== group) { group = s.group; fs = h('fieldset', {}, h('legend', {}, group)); out.append(fs); }
      const id = 's_' + s.key.replace('.', '_');
      const k = s.kind;
      let input;
      if (k.type === 'bool') { input = h('select', { id }, h('option', { value: 'true' }, 'On'), h('option', { value: 'false' }, 'Off')); input.value = String(s.value); }
      else if (k.type === 'choice') { input = h('select', { id }, k.options.map((o) => h('option', { value: o }, o))); input.value = s.value; }
      else { input = h('input', { id, type: k.type === 'int' ? 'number' : 'text', value: s.value, min: k.min, max: k.max }); }
      inputs.push([s, input]);
      fs.append(h('p', {}, h('label', { for: id }, s.key + ' '), input, h('br'), h('small', { id: id + '_h' }, `${s.help} ${s.live ? '' : '(applies at the next start)'}`)));
      input.setAttribute('aria-describedby', id + '_h');
    }
    out.append(h('p', {}, h('button', { type: 'button', onclick: async () => {
      const body = {};
      for (const [s, i] of inputs) if (i.value !== String(s.value)) body[s.key] = i.value;
      if (!Object.keys(body).length) { msg.textContent = 'Nothing changed.'; return; }
      try { const r = await json('/v1/settings', { method: 'PUT', body }); msg.textContent = `Saved. ${r.restart_needed.length ? 'Applies at the next start: ' + r.restart_needed.join(', ') + '.' : ''}`; load(); } catch (e) { msg.textContent = e.message; }
    } }, 'Save changes')));
    const d = (await json('/v1/devices')).devices;
    devs.replaceChildren(...d.map((x) => h('li', {}, `${x.name}${x.revoked ? ' (removed)' : ''}${device && x.id === device.id ? ' (this browser)' : ''} `, x.revoked ? null : h('button', { type: 'button', onclick: async () => { if (confirm(`Remove ${x.name}? It will have to pair again.`)) { await api(`/v1/devices/${x.id}`, { method: 'DELETE' }); load(); } } }, 'Remove'))));
  }
  const look = themes.length ? field('Look', h('select', { onchange: (e) => setTheme(e.target.value) }, themes.map((t) => { const k = `${t.name}/${t.mode}`; return h('option', { value: k, selected: k === (pref('theme') || '') }, `${t.name} ${t.mode}`); }))) : null;
  show('Settings', look, msg, out, h('h3', {}, 'Paired devices'),
    h('p', {}, 'A browser keeps its key inside the browser, which is weaker than a phone\'s secure storage. Remove any device you do not recognise.'), devs);
  load().catch((e) => { msg.textContent = e.message; });
}

// ---- routing ----
const views = { memory: memoryView, review: reviewView, team: teamView, settings: settingsView };
function route() {
  if (!signedIn) return;
  const name = location.hash.replace(/^#\/?/, '') || 'chat';
  say('');
  for (const b of document.querySelectorAll('.rail [data-go]')) b.setAttribute('aria-pressed', String(b.dataset.go === name || (b.dataset.go === 'chats' && !$('#chats').hidden && name === 'chat')));
  if (name === 'chat' || !views[name]) { $('#panel').hidden = true; return; }
  views[name]().catch((e) => say(e.message));
}
function enter() {
  signedIn = true; gate.replaceChildren(); $('#app').hidden = false;
  loadThemes(); chatSetup(); route();
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
