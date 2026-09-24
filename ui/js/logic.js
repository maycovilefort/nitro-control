export const MODES = [
  { id: 'eco', label: 'Eco' },
  { id: 'quiet', label: 'Silencioso' },
  { id: 'balanced', label: 'Equilibrado' },
  { id: 'performance', label: 'Desempenho' },
  { id: 'turbo', label: 'Turbo' },
];
export const FAN_MAX_RPM = 7050;

export const modeLabel = (id) => MODES.find((m) => m.id === id)?.label ?? '—';

const DESC = {
  eco: 'Menor consumo, ideal na bateria',
  quiet: 'Ventoinhas baixas, uso leve',
  balanced: 'Equilíbrio entre ruído e desempenho',
  performance: 'Mais potência para jogos',
  turbo: 'Potência máxima da CPU e GPU',
};
export const modeDesc = (id) => DESC[id] ?? '';

/** Modo seguinte (dir=1) ou anterior (dir=-1), dando a volta. */
export function nextModeId(id, dir) {
  const i = Math.max(0, MODES.findIndex((m) => m.id === id));
  return MODES[(i + dir + MODES.length) % MODES.length].id;
}

/** Arco de RPM da ventoinha: 270° de um círculo r=55 (259.2 de 345.6). */
export const fanArcDash = (rpm) => `${((259.2 * pct(rpm, FAN_MAX_RPM)) / 100).toFixed(1)} 345.6`;

/** Arco do dial até o modo idx (0..4): 240° de um círculo r=150. */
export const dialDash = (idx) => `${((628.3 * idx) / 4).toFixed(1)} 942.5`;

/** Posição (em % da caixa 400×400) do nó i do dial no raio r. */
export function dialPoint(i, r) {
  const a = ((150 + i * 60) * Math.PI) / 180;
  return { x: `${((200 + r * Math.cos(a)) / 4).toFixed(2)}%`, y: `${((200 + r * Math.sin(a)) / 4).toFixed(2)}%` };
}
export const fmtTemp = (v) => (v == null ? '—' : `${Math.round(v)}°`);
export const fmtRpm = (v) => (v == null ? '—' : `${Math.round(v)} RPM`);
export const fmtPct = (v) => (v == null ? '—' : `${Math.round(v)}%`);

/** Segundos por volta da hélice animada: mais RPM, volta mais curta. */
export function fanSpinSeconds(rpm) {
  if (!rpm || rpm <= 0) return 0;
  return Math.max(0.25, Math.min(3, 2400 / rpm));
}

export function pct(v, max) {
  if (v == null || !max) return 0;
  return Math.max(0, Math.min(100, (v / max) * 100));
}

export function hexToRgb(h) {
  const n = parseInt(h.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function mix(a, b, t) {
  const A = hexToRgb(a);
  const B = hexToRgb(b);
  return '#' + A.map((x, i) => Math.round(x + (B[i] - x) * t).toString(16).padStart(2, '0')).join('');
}

export const isHex = (s) => /^#[0-9a-fA-F]{6}$/.test(s ?? '');

export function themeVars({ primary, secondary }, mode) {
  const [r, g, b] = hexToRgb(primary);
  return {
    '--r': primary,
    '--r2': secondary,
    '--glow': `rgba(${r},${g},${b},.5)`,
    '--soft': `rgba(${r},${g},${b},.14)`,
    '--bg1': mix(primary, '#000000', mode === 'turbo' ? 0.72 : 0.78),
  };
}

/** Caminho SVG de uma série; `null` quebra a linha (novo "M"). */
export function sparkPath(values, w, h, min, max) {
  if (!values.length) return '';
  const step = values.length > 1 ? w / (values.length - 1) : 0;
  const span = max - min || 1;
  let d = '';
  let pen = false;
  values.forEach((v, i) => {
    if (v == null) {
      pen = false;
      return;
    }
    const x = (i * step).toFixed(1);
    const y = (h - ((Math.max(min, Math.min(max, v)) - min) / span) * h).toFixed(1);
    d += `${d ? ' ' : ''}${pen ? 'L' : 'M'}${x},${y}`;
    pen = true;
  });
  return d;
}
