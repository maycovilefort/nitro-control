import { MODES, modeLabel, fmtTemp, fmtPct } from '../logic.js';
import { panel, ring, setRing, chips, setChips } from '../widgets.js';

const el = {};
const FX = { off: 'Desligado', static: 'Estático', breathing: 'Respiração', neon: 'Neon' };

export function mount(root, ctx) {
  root.innerHTML = '<div class="home"></div>';
  const g = root.firstElementChild;

  const gpu = panel('GPU');
  el.gpuRing = ring('MHz', true);
  gpu.querySelector('.body').append(el.gpuRing);
  gpu.querySelector('.body').insertAdjacentHTML('beforeend',
    '<div class="kv" style="margin-top:10px"><span>Uso GPU</span><b data-k="gpuUse">—</b></div><div class="kv"><span>Uso CPU</span><b data-k="cpuUse">—</b></div><div class="kv"><span>Energia GPU</span><b data-k="gpuW">—</b></div>');

  const center = document.createElement('div');
  center.className = 'pn mode-title';
  center.innerHTML = '<div class="h">NITRO<span>CONTROL</span></div><div class="u" style="letter-spacing:.2em">MODO DO SISTEMA</div><div class="curmode">—</div>';
  el.cur = center.querySelector('.curmode');
  el.modes = chips(MODES, (id) => ctx.run(() => ctx.api.setProfile(id)));
  el.modes.style.justifyContent = 'center';
  center.append(el.modes);
  center.insertAdjacentHTML('beforeend', '<div class="ttl" style="margin-top:14px">Ventoinha</div>');
  el.fans = chips([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => ctx.run(() => ctx.api.setFan(id)));
  el.fans.style.justifyContent = 'center';
  center.append(el.fans);
  center.insertAdjacentHTML('beforeend', '<div class="ttl" style="margin-top:14px">Teclado</div><div class="kb"><div></div><div></div><div></div><div></div></div>');
  el.kb = [...center.querySelectorAll('.kb div')];

  const temps = panel('Temperatura');
  el.tGpu = ring('GPU');
  el.tCpu = ring('CPU');
  el.tSys = ring('Sistema');
  temps.querySelector('.body').append(el.tGpu, el.tCpu, el.tSys);
  temps.querySelector('.body').style.cssText = 'display:flex;flex-direction:column;gap:10px';

  const right = document.createElement('div');
  right.className = 'stack';
  const prof = panel('Perfil ativo');
  prof.querySelector('.body').innerHTML = '<div class="kv"><span>Modo</span><b data-k="mode">—</b></div><div class="kv"><span>Ventoinha</span><b data-k="fan">—</b></div><div class="kv"><span>Efeito</span><b data-k="fx">—</b></div><div class="kv"><span>Bateria</span><b data-k="bat">—</b></div>';
  const mon = panel('Monitor');
  mon.querySelector('.body').innerHTML = '<div class="grid3"><div class="cell">GPU<b data-k="c1">—</b></div><div class="cell">GPU<b data-k="c2">—</b></div><div class="cell">CPU<b data-k="c3">—</b></div><div class="cell">CPU<b data-k="c4">—</b></div><div class="cell">RAM<b data-k="c5">—</b></div><div class="cell">SSD<b data-k="c6">—</b></div></div>';
  right.append(prof, mon);

  g.append(gpu, center, temps, right);
  el.root = root;
}

const put = (k, v) => {
  const n = el.root.querySelector(`[data-k="${k}"]`);
  if (n) n.textContent = v;
};

export function update({ store }) {
  const s = store.snap;
  const cfg = store.config;
  const sn = s.sensors;
  const g = sn.gpu?.state === 'active' ? sn.gpu : null;
  setRing(el.gpuRing, g?.clockMhz, 2500, g ? `${Math.round(g.clockMhz ?? 0)}` : sn.gpu?.state === 'sleeping' ? 'zZ' : '—', g ? 'frequência' : sn.gpu?.state === 'sleeping' ? 'em repouso' : '');
  put('gpuUse', fmtPct(g?.usage));
  put('cpuUse', fmtPct(sn.cpuUsage));
  put('gpuW', g?.powerW != null ? `${Math.round(g.powerW)} W` : '—');
  el.cur.textContent = modeLabel(s.mode);
  setChips(el.modes, s.mode);
  setChips(el.fans, s.fanMode);
  setRing(el.tGpu, sn.gpuTemp, 100, sn.gpu?.state === 'sleeping' ? 'zZ' : fmtTemp(sn.gpuTemp));
  setRing(el.tCpu, sn.cpuTemp, 100, fmtTemp(sn.cpuTemp));
  setRing(el.tSys, sn.sysTemp, 100, fmtTemp(sn.sysTemp));
  const kbColors = cfg.keyboard.effect === 'off'
    ? ['#111', '#111', '#111', '#111']
    : cfg.keyboard.followMode && s.mode ? Array(4).fill(cfg.palette[s.mode].primary) : cfg.keyboard.zones;
  el.kb.forEach((d, i) => { d.style.background = kbColors[i]; d.style.color = kbColors[i]; });
  put('mode', modeLabel(s.mode));
  put('fan', s.fanMode === 'maximum' ? 'Máximo' : s.fanMode === 'auto' ? 'Auto' : '—');
  put('fx', FX[cfg.keyboard.effect]);
  put('bat', s.platform?.batteryLimit ? 'Limite 80%' : 'Carga total');
  put('c1', fmtPct(g?.usage));
  put('c2', fmtTemp(sn.gpuTemp));
  put('c3', fmtPct(sn.cpuUsage));
  put('c4', fmtTemp(sn.cpuTemp));
  put('c5', sn.ramUsedGb != null ? `${sn.ramUsedGb.toFixed(1)}G` : '—');
  put('c6', fmtTemp(sn.ssdTemp));
}
