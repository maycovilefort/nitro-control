import { MODES } from '../logic.js';
import { h, seg, setSeg, sw, setSw } from '../widgets.js';

const el = {};
const TOGGLES = [
  { key: 'BATTERY_LIMIT', field: 'batteryLimit', label: 'Limitar carga da bateria em 80%' },
  { key: 'KEYBOARD_TIMEOUT', field: 'keyboardTimeout', label: 'Apagar a luz do teclado após inatividade' },
  { key: 'BOOT_SOUND', field: 'bootSound', label: 'Som ao ligar' },
  { key: 'LCD_OVERRIDE', field: 'lcdOverride', label: 'LCD override' },
];
const FANS = [{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }];

const row = (label, cls = '') => h(`<div class="srow ${cls}"><span>${label}</span></div>`);

function saveLogin(ctx, patch) {
  const cfg = structuredClone(ctx.store.config);
  Object.assign(cfg.login, patch);
  ctx.run(() => ctx.saveConfig(cfg));
}

export function mount(root, ctx) {
  root.innerHTML = '<div class="systab enter"><div class="sect"><div class="label">BATERIA E HARDWARE</div></div><div class="sect"><div class="label">AO INICIAR A SESSÃO</div></div></div>';
  const [plat, login] = root.querySelectorAll('.sect');

  el.toggles = TOGGLES.map((t) => {
    const r = row(t.label);
    const s = sw((on) => ctx.run(() => ctx.api.setPlatform(t.key, on ? 'ON' : 'OFF')), 'needs-daemon');
    r.append(s);
    plat.append(r);
    return { t, s };
  });
  const usb = row('Carregar USB com o notebook desligado', 'segrow');
  el.usb = seg([{ id: '0', label: 'Não' }, { id: '10', label: '10%' }, { id: '20', label: '20%' }, { id: '30', label: '30%' }],
    (id) => ctx.run(() => ctx.api.setPlatform('USB_CHARGING', id)), 'sm needs-daemon');
  usb.append(el.usb);
  const cal = row('Calibração da bateria', 'segrow');
  el.cal = h('<button class="outline needs-daemon"></button>');
  el.cal.addEventListener('click', () => {
    const running = ctx.store.snap.platform?.batteryCalibration;
    ctx.run(() => ctx.api.setPlatform('BATTERY_CALIBRATION', running ? 'STOP' : 'START'));
  });
  cal.append(el.cal);
  plat.append(usb, cal);

  const apply = row('Aplicar padrões no login');
  el.apply = sw((on) => saveLogin(ctx, { apply: on }));
  apply.append(el.apply);
  const lm = row('Modo inicial', 'segrow');
  el.loginMode = seg(MODES, (id) => saveLogin(ctx, { mode: id }), 'sm xs');
  lm.append(el.loginMode);
  const lf = row('Ventoinha inicial', 'segrow');
  el.loginFan = seg(FANS, (id) => saveLogin(ctx, { fan: id }), 'sm');
  lf.append(el.loginFan);
  const auto = row('Iniciar o Nitro Control com o sistema');
  el.auto = sw((on) => ctx.run(async () => {
    await ctx.api.setAutostart(on);
    setSw(el.auto, await ctx.api.getAutostart());
  }));
  auto.append(el.auto);
  login.append(apply, lm, lf, auto);
  ctx.api.getAutostart().then((v) => setSw(el.auto, v));
}

export function update({ store }) {
  const p = store.snap.platform ?? {};
  for (const { t, s } of el.toggles) setSw(s, p[t.field]);
  setSeg(el.usb, p.usbCharging != null ? String(p.usbCharging) : null);
  el.cal.textContent = p.batteryCalibration ? 'Parar calibração' : 'Iniciar calibração';
  const l = store.config.login;
  setSw(el.apply, l.apply);
  setSeg(el.loginMode, l.mode);
  setSeg(el.loginFan, l.fan);
}
