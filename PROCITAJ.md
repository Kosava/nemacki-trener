# Nemački Trener 3.0.0 — kompletan paket

Srpsko-nemački rečnik i vežbe po nivoima A1–B2 (6.164 zapisa). Rust + Slint.

## Šta je u paketu

| Folder | Šta je unutra | Za koga |
|---|---|---|
| `linux/` | Gotov program + `instaliraj.sh` | Bilo koji Linux (x86_64), odmah radi |
| `arch/` | `PKGBUILD` + arhiva izvornog koda | Arch Linux — pravi pravi pacman paket |
| `windows/` | Uputstvo za pravljenje `setup.exe` | Windows 10/11 |
| `izvorni-kod/` | Ceo projekat (kod, rečnik, skripte) | Razvoj i izmene |
| `slike/` | Slike ekrana | — |
| `CHANGELOG.md` | Istorija izmena | — |
| `LICENSE` | GPL-3.0 | — |
| `SHA256SUMS` | Kontrolni zbirovi fajlova | Provera: `sha256sum -c SHA256SUMS` |

## Najbrži put

**Linux (bez paketa, bez sudo):**

```
cd linux
./instaliraj.sh
```

Program se pojavi u meniju kao „Nemački Trener”. Uklanjanje: `./deinstaliraj.sh`.
Može i bez instalacije: `./nemacki-trener`.

**Arch Linux (pacman paket):**

```
cd arch
makepkg -si
```

Prevodi se iz izvornog koda (prvi put 5–15 minuta) i instalira kao paket `nemacki-trener`.
Uklanjanje: `sudo pacman -R nemacki-trener`.

**Windows:** vidi `windows/PROCITAJ.txt` — jedna skripta napravi `setup.exe` i prenosivi zip.

## Tvoj rečnik i napredak

| Sistem | Folder |
|---|---|
| Linux | `~/.local/share/nemacki-trener/` |
| Windows | `%APPDATA%\NemackiTrener\` |

Da preneseš napredak iz Python verzije (2.1 ili 2.2): zatvori program i prekopiraj svoj
`nemacki_recnik.db` u taj folder. Nove reči se dodaju same pri sledećem pokretanju.
Instalacija, nadogradnja i deinstalacija ne diraju ovaj folder.

Provera stanja bez prozora: `nemacki-trener --provera`.

## Šta je provereno, a šta nije

- Program za Linux je preveden i pokrenut; testovi prolaze (9 od 9).
- Koraci iz `PKGBUILD`-a (prevođenje i testovi iz arhive) su prošli, ali sam `makepkg` nije pokretan.
- Windows instalacija nije pravljena ni isprobana — skripta i Inno Setup fajl su napisani, ali ih treba
  prvi put pokrenuti na Windowsu.

Detaljno uputstvo, prečice i format lista reči: `izvorni-kod/nemacki-trener-3.0.0/README.md`.
