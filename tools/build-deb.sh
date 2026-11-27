#!/usr/bin/env bash
# Builds the Debian/Ubuntu/Mint package (.deb) of cupcraft from an already
# compiled Linux binary. Run from the repository root.
#
# Usage: tools/build-deb.sh <version> <binary> <output.deb>
#   e.g. tools/build-deb.sh 0.4.10 out/cupcraft-linux-x86_64 out/cupcraft-linux-x86_64.deb
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <version> <binary> <output.deb>" >&2
  exit 2
fi
version="$1"
bin="$2"
out="$3"

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT

install -Dm755 "$bin" "$root/usr/bin/cupcraft"
install -Dm644 tools/linux/cupcraft.desktop "$root/usr/share/applications/cupcraft.desktop"
install -Dm644 rust/crates/cupcraft-app/src/logo.png "$root/usr/share/icons/hicolor/256x256/apps/cupcraft.png"

mkdir -p "$root/DEBIAN"
size="$(du -ks "$root/usr" | cut -f1)"
cat > "$root/DEBIAN/control" <<EOF
Package: cupcraft
Version: $version
Section: video
Priority: optional
Architecture: amd64
Installed-Size: $size
Depends: libc6, libgtk-3-0 | libgtk-3-0t64, libasound2 | libasound2t64, libxkbcommon0
Recommends: ffmpeg, pkexec | policykit-1
Maintainer: cupcraft <cupcraft@users.noreply.github.com>
Homepage: https://github.com/Aitor2010aitor/cupcraft
Description: cupcraft video editor
 Video editor. The in-app updater asks for administrator permission
 (through pkexec) to replace /usr/bin/cupcraft with the new version.
 FFmpeg is needed to export.
EOF

# Refresh the icon cache and the application menu after install/remove.
cat > "$root/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q /usr/share/applications || true
  command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 "$root/DEBIAN/postinst"

dpkg-deb --build --root-owner-group "$root" "$out"
echo "Built $out"
