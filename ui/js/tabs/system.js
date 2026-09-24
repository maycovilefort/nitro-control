import { MODES } from '../logic.js';
import { panel, chips, setChips } from '../widgets.js';

const el = {};
const TOGGLES = [
  { key: 'BATTERY_LIMIT', field: 'batteryLimit', label: 'Limitar carga da bateria em 80%' },
  { key: 'KEYBOARD_TIMEOUT', field: 'keyboardTimeout', label: 'Apagar a luz do teclado após inatividade' },
  { key: 'BOOT_SOUND', field: 'bootSound', label: 'Som ao ligar' },
  { key: 'LCD_OVERRIDE', field: 'lcdOverride', label: 'LCD override' },
];

export function mount(root, ctx) {
  root.innerHTML = '<div class="systab"></div>';
  const w = root.firstElementChild;

  const plat = panel('Bateria e hardware');
  const body = plat.querySelector('.body');
  for (const t of TOGGLES) {
    body.insertAdjacentHTML('beforeend', `<div class="field"><span>${t.label}</span><button class="switch needs-daemon" data-p="${t.key}"></button></div>`);
  }
  body.querySelectorAll('[data-p]').forEach((b) => b.addEventListener('click', () => {
    const on = !b.classList.contains('on');
    ctx.run(() => ctx.api.setPlatform(b.dataset.p, on ? 'ON' : 'OFF'));
  }));
  body.insertAdjacentHTML('beforeend', '<div class="field"><span>Carregar USB com o notebook desligado</span><span data-usb></span></div>');
  el.usb = chips([{ id: '0', label: 'Não' }, { id: '10', label: '10%' }, { id: '20', label: '20%' }, { id: '30', label: '30%' }],
    (id) => ctx.run(() => ctx.api.setPlatform('USB_CHARGING', id)));
  body.querySelector('[data-usb]').append(el.usb);
  body.insertAdjacentHTML('beforeend', '<div class="field"><span>Calibração da bateria</span><button class="chip needs-daemon" data-cal></button></div>');
  el.cal = body.querySelector('[data-cal]');
  el.cal.addEventListener('click', () => {
    const running = ctx.store.snap.platform?.batteryCalibration;
    ctx.run(() => ctx.api.setPlatform('BATTERY_CALIBRATION', running ? 'STOP' : 'START'));
  });

  const login = panel('Ao iniciar a sessão');
  login.classList.add('login');
  const lb = login.querySelector('.body');
  lb.innerHTML = '<div class="field"><span>Aplicar padrões no login</span><button class="switch" data-login-apply></button></div><div class="ttl" style="margin-top:10px">Modo inicial</div>';
  el.loginMode = chips(MODES, (id) => saveLogin(ctx, { mode: id }));
  lb.append(el.loginMode);
  lb.insertAdjacentHTML('beforeend', '<div class="ttl" style="margin-top:10px">Ventoinha inicial</div>');
  el.loginFan = chips([{ id: 'auto', label: 'Auto' }, { id: 'maximum', label: 'Máximo' }], (id) => saveLogin(ctx, { fan: id }));
  lb.append(el.loginFan);
  lb.insertAdjacentHTML('beforeend', '<div class="field" style="margin-top:10px"><span>Iniciar o Nitro Control com o sistema</span><button class="switch" data-autostart></button></div>');
  lb.querySelector('[data-login-apply]').addEventListener('click', () => saveLogin(ctx, { apply: !ctx.store.config.login.apply }));
  el.auto = lb.querySelector('[data-autostart]');
  el.auto.addEventListener('click', () => ctx.run(async () => {
    await ctx.api.setAutostart(!el.auto.classList.contains('on'));
    el.auto.classList.toggle('on', await ctx.api.getAutostart());
  }));
  ctx.api.getAutostart().then((v) => el.auto.classList.toggle('on', v));

  w.append(plat, login);
  el.root = root;
}

function saveLogin(ctx, patch) {
  const cfg = structuredClone(ctx.store.config);
  Object.assign(cfg.login, patch);
  ctx.run(() => ctx.saveConfig(cfg));
}

export function update({ store }) {
  const p = store.snap.platform ?? {};
  el.root.querySelectorAll('[data-p]').forEach((b) => {
    const t = TOGGLES.find((x) => x.key === b.dataset.p);
    b.classList.toggle('on', !!p[t.field]);
  });
  setChips(el.usb, p.usbCharging != null ? String(p.usbCharging) : null);
  el.cal.textContent = p.batteryCalibration ? 'Parar calibração' : 'Iniciar calibração';
  const l = store.config.login;
  el.root.querySelector('[data-login-apply]').classList.toggle('on', l.apply);
  setChips(el.loginMode, l.mode);
  setChips(el.loginFan, l.fan);
}
