# Maintainer: Barbel <barbel@barbel.org>
pkgname=charist
pkgver=0.3.0
pkgrel=1
pkgdesc="Intuitive Bible reader"
arch=('x86_64')
license=('AGPL-3.0-only')
depends=('gcc-libs' 'glibc' 'wayland' 'libxkbcommon')
options=('!strip')

package() {
  install -Dm755 "${startdir}/target/release/${pkgname}" "${pkgdir}/usr/bin/${pkgname}"

  install -Dm644 "${startdir}/resources/org.barbel.Charist.desktop" \
    "${pkgdir}/usr/share/applications/org.barbel.Charist.desktop"

  install -Dm644 "${startdir}/resources/icons/hicolor/scalable/apps/org.barbel.Charist.svg" \
    "${pkgdir}/usr/share/icons/hicolor/scalable/apps/org.barbel.Charist.svg"
}