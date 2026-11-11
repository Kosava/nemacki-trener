#!/usr/bin/env bash
# Pravi Arch paket iz ovog izvornog koda.
#   ./napravi-paket.sh        -> nemacki-trener-<verzija>-1-x86_64.pkg.tar.zst
#   ./napravi-paket.sh -i     -> napravi i odmah instaliraj (makepkg -si)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
ver="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
name="nemacki-trener-$ver"

# verzija u PKGBUILD-u prati Cargo.toml
sed -i "s/^pkgver=.*/pkgver=$ver/" "$here/PKGBUILD"

echo "==> Pakujem izvorni kod ($name.tar.gz)"
tar --exclude='./target' --exclude='./dist' --exclude='./.git' \
    --exclude='./packaging/arch/pkg' --exclude='./packaging/arch/src' \
    --exclude='*.pkg.tar.zst' --exclude="$name.tar.gz" \
    --transform "s,^\.,$name," -czf "$here/$name.tar.gz" -C "$root" .

cd "$here"
if [[ "${1:-}" == "-i" ]]; then
  makepkg -si --noconfirm
else
  makepkg -s
  echo
  echo "Gotovo. Instalacija:  sudo pacman -U $here/$name-1-$(uname -m).pkg.tar.zst"
fi
