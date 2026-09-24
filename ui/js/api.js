const T = window.__TAURI__;
export const isDemo = !T || new URLSearchParams(location.search).has('demo');

function tauriApi() {
  const inv = (cmd, args) => T.core.invoke(cmd, args);
  return {
    getSnapshot: () => inv('get_snapshot'),
    getHistory: () => inv('get_history'),
    getConfig: () => inv('get_config'),
    saveConfig: (config) => inv('save_config', { config }),
    setProfile: (mode) => inv('set_profile', { mode }),
    setFan: (fan) => inv('set_fan', { fan }),
    setPlatform: (key, value) => inv('set_platform', { key, value }),
    getAutostart: () => inv('get_autostart'),
    setAutostart: (enabled) => inv('set_autostart', { enabled }),
    hideWindow: () => inv('hide_window'),
    minimize: () => T.window.getCurrentWindow().minimize(),
    on: (name, cb) => T.event.listen(name, (e) => cb(e.payload)),
  };
}

export const api = isDemo ? (await import('./demo.js')).demoApi() : tauriApi();
