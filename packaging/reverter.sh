#!/bin/bash
# Desfaz o migrar.sh a partir de um diretório de backup.
set -euo pipefail
BK="${1:?uso: reverter.sh <dir-de-backup>}"
AUTO="$HOME/.config/autostart"
KEY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/asense/

for f in asense_turbo_fans.desktop rgb_config_acer_gkbbl_0.desktop; do
  if [ -f "$BK/$f" ]; then cp "$BK/$f" "$AUTO/" && echo "restaurado: $f"; fi
done
rm -f "$AUTO/nitro-control.desktop"
CMD=$(cat "$BK/keybinding-command.txt" 2>/dev/null || true)
dconf write "${KEY}command" "${CMD:-'/usr/bin/asense --toggle'}"
dconf write "${KEY}name" "'ASense'"
pkill -x nitro-control 2>/dev/null || true
echo "revertido; a tecla NitroSense volta a abrir o ASense"
