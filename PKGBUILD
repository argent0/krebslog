# Maintainer: Aner <argent0@github.com>
pkgname=krebslog
pkgver=0.1.0
pkgrel=1
pkgdesc="Metabolic reports, Krebs cycle visualizer & redox insights — pulls from nutlog + repslog (LLM-agent friendly)"
arch=('x86_64')
url="https://github.com/argent0/krebslog"
license=('custom')
depends=('gcc-libs')
optdepends=(
  'nutlog: nutrition and consumption data source'
  'repslog: workout and training data source'
)
provides=('krebslog')
makedepends=('git' 'rust' 'cargo')
source=("${pkgname}::git+ssh://git@github.com/argent0/krebslog.git")
sha256sums=('SKIP')

pkgver() {
  cd "$srcdir/$pkgname"
  local _ver=$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)
  echo "${_ver}.r$(git rev-list --count HEAD).$(git rev-parse --short HEAD)"
}

build() {
  cd "$srcdir/$pkgname"
  cargo build --release --locked
}

package() {
  cd "$srcdir/$pkgname"
  install -Dm755 "target/release/krebslog" "$pkgdir/usr/bin/krebslog"

  install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 "AGENTS.md" "$pkgdir/usr/share/doc/$pkgname/AGENTS.md"
  install -Dm644 "CODING_PRACTICES.md" "$pkgdir/usr/share/doc/$pkgname/CODING_PRACTICES.md"

  # Detailed documentation (data model, agent usage, command reference, etc.)
  install -d "$pkgdir/usr/share/doc/$pkgname/docs"
  install -Dm644 docs/*.md "$pkgdir/usr/share/doc/$pkgname/docs/"

  # Note: krebslog is designed to work alongside nutlog and repslog.
  # The optdepends above reflect the practical runtime relationship.
}
