#!/usr/bin/env bash
# Uklanja program koji je instalirala skripta instaliraj.sh.
# Rečnik i napredak (~/.local/share/nemacki-trener/) ostaju.
set -euo pipefail
rm -f "$HOME/.local/bin/nemacki-trener" \
      "$HOME/.local/share/applications/nemacki-trener.desktop" \
      "$HOME/.local/share/icons/hicolor/256x256/apps/nemacki-trener.png"
echo "Program je uklonjen. Podaci su ostali u ~/.local/share/nemacki-trener/"
