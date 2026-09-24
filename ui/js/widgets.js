import { fanSpinSeconds, fmtRpm, pct, sparkPath, FAN_MAX_RPM } from './logic.js';

const h = (html) => {
  const t = document.createElement('template');
  t.innerHTML = html.trim();
  return t.content.firstElementChild;
};

export function panel(title) {
  return h(`<div class="pn"><div class="ttl">${title}</div><div class="body"></div></div>`);
}

export function ring(label, big = false) {
  return h(`<div class="ring ${big ? 'big' : 'sm'}"><div><span class="u ring-top"></span><span class="val">—</span><span class="u lbl">${label}</span></div></div>`);
}

export function setRing(el, value, max, text, top = '') {
  el.style.setProperty('--p', `${pct(value, max) * 0.75}%`);
  el.querySelector('.val').textContent = text;
  el.querySelector('.ring-top').textContent = top;
}

const BLADE = 'M50 50 C44 30 50 12 62 10 C60 26 58 40 50 50Z';
export function fan(label) {
  const blades = [0, 72, 144, 216, 288].map((a) => `<path d="${BLADE}" transform="rotate(${a} 50 50)"/>`).join('');
  return h(`<div class="fan">
    <svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" class="fan-ring"/>
      <g class="rot"><g class="blades">${blades}</g><circle cx="50" cy="50" r="9" class="hub"/></g></svg>
    <div class="ttl">${label}</div><div class="rpm">—</div><div class="bar"><div></div></div></div>`);
}

export function setFan(el, rpm) {
  const s = fanSpinSeconds(rpm);
  const rot = el.querySelector('.rot');
  rot.style.animationDuration = s ? `${s}s` : '0s';
  rot.style.animationPlayState = s ? 'running' : 'paused';
  el.querySelector('.rpm').textContent = fmtRpm(rpm);
  el.querySelector('.bar > div').style.width = `${pct(rpm, FAN_MAX_RPM)}%`;
}

export function chips(items, onPick) {
  const wrap = h('<div class="chips"></div>');
  for (const it of items) {
    const b = h(`<button class="chip" data-id="${it.id}">${it.label}</button>`);
    b.addEventListener('click', () => onPick(it.id));
    wrap.appendChild(b);
  }
  return wrap;
}

export function setChips(el, activeId) {
  el.querySelectorAll('.chip').forEach((b) => b.classList.toggle('on', b.dataset.id === activeId));
}

export function chart(title, series) {
  const legend = series.map((s) => `<span style="color:${s.color}">■ ${s.label}</span>`).join(' ');
  const paths = series.map((s) => `<path data-key="${s.key}" style="stroke:${s.color}"/>`).join('');
  return h(`<div class="pn chart"><div class="ttl">${title} <span class="legend">${legend}</span></div>
    <svg viewBox="0 0 300 80" preserveAspectRatio="none">${paths}</svg><div class="axis"><span class="max"></span><span class="min"></span></div></div>`);
}

export function setChart(el, samples, min, max, unit = '') {
  el.querySelectorAll('path').forEach((p) => {
    p.setAttribute('d', sparkPath(samples.map((s) => s[p.dataset.key] ?? null), 300, 80, min, max));
  });
  el.querySelector('.max').textContent = `escala ${min}–${max}${unit}`;
  el.querySelector('.min').textContent = 'últimos 5 min';
}
