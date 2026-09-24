import { chart, setChart } from '../widgets.js';
import { FAN_MAX_RPM } from '../logic.js';

const el = {};
let samples = [];
let lastT = 0;

export function mount(root) {
  root.innerHTML = '<div class="montab"></div>';
  const w = root.firstElementChild;
  el.temp = chart('Temperatura (5 min)', [{ key: 'cpuTemp', label: 'CPU', color: 'var(--r)' }, { key: 'gpuTemp', label: 'GPU', color: '#7dd3fc' }]);
  el.use = chart('Uso', [{ key: 'cpuUsage', label: 'CPU', color: 'var(--r)' }, { key: 'gpuUsage', label: 'GPU', color: '#7dd3fc' }]);
  el.power = chart('Energia da GPU', [{ key: 'gpuPower', label: 'W', color: 'var(--r2)' }]);
  el.fans = chart('Ventoinhas (RPM)', [{ key: 'cpuFan', label: 'CPU', color: 'var(--r)' }, { key: 'gpuFan', label: 'GPU', color: '#7dd3fc' }]);
  w.append(el.temp, el.use, el.power, el.fans);
}

function draw() {
  const maxW = Math.max(100, ...samples.map((s) => s.gpuPower ?? 0));
  setChart(el.temp, samples, 20, 100, '°');
  setChart(el.use, samples, 0, 100, '%');
  setChart(el.power, samples, 0, Math.ceil(maxW / 20) * 20, ' W');
  setChart(el.fans, samples, 0, FAN_MAX_RPM, '');
}

export async function shown({ api }) {
  samples = await api.getHistory();
  lastT = Math.floor(samples.at(-1)?.t ?? 0);
  draw();
}

export function update({ store }) {
  const s = store.snap.sensors;
  const now = Math.floor(Date.now() / 1000);
  if (now === lastT) return;
  lastT = now;
  const g = s.gpu?.state === 'active' ? s.gpu : {};
  samples.push({ t: now, cpuTemp: s.cpuTemp, gpuTemp: s.gpuTemp, cpuUsage: s.cpuUsage, gpuUsage: g.usage ?? null, gpuPower: g.powerW ?? null, cpuFan: s.cpuFanRpm, gpuFan: s.gpuFanRpm });
  if (samples.length > 300) samples.shift();
  if (document.getElementById('tab-monitor').classList.contains('on')) draw();
}
