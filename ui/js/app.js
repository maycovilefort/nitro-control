import { api, isDemo } from './api.js';
import { themeVars, modeLabel, fmtTemp } from './logic.js';
import { spinFans } from './widgets.js';
import * as home from './tabs/home.js';
import * as keyboard from './tabs/keyboard.js';
import * as monitor from './tabs/monitor.js';
import * as system from './tabs/system.js';

const tabs = { home, keyboard, monitor, system };
/** `hist`: últimos 300 s, compartilhado entre Início (sparklines) e Monitor. */
const store = { snap: null, config: null, hist: [] };
let lastSampleT = 0;
let switchCount = 0;

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
  for (const [k, v] of Object.entries(themeVars(colors, mode))) document.documentElement.style.setProperty(k, v);
  document.body.dataset.mode = mode;
}

function playModeAnimation(mode) {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  document.getElementById('ghost').textContent = modeLabel(mode).toUpperCase();
  switchCount += 1;
  const b = document.body.classList;
  b.remove('swA', 'swB');
  b.add(switchCount % 2 ? 'swA' : 'swB');
}

function pushSample(s) {
  const now = Math.floor(Date.now() / 1000);
  if (now === lastSampleT) return;
  lastSampleT = now;
  const sn = s.sensors;
  const g = sn.gpu?.state === 'active' ? sn.gpu : {};
  store.hist.push({ t: now, cpuTemp: sn.cpuTemp, gpuTemp: sn.gpuTemp, cpuUsage: sn.cpuUsage, gpuUsage: g.usage ?? null, gpuPower: g.powerW ?? null, cpuFan: sn.cpuFanRpm, gpuFan: sn.gpuFanRpm });
  if (store.hist.length > 300) store.hist.shift();
}

const CONN_TEXT = { connected: '', disconnected: 'ASense desconectado', busy: 'Feche a interface do ASense' };

function render() {
  const s = store.snap;
  if (!s) return;
  document.body.classList.toggle('offline', s.connection !== 'connected');
  document.getElementById('conn').textContent = (isDemo ? 'DEMO ' : '') + CONN_TEXT[s.connection];
  document.getElementById('hdr-cpu').textContent = fmtTemp(s.sensors.cpuTemp);
  document.getElementById('hdr-gpu').textContent = s.sensors.gpu?.state === 'sleeping' ? 'zZ' : fmtTemp(s.sensors.gpuTemp);
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
  document.addEventListener('visibilitychange', () => spinFans(!document.hidden));
}

async function main() {
  setupChrome();
  store.config = await api.getConfig();
  store.hist = (await api.getHistory()).slice(-300);
  lastSampleT = Math.floor(store.hist.at(-1)?.t ?? 0);
  for (const [name, t] of Object.entries(tabs)) t.mount(document.getElementById(`tab-${name}`), ctx);
  store.snap = await api.getSnapshot();
  applyTheme(store.snap.mode ?? 'balanced');
  render();
  spinFans(!document.hidden);
  api.on('snapshot', (s) => {
    store.snap = s;
    pushSample(s);
    render();
  });
  api.on('mode-changed', ({ from, to }) => {
    applyTheme(to);
    if (from) playModeAnimation(to);
  });
  api.on('toast', toast);
}

main().catch((e) => toast(`Falha ao iniciar a interface: ${e}`));
