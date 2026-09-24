import { fanArcDash, fanSpinSeconds, sparkPath } from './logic.js';

export const h = (html) => {
  const t = document.createElement('template');
  t.innerHTML = html.trim();
  return t.content.firstElementChild;
};

/** Botões segmentados; `cls` acrescenta variações (sm, xs, grid4, needs-daemon). */
export function seg(items, onPick, cls = '') {
  const el = h(`<div class="seg ${cls}"></div>`);
  for (const it of items) {
    const b = h(`<button data-id="${it.id}">${it.label}</button>`);
    b.addEventListener('click', () => onPick(it.id));
    el.append(b);
  }
  return el;
}

export function setSeg(el, activeId) {
  el.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b.dataset.id === activeId));
}

export function sw(onToggle, cls = '') {
  const b = h(`<button class="switch ${cls}"></button>`);
  b.addEventListener('click', () => onToggle(!b.classList.contains('on')));
  return b;
}

export const setSw = (el, on) => el.classList.toggle('on', !!on);

const BLADE = 'M57.1 46.3 C58 36 70 26 82 21.9 Q88 25 92.7 30.6 C82 36 72 42 62.9 46.3 Z';
const blades = (extra) =>
  Array.from({ length: 11 }, (_, i) => `<path d="${BLADE}" transform="rotate(${((i * 360) / 11).toFixed(2)} 60 60)" ${extra}/>`).join('');

/** Ventoinha HUD: 11 pás com rastro, cubo metálico e arco de RPM. O rotor é girado por `spinFans`. */
export function fan(key) {
  const el = h(`<div class="fanbox">
    <div class="corner tl"></div><div class="corner tr"></div><div class="corner bl"></div><div class="corner br"></div>
    <svg viewBox="0 0 120 120">
      <defs>
        <radialGradient id="glow${key}" cx="50%" cy="50%" r="50%">
          <stop offset="0%" class="f-stop" stop-opacity=".34"/><stop offset="70%" class="f-stop" stop-opacity=".06"/><stop offset="100%" class="f-stop" stop-opacity="0"/>
        </radialGradient>
        <radialGradient id="blade${key}" gradientUnits="userSpaceOnUse" cx="60" cy="60" r="46">
          <stop offset="30%" class="f-stop"/><stop offset="100%" class="f-stop2"/>
        </radialGradient>
        <radialGradient id="hub${key}" cx="38%" cy="34%" r="72%">
          <stop offset="0%" stop-color="#4a4753"/><stop offset="60%" stop-color="#16151b"/><stop offset="100%" stop-color="#08070a"/>
        </radialGradient>
      </defs>
      <circle cx="60" cy="60" r="59" fill="none" stroke="rgba(255,255,255,.05)" stroke-width="1"/>
      <circle cx="60" cy="60" r="55" transform="rotate(135 60 60)" fill="none" stroke="rgba(255,255,255,.07)" stroke-width="2.5" stroke-dasharray="259.2 345.6"/>
      <circle class="f-arc" cx="60" cy="60" r="55" transform="rotate(135 60 60)" style="stroke-dasharray:0 345.6"/>
      <circle cx="60" cy="60" r="50" fill="#08070a" stroke="rgba(255,255,255,.09)" stroke-width="1"/>
      <circle cx="60" cy="60" r="47" fill="none" stroke="rgba(255,255,255,.07)" stroke-width="2.5" stroke-dasharray="1 3.1"/>
      <circle cx="60" cy="60" r="45" fill="url(#glow${key})"/>
      <g class="rotor">
        <g transform="rotate(-18 60 60)" style="opacity:.25;filter:blur(1.4px)">${blades(`fill="url(#blade${key})"`)}</g>
        <g>${blades(`fill="url(#blade${key})" stroke="#08070a" stroke-width=".6"`)}</g>
        <circle class="f-hub" cx="60" cy="60" r="15" fill="url(#hub${key})"/>
        <circle cx="60" cy="60" r="10" fill="none" stroke="rgba(255,255,255,.16)" stroke-width="1" stroke-dasharray="2 2.2"/>
      </g>
      <circle class="f-axle" cx="60" cy="60" r="3"/>
    </svg>
  </div>`);
  el._rotor = el.querySelector('.rotor');
  el._arc = el.querySelector('.f-arc');
  el._angle = 0;
  el._rpm = null;
  FANS.add(el);
  return el;
}

export function setFan(el, rpm) {
  el._rpm = rpm;
  el._arc.style.strokeDasharray = fanArcDash(rpm);
}

const FANS = new Set();
let lastT = null;
let rafId = null;

/**
 * Gira os rotores quadro a quadro: a velocidade acompanha o RPM sem os saltos
 * que trocar `animation-duration` causaria.
 */
export function spinFans(running) {
  if (!running || matchMedia('(prefers-reduced-motion: reduce)').matches) {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = null;
    lastT = null;
    return;
  }
  if (rafId) return;
  const loop = (t) => {
    const dt = lastT == null ? 0 : Math.min(0.1, (t - lastT) / 1000);
    lastT = t;
    for (const el of FANS) {
      const s = fanSpinSeconds(el._rpm);
      if (s) el._angle = (el._angle + (dt * 360) / s) % 360;
      el._rotor.setAttribute('transform', `rotate(${el._angle.toFixed(2)} 60 60)`);
    }
    rafId = requestAnimationFrame(loop);
  };
  rafId = requestAnimationFrame(loop);
}

/** Linha de estatística com barra: retorna o elemento; atualize com `setStat`. */
export function stat(label) {
  return h(`<div class="stat"><div class="kv"><span>${label}</span><b>—</b></div><div class="bar"><div></div></div></div>`);
}

export function setStat(el, text, pctValue) {
  el.querySelector('b').textContent = text;
  el.querySelector('.bar > div').style.width = `${pctValue}%`;
}

/** Área + linha de uma série (viewBox 0 0 w hgt). */
export function setSpark(areaEl, lineEl, values, w, hgt, min, max) {
  const d = sparkPath(values, w, hgt, min, max);
  lineEl.setAttribute('d', d);
  areaEl.setAttribute('d', d ? `${d} L${w},${hgt} L0,${hgt} Z` : '');
}
