  // ---------- Tools in own 64 coords, centred near (32,32) ----------
  const TOOLS = {
    lead: {r:0, draw:(c,k)=>`<circle cx="32" cy="32" r="29" fill="${c}"/><circle cx="32" cy="32" r="21" fill="${k}"/><g class="needle"><path d="M32 13L39 32H25z" fill="${c}"/><path d="M25 32H39L32 51z" fill="${c}" opacity=".45"/></g><circle cx="32" cy="32" r="3.4" fill="${k}"/>`},
    research: {r:0, draw:(c,k)=>`<path d="M37 39l6-6 15 15a4 4 0 0 1 0 5.5l-.5.5a4 4 0 0 1-5.5 0z" fill="${c}"/><circle cx="26" cy="26" r="21" fill="${c}"/><circle cx="26" cy="26" r="13.5" fill="${k}"/><path d="M18 21a9.5 9.5 0 0 1 7-6" fill="none" stroke="${c}" stroke-width="3.6" stroke-linecap="round" opacity=".75"/>`},
    writer: {r:45, draw:(c,k)=>`<rect x="21" y="3" width="22" height="40" rx="8" fill="${c}"/><path d="M21 42H43L35 58Q32 62 29 58z" fill="${c}"/><path d="M21 13H43M21 42.5H43" stroke="${k}" stroke-width="3.2"/><path d="M32 49V59" stroke="${k}" stroke-width="2.8" stroke-linecap="round"/><circle cx="32" cy="48.5" r="2.8" fill="${k}"/>`},
    coder: {r:0, draw:(c,k)=>`<rect x="3" y="9" width="58" height="46" rx="8" fill="${c}"/><path d="M3 21H61" stroke="${k}" stroke-width="3.6"/><path d="M14 30l8 7-8 7" fill="none" stroke="${k}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/><rect class="cursor" x="28" y="41" width="15" height="5" rx="2.5" fill="${k}"/>`},
    designer: {r:45, draw:(c,k)=>`<rect x="27.5" y="0" width="9" height="24" rx="4.5" fill="${c}"/><rect x="21" y="20" width="22" height="11" rx="2" fill="${c}"/><path d="M17 31H47V36C47 47 39 54 32 62C25 54 17 47 17 36z" fill="${c}"/><path d="M21 21H43M21 30.5H43" stroke="${k}" stroke-width="3.2"/>`},
    planner: {r:0, draw:(c,k)=>`<rect x="9" y="8" width="46" height="54" rx="7" fill="${c}"/><rect x="20" y="2" width="24" height="14" rx="4" fill="${c}" stroke="${k}" stroke-width="3.6"/><path d="M18 30h15M18 40h20M18 50h12" stroke="${k}" stroke-width="4" stroke-linecap="round"/><path class="tick" d="M40 46l4 4 7-7" fill="none" stroke="${k}" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>`}
  };
  const C={accent:'#e0af68',blue:'#7aa2f7',magenta:'#bb9af7',green:'#9ece6a',red:'#f7768e',cyan:'#7dcfff'}, BG='#1a1b26';
  const ROLES=[
    {k:'lead',n:'Lead',a:'accent',d:'Compass. The needle spins while it decides the direction.'},
    {k:'research',n:'Researcher',a:'blue',d:'Magnifier. It sweeps the lens over the page.'},
    {k:'writer',n:'Writer',a:'magenta',d:'Pen, nib toward the head. It scribbles.'},
    {k:'coder',n:'Coder',a:'green',d:'Terminal. The cursor blinks and the window bounces with each keystroke.'},
    {k:'designer',n:'Designer',a:'red',d:'Brush, tip toward the head. It dabs.'},
    {k:'planner',n:'Planner',a:'cyan',d:'Clipboard. The tick draws itself.'}
  ];
