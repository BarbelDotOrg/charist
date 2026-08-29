# Maintainer: Barbel <barbel@barbel.org>
pkgname=charist
pkgver=0.3.0
pkgrel=1
pkgdesc="Intuitive Bible reader"
arch=('x86_64')
license=('AGPL-3.0-only')
depends=('gcc-libs' 'glibc' 'wayland' 'libxkbcommon')
makedepends=('cargo')
options=('!strip' '!lto')

prepare() {
  cd "${startdir}"
  echo "== DEBUG =="
  echo "pwd: $(pwd)"
  rustc --version
  cargo --version
  rustup --version 2>&1 || echo "rustup not installed"
  whoami
  git -C "${startdir}" status 2>&1 || echo "git status failed"
  git -C "${startdir}" rev-parse --show-toplevel 2>&1 || echo "rev-parse failed"
  ls -la "${startdir}/src" 2>&1 || echo "src missing"
  echo "== END DEBUG =="
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "${startdir}"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release
}

package() {
  if [ -f "${startdir}/target/x86_64-unknown-linux-gnu/release/${pkgname}" ]; then
    BIN_PATH="${startdir}/target/x86_64-unknown-linux-gnu/release/${pkgname}"
  else
    BIN_PATH="${startdir}/target/release/${pkgname}"
  fi
  install -Dm755 "${BIN_PATH}" "${pkgdir}/usr/bin/${pkgname}"
  install -Dm644 "${startdir}/resources/org.barbel.Charist.desktop" \
    "${pkgdir}/usr/share/applications/org.barbel.Charist.desktop"
  install -Dm644 "${startdir}/resources/icons/hicolor/scalable/apps/org.barbel.Charist.svg" \
    "${pkgdir}/usr/share/icons/hicolor/scalable/apps/org.barbel.Charist.svg"
}