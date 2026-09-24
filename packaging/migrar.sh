#!/bin/bash
# Migra do ASense GUI para o Nitro Control. Guarda tudo em ~/nitro-control/packaging/backup-<data>.
set -euo pipefail
BK="$HOME/nitro-control/packaging/backup-$(date +%Y%m%d-%H%M%S)"
AUTO="$HOME/.config/autostart"
KEY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/asense/
mkdir -p "$BK"

for f in asense_turbo_fans.desktop rgb_config_acer_gkbbl_0.desktop; do
  if [ -f "$AUTO/$f" ]; then mv "$AUTO/$f" "$BK/"; echo "autostart removido: $f"; fi
done

dconf read "${KEY}command" > "$BK/keybinding-command.txt" || true
dconf write "${KEY}command" "'/usr/bin/nitro-control --toggle'"
dconf write "${KEY}name" "'Nitro Control'"
echo "tecla NitroSense (XF86Launch1) -> nitro-control --toggle"

pkill -x asense 2>/dev/null || true

mkdir -p "$AUTO"
cat > "$AUTO/nitro-control.desktop" <<'DESK'
[Desktop Entry]
Type=Application
Name=Nitro Control
Comment=Painel de controle do Acer Nitro
Exec=nitro-control --hidden
Icon=nitro-control
Terminal=false
X-GNOME-Autostart-enabled=true
DESK
echo "autostart do Nitro Control instalado"
echo "backup em $BK (reverter: packaging/reverter.sh $BK)"
