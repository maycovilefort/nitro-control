import { api, isDemo } from './api.js';
import { themeVars, modeLabel } from './logic.js';
import * as home from './tabs/home.js';
import * as perf from './tabs/perf.js';
import * as keyboard from './tabs/keyboard.js';
import * as monitor from './tabs/monitor.js';
import * as system from './tabs/system.js';

const tabs = { home, perf, keyboard, monitor, system };
const store = { snap: null, config: null };

export function toast(msg) {
  const el = document.createElement('div');
  el.className = 'toast';
  el.textContent = msg;
  document.getElementById('toasts').appendChild(el);
  setTimeout(() => el.remove(), 5000);
}

async function run(fn) {
  try {
    await fn();
  } catch (e) {
    toast(String(e));
  }
}

function applyTheme(mode) {
  const colors = store.config?.palette?.[mode];
  if (!colors) return;
  for (const [k, v] of Object.entries(themeVars(colors))) document.documentElement.style.setProperty(k, v);
  document.body.dataset.mode = mode;
}

function playModeAnimation(mode) {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  document.getElementById('banner').textContent = modeLabel(mode).toUpperCase();
  document.body.classList.remove('sw');
  void document.body.offsetWidth;
  document.body.classList.add('sw');
}

const CONN_TEXT = { connected: '', disconnected: 'ASense desconectado', busy: 'Feche a interface do ASense' };

function render() {
  const s = store.snap;
  if (!s) return;
  document.body.classList.toggle('offline', s.connection !== 'connected');
  const conn = document.getElementById('conn');
  conn.textContent = (isDemo ? 'DEMO ' : '') + CONN_TEXT[s.connection];
  conn.classList.toggle('bad', s.connection !== 'connected');
  for (const t of Object.values(tabs)) t.update(ctx);
}

const ctx = {
  api,
  store,
  run,
  toast,
  async saveConfig(cfg) {
    await api.saveConfig(cfg);
    store.config = await api.getConfig();
    applyTheme(store.snap?.mode);
    render();
  },
};

function setupChrome() {
  document.querySelectorAll('#tabs button').forEach((b) =>
    b.addEventListener('click', () => {
      document.querySelectorAll('#tabs button').forEach((x) => x.classList.toggle('on', x === b));
      document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('on', t.id === `tab-${b.dataset.tab}`));
      tabs[b.dataset.tab].shown?.(ctx);
    }),
  );
  document.getElementById('btn-close').addEventListener('click', () => run(api.hideWindow));
  document.getElementById('btn-min').addEventListener('click', () => run(api.minimize));
}

async function main() {
  setupChrome();
  store.config = await api.getConfig();
  for (const [name, t] of Object.entries(tabs)) t.mount(document.getElementById(`tab-${name}`), ctx);
  store.snap = await api.getSnapshot();
  applyTheme(store.snap.mode ?? 'balanced');
  render();
  api.on('snapshot', (s) => {
    store.snap = s;
    render();
  });
  api.on('mode-changed', ({ from, to }) => {
    applyTheme(to);
    if (from) playModeAnimation(to);
  });
  api.on('toast', toast);
}

main().catch((e) => toast(`Falha ao iniciar a interface: ${e}`));
