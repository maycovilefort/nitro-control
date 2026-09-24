import { MODES } from '../logic.js';
import { panel, fan, setFan, chips, setChips } from '../widgets.js';

const el = {};
const DESC = {
  eco: 'Menor consumo, ideal na bateria',
  quiet: 'Ventoinhas baixas, uso leve',
  balanced: 'Equilíbrio entre ruído e desempenho',
  performance: 'Mais potência para jogos',
  turbo: 'Potência máxima da CPU e GPU',
};

export function mount(root, ctx) {
  root.innerHTML = '<div class="perf"></div>';
  const wrap = root.firstElementChild;
  const cards = document.createElement('div');
  cards.className = 'mode-cards';
  el.cards = {};
  for (const m of MODES) {
    const c = document.createElement('button');
    c.className = 'pn mode-card';
    c.dataset.mode = m.id;
    c.innerHTML = `<div class="swatch"></div><div class="name">${m.label}</div><div class="u">${DESC[m.id]}</div>`;
    c.addEventListener('click', () => ctx.run(() => ctx.api.setProfile(m.id)));
    cards.append(c);
    el.cards[m.id] = c;
  }
  const fans = panel('Ventoinhas');
  const row = document.createElement('div');
  row.style.cssText = 'display:flex;gap:16px';
  el.cpu = fan('CPU');
  el.gpu = fan('GPU');
  row.append(el.cpu, el.gpu);
  el.fanChips = chips([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => ctx.run(() => ctx.api.setFan(id)));
  el.fanChips.style.cssText = 'justify-content:center;margin-top:12px';
  fans.querySelector('.body').append(row, el.fanChips);
  wrap.append(cards, fans);
}

export function update({ store }) {
  const s = store.snap;
  for (const [id, c] of Object.entries(el.cards)) {
    c.classList.toggle('on', s.mode === id);
    c.querySelector('.swatch').style.background = store.config.palette[id].primary;
    c.querySelector('.swatch').style.color = store.config.palette[id].primary;
  }
  setFan(el.cpu, s.sensors.cpuFanRpm);
  setFan(el.gpu, s.sensors.gpuFanRpm);
  setChips(el.fanChips, s.fanMode);
}
