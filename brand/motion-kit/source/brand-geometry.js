// LastHarness mark: Glance + Slot, pure. 64 grid. Everything sits inside the circle (32,32) r30.
// Face: rounded square x 8..46, y 18..56, rx 15. Slot: circle (49,15) r6. Eyes glance at the slot.
const LH={
  face:{x:8,y:18,w:38,h:38,rx:15}, slot:{cx:49,cy:15,r:6}, centre:[27,37],
  glance:'translate(2.4 -2) rotate(-8 27 37)'
};
const _r=(x,y,w,h,r,f,ex='')=>`<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" fill="${f}" ${ex}/>`;
// eye shapes for each mood. Eyes are drawn in face coordinates, then the mood's transform is applied.
function lhEyes(c,mood='glance',dist=false){
  const n=dist?[18.1,29.8,7,12.6,3.5]:[18.6,30.6,6,11,3], f=dist?[29.6,30.6,5.6,11.8,2.8]:[30.1,31.4,4.6,10.2,2.3];
  const pill=([x,y,w,h,r])=>_r(x,y,w,h,r,c);
  const line=(d,w=2.6)=>`<path d="${d}" fill="none" stroke="${c}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"/>`;
  switch(mood){
    case 'front': return pill(n)+pill([28.4,30.6,6,11,3]);
    case 'glance': return `<g transform="${LH.glance}">${pill(n)+pill(f)}</g>`;
    case 'think': return `<g transform="translate(1.4 -3.6) rotate(-4 27 37)">${pill(n)+pill(f)}</g>`;
    case 'happy': return `<g transform="translate(1.2 -1) rotate(-6 27 37)">${line('M18.4 38.2Q21.6 32.6 24.8 38.2',3)+line('M29.8 38.2Q32.4 33.4 35 38.2',2.7)}</g>`;
    case 'sleep': return line('M18.6 38.6H24.6',2.8)+line('M29.4 38.6H35',2.8);
    case 'error': return line('M18.8 33.2L24.4 38.8M24.4 33.2L18.8 38.8',2.6)+line('M29.6 33.2L35.2 38.8M35.2 33.2L29.6 38.8',2.6);
    case 'ask': return _r(17.8,29.4,7,12.6,3.5,c)+_r(29.2,29.4,7,12.6,3.5,c);
  }
}
// o: {face, eye, slot, mood, dist, cls:{eyes,slot,face}}
function lhMark(o){
  const {face,eye,slot=face,mood='glance',dist=false,cls={}}=o, F=LH.face, s=dist?6.6:LH.slot.r;
  return `<g class="${cls.face||''}" style="transform-origin:27px 37px">${_r(F.x,F.y,F.w,F.h,F.rx,face)}</g>`
   +`<g class="${cls.eyes||''}" style="transform-origin:27px 37px">${lhEyes(eye,mood,dist)}</g>`
   +`<circle class="${cls.slot||''}" style="transform-origin:49px 15px" cx="${LH.slot.cx}" cy="${LH.slot.cy}" r="${s}" fill="${slot}"/>`;
}
// 16 px pixel version: X face, e eye, O slot
const LH_PX=[
  '.............OO.',
  '............OOOO',
  '............OOOO',
  '.............OO.',
  '................',
  '...XXXXXXX......',
  '..XXXXXXXXX.....',
  '.XXXXXXXeXXX....',
  '.XXXeeXXeXXX....',
  '.XXXeeXXeXXX....',
  '.XXXeeXXXXXX....',
  '.XXXXXXXXXXX....',
  '.XXXXXXXXXXX....',
  '.XXXXXXXXXXX....',
  '..XXXXXXXXX.....',
  '...XXXXXXX......'
];
function lhPixel(face,eye,slot=face){
  let s='';LH_PX.forEach((row,y)=>[...row].forEach((ch,x)=>{
    const c=ch==='X'?face:ch==='e'?eye:ch==='O'?slot:null; if(c&&c!=='none') s+=`<rect x="${x}" y="${y}" width="1.02" height="1.02" fill="${c}"/>`;}));
  return s;
}
// custom wordmark, lowercase geometric, even stroke, round ends. viewBox 0 0 426 74 (baseline 66, x-height 32..66)
function lhWord(c,a=c,w=8.5){
  const s=cx=>`M${cx+11} 39C${cx+11} 34 ${cx+6} 32 ${cx} 32C${cx-8} 32 ${cx-12} 36 ${cx-12} 40C${cx-12} 44 ${cx-8} 46 ${cx} 47C${cx+8} 48 ${cx+12} 50 ${cx+12} 54C${cx+12} 59 ${cx+7} 62 ${cx} 62C${cx-7} 62 ${cx-11} 59 ${cx-12} 55`;
  const g=(col,d)=>`<g fill="none" stroke="${col}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round">${d}</g>`;
  return g(c,`<path d="M6 8V64"/><circle cx="35" cy="47" r="15"/><path d="M50 32V64"/><path d="${s(77)}"/><path d="M110 16V51A13 13 0 0 0 123 64H126M101 32H125"/>`)
   +g(a,`<path d="M140 8V64M140 47A15 15 0 0 1 170 47V64"/><circle cx="201" cy="47" r="15"/><path d="M216 32V64"/><path d="M232 32V64M232 46A14 14 0 0 1 246 32H252"/><path d="M266 32V64M266 47A15 15 0 0 1 296 47V64"/><path d="M312 47H342A15 15 0 1 0 337.6 57.6"/><path d="${s(369)}"/><path d="${s(407)}"/>`);
}
// horizontal lockup in one SVG: viewBox 0 0 352 64. Wordmark ascender (8..64) matched to the face height (18..56).
function lhLockup(o){const k=38/56;return lhMark(o)+`<g transform="translate(70 ${56-64*k}) scale(${k})">${lhWord(o.word||o.face,o.word2||o.word||o.face)}</g>`;}
