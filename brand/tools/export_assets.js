// Exports the static LastHarness mark (logo glance pose) as SVG and PNG into brand/assets/.
// Re-runnable: `node brand/tools/export_assets.js`. Needs rsvg-convert (librsvg) on PATH.
// Geometry and colours come only from the brand kit: motion-kit/source/brand-geometry.js and tokens.json.
// The 16 px sizes use the hand-placed pixel version (LH_PX), the others the vector mark.
const fs = require('fs'), path = require('path'), vm = require('vm'), { execFileSync } = require('child_process');
const kit = path.join(__dirname, '..', 'motion-kit', 'source');
const out = path.join(__dirname, '..', 'assets');
const ctx = {};
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(path.join(kit, 'brand-geometry.js'), 'utf8') + '\nthis.lhMark=lhMark;this.lhPixel=lhPixel;', ctx);
const tokens = JSON.parse(fs.readFileSync(path.join(kit, 'tokens.json'), 'utf8'));
const grove = tokens.palettes.grove;
const colours = (variant) => {
  const m = tokens.markColours[variant];
  return { face: grove[m.face], eye: grove[m.eye], slot: grove[m.slot] };
};
fs.mkdirSync(out, { recursive: true });
const svg = (body, vb) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}" width="512" height="512">${body}</svg>\n`;
for (const variant of ['onDark', 'onLight']) {
  const c = colours(variant);
  const name = `lastharness-mark-${variant === 'onDark' ? 'on-dark' : 'on-light'}`;
  const vector = path.join(out, `${name}.svg`), pixel = path.join(out, `${name}-16px.svg`);
  fs.writeFileSync(vector, svg(ctx.lhMark({ face: c.face, eye: c.eye, slot: c.slot, mood: 'glance' }), '0 0 64 64'));
  fs.writeFileSync(pixel, svg(ctx.lhPixel(c.face, c.eye, c.slot), '0 0 16 16').replace('width="512" height="512"', 'width="16" height="16" shape-rendering="crispEdges"'));
  for (const size of [16, 32, 48, 64, 128, 256, 512]) {
    const src = size === 16 ? pixel : vector;
    execFileSync('rsvg-convert', ['-w', String(size), '-h', String(size), '-o', path.join(out, `${name}-${size}.png`), src]);
  }
}
// Linux app icon, as on the identity page: rounded square (rx 14) in forest, the on-dark mark at translate(6 6) scale(.8125).
// At 16 px the hand-placed pixel mark stands in.
{
  const c = colours('onDark');
  const tile = path.join(out, 'lastharness-app-icon.svg');
  fs.writeFileSync(tile, svg(`<defs><clipPath id="t"><rect width="64" height="64" rx="14"/></clipPath></defs><g clip-path="url(#t)"><rect width="64" height="64" fill="${grove.forest}"/><g transform="translate(6 6) scale(.8125)">${ctx.lhMark({ face: c.face, eye: c.eye, slot: c.slot, mood: 'glance' })}</g></g>`, '0 0 64 64'));
  for (const size of [16, 32, 48, 64, 128, 256, 512]) {
    const src = size === 16 ? path.join(out, 'lastharness-mark-on-dark-16px.svg') : tile;
    execFileSync('rsvg-convert', ['-w', String(size), '-h', String(size), '-o', path.join(out, `lastharness-app-icon-${size}.png`), src]);
  }
}
console.log('wrote', out);
