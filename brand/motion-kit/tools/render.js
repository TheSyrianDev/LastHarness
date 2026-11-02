// node render.js <lottie.json> <outdir> [size] [everyNth]  -> PNG frames via lottie-web canvas renderer
const { chromium } = require('playwright');const fs=require('fs');const path=require('path');
(async()=>{const [src,out,size='512',nth='1']=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
const b=await chromium.launch();const p=await b.newPage({viewport:{width:+size,height:+size}});
p.on('pageerror',e=>console.log('ERR',e.message));
await p.setContent(`<html><body style="margin:0;background:transparent"><div id="c" style="width:${size}px;height:${size}px"></div></body></html>`);
await p.addScriptTag({path:path.join(__dirname,'node_modules/lottie-web/build/player/lottie.min.js')});
const data=fs.readFileSync(src,'utf8');
const n=await p.evaluate(d=>{window.A=lottie.loadAnimation({container:document.getElementById('c'),renderer:'canvas',loop:false,autoplay:false,animationData:JSON.parse(d),rendererSettings:{clearCanvas:true}});return A.totalFrames;},data);
for(let f=0;f<n;f+=+nth){const url=await p.evaluate(f=>{A.goToAndStop(f,true);return document.querySelector('canvas').toDataURL('image/png');},f);
 fs.writeFileSync(`${out}/f${String(f).padStart(4,'0')}.png`,Buffer.from(url.split(',')[1],'base64'));}
console.log('frames',n);await b.close();})();
