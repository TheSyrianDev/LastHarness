// node render_v1.js <json> <outdir> <size> <frames comma list | start:end:step> [bg]
const { chromium } = require('playwright-core');const fs=require('fs');const path=require('path');
(async()=>{const [src,out,size='256',spec='0:-1:10',bg='transparent']=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
const b=await chromium.launch({executablePath:'/opt/brave-origin-bin/brave-origin',args:['--headless=new']});
const p=await b.newPage({viewport:{width:+size,height:+size}});
const errs=[];p.on('pageerror',e=>errs.push(e.message));p.on('console',m=>{if(m.type()==='error')errs.push(m.text())});
await p.setContent(`<html><body style="margin:0;background:${bg}"><div id="c" style="width:${size}px;height:${size}px"></div></body></html>`);
await p.addScriptTag({path:path.join(__dirname,'node_modules/lottie-web/build/player/lottie.min.js')});
const n=await p.evaluate(d=>{window.A=lottie.loadAnimation({container:document.getElementById('c'),renderer:'canvas',loop:false,autoplay:false,animationData:JSON.parse(d),rendererSettings:{clearCanvas:true}});return A.totalFrames;},fs.readFileSync(src,'utf8'));
let fr=[];if(spec.includes(':')){let[a,e,s]=spec.split(':').map(Number);if(e<0)e=n;for(let f=a;f<=Math.min(e,n);f+=s)fr.push(f);}else fr=spec.split(',').map(Number);
for(const f of fr){const url=await p.evaluate(f=>{A.goToAndStop(f,true);return document.querySelector('canvas').toDataURL('image/png');},Math.min(f,n-0.001));
 fs.writeFileSync(`${out}/f${String(f).padStart(4,'0')}.png`,Buffer.from(url.split(',')[1],'base64'));}
console.log('frames',n,'rendered',fr.length,errs.length?'ERRORS '+errs.join('|'):'');await b.close();})();
