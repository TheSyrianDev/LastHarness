// Runs the same WebCrypto calls as web/app.js against a real core, to prove a browser's keys, proof and
// signatures are accepted: pair, accept on the "machine", sign in, make an authenticated request.
const [https, local, token] = process.argv.slice(2);
const enc = new TextEncoder();
const hex = (b) => [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, '0')).join('');
const post = (u, body, h = {}) => fetch(u, { method: 'POST', headers: { 'content-type': 'application/json', ...h }, body: JSON.stringify(body) });

const kp = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign', 'verify']);
const jwk = await crypto.subtle.exportKey('jwk', kp.publicKey);
const pub = { kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y };
let privateExportable = true;
try { await crypto.subtle.exportKey('jwk', kp.privateKey); } catch { privateExportable = false; }
if (privateExportable) throw new Error('the private key must not be exportable');

const pin = '482916';
const mac = await crypto.subtle.importKey('raw', enc.encode(pin), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
const proof = hex(await crypto.subtle.sign('HMAC', mac, enc.encode(`${pub.x}.${pub.y}`)));
const start = await (await post(`${https}/pair/start`, { name: 'node', public_key: pub, proof })).json();
if (!start.id) throw new Error('pair/start: ' + JSON.stringify(start));
const acc = await (await post(`${local}/v1/devices/accept`, { pin }, { authorization: `Bearer ${token}` })).json();
if (!acc.id) throw new Error('accept: ' + JSON.stringify(acc));
const st = await (await fetch(`${https}/pair/status/${start.id}`)).json();
if (st.state !== 'paired' || st.device_id !== acc.id) throw new Error('status: ' + JSON.stringify(st));

const { nonce } = await (await post(`${https}/auth/challenge`, { device_id: acc.id })).json();
const sig = hex(await crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, kp.privateKey, enc.encode(nonce)));
const v = await post(`${https}/auth/verify`, { device_id: acc.id, nonce, signature: sig });
if (!v.ok) throw new Error('verify: ' + v.status);
const cookie = v.headers.get('set-cookie').split(';')[0];
const r = await fetch(`${https}/v1/models`, { headers: { cookie } });
const body = await r.json();
if (r.status !== 200 || body.models[0] !== 'm1') throw new Error('request: ' + r.status + JSON.stringify(body));
console.log('interop ok');
