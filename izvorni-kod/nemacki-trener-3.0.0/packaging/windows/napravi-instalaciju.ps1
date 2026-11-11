# Pravi Windows instalaciju (setup .exe) za Nemački Trener.
#
# Potrebno (jednom):
#   1. Rust            https://rustup.rs            (pri instalaciji prihvati "Visual Studio Build Tools")
#   2. Inno Setup 6    https://jrsoftware.org/isdl.php     ili:  winget install JRSoftware.InnoSetup
#
# Pokretanje iz foldera projekta (PowerShell):
#   powershell -ExecutionPolicy Bypass -File packaging\windows\napravi-instalaciju.ps1
#
# Rezultat:  dist\NemackiTrener-<verzija>-setup.exe
#            dist\NemackiTrener-<verzija>-portable.zip   (samo .exe, bez instalacije)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Nije pronadjen 'cargo'. Instaliraj Rust sa https://rustup.rs pa ponovo otvori PowerShell."
}

$verLine = Select-String -Path "Cargo.toml" -Pattern '^version = "(.*)"' | Select-Object -First 1
$version = $verLine.Matches[0].Groups[1].Value
Write-Host "==> Nemacki Trener $version" -ForegroundColor Cyan

Write-Host "==> Prevodim (cargo build --release) - prvi put traje 5-15 minuta" -ForegroundColor Cyan
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw "Prevodjenje nije uspelo." }

New-Item -ItemType Directory -Force -Path "dist" | Out-Null
$exe = "target\release\nemacki-trener.exe"

Write-Host "==> Pravim prenosivu verziju (zip)" -ForegroundColor Cyan
$zip = "dist\NemackiTrener-$version-portable.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $exe, "README.md" -DestinationPath $zip

$iscc = @(
    (Get-Command iscc -ErrorAction SilentlyContinue).Source,
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1

if (-not $iscc) {
    Write-Warning "Inno Setup nije pronadjen - instalacija (setup.exe) nije napravljena."
    Write-Host "Instaliraj ga:  winget install JRSoftware.InnoSetup   pa pokreni skriptu ponovo."
    Write-Host "Prenosiva verzija je spremna: $zip"
    exit 0
}

Write-Host "==> Pravim instalaciju (Inno Setup)" -ForegroundColor Cyan
& $iscc "/DAppVersion=$version" "packaging\windows\nemacki-trener.iss"
if ($LASTEXITCODE -ne 0) { throw "Inno Setup nije uspeo." }

Write-Host ""
Write-Host "Gotovo:" -ForegroundColor Green
Get-ChildItem dist | ForEach-Object { Write-Host "  $($_.FullName)" }
