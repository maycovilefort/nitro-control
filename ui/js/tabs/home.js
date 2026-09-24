import { MODES, modeLabel, modeDesc, nextModeId, fmtTemp, fmtPct, pct, dialDash, dialPoint, hexToRgb } from '../logic.js';
import { h, seg, setSeg, fan, setFan, stat, setStat, setSpark } from '../widgets.js';

const el = {};
const FX = { off: 'Desligado', static: 'Estático', breathing: 'Respiração', neon: 'Neon' };
const WHEEL_LOCK_MS = 450;
let lastWheel = 0;

function side(cls, label) {
  const root = h(`<div class="side ${cls}">
    <div class="side-head"><span class="k">${label}</span><span class="t cond">—</span></div>
    <div class="fanslot"></div>
    <div class="rpm"><span class="n cond">—</span><span class="u">RPM</span></div>
    <div class="stats"></div>
    <div class="spark"><div class="hd"><span>TEMPERATURA</span><span>5 MIN</span></div>
      <svg viewBox="0 0 200 36" preserveAspectRatio="none"><path class="area"/><path class="line"/></svg></div>
  </div>`);
  const f = fan(label);
  root.querySelector('.fanslot').replaceWith(f);
  return { root, fan: f, temp: root.querySelector('.t'), rpm: root.querySelector('.rpm .n'), stats: root.querySelector('.stats'),
    area: root.querySelector('.area'), line: root.querySelector('.line') };
}

function dial(ctx) {
  const root = h(`<div class="dialwrap"><div class="dial">
    <svg viewBox="0 0 400 400">
      <circle cx="200" cy="200" r="150" transform="rotate(150 200 200)" fill="none" stroke="rgba(255,255,255,.07)" stroke-width="4" stroke-dasharray="628.3 942.5"/>
      <circle class="d-arc" cx="200" cy="200" r="150" transform="rotate(150 200 200)" style="stroke-dasharray:0 942.5"/>
      <circle cx="200" cy="200" r="138" fill="none" stroke="rgba(255,255,255,.14)" stroke-width="5" stroke-dasharray="1 13.451"/>
      <circle cx="200" cy="200" r="126" fill="none" stroke="rgba(255,255,255,.08)" stroke-width="6" stroke-dasharray="1 7"/>
      <circle cx="200" cy="200" r="112" fill="rgba(0,0,0,.4)" stroke="rgba(255,255,255,.05)" stroke-width="1"/>
      <g class="d-seeker"><circle cx="70.1" cy="275" r="6"/></g>
    </svg>
    <div class="radar"></div>
    <button class="dcenter needs-daemon" title="Clique: próximo modo · botão direito: anterior · roda do mouse também troca">
      <span class="k">MODO DO SISTEMA</span><span class="name">—</span><span class="desc"></span>
      <span class="hint"><i>‹</i>CLIQUE PARA TROCAR<i>›</i></span>
    </button>
    <div class="wave w1"></div><div class="wave w2"></div>
  </div></div>`);
  const box = root.querySelector('.dial');
  const pick = (id) => ctx.run(() => ctx.api.setProfile(id));
  const nodes = [];
  const labels = [];
  MODES.forEach((m, i) => {
    const p = dialPoint(i, 150);
    const n = h(`<button class="node needs-daemon" title="${m.label}" style="left:${p.x};top:${p.y}"><span></span></button>`);
    n.addEventListener('click', () => pick(m.id));
    const lp = dialPoint(i, i === 2 ? 176 : 166);
    const tf = i === 2 ? 'translate(-50%,-50%)' : i < 2 ? 'translate(-100%,-50%)' : 'translate(0,-50%)';
    const l = h(`<button class="nlabel needs-daemon" style="left:${lp.x};top:${lp.y};transform:${tf}">${m.label}</button>`);
    l.addEventListener('click', () => pick(m.id));
    nodes.push(n);
    labels.push(l);
  });
  box.querySelector('.radar').after(...nodes, ...labels);
  const center = box.querySelector('.dcenter');
  const cur = () => ctx.store.snap?.mode;
  center.addEventListener('click', () => pick(nextModeId(cur(), 1)));
  center.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    pick(nextModeId(cur(), -1));
  });
  box.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (document.body.classList.contains('offline')) return;
    const now = Date.now();
    if (now - lastWheel < WHEEL_LOCK_MS) return;
    lastWheel = now;
    pick(nextModeId(cur(), e.deltaY > 0 ? 1 : -1));
  }, { passive: false });
  return { root, arc: box.querySelector('.d-arc'), seeker: box.querySelector('.d-seeker'), nodes, labels,
    name: center.querySelector('.name'), desc: center.querySelector('.desc') };
}

export function mount(root, ctx) {
  root.innerHTML = '<div class="home"><div class="hud"></div><div class="strip"></div></div>';
  const hud = root.querySelector('.hud');
  el.cpu = side('cpu', 'CPU');
  el.gpu = side('gpu', 'GPU');
  el.cpuStats = { use: stat('Uso'), ram: stat('RAM'), sys: stat('Sistema') };
  el.gpuStats = { use: stat('Uso'), clk: stat('Frequência'), pw: stat('Energia') };
  el.cpu.stats.append(...Object.values(el.cpuStats));
  el.gpu.stats.append(...Object.values(el.gpuStats));
  el.dial = dial(ctx);
  hud.append(el.cpu.root, el.dial.root, el.gpu.root);

  const strip = root.querySelector('.strip');
  const cFan = h('<div class="cell"><div class="label">VENTOINHA</div></div>');
  el.fanSeg = seg([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => ctx.run(() => ctx.api.setFan(id)), 'needs-daemon');
  cFan.append(el.fanSeg);
  const cKb = h('<div class="cell" style="gap:16px"><div class="label">TECLADO</div><div class="kbzones"><div></div><div></div><div></div><div></div></div><div class="fxname"></div></div>');
  el.kb = [...cKb.querySelectorAll('.kbzones div')];
  el.fx = cKb.querySelector('.fxname');
  const cMini = h(`<div class="cell">
    <div class="mini"><span class="k">SSD</span><span class="v" data-k="ssd">—</span></div>
    <div class="mini"><span class="k">BATERIA</span><span class="v" data-k="bat">—</span></div>
    <div class="mini"><span class="k">ASENSE</span><span class="v"><span class="dot"></span><span data-k="asense">—</span></span></div>
  </div>`);
  el.mini = cMini;
  strip.append(cFan, cKb, cMini);
}

function sideTemps(side, temp, rpm, hist, key, asleep) {
  side.temp.textContent = asleep ? 'zZ' : fmtTemp(temp);
  side.rpm.textContent = rpm == null ? '—' : String(Math.round(rpm));
  setFan(side.fan, rpm);
  setSpark(side.area, side.line, hist.map((x) => x[key] ?? null), 200, 36, 30, 95);
}

export function update({ store }) {
  const s = store.snap;
  const cfg = store.config;
  const sn = s.sensors;
  const g = sn.gpu?.state === 'active' ? sn.gpu : null;
  const asleep = sn.gpu?.state === 'sleeping';

  sideTemps(el.cpu, sn.cpuTemp, sn.cpuFanRpm, store.hist, 'cpuTemp', false);
  sideTemps(el.gpu, sn.gpuTemp, sn.gpuFanRpm, store.hist, 'gpuTemp', asleep);
  setStat(el.cpuStats.use, fmtPct(sn.cpuUsage), pct(sn.cpuUsage, 100));
  setStat(el.cpuStats.ram, sn.ramUsedGb != null ? `${sn.ramUsedGb.toFixed(1)} / ${sn.ramTotalGb.toFixed(1)} GB` : '—', pct(sn.ramUsedGb, sn.ramTotalGb));
  setStat(el.cpuStats.sys, fmtTemp(sn.sysTemp), pct(sn.sysTemp, 100));
  const maxW = Math.ceil(Math.max(100, ...store.hist.map((x) => x.gpuPower ?? 0)) / 20) * 20;
  setStat(el.gpuStats.use, fmtPct(g?.usage), pct(g?.usage, 100));
  setStat(el.gpuStats.clk, g?.clockMhz != null ? `${Math.round(g.clockMhz)} MHz` : '—', pct(g?.clockMhz, 2500));
  setStat(el.gpuStats.pw, g?.powerW != null ? `${Math.round(g.powerW)} W` : '—', pct(g?.powerW, maxW));

  const idx = Math.max(0, MODES.findIndex((m) => m.id === s.mode));
  el.dial.arc.style.strokeDasharray = dialDash(s.mode ? idx : 0);
  el.dial.seeker.style.transform = `rotate(${idx * 60}deg)`;
  MODES.forEach((m, i) => {
    const on = m.id === s.mode;
    const [r, gg, b] = hexToRgb(cfg.palette[m.id].primary);
    const span = el.dial.nodes[i].firstElementChild;
    span.style.background = `rgba(${r},${gg},${b},.5)`;
    span.style.border = `1px solid rgba(${r},${gg},${b},.9)`;
    el.dial.nodes[i].classList.toggle('on', on);
    el.dial.labels[i].classList.toggle('on', on);
  });
  const label = modeLabel(s.mode);
  el.dial.name.textContent = label;
  el.dial.name.classList.toggle('long', label.length > 8);
  el.dial.desc.textContent = modeDesc(s.mode);

  setSeg(el.fanSeg, s.fanMode);
  const kbColors = cfg.keyboard.effect === 'off'
    ? Array(4).fill('#15141a')
    : cfg.keyboard.followMode && s.mode ? Array(4).fill(cfg.palette[s.mode].primary) : cfg.keyboard.zones;
  el.kb.forEach((d, i) => { d.style.background = kbColors[i]; d.style.color = kbColors[i]; });
  el.fx.textContent = FX[cfg.keyboard.effect];
  el.mini.querySelector('[data-k=ssd]').textContent = fmtTemp(sn.ssdTemp);
  el.mini.querySelector('[data-k=bat]').textContent = s.platform?.batteryLimit ? 'Limite 80%' : 'Carga total';
  el.mini.querySelector('[data-k=asense]').textContent = s.connection === 'connected' ? 'Conectado' : 'Desconectado';
}
