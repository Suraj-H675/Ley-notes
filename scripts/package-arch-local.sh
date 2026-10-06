#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

for command in makepkg ar bsdtar sha256sum; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'Required Arch packaging tool is missing: %s\n' "$command" >&2
    exit 1
  fi
done

version="$(node -p "require('./package.json').version")"
arch="$(uname -m)"
if [[ "$arch" != "x86_64" ]]; then
  printf 'Local Ley Arch packaging currently supports x86_64 only; found %s.\n' "$arch" >&2
  exit 1
fi

VITE_LEY_UPDATER_ENABLED=false npx tauri build --bundles deb

deb="$(find target/release/bundle/deb -maxdepth 1 -type f -name 'Ley_*_amd64.deb' -print -quit)"
if [[ -z "$deb" ]]; then
  echo 'Tauri did not produce the expected x86_64 DEB artifact.' >&2
  exit 1
fi

work="$root/target/release/bundle/arch-build"
out="$root/target/release/bundle/arch"
rm -rf "$work"
mkdir -p "$work" "$out"

deb_name="$(basename "$deb")"
cp "$deb" "$work/$deb_name"
deb_sha="$(sha256sum "$work/$deb_name" | awk '{print $1}')"

cat > "$work/PKGBUILD" <<EOF
# Local production-like package generated from Ley's Tauri DEB artifact.
pkgname=ley-bin
pkgver=${version}
pkgrel=1
pkgdesc='Local continuity for coding agents'
arch=('x86_64')
url='https://github.com/Suraj-H675/Ley-notes'
license=('MIT' 'Apache-2.0')
depends=('cairo' 'desktop-file-utils' 'gdk-pixbuf2' 'glib2' 'gtk3' 'hicolor-icon-theme' 'pango' 'webkit2gtk-4.1')
provides=('ley')
conflicts=('ley')
options=('!strip' '!debug')
source=('${deb_name}')
noextract=('${deb_name}')
sha256sums=('${deb_sha}')

package() {
  local extract="\${srcdir}/deb-extract"
  mkdir -p "\${extract}"
  cd "\${extract}"
  ar x "\${srcdir}/${deb_name}"
  bsdtar -xf data.tar.*
  cp -a usr "\${pkgdir}/"
  install -Dm644 "\${pkgdir}/usr/lib/Ley/generated/LICENSE-MIT" \
    "\${pkgdir}/usr/share/licenses/ley-bin/LICENSE-MIT"
  install -Dm644 "\${pkgdir}/usr/lib/Ley/generated/LICENSE-APACHE" \
    "\${pkgdir}/usr/share/licenses/ley-bin/LICENSE-APACHE"
}
EOF

(
  cd "$work"
  PKGDEST="$out" makepkg --force --clean --cleanbuild
)

package="$(find "$out" -maxdepth 1 -type f -name 'ley-bin-*.pkg.tar.zst' -print -quit)"
if [[ -z "$package" ]]; then
  echo 'makepkg completed without producing the expected Ley Arch package.' >&2
  exit 1
fi

printf 'Built native Arch package:\n%s\n' "$package"
printf 'Install it with:\nsudo pacman -U %q\n' "$package"
