import { MODES, isHex } from '../logic.js';
import { h, seg, setSeg, sw, setSw } from '../widgets.js';

const el = {};
let draft = null;
let saveTimer = null;

const FX = { off: 'Desligado', static: 'Estático', breathing: 'Respiração', neon: 'Neon' };
const EFFECTS = Object.entries(FX).map(([id, label]) => ({ id, label }));

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
  const changed = () => { scheduleSave(ctx); update(ctx); };
  root.innerHTML = `<div class="kbtab enter">
    <div class="kbleft">
      <div class="preview"><div class="label">PRÉVIA</div><div class="info"></div><div class="zones"><div></div><div></div><div></div><div></div></div></div>
      <div>
        <div class="palhead"><div class="label">PALETA DOS MODOS</div><button class="link" data-reset>Restaurar padrão</button></div>
        <div class="palgrid"></div>
      </div>
    </div>
    <div class="kbright">
      <div class="kbrow fxrow"><div class="label">EFEITO</div></div>
      <div class="kbrow"><div class="hd"><span>Brilho</span><span data-v="brightness"></span></div><input type="range" min="0" max="100" data-k="brightness"></div>
      <div class="kbrow" data-row="speed"><div class="hd"><span>Velocidade (Neon)</span><span data-v="speed"></span></div><input type="range" min="0" max="9" data-k="speed"></div>
      <div class="kbrow inline" data-row="follow"><span>Acompanhar a cor do modo</span></div>
      <div class="kbrow inline fixrow"><span>Cor fixa<span class="sub">Teclado inteiro, com “Acompanhar” desligado</span></span>
        <label class="fixed"><input type="color" class="swatch-in" data-fixed></label></div>
      <div class="kbrow inline" data-row="gamma"><span>Correção de cor<span class="sub">Deixa as cores do teclado parecidas com as da tela</span></span></div>
      <div class="kbrow"><div class="hd"><span>Balanço do teclado</span><span data-v="balance"></span></div>
        <div class="balance">
          <label><span style="color:#ff5a5a">R</span><input type="range" min="0" max="100" data-bal="0"></label>
          <label><span style="color:#5aff8a">G</span><input type="range" min="0" max="100" data-bal="1"></label>
          <label><span style="color:#5aa8ff">B</span><input type="range" min="0" max="100" data-bal="2"></label>
        </div></div>
    </div>
  </div>`;

  el.preview = root.querySelector('.preview');
  el.fx = seg(EFFECTS, (id) => { draft.keyboard.effect = id; changed(); }, 'grid4');
  root.querySelector('.kbrow.fxrow').append(el.fx);
  root.querySelectorAll('input[type=range]').forEach((r) =>
    r.addEventListener('input', () => { draft.keyboard[r.dataset.k] = Number(r.value); changed(); }));
  el.follow = sw((on) => { draft.keyboard.followMode = on; changed(); });
  root.querySelector('[data-row=follow]').append(el.follow);
  el.gamma = sw((on) => { draft.keyboard.gamma = on; changed(); });
  root.querySelector('[data-row=gamma]').append(el.gamma);
  root.querySelectorAll('input[data-bal]').forEach((r) =>
    r.addEventListener('input', () => { draft.keyboard.balance[Number(r.dataset.bal)] = Number(r.value); changed(); }));
  el.fixedRow = root.querySelector('.fixrow');
  el.fixed = root.querySelector('[data-fixed]');
  el.fixed.addEventListener('input', () => { draft.keyboard.zones = Array(4).fill(el.fixed.value); changed(); });

  const grid = root.querySelector('.palgrid');
  for (const m of MODES) {
    const col = h(`<div class="palcol"><div class="palblock">
        <label class="p"><input type="color" class="swatch-in" data-mode="${m.id}" data-which="primary"></label>
        <label class="s"><input type="color" class="swatch-in" data-mode="${m.id}" data-which="secondary"></label>
      </div><div class="nm">${m.label}</div></div>`);
    grid.append(col);
  }
  grid.querySelectorAll('input[data-mode]').forEach((inp) =>
    inp.addEventListener('input', () => {
      if (!isHex(inp.value)) return;
      draft.palette[inp.dataset.mode][inp.dataset.which] = inp.value;
      changed();
    }));
  root.querySelector('[data-reset]').addEventListener('click', () => { draft.palette = structuredClone(DEFAULT_PALETTE); changed(); });
  el.root = root;
}

export function update({ store }) {
  if (!draft || !saveTimer) draft = structuredClone(store.config);
  const k = draft.keyboard;
  const mode = store.snap?.mode;
  setSeg(el.fx, k.effect);
  for (const key of ['brightness', 'speed']) {
    const r = el.root.querySelector(`input[data-k=${key}]`);
    if (document.activeElement !== r) r.value = k[key];
    el.root.querySelector(`[data-v=${key}]`).textContent = k[key];
  }
  el.root.querySelector('[data-row=speed]').classList.toggle('dimmed', k.effect !== 'neon');
  setSw(el.follow, k.followMode);
  setSw(el.gamma, k.gamma);
  el.root.querySelectorAll('input[data-bal]').forEach((r) => {
    if (document.activeElement !== r) r.value = k.balance[Number(r.dataset.bal)];
  });
  el.root.querySelector('[data-v=balance]').textContent = k.balance.join(' · ');
  el.fixedRow.classList.toggle('dimmed', k.followMode);
  if (document.activeElement !== el.fixed) el.fixed.value = k.zones[0];
  el.fixed.parentElement.style.background = k.zones[0];

  const colors = k.effect === 'off' ? Array(4).fill('#15141a') : k.followMode && mode ? Array(4).fill(draft.palette[mode].primary) : k.zones;
  const zones = el.preview.querySelector('.zones');
  zones.style.opacity = 0.3 + (0.7 * k.brightness) / 100;
  [...zones.children].forEach((d, i) => { d.style.background = colors[i]; d.style.color = colors[i]; });
  el.preview.querySelector('.info').textContent = `${FX[k.effect]} · ${k.brightness}%`;

  el.root.querySelectorAll('input[data-mode]').forEach((inp) => {
    const v = draft.palette[inp.dataset.mode][inp.dataset.which];
    if (document.activeElement !== inp) inp.value = v;
    inp.parentElement.style.background = v;
  });
}
