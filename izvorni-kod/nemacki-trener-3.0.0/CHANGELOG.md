# Istorija izmena

## 3.0.0 — novembar 2026

Program je ponovo napisan u Rustu sa Slint interfejsom (ranije Python + PyQt6).

### Novo
- Jedan izvršni fajl; rečnik je ugrađen u program, ne treba Python.
- 978 novih reči sa primerima: A1 +103, A2 +347, B1 +393, B2 +135 (ukupno 6.164 zapisa).
- Arch Linux paket (PKGBUILD) i skripta za Windows instalaciju (Inno Setup).
- `nemacki-trener --provera` — ispis stanja baze bez otvaranja prozora.
- Sopstvene liste reči u folderu `liste/` pored baze.

### Prebačeno iz verzije 2.2
- Rečnik, Fraze i rečenice (pretraga bez kvačica, unos, izmena, brisanje).
- Učenje sa kutijama (okretanje kartice ili kucanje odgovora).
- Der / Die / Das sa pravilima za rod.
- Svetla i tamna tema.
- Baza `nemacki_recnik.db` je istog formata — napredak se prenosi kopiranjem fajla.

### Još nije prebačeno (ostaje u Python verziji 2.2)
- Tab Vežbe (popuni prazninu, složi rečenicu, množina i Perfekt).
- Statistika.
- Izgovor zvukom.
- Uvoz i izvoz u CSV/JSON.

### Ispravke
- „besonders" je bilo upisano i kao pridev „poseban"; sada je „besonderer".

## 2.2 — oktobar 2026 (Python)
- Tab Vežbe: popuni prazninu, složi rečenicu, množina i Perfekt.

## 2.1 — oktobar 2026 (Python)
- Nivo B2 (1.375 zapisa); stručne reči u posebnoj grupi „Dodatno".

## 2.0 — oktobar 2026 (Python)
- Nivoi A1–B1 sa primerima, odvojeni tabovi za reči i rečenice.

## 1.2 – 1.5 (Python)
- Ponavljanje po kutijama, Der/Die/Das, kucanje odgovora, izgovor zvukom, statistika.
