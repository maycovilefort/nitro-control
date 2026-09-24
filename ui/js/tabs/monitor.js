import { h } from '../widgets.js';
import { sparkPath, FAN_MAX_RPM, fmtTemp } from '../logic.js';

const pctFmt = (v) => (v == null ? '—' : `${Math.round(v)}%`);
const wFmt = (v) => (v == null ? '—' : `${Math.round(v)} W`);
const rpmFmt = (v) => (v == null ? '—' : String(Math.round(v)));

const CHARTS = [
  { title: 'TEMPERATURA', scale: () => '20–100°', min: 20, max: () => 100,
    series: [{ key: 'cpuTemp', label: 'CPU', color: 'var(--r)', fmt: fmtTemp }, { key: 'gpuTemp', label: 'GPU', color: '#7dd3fc', fmt: fmtTemp }] },
  { title: 'USO', scale: () => '0–100%', min: 0, max: () => 100,
    series: [{ key: 'cpuUsage', label: 'CPU', color: 'var(--r)', fmt: pctFmt }, { key: 'gpuUsage', label: 'GPU', color: '#7dd3fc', fmt: pctFmt }] },
  { title: 'ENERGIA DA GPU', scale: (pw) => `0–${pw} W`, min: 0, max: (pw) => pw,
    series: [{ key: 'gpuPower', label: 'GPU', color: 'var(--r2)', fmt: wFmt }] },
  { title: 'VENTOINHAS', scale: () => `0–${FAN_MAX_RPM} RPM`, min: 0, max: () => FAN_MAX_RPM,
    series: [{ key: 'cpuFan', label: 'CPU', color: 'var(--r)', fmt: rpmFmt }, { key: 'gpuFan', label: 'GPU', color: '#7dd3fc', fmt: rpmFmt }] },
];

const rows = [];

export function mount(root) {
  root.innerHTML = '<div class="montab enter"></div>';
  const list = root.firstElementChild;
  for (const c of CHARTS) {
    const row = h(`<div class="mrow"><div><div class="label">${c.title}</div><div class="mseries"></div></div>
      <div class="mchart"><div class="mguides"><div style="background:rgba(255,255,255,.05)"></div><div style="background:rgba(255,255,255,.03)"></div><div style="background:rgba(255,255,255,.07)"></div></div>
      <svg viewBox="0 0 300 80" preserveAspectRatio="none"></svg><div class="mscale"></div></div></div>`);
    const vals = [];
    for (const s of c.series) {
      const sv = h(`<div class="s"><span class="l" style="color:${s.color}">${s.label}</span><span class="v cond">—</span></div>`);
      row.querySelector('.mseries').append(sv);
      const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      p.style.stroke = s.color;
      row.querySelector('svg').append(p);
      vals.push({ s, v: sv.querySelector('.v'), p });
    }
    list.append(row);
    rows.push({ c, vals, scale: row.querySelector('.mscale') });
  }
  list.append(h('<div class="mfoot">ÚLTIMOS 5 MIN</div>'));
}

export function update({ store }) {
  if (!document.getElementById('tab-monitor').classList.contains('on')) return;
  const hist = store.hist;
  const last = hist.at(-1) ?? {};
  const pw = Math.ceil(Math.max(100, ...hist.map((x) => x.gpuPower ?? 0)) / 20) * 20;
  for (const { c, vals, scale } of rows) {
    scale.textContent = c.scale(pw);
    for (const { s, v, p } of vals) {
      v.textContent = s.fmt(last[s.key]);
      p.setAttribute('d', sparkPath(hist.map((x) => x[s.key] ?? null), 300, 80, c.min, c.max(pw)));
    }
  }
}

export function shown(ctx) {
  update(ctx);
}
