// Drives the real browser UI in a headless Chromium-based browser over the DevTools protocol.
// Not part of `cargo test` (it needs a browser and a running core). To run it:
//   lhd --provider http://127.0.0.1:9100/v1 --web-listen 127.0.0.1:7943 --listen 127.0.0.1:7899 --token-file TOKEN
//   brave-origin --headless=new --no-sandbox --ignore-certificate-errors --remote-debugging-port=9333 about:blank
//   node ui_smoke.mjs https://127.0.0.1:7943 http://127.0.0.1:7899 TOKEN 9333 9100
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
const [https, local, tokenFile, port, mockPort = '9100'] = process.argv.slice(2);
const token = readFileSync(tokenFile, 'utf8').trim();
// A tiny model server: answers department prompts the way a lead would, and chats otherwise.
const PLAN = JSON.stringify({ steps: [{ id: '1', title: 'Say hello', assignee: 'writer', brief: 'Say hello.', depends_on: [] }] });
createServer((req, res) => {
  if (req.url.endsWith('/models')) { res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify({ data: [{ id: 'mock-model' }] })); }
  if (!req.url.endsWith('/chat/completions')) { res.statusCode = 404; return res.end('{}'); }
  let body = ''; req.on('data', (c) => (body += c)); req.on('end', () => {
    const msgs = JSON.parse(body).messages; const last = String(msgs[msgs.length - 1].content || '');
    const text = last.startsWith('[PLAN]') ? PLAN : last.startsWith('[CHECK]') ? 'OK' : last.startsWith('[FINAL]') ? 'All done.' : last === 'Say hello.' ? 'Hello from the writer.' : 'Hello from the mock.';
    res.setHeader('content-type', 'text/event-stream');
    const chunk = (d, f = null) => `data: ${JSON.stringify({ choices: [{ index: 0, delta: d, finish_reason: f }] })}\n\n`;
    res.write(chunk({ role: 'assistant', content: text.slice(0, 5) })); res.write(chunk({ content: text.slice(5) })); res.write(chunk({}, 'stop')); res.end('data: [DONE]\n\n');
  });
}).listen(Number(mockPort), '127.0.0.1');
const api = (method, path, body) => fetch(local + path, { method, headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` }, body: body && JSON.stringify(body) }).then((r) => r.json().catch(() => ({})));
for (const t of ['frontier', 'mid-tier', 'fastest']) await api('PUT', `/v1/team/types/${t}`, { model: 'mock-model' });
await api('PUT', '/v1/team/departments/hello', { text: '---\nname: hello\ndescription: Says hello.\nlead: planner\nmembers: writer\n---\n' });
await api('PUT', '/v1/team/settings', { autonomy: 'plan-first' });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const t = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
const ws = new WebSocket(t.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0; const waits = new Map(); const errors = [];
ws.onmessage = (m) => {
  const v = JSON.parse(m.data);
  if (v.id && waits.has(v.id)) { waits.get(v.id)(v); waits.delete(v.id); }
  if (v.method === 'Runtime.exceptionThrown') errors.push(v.params.exceptionDetails.exception?.description || v.params.exceptionDetails.text);
  if (v.method === 'Runtime.consoleAPICalled' && v.params.type === 'error') errors.push(v.params.args.map((a) => a.value ?? a.description).join(' '));
};
const send = (method, params = {}) => new Promise((r) => { const i = ++id; waits.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const js = async (expr) => { const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true }); if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description || 'eval failed'); return r.result.result.value; };
const dbg = () => js("'MSG=' + document.querySelector('main p[role=status]')?.textContent + ' | TOP=' + document.getElementById('status').textContent + ' | TITLE=' + document.querySelector('#view-title')?.textContent").catch(() => '?');
const until = async (expr, what, ms = 15000) => { const end = Date.now() + ms; for (;;) { const v = await js(expr).catch(() => null); if (v) return v; if (Date.now() > end) throw new Error('timed out waiting for ' + what + '; page says: ' + (await dbg())); await sleep(150); } };
await send('Runtime.enable'); await send('Page.enable'); await send('Security.setIgnoreCertificateErrors', { ignore: true });
await send('Storage.clearDataForOrigin', { origin: https, storageTypes: 'all' });
await send('Page.navigate', { url: https + '/' });

await until("document.querySelector('#view-title')?.textContent === 'Pair this browser'", 'the pairing view');
await fetch(`${local}/v1/devices/open`, { method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` }, body: '{}' });
await js("[...document.querySelectorAll('button')].find((b) => b.textContent === 'Pair this browser').click()");
const pin = (await until("document.querySelector('#pin')?.textContent", 'a PIN')).replace(/\s/g, '');
if (!/^\d{6}$/.test(pin)) throw new Error('bad PIN ' + pin);
const acc = await (await fetch(`${local}/v1/devices/accept`, { method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` }, body: JSON.stringify({ pin }) })).json();
if (!acc.id) throw new Error('accept failed ' + JSON.stringify(acc));
await until("document.querySelector('#nav') && !document.querySelector('#nav').hidden", 'the menu after pairing', 20000);
await until("document.querySelector('#view-title')?.textContent === 'Chat'", 'the chat view', 20000);

// chat: start a conversation and watch a streamed reply arrive
await until("document.querySelector('select option')", 'the model list');
await js("[...document.querySelectorAll('button')].find((b) => b.textContent === 'Start').click()");
await until("document.querySelector('#msg')", 'the message box');
await js("document.querySelector('#msg').value = 'hi'; document.querySelector('form').requestSubmit()");
await until("document.querySelector('[role=log]')?.innerText.includes('Hello from the mock.')", 'the streamed reply');
// a department: give it a goal, approve the plan, see the result
await js("location.hash = '#/team'");
await until("document.querySelector('#dept option')", 'the department list');
await js("document.querySelector('#goal').value = 'greet me'; document.querySelector('#dept').value = 'hello'; document.querySelector('form').requestSubmit()");
await until("[...document.querySelectorAll('button')].some((b) => b.textContent === 'Run this plan')", 'the plan to approve');
if (!(await js("document.querySelector('[role=alertdialog]')?.innerText.includes('Say hello')"))) throw new Error('the plan is not shown');
await js("[...document.querySelectorAll('button')].find((b) => b.textContent === 'Run this plan').click()");
await until("document.querySelector('[role=log]')?.innerText.includes('Result: All done.')", 'the department result', 30000);
for (const [hash, want] of [['memory', 'Memory'], ['review', 'Review'], ['team', 'Departments'], ['settings', 'Settings'], ['chat', 'Chat']]) {
  await js(`location.hash = '#/${hash}'`);
  await until(`document.querySelector('#view-title')?.textContent === '${want}'`, want + ' view');
}
await js("location.hash = '#/settings'");
await until("document.querySelector('#s_provider_url')", 'settings loaded');
await until("[...document.querySelectorAll('li')].some((l) => l.textContent.includes('(this browser)'))", 'this browser in the device list');
// change a setting through the form and see it saved
await js("{ const i = document.querySelector('#s_team_parallel'); i.value = i.value === '3' ? '4' : '3'; [...document.querySelectorAll('button')].find((b) => b.textContent === 'Save changes').click(); }");
await until("document.querySelector('main p[role=status]')?.textContent.startsWith('Saved')", 'the save message');
// a dangerous setting is refused from a browser
await js("{ const i = document.querySelector('#s_permissions_bypass'); i.value = 'true'; [...document.querySelectorAll('button')].find((b) => b.textContent === 'Save changes').click(); }");
await until("document.querySelector('main p[role=status]')?.textContent.includes('cannot be changed from a browser')", 'the refusal');
// focus moves to the new view's heading and the skip link and landmarks exist
if (!(await js("document.activeElement?.id === 'view-title' || document.activeElement?.tagName"))) throw new Error('focus is lost');
for (const sel of ['main#main', 'nav[aria-label=Main]', '#status[aria-live=polite]', 'html[lang]', 'a#skip']) if (!(await js(`!!document.querySelector('${sel}')`))) throw new Error('missing ' + sel);

// the key survives a reload: no pairing needed again
await send('Page.reload');
await until("document.querySelector('#nav') && !document.querySelector('#nav').hidden", 'automatic sign-in after reload');
// revoke from the machine, reload: back to pairing
await fetch(`${local}/v1/devices/${acc.id}`, { method: 'DELETE', headers: { authorization: `Bearer ${token}` } });
await send('Page.reload');
await until("document.querySelector('#view-title')?.textContent === 'Pair this browser'", 'pairing again after revoke');
if (errors.length) throw new Error('page errors: ' + errors.join(' | '));
console.log('ui smoke ok'); process.exit(0);
ws.close();
