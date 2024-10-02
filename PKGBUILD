# This is an example PKGBUILD file. Use this as a start to creating your own,
# and remove these comments. For more information, see 'man PKGBUILD'.
# NOTE: Please fill out the license field for your package! If it is unknown,
# then please put 'unknown'.

# Maintainer: paypur <pieceofpaypur@gmail.com>
pkgname=mpressed
pkgver=0.1.0
pkgrel=1
epoch=
pkgdesc=""
arch=("x86_64")
url=""
license=("GPL v3")
groups=()
depends=()
makedepends=("rustup")
checkdepends=()
optdepends=()
provides=("mpressed")
conflicts=("mpressed")
replaces=()
backup=()
options=()
install=
changelog=
source=("git+https://github.com/paypur/mpressed.git")
noextract=()
sha256sums=()
validpgpkeys=()

# https://wiki.archlinux.org/title/Creating_packages#Creating_a_PKGBUILD
# https://wiki.archlinux.org/title/Rust_package_guidelines
# https://man.archlinux.org/man/PKGBUILD.5#USING_VCS_SOURCES

build() {
    cd "$pkgname"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --bin mpressed-daemon
}

package() {
    cd "$pkgname"
    install -Dm755 "target/release/mpressed-daemon" -t "$pkgdir/usr/bin"
    install -Dm644 "systemd/mpressed-daemon.service" "$pkgdir/usr/lib/systemd/user/mpressed-daemon.service"
    echo "Run 'systemctl --user enable --now mpressed-daemon.service' to enable the service"
}
