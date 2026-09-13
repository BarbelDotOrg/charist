# Maintainer: Barbel <barbel@barbel.org>
pkgname=charist
pkgver=0.3.1
pkgrel=1
pkgdesc="Intuitive Bible reader"
arch=('x86_64')
license=('AGPL-3.0-only')
depends=('gcc-libs' 'glibc' 'wayland' 'libxkbcommon')
makedepends=('cargo' 'lld')
options=('!strip' '!lto')

prepare() {
  # Copy source files to the standard makepkg srcdir if building from repository root
  cd "$startdir"
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "$startdir"
  export CARGO_TARGET_DIR="$startdir/target"
  cargo build --frozen --release
}

package() {
  cd "$startdir"
  install -Dm755 "target/release/${pkgname}" "${pkgdir}/usr/bin/${pkgname}"
  install -Dm644 "resources/org.barbel.Charist.desktop" \
    "${pkgdir}/usr/share/applications/org.barbel.Charist.desktop"
  install -Dm644 "resources/icons/hicolor/scalable/apps/org.barbel.Charist.svg" \
    "${pkgdir}/usr/share/icons/hicolor/scalable/apps/org.barbel.Charist.svg"
}