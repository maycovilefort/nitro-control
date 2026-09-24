const listeners = {};
const emit = (n, p) => (listeners[n] || []).forEach((cb) => cb(p));
const clone = (o) => JSON.parse(JSON.stringify(o));

export function demoApi() {
  let config = {
    palette: {
      eco: { primary: '#22c55e', secondary: '#86efac' },
      quiet: { primary: '#38bdf8', secondary: '#a5e3ff' },
      balanced: { primary: '#ff8a1f', secondary: '#ffc07a' },
      performance: { primary: '#ff2a1a', secondary: '#ff6a2b' },
      turbo: { primary: '#b026ff', secondary: '#ff3df2' },
    },
    keyboard: { followMode: true, effect: 'static', brightness: 100, speed: 5, zones: ['#ff2a1a', '#ff2a1a', '#ff8a1f', '#ff8a1f'], gamma: true, balance: [100, 100, 100] },
    login: { apply: true, mode: 'turbo', fan: 'auto' },
  };
  const target = { eco: 2600, quiet: 3000, balanced: 3800, performance: 4800, turbo: 5600 };
  const snap = {
    connection: 'connected', mode: 'balanced', fanMode: 'auto',
    sensors: { cpuTemp: 58, gpuTemp: 46, sysTemp: 44, ssdTemp: 36, cpuFanRpm: 3800, gpuFanRpm: 3700, cpuUsage: 18, ramUsedGb: 7.9, ramTotalGb: 14.6,
      gpu: { state: 'active', temp: 46, usage: 22, clockMhz: 1410, powerW: 38 } },
    platform: { batteryLimit: false, batteryCalibration: false, usbCharging: 30, keyboardTimeout: false, bootSound: true, lcdOverride: false },
  };
  const history = [];
  let autostart = true;
  setInterval(() => {
    const s = snap.sensors;
    const goal = snap.fanMode === 'maximum' ? 7040 : target[snap.mode];
    s.cpuFanRpm += (goal - s.cpuFanRpm) * 0.25;
    s.gpuFanRpm += (goal - 80 - s.gpuFanRpm) * 0.25;
    s.cpuTemp = 50 + Math.random() * 15;
    s.gpuTemp = s.gpu.temp = 40 + Math.random() * 12;
    s.cpuUsage = Math.random() * 40;
    s.gpu.usage = Math.random() * 60;
    s.gpu.powerW = 20 + Math.random() * 60;
    history.push({ t: Date.now() / 1000, cpuTemp: s.cpuTemp, gpuTemp: s.gpuTemp, cpuUsage: s.cpuUsage, gpuUsage: s.gpu.usage, gpuPower: s.gpu.powerW, cpuFan: s.cpuFanRpm, gpuFan: s.gpuFanRpm });
    if (history.length > 300) history.shift();
    emit('snapshot', clone(snap));
  }, 1000);
  const ok = () => new Promise((r) => setTimeout(r, 150));
  return {
    getSnapshot: async () => clone(snap),
    getHistory: async () => clone(history),
    getConfig: async () => clone(config),
    saveConfig: async (c) => { config = clone(c); await ok(); },
    setProfile: async (mode) => { await ok(); const from = snap.mode; snap.mode = mode; emit('mode-changed', { from, to: mode }); emit('snapshot', clone(snap)); },
    setFan: async (fan) => { await ok(); snap.fanMode = fan; emit('snapshot', clone(snap)); },
    setPlatform: async (key, value) => {
      await ok();
      const map = { BATTERY_LIMIT: 'batteryLimit', KEYBOARD_TIMEOUT: 'keyboardTimeout', BOOT_SOUND: 'bootSound', LCD_OVERRIDE: 'lcdOverride' };
      if (map[key]) snap.platform[map[key]] = value === 'ON';
      if (key === 'USB_CHARGING') snap.platform.usbCharging = Number(value);
      if (key === 'BATTERY_CALIBRATION') snap.platform.batteryCalibration = value === 'START';
      emit('snapshot', clone(snap));
    },
    getAutostart: async () => autostart,
    setAutostart: async (v) => { autostart = v; },
    hideWindow: async () => emit('toast', 'Demo: a janela seria escondida'),
    minimize: async () => {},
    on: (n, cb) => { (listeners[n] ||= []).push(cb); },
  };
}
