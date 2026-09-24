import test from 'node:test';
import assert from 'node:assert/strict';
import { modeLabel, modeDesc, fmtTemp, fmtRpm, fmtPct, fanSpinSeconds, pct, hexToRgb, mix, themeVars, isHex, sparkPath, fanArcDash, dialDash, dialPoint, nextModeId, needsHistoryRefetch, FAN_MAX_RPM } from '../ui/js/logic.js';

test('rótulos dos modos', () => {
  assert.equal(modeLabel('turbo'), 'Turbo');
  assert.equal(modeLabel('quiet'), 'Silencioso');
  assert.equal(modeLabel(null), '—');
});

test('formatação', () => {
  assert.equal(fmtTemp(57.4), '57°');
  assert.equal(fmtTemp(null), '—');
  assert.equal(fmtRpm(5674), '5674 RPM');
  assert.equal(fmtRpm(undefined), '—');
  assert.equal(fmtPct(12.6), '13%');
});

test('velocidade da hélice cresce com o RPM', () => {
  assert.equal(fanSpinSeconds(0), 0);
  assert.equal(fanSpinSeconds(null), 0);
  assert.ok(fanSpinSeconds(7000) < fanSpinSeconds(4000));
  assert.ok(fanSpinSeconds(100) <= 3);
  assert.ok(fanSpinSeconds(99999) >= 0.25);
});

test('pct é limitado a 0..100', () => {
  assert.equal(pct(FAN_MAX_RPM * 2, FAN_MAX_RPM), 100);
  assert.equal(pct(-5, 100), 0);
  assert.equal(pct(null, 100), 0);
  assert.equal(pct(50, 100), 50);
});

test('cores', () => {
  assert.deepEqual(hexToRgb('#ff8a1f'), [255, 138, 31]);
  assert.equal(mix('#ffffff', '#000000', 0.5), '#808080');
  assert.ok(isHex('#b026ff'));
  assert.ok(!isHex('b026ff'));
  const v = themeVars({ primary: '#b026ff', secondary: '#ff3df2' }, 'turbo');
  assert.equal(v['--r'], '#b026ff');
  assert.equal(v['--r2'], '#ff3df2');
  assert.equal(v['--glow'], 'rgba(176,38,255,.5)');
  assert.equal(v['--soft'], 'rgba(176,38,255,.14)');
  assert.equal(v['--bg1'], mix('#b026ff', '#000000', 0.72), 'Turbo tinge mais forte');
  assert.equal(themeVars({ primary: '#ff8a1f', secondary: '#ffc07a' }, 'balanced')['--bg1'], mix('#ff8a1f', '#000000', 0.78));
});

test('descrições dos modos', () => {
  assert.equal(modeDesc('turbo'), 'Potência máxima da CPU e GPU');
  assert.equal(modeDesc(null), '');
});

test('arco de RPM da ventoinha (270° = 259.2 de 345.6)', () => {
  assert.equal(fanArcDash(0), '0.0 345.6');
  assert.equal(fanArcDash(null), '0.0 345.6');
  assert.equal(fanArcDash(FAN_MAX_RPM), '259.2 345.6');
  assert.equal(fanArcDash(FAN_MAX_RPM * 3), '259.2 345.6');
});

test('dial: arco e posições dos nós', () => {
  assert.equal(dialDash(0), '0.0 942.5');
  assert.equal(dialDash(4), '628.3 942.5');
  const eco = dialPoint(0, 150);
  assert.equal(eco.x, '17.52%');
  assert.equal(eco.y, '68.75%');
  const top = dialPoint(2, 150);
  assert.equal(top.x, '50.00%');
  assert.equal(top.y, '12.50%');
});

test('próximo/anterior modo dá a volta', () => {
  assert.equal(nextModeId('turbo', 1), 'eco');
  assert.equal(nextModeId('eco', -1), 'turbo');
  assert.equal(nextModeId('quiet', 1), 'balanced');
  assert.equal(nextModeId(null, 1), 'quiet');
});

test('sparkPath ignora buracos', () => {
  assert.equal(sparkPath([], 100, 10, 0, 10), '');
  assert.equal(sparkPath([0, 10], 100, 10, 0, 10), 'M0.0,10.0 L100.0,0.0');
  assert.equal(sparkPath([0, null, 10], 100, 10, 0, 10), 'M0.0,10.0 M100.0,0.0');
});

test('histórico é rebuscado quando há buraco (janela ficou escondida)', () => {
  assert.equal(needsHistoryRefetch([], 100), true);
  assert.equal(needsHistoryRefetch([{ t: 99 }], 100), false);
  assert.equal(needsHistoryRefetch([{ t: 97 }], 100), false);
  assert.equal(needsHistoryRefetch([{ t: 90 }], 100), true);
});
