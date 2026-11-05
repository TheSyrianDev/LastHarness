// node tools/tray16.js <outdir>: 16 px tray sprites from the hand-placed pixel mark (brand-geometry.js LH_PX).
// One strip per status: frame 0 = slot lit, frame 1 = slot dimmed (for a blink/pulse). Grove on-dark colours.
const fs=require('fs'),path=require('path'),vm=require('vm'),{execFileSync}=require('child_process');
const src=path.join(__dirname,'..','source');const ctx={};vm.createContext(ctx);
vm.runInContext(fs.readFileSync(path.join(src,'brand-geometry.js'),'utf8')+'\nthis.lhPixel=lhPixel;',ctx);
const G=JSON.parse(fs.readFileSync(path.join(src,'tokens.json'),'utf8')).palettes.grove;const out=process.argv[2];
for(const [n,c] of Object.entries({idle:G.moss,busy:G.busy,ready:G.ready,'needs-you':G.needsYou})){
  const f=(slot,x)=>`<g transform="translate(${x} 0)">${ctx.lhPixel(G.lichen,G.forest,slot)}</g>`;
  const svg=`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 16" width="32" height="16" shape-rendering="crispEdges">${f(c,0)}<g opacity=".45">${f(c,16).replace(/<rect[^>]*fill="${G.lichen}"[^>]*\/>|<rect[^>]*fill="${G.forest}"[^>]*\/>/g,'')}</g>${f('none',16)}</svg>`;
  const p=path.join(out,`tray-16-${n}`);fs.writeFileSync(p+'.svg',svg);execFileSync('rsvg-convert',['-o',p+'.png',p+'.svg']);}
