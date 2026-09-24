import test from 'node:test';
import assert from 'node:assert/strict';
import { modeLabel, fmtTemp, fmtRpm, fmtPct, fanSpinSeconds, pct, hexToRgb, mix, themeVars, isHex, sparkPath, FAN_MAX_RPM } from '../ui/js/logic.js';

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
  const v = themeVars({ primary: '#b026ff', secondary: '#ff3df2' });
  assert.equal(v['--r'], '#b026ff');
  assert.equal(v['--r2'], '#ff3df2');
  assert.match(v['--glow'], /^rgba\(176,38,255,/);
});

test('sparkPath ignora buracos', () => {
  assert.equal(sparkPath([], 100, 10, 0, 10), '');
  assert.equal(sparkPath([0, 10], 100, 10, 0, 10), 'M0.0,10.0 L100.0,0.0');
  assert.equal(sparkPath([0, null, 10], 100, 10, 0, 10), 'M0.0,10.0 M100.0,0.0');
});
