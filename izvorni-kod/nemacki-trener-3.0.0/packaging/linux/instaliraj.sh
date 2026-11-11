#!/usr/bin/env bash
# Instalira gotov program za trenutnog korisnika (bez sudo):
#   program   -> ~/.local/bin/nemacki-trener
#   meni      -> ~/.local/share/applications/nemacki-trener.desktop
#   ikonica   -> ~/.local/share/icons/hicolor/256x256/apps/nemacki-trener.png
# Rečnik i napredak su u ~/.local/share/nemacki-trener/ i ova skripta ih ne dira.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"

bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor/256x256/apps"

# fajlovi su ili pored skripte (gotov paket) ili u izvornom kodu (posle cargo build --release)
find_file() {
  for f in "$@"; do [[ -f "$f" ]] && { echo "$f"; return; }; done
  echo "Nije pronađen fajl: $1" >&2
  echo "Ako instaliraš iz izvornog koda, prvo pokreni: cargo build --release" >&2
  exit 1
}
prog="$(find_file "$here/nemacki-trener" "$here/../../target/release/nemacki-trener")"
icon="$(find_file "$here/nemacki-trener.png" "$here/../nemacki-trener.png")"
desk="$(find_file "$here/nemacki-trener.desktop" "$here/../nemacki-trener.desktop")"

install -Dm755 "$prog" "$bin/nemacki-trener"
install -Dm644 "$icon" "$icons/nemacki-trener.png"
install -Dm644 "$desk" "$apps/nemacki-trener.desktop"
sed -i "s|^Exec=.*|Exec=$bin/nemacki-trener|" "$apps/nemacki-trener.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$apps" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo "Instalirano: $bin/nemacki-trener"
echo "Program je u meniju kao „Nemački Trener”."
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Napomena: $bin nije u PATH-u, pa iz terminala pokreći punom putanjom." ;;
esac
