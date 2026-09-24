import { MODES, isHex } from '../logic.js';
import { panel, chips, setChips } from '../widgets.js';

const el = {};
let draft = null;
let saveTimer = null;

const EFFECTS = [
  { id: 'off', label: 'Desligado' },
  { id: 'static', label: 'Estático' },
  { id: 'breathing', label: 'Respiração' },
  { id: 'neon', label: 'Neon' },
];

const DEFAULT_PALETTE = {
  eco: { primary: '#22c55e', secondary: '#86efac' },
  quiet: { primary: '#38bdf8', secondary: '#a5e3ff' },
  balanced: { primary: '#ff8a1f', secondary: '#ffc07a' },
  performance: { primary: '#ff2a1a', secondary: '#ff6a2b' },
  turbo: { primary: '#b026ff', secondary: '#ff3df2' },
};

function scheduleSave(ctx) {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    await ctx.run(() => ctx.saveConfig(structuredClone(draft)));
    saveTimer = null;
  }, 250);
}

export function mount(root, ctx) {
  root.innerHTML = '<div class="kbtab"></div>';
  const wrap = root.firstElementChild;

  const fx = panel('Efeito');
  el.fx = chips(EFFECTS, (id) => { draft.keyboard.effect = id; scheduleSave(ctx); update(ctx); });
  fx.querySelector('.body').append(el.fx);
  fx.querySelector('.body').insertAdjacentHTML('beforeend', `
    <div class="field"><span>Brilho</span><span><input type="range" min="0" max="100" data-k="brightness"> <b data-v="brightness"></b></span></div>
    <div class="field" data-row="speed"><span>Velocidade (Neon)</span><span><input type="range" min="0" max="9" data-k="speed"> <b data-v="speed"></b></span></div>
    <div class="field"><span>Acompanhar a cor do modo</span><button class="switch" data-k="followMode"></button></div>`);
  fx.querySelectorAll('input[type=range]').forEach((r) =>
    r.addEventListener('input', () => { draft.keyboard[r.dataset.k] = Number(r.value); scheduleSave(ctx); update(ctx); }));
  fx.querySelector('[data-k=followMode]').addEventListener('click', () => {
    draft.keyboard.followMode = !draft.keyboard.followMode; scheduleSave(ctx); update(ctx);
  });

  // O firmware deste modelo aceita só uma cor global para o teclado inteiro.
  const zones = panel('Cor fixa (com "Acompanhar" desligado)');
  zones.querySelector('.body').innerHTML = '<label class="zone"><input type="color" data-fixed><span>Teclado inteiro</span></label>';
  zones.querySelector('[data-fixed]').addEventListener('input', (e) => {
    draft.keyboard.zones = Array(4).fill(e.target.value);
    scheduleSave(ctx);
  });
  el.zonesPanel = zones;

  const pal = panel('Paleta dos modos (janela e teclado)');
  pal.querySelector('.body').innerHTML = MODES.map((m) =>
    `<div class="field"><span>${m.label}</span><span><input type="color" data-mode="${m.id}" data-which="primary"> <input type="color" data-mode="${m.id}" data-which="secondary"></span></div>`).join('') +
    '<div style="margin-top:10px"><button class="chip" data-reset>Restaurar paleta padrão</button></div>';
  pal.querySelectorAll('input[data-mode]').forEach((inp) =>
    inp.addEventListener('input', () => {
      if (!isHex(inp.value)) return;
      draft.palette[inp.dataset.mode][inp.dataset.which] = inp.value;
      scheduleSave(ctx);
    }));
  pal.querySelector('[data-reset]').addEventListener('click', () => {
    draft.palette = structuredClone(DEFAULT_PALETTE);
    scheduleSave(ctx);
    update(ctx);
  });

  wrap.append(fx, zones, pal);
  el.root = root;
}

export function update({ store }) {
  if (!draft || !saveTimer) draft = structuredClone(store.config);
  const k = draft.keyboard;
  setChips(el.fx, k.effect);
  for (const key of ['brightness', 'speed']) {
    const r = el.root.querySelector(`input[data-k=${key}]`);
    if (document.activeElement !== r) r.value = k[key];
    el.root.querySelector(`[data-v=${key}]`).textContent = k[key];
  }
  const speedRow = el.root.querySelector('[data-row=speed]');
  speedRow.style.opacity = k.effect === 'neon' ? 1 : 0.35;
  speedRow.style.pointerEvents = k.effect === 'neon' ? 'auto' : 'none';
  el.root.querySelector('[data-k=followMode]').classList.toggle('on', k.followMode);
  el.zonesPanel.style.opacity = k.followMode ? 0.4 : 1;
  el.zonesPanel.style.pointerEvents = k.followMode ? 'none' : 'auto';
  const fixed = el.root.querySelector('[data-fixed]');
  if (document.activeElement !== fixed) fixed.value = k.zones[0];
  el.root.querySelectorAll('input[data-mode]').forEach((inp) => {
    if (document.activeElement !== inp) inp.value = draft.palette[inp.dataset.mode][inp.dataset.which];
  });
}
