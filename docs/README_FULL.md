# Le Compositeur

EN | DE | SR | 中文

Project status: active early-stage prototype.
Documentation note: the English section is the canonical up-to-date reference during rapid iteration.
Important: hobby project, use at your own risk ("auf eigene Gefahr"). Bugs can be reported, but fix timing is not guaranteed.

## Support / Unterstutzung / Podrska / 支持

EN: If you like the project, you can support diceteachbeograd-Team via:
- XRP: `raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5`
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`

DE: Wenn dir das Projekt gefallt, kannst du diceteachbeograd-Team unterstutzen:
- XRP: `raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5`
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`

SR: Ako ti se projekat dopada, mozes podrzati diceteachbeograd-Team:
- XRP: `raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5`
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`

中文: 如果你喜欢这个项目，也可以支持 diceteachbeograd-Team：
- XRP: `raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5`
- Litecoin: `LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt`

QR:
- XRP: [open QR](https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5)
- LTC: [open QR](https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt)

## Update 2026-03-08 (EN/DE/SR/中文)

EN:
- New tabs: `Weather` and `News`.
- `Ordering` now includes `Background/Quote/Clock/Weather/News` layer toggles and draggable placement.
- CLI renderer now writes weather/news overlay text onto generated wallpapers.
- Login autostart now uses delayed startup + warmup run for better reboot reliability.
- Note: embedded live video inside a static wallpaper is not implemented yet; current News widget renders source/headline text + link.

DE:
- Neue Tabs: `Weather` und `News`.
- `Ordering` hat nun Layer-Schalter fur `Background/Quote/Clock/Weather/News` und Drag-&-Drop-Positionierung.
- Der CLI-Renderer schreibt Wetter/News-Overlaytexte in das erzeugte Wallpaper.
- Login-Autostart nutzt jetzt Startverzogerung + Warmup-Lauf fur stabileren Reboot-Start.
- Hinweis: eingebettetes Live-Video in einem statischen Wallpaper ist noch nicht implementiert; aktuell zeigt das News-Widget Quelle/Headline + Link als Text.

SR:
- Novi tabovi: `Weather` i `News`.
- `Ordering` sada ima layer prekidace za `Background/Quote/Clock/Weather/News` i prevlacenje pozicija.
- CLI renderer sada upisuje weather/news tekstualne overlaje u generisanu pozadinu.
- Login autostart sada koristi odlozeno pokretanje + warmup run za stabilniji start posle restarta.
- Napomena: ugradjeni live video u staticku pozadinu jos nije implementiran; trenutno News widget prikazuje izvor/naslov + link kao tekst.

中文:
- 新增 `Weather` 与 `News` 选项卡。
- `Ordering` 现支持 `Background/Quote/Clock/Weather/News` 图层开关与拖拽定位。
- CLI 渲染器现在会把天气/新闻文本叠加到生成壁纸中。
- 登录自启采用延迟启动 + 预热运行，提高重启后的稳定性。
- 说明：静态壁纸内嵌实时视频尚未实现；当前 News 组件显示来源/标题与链接文本。

## English

### 1. What this project is
Le Compositeur is an open-source Rust application for Linux desktop environments.
It will generate dynamic wallpapers from:
- an image folder,
- rotating quotes from `.txt`/`.md`,
- current time overlay.

Target later stages include login-screen and boot-screen integration (optional, distro/display-manager specific).

### 2. Current implementation status (as of 2026-03-06)
Implemented:
- Rust workspace with crates:
  - `wc-cli`
  - `wc-core`
  - `wc-render`
- CLI commands:
  - `doctor`
  - `init`
  - `render-preview` (image/quote rotation + metadata sidecar)
  - `run` (`--once` for single cycle, loop mode otherwise)
- shared rotation timer (`refresh_seconds`) for both image and quote updates
- no-repeat history for local sources (history-based rotation memory)
- dynamic canvas sizing from current desktop resolution (auto-detected at runtime, fallback `1920x1080`)
- ImageMagick render path with explicit background layer + quote/clock overlay
- quote box size presets (`quarter`, `third`, `half`, `full`, `custom`) applied relative to current image dimensions
- quote font rendered at configured size (no image-size-based downscale)
- curated quote sample file: `assets/examples/quotes.md` (10 English quotes)
- starter config generation (`init`)
- baseline tests and lint/test workflow

Not implemented yet:
- actual image composition pipeline
- full wallpaper backend coverage for GNOME/KDE/Sway production edge cases
- packaging (`rpm`, `deb`)
- login/boot integration

### 3. Technology stack and versions
Core stack:
- Language: Rust (edition `2024`)
- Workspace version: `2026.03.09-2`
- License: `GPL-3.0-or-later`

Crates currently in use:
- `anyhow = 1.0`
- `clap = 4.5` (derive)
- `chrono = 0.4`

Toolchain baseline:
- Rust stable toolchain (tested with `rustc 1.93.1`)
- Cargo from rustup-managed toolchain

### 4. Repository layout
```txt
20260304 WallpaperComposer/
  Cargo.toml
  CHANGELOG.md
  README.md
  docs/
    PROJECT_PLAYBOOK.md
    ARCHITECTURE.md
    PACKAGING.md
    TEST_MATRIX.md
  crates/
    wc-backend/
    wc-cli/
    wc-core/
    wc-render/
    wc-source/
```

### 5. Installation and setup
Current stage is source-based usage.

Linux/macOS developer setup:
```bash
# Install rustup first (recommended)
# macOS example with Homebrew:
brew install rustup-init

# initialize rustup
rustup default stable
rustup component add rustfmt clippy

# clone/open project
cd "20260304 WallpaperComposer"

# quality checks
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

If `cargo` points to a Homebrew Rust install, prefer rustup-managed binaries in your `PATH`.

### 6. Build and run
```bash
cargo build
cargo run -p wc-cli -- doctor
cargo run -p wc-cli -- init
cargo run -p wc-cli -- render-preview
cargo run -p wc-cli -- run --once
cargo run -p wc-cli -- presets
cargo run -p wc-cli -- preset-catalog
cargo run -p wc-cli -- validate
cargo run -p wc-cli -- export-schema
cargo run -p wc-cli -- ui-blueprint
cargo run -p wc-cli -- migrate
```

Convenience commands:
```bash
make check
make run-doctor
make run-init
make run-preview
make run-preset-catalog
make validate
make schema
make ui-blueprint
make migrate
```

Optional (if `just` is installed):
```bash
just check
just run-doctor
just run-init
just run-preview
just run-preset-catalog
just validate
just schema
just ui-blueprint
just migrate
```

Alpha packaging helpers:
```bash
./scripts/build-alpha-rpm.sh 2026.03.09-2
./scripts/build-alpha-deb.sh 2026.03.09-2
```

### 6.1 Run on your system (release)
Versioning scheme:
- release version format: `YYYY.MM.DD-N`
- example: `2026.03.09-2`
- if multiple builds happen on the same date, increment `N` (`.2`, `.3`, ...)

You can choose either path:
- `Option A`: download prebuilt release artifacts from GitHub Releases (tag `v...` or numeric tag like `2026.03.09-2`)
- `Option B`: build packages locally from source

Fedora / RHEL (RPM):
```bash
# Option A: install prebuilt RPM
# install
sudo dnf install ./wallpaper-composer-*.rpm

# Option B: local build + install
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.09-2
sudo rpm -Uvh --replacepkgs ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.09-3-1*.rpm

# start GUI
wc-gui

# one-shot CLI run
wc-cli run --once
```

Ubuntu / Debian (DEB):
```bash
# Option A: install prebuilt DEB
# install
sudo apt install ./wallpaper-composer_*_amd64.deb

# Option B: local build + install
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.09-2
sudo apt install ./dist/le-compositeur_2026.03.09-3_amd64.deb

# start GUI
wc-gui

# one-shot CLI run
wc-cli run --once
```

Windows (ZIP artifact):
```powershell
# unpack and run
.\wallpaper-composer-windows-x86_64\bin\wc-gui.exe
```

macOS Intel / Apple Silicon (tar.gz artifact):
```bash
# unpack and run
./wallpaper-composer-macos-*/bin/wc-gui
```

### 6.1.1 GitHub Actions release notes (important)
- Current workflow artifact types: `tar.gz` (Linux/macOS) and `zip` (Windows).
- Release also ships direct binaries per platform:
  - `wc-cli-<platform>` / `wc-gui-<platform>` (Linux/macOS)
  - `wc-cli-<platform>.exe` / `wc-gui-<platform>.exe` (Windows)
- Current workflow does not yet publish native installers (`.rpm`, `.deb`, `.dmg`, `.msi/.exe installer`) automatically.
- Native Linux packages (`.rpm`, `.deb`) are created by local scripts:
  - RPM output: `~/rpmbuild/RPMS/x86_64/`
  - DEB output: `./dist/`
- GitHub locations:
  - Temporary run artifacts: `Actions -> Release Artifacts run -> Artifacts`
  - Permanent downloads: `Releases -> <tag> -> Assets`
- Known CI issue seen on 2026-03-08:
  - `Build macos-x86_64` can fail with: `The configuration 'macos-13-us-default' is not supported`
  - In that case, keep `macos-arm64` enabled and switch Intel runner label to a supported image for your GitHub plan/org.

First-run setup (all platforms):
```bash
# create starter config
wc-cli init

# init also creates local quotes at:
# ~/Documents/wallpaper-composer/quotes.md
```

### 6.2 Size and runtime profile (reference values)
Measured release binary sizes:
- `wc-cli`: about `1.3 MB`
- `wc-gui`: about `11 MB`

Expected runtime usage (typical desktop Linux):
- `wc-cli run` loop: low CPU between wake-ups, usually near idle except render/apply moments
- `wc-gui`: mostly UI-idle load; higher only during thumbnail decode and preview actions
- Memory depends on desktop/driver stack and image sizes; verify on your target VM/device

Quick local checks:
```bash
ls -lh target/release/wc-cli target/release/wc-gui
top -p "$(pgrep -d',' -f 'wc-cli|wc-gui')"
```

### 6.3 Install, update, uninstall (Linux package manager)
Fedora / RHEL:
```bash
# install
sudo dnf install ./wallpaper-composer-*.rpm

# update to a newer local RPM
sudo dnf upgrade ./wallpaper-composer-*.rpm

# uninstall
sudo dnf remove wallpaper-composer
```

Ubuntu / Debian:
```bash
# install
sudo apt install ./wallpaper-composer_*_amd64.deb

# remove
sudo apt remove wallpaper-composer
```

### 7. CLI reference
`doctor`
- prints local diagnostic information (project/profile/local time)

`init [--config <PATH>] [--force]`
- creates starter config file
- default path: `~/.config/wallpaper-composer/config.toml`
- use `--force` to overwrite

`render-preview [--config <PATH>]`
- reads config, rotates image and quote selection based on time cycle index
- writes `output_image`
  - if ImageMagick (`magick`/`convert`) is available:
    - source image is first prepared as background layer
    - quote/author/clock overlays are composited in a separate text layer step
  - otherwise: source image is copied as fallback
- writes sidecar metadata file next to output image (`<output_file>.meta.txt`) with quote and clock
- optionally applies wallpaper if enabled in config
- supports source selection via `local`, `preset`, `url` for both images and quotes
- local source mode supports no-repeat history over the last 3 picks for images and quotes (when `*_avoid_repeat = true`)
- in random mode, no-repeat uses source-count-aware history windows to reduce quick repeats

`run [--config <PATH>] [--once]`
- runs the generation cycle
- with `--once`, executes exactly one cycle and exits
- without `--once`, loops using `refresh_seconds` from config
- image timer is the master interval (`image_refresh_seconds`) and quote changes follow it

`presets`
- prints built-in public source presets (for future GUI/source settings)

`preset-catalog`
- prints structured preset catalog JSON with GUI-friendly fields:
  - `display_label`
  - `category`
  - `auth`
  - `rate_limit`

`validate [--config <PATH>]`
- validates config keys and source/backend requirements without rendering

`export-schema`
- prints a machine-readable JSON schema of config fields for GUI/tooling generation
- includes GUI contract metadata: `groups`, field `label`, `description`, and enum `options`
- includes conditional UI hints: `visible_when` and `enabled_when`
- includes `ui_widget` hints, e.g. `directory-picker`, `file-picker`, `file-save`

`ui-blueprint`
- prints a UI-oriented blueprint (sections, field ordering, and condition wiring)
- useful for quickly scaffolding forms in a future GUI app

Prototype UI:
- `prototype/settings-prototype.html` (local static preview of sections/conditions)

`migrate [--config <PATH>]`
- rewrites config into the latest normalized format
- creates an automatic backup file next to the original config

### 8. Configuration reference (starter)
Generated example:
```toml
# Le Compositeur config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "local"
image_source_url = ""
image_source_preset = "picsum_random_hd"
quote_source = "local"
quote_source_url = ""
quote_source_preset = "zenquotes_daily"
quote_format = "lines"
quote_font_size = 36
quote_pos_x = 80
quote_pos_y = 860
clock_font_size = 44
clock_pos_x = 1600
clock_pos_y = 960
rotation_use_persistent_state = true
rotation_state_file = "~/.local/state/wallpaper-composer/rotation.state"
output_image = "~/.local/state/wallpaper-composer/current.png"
refresh_seconds = 300
time_format = "%H:%M"
apply_wallpaper = false
wallpaper_backend = "auto"
```

Local quote templates:
- `assets/quotes/local/local-quotes.md` (default multilingual pack-in file)
- `assets/examples/quotes.md` (short English example)

Installed package path (RPM/DEB):
- `/usr/share/wallpaper-composer/quotes/local-quotes.md`

Local quote file format (block mode, recommended):
```txt
***
German line
English line
Serbian line
:
Author Name
***
```

Rules:
- each quote block starts and ends with `***`
- keep one quote text line per language (or any text lines you want)
- optional `:` line marks the next non-empty line as the author
- save as `.txt` or `.md` and set `quotes_path` to this file

Field meanings:
- `image_dir`: source folder for wallpaper images
- `quotes_path`: quote source file (`.txt` or `.md`)
- `image_source`: `local` today; remote/public modes are prepared for GUI phase
- `image_source`: `local`, `preset`, `url`
- `image_source_preset`: built-in public image source id
- `image_source_url`: custom public image endpoint(s), one per line or `;`-separated
- `quote_source`: `local`, `preset`, `url`
- `quote_source_preset`: built-in public quote source id
- `quote_source_url`: custom public quote endpoint
- `quote_format`: quote parsing mode (`lines` currently active)
- `quote_font_size`: quote font size (GUI-ready setting, min 8)
- `quote_pos_x`, `quote_pos_y`: quote position coordinates (GUI-ready setting)
- `clock_font_size`: clock font size (GUI-ready setting, min 8)
- `clock_pos_x`, `clock_pos_y`: clock position coordinates (GUI-ready setting)
- `rotation_use_persistent_state`: if `true`, rotation index persists across restarts
- `rotation_state_file`: state file path for persisted rotation index
- `output_image`: rendered output path
- `refresh_seconds`: regeneration interval
- `time_format`: clock format (`chrono` style)
- `apply_wallpaper`: if `true`, attempts to set desktop wallpaper
- `wallpaper_backend`: `auto`, `noop`, `gnome`, `sway`, `feh`

Quote file authoring rules are documented in:
- `docs/USER_CONTENT_FORMAT.md`

Remote source notes:
- remote fetching currently uses `curl` and local cache under XDG cache/home tmp fallback
- presets are starter integrations and may change availability based on third-party providers
- custom URL mode supports multiple endpoints (one per line or `;`-separated), cycled over time
- visual text/clock rendering currently relies on ImageMagick availability for overlay mode
- fallback render order: `imagemagick-overlay` -> `native-bmp-overlay` (for 24-bit BMP input) -> `copy-source`
- wallpaper backend execution is handled through a dedicated `wc-backend` crate
- remote source fetching/parsing is handled through a dedicated `wc-source` crate

### 9. Development workflow for contributors
1. Read `docs/PROJECT_PLAYBOOK.md` first.
2. Follow architecture/test documents in `docs/`.
3. Create focused branches (`codex/<topic>` recommended).
4. Run fmt/clippy/tests before each PR.
5. Document user-visible changes in README/CHANGELOG.

CI:
- GitHub Actions workflow: `.github/workflows/ci.yml`
- Checks: `fmt --check`, `clippy -D warnings`, `cargo test --all`

### 10. Roadmap
MVP:
- real render-preview output (image + quote + time)
- runtime loop to rotate content and refresh output
- first Linux wallpaper backend integration

Next:
- backend abstraction by desktop/compositor
- package builds for Fedora/Ubuntu ecosystems
- alpha/beta test matrix on multiple VMs

Packaging skeletons:
- RPM spec template: `packaging/rpm/wallpaper-composer.spec`
- DEB control template: `packaging/deb/control.template`
- Packaging notes: `docs/PACKAGING.md`

### 11. License and disclaimer
License: GPL-3.0-or-later.
This software is provided "as is", without warranty of any kind.

---

## Deutsch
Wichtiger Hinweis: Dieses Projekt ist ein Hobby-Projekt und Nutzung erfolgt auf eigene Gefahr. Bugs konnen gemeldet werden, aber es gibt keine garantierte Bearbeitungszeit.

### 1. Projektziel
Le Compositeur ist eine Open-Source-Anwendung in Rust fur Linux-Desktops.
Das Programm erzeugt dynamische Hintergrunde aus:
- einem Bildordner,
- rotierenden Spruchen aus `.txt`/`.md`,
- eingeblendeter Uhrzeit.

Spater optional: Login-Screen- und Boot-Screen-Integration (je nach Distribution/Display-Manager).

### 2. Aktueller Stand (2026-03-06)
Bereits umgesetzt:
- Rust-Workspace mit:
  - `wc-cli`
  - `wc-core`
  - `wc-render`
  - `wc-gui`
- CLI-Kommandos:
  - `doctor`
  - `init`
  - `render-preview`
  - `run` (`--once` oder Loop)
- Master-Timer: Bildtimer (`image_refresh_seconds`) steuert auch den Spruchwechsel
- Dynamische Canvas-Grosse pro Lauf anhand aktueller Bildschirmauflosung (Fallback `1920x1080`)
- No-repeat-Logik mit Verlaufsspeicher (insbesondere fur `random` verbessert)
- ImageMagick-Overlay mit getrenntem Hintergrund-/Text-Layer
- GUI-Hilfetexte (mehrsprachig) und erweiterte Source-Presets
- Erzeugung einer Starter-Konfiguration (`init`)
- Basis-Tests und Lint/Test-Workflow

Noch offen:
- letzte Produktionsdetails fur alle Desktop-Backends
- stabile Paket-Publish-Pipeline
- Login/Boot-Integration

### 3. Technologien und Versionen
- Sprache: Rust (Edition `2024`)
- Projektversion: `2026.03.09-2`
- Lizenz: `GPL-3.0-or-later`

Aktuell genutzte Crates:
- `anyhow = 1.0`
- `clap = 4.5` (derive)
- `chrono = 0.4`

Toolchain-Basis:
- Rust stable (getestet mit `rustc 1.93.1`)
- Cargo aus rustup-Verwaltung

### 4. Projektstruktur
```txt
20260304 WallpaperComposer/
  Cargo.toml
  CHANGELOG.md
  README.md
  docs/
  packaging/
  assets/
  scripts/
  crates/
    wc-cli/
    wc-core/
    wc-gui/
    wc-render/
```

### 5. Installation und Setup
Aktuell wird aus dem Quellcode gearbeitet.

Setup fur Linux/macOS:
```bash
brew install rustup-init
rustup default stable
rustup component add rustfmt clippy
cd "20260304 WallpaperComposer"
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Wenn `cargo` auf Homebrew-Rust zeigt, `PATH` auf rustup-Toolchain priorisieren.

### 6. Bauen und Starten
```bash
cargo build
cargo run -p wc-cli -- doctor
cargo run -p wc-cli -- init
cargo run -p wc-cli -- render-preview
cargo run -p wc-cli -- run --once
cargo run -p wc-gui
```

Alpha-Paket-Helfer:
```bash
./scripts/build-alpha-rpm.sh 2026.03.09-2
./scripts/build-alpha-deb.sh 2026.03.09-2
```

### 6.1 Auf deinem System starten (release)
Versionsschema:
- Release-Format: `YYYY.MM.DD-N`
- Beispiel: `2026.03.09-2`
- bei mehreren Builds am gleichen Tag `N` hochzahlen (`.2`, `.3`, ...)

Zwei Wege:
- `Option A`: fertige Release-Artefakte aus GitHub Releases laden (Tag `v...` oder numerisch wie `2026.03.09-2`)
- `Option B`: lokal aus Source bauen

Fedora / RHEL (RPM):
```bash
# Option A
sudo dnf install ./wallpaper-composer-*.rpm

# Option B
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.09-2
sudo rpm -Uvh --replacepkgs ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.09-3-1*.rpm

wc-gui
wc-cli run --once
```

Ubuntu / Debian (DEB):
```bash
# Option A
sudo apt install ./wallpaper-composer_*_amd64.deb

# Option B
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.09-2
sudo apt install ./dist/le-compositeur_2026.03.09-3_amd64.deb

wc-gui
wc-cli run --once
```

Windows (ZIP-Artefakt):
```powershell
.\wallpaper-composer-windows-x86_64\bin\wc-gui.exe
```

macOS Intel / Apple Silicon (tar.gz-Artefakt):
```bash
./wallpaper-composer-macos-*/bin/wc-gui
```

### 6.1.1 GitHub Actions Hinweise (wichtig)
- Aktuelle Workflow-Artefakte: `tar.gz` (Linux/macOS) und `zip` (Windows).
- Im Release liegen zusatzlich direkte Binaries pro Plattform:
  - `wc-cli-<platform>` / `wc-gui-<platform>` (Linux/macOS)
  - `wc-cli-<platform>.exe` / `wc-gui-<platform>.exe` (Windows)
- Aktuell werden keine nativen Installer (`.rpm`, `.deb`, `.dmg`, `.msi/.exe installer`) automatisch in GitHub Actions gebaut.
- Native Linux-Pakete (`.rpm`, `.deb`) kommen aus lokalen Build-Skripten:
  - RPM-Ausgabe: `~/rpmbuild/RPMS/x86_64/`
  - DEB-Ausgabe: `./dist/`
- GitHub-Ablageorte:
  - temporar: `Actions -> Release Artifacts run -> Artifacts`
  - dauerhaft: `Releases -> <tag> -> Assets`
- Bekannter CI-Fall (2026-03-08):
  - `Build macos-x86_64` kann fehlschlagen mit: `The configuration 'macos-13-us-default' is not supported`
  - Dann `macos-arm64` aktiv lassen und den Intel-Runner auf ein unterstutztes Image fur euren GitHub-Plan umstellen.

### 6.2 GUI-Schnelleinstieg (neu)
Tab-Reihenfolge:
- `Ordering`: Hauptlayout im 16:9-Rahmen (Neon-Boxen) fur `Quote`, `Clock`, `Weather`, `News`.
- `Images`: Hintergrundquelle + Wallpaper-Backend/Fit.
- `Quotes`: Spruchquelle + Reihenfolge.
- `Style`: Kontur, Unterlegung, Schatten.
- `Weather`: Widget-1-Settings (Auto-Location/System oder manuelle Location, Refresh, Live-Vorschau).
- `News`: Widget-2-Settings (freie Sender-Presets, Custom-URL, FPS, Audio-Flag).
- `System`: Laufzeit, Integrationen, Autostart.

Wichtige Bedienlogik:
- Layer lassen sich in `Ordering` einzeln ein-/ausschalten.
- Positionierung erfolgt per Drag direkt im Rahmen.
- Beim Klick auf eine Box erscheinen die passenden Einstellungen darunter.
- Hover-Hilfen sind mehrsprachig (EN/DE/SR/中文) und erklaren Zweck + empfohlene Defaults.

Autostart nach Login:
- Im `System`-Tab per Checkbox `Start automatically after login`.
- Der Eintrag startet mit Delay (`sleep 12`), fuhrt einen Warmup-Run aus und startet dann den Loop.
- Das reduziert fehlerhafte Wallpaper-Zustaende direkt nach Desktop-Login.

### 7. CLI-Referenz
`doctor`
- zeigt lokale Diagnosewerte (Projekt/Profil/Uhrzeit)

`init [--config <PFAD>] [--force]`
- legt eine Start-Konfiguration an
- Standardpfad: `~/.config/wallpaper-composer/config.toml`
- `--force` uberschreibt vorhandene Datei

`render-preview [--config <PFAD>]`
- rendert ein Ausgabe-Bild mit aktuellem Bild, Spruch und Uhrzeit
- nutzt lokale oder Remote-Quellen (`local`, `preset`, `url`)

`run [--config <PFAD>] [--once]`
- startet den Zyklus
- `--once`: genau ein Lauf
- ohne `--once`: Endlosschleife mit Timer
- Spruche folgen dem Bild-Master-Timer (`image_refresh_seconds`)

### 8. Konfigurationsreferenz
```toml
# Le Compositeur config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "local"
quote_source = "local"
image_order_mode = "sequential"
quote_order_mode = "random"
quote_avoid_repeat = true
quote_font_size = 36
quote_pos_x = 80
quote_pos_y = 860
clock_font_size = 44
clock_pos_x = 1600
clock_pos_y = 960
output_image = "/tmp/wallpaper-composer-current.png"
image_refresh_seconds = 300
quote_refresh_seconds = 300
time_format = "%H:%M"
show_weather_layer = false
show_news_layer = false
weather_pos_x = 120
weather_pos_y = 120
news_pos_x = 980
news_pos_y = 180
weather_refresh_seconds = 600
weather_use_system_location = true
weather_location_override = ""
news_source = "euronews"
news_custom_url = ""
news_fps = 1.0
news_audio_enabled = false
```

Bedeutung:
- `image_dir`: Quellordner fur Bilder
- `quotes_path`: Datei mit Spruchen (`.txt`/`.md`)
- `image_source` / `quote_source`: Quelle (`local`, `preset`, `url`)
- `image_order_mode` / `quote_order_mode`: `sequential` oder `random`
- `quote_avoid_repeat`: reduziert schnelle Wiederholungen
- `quote_font_size`, `quote_pos_x`, `quote_pos_y`: Textstil/Position
- `clock_font_size`, `clock_pos_x`, `clock_pos_y`: Uhrstil/Position
- `output_image`: Zielbild
- `image_refresh_seconds`: Master-Zeitintervall
- `quote_refresh_seconds`: wird an den Master angeglichen
- `time_format`: Uhrzeitformat (chrono-Stil)
- `show_weather_layer` / `show_news_layer`: Widget-Layer ein/aus
- `weather_*`: Position, Refresh und Location-Modus fur Wetter-Widget
- `news_*`: Quelle/URL, FPS und Audio-Flag fur News-Widget

Sicherheit/Privatsphare:
- Wetter- und News-Widget sind bei Erstinstallation standardmaßig deaktiviert.
- Bei Aktivierung nutzen beide Widgets Netzwerkzugriffe auf externe Endpunkte.

Lokale Quote-Datei (empfohlenes Block-Format):
```txt
***
Deutsche Zeile
English line
Srpska linija
:
Autorname
***
```

Hinweise:
- Standarddatei im Repo: `assets/quotes/local/local-quotes.md`
- `wc-cli init` erstellt automatisch: `~/Documents/wallpaper-composer/quotes.md`
- Installierter Paketpfad (RPM/DEB): `/usr/share/wallpaper-composer/quotes/local-quotes.md`

### 9. Beitragen/Weiterentwickeln
1. Zuerst `docs/PROJECT_PLAYBOOK.md` lesen.
2. An `docs/ARCHITECTURE.md` und `docs/TEST_MATRIX.md` orientieren.
3. Kleine fokussierte Branches (`codex/<topic>`).
4. Vor jedem PR: `fmt`, `clippy`, `test`.
5. Nutzerrelevante Anderungen in README/CHANGELOG dokumentieren.

CI:
- `.github/workflows/ci.yml` (fmt/clippy/test)
- `.github/workflows/release-alpha.yml` (Alpha-Artefakte fur Linux/Windows/macOS)

### 10. Roadmap
MVP:
- echte Preview-Ausgabe (Bild + Spruch + Uhrzeit)
- periodische Aktualisierung
- erstes Wallpaper-Backend

Danach:
- Backend-Abstraktion pro Desktop/Compositor
- stabile Paketierung fur Fedora/Ubuntu + Release-Uploads
- Alpha/Beta auf VM-Matrix

### 11. Lizenz und Haftung
Lizenz: GPL-3.0-or-later.
Bereitstellung ohne Gewahrleistung.

---

## Srpski
Vazna napomena: ovo je hobi projekat i koristi se na sopstvenu odgovornost. Bug prijave su dobrodosle, ali vreme ispravke nije garantovano.

### 1. Cilj projekta
Le Compositeur je open-source aplikacija u Rust-u za Linux desktop okruzenja.
Program pravi dinamicne pozadine iz:
- foldera sa slikama,
- rotirajucih citata iz `.txt`/`.md`,
- prikaza trenutnog vremena.

Kasnije opcionalno: integracija login ekrana i boot ekrana (zavisno od distribucije i display manager-a).

### 2. Trenutni status (2026-03-06)
Uradjeno:
- Rust workspace sa paketima:
  - `wc-cli`
  - `wc-core`
  - `wc-render`
  - `wc-gui`
- CLI komande:
  - `doctor`
  - `init`
  - `render-preview`
  - `run` (`--once` ili loop)
- Master tajmer: `image_refresh_seconds` upravlja i promenom citata
- Dinamicko odredjivanje rezolucije pri renderu (fallback `1920x1080`)
- No-repeat logika sa istorijom izbora (bolje ponasanje u `random`)
- ImageMagick overlay sa odvojenim layer-ima (pozadina/tekst)
- generisanje pocetnog config fajla (`init`)
- osnovni test/lint workflow

Nije jos uradjeno:
- finalna produkciona stabilizacija backend-a
- potpuno automatizovan publishing paketa
- login/boot integracija

### 3. Tehnologije i verzije
- Jezik: Rust (edition `2024`)
- Verzija projekta: `2026.03.09-2`
- Licenca: `GPL-3.0-or-later`

Crate-ovi:
- `anyhow = 1.0`
- `clap = 4.5` (derive)
- `chrono = 0.4`

Toolchain:
- Rust stable (testirano sa `rustc 1.93.1`)
- Cargo iz rustup toolchain-a

### 4. Struktura repozitorijuma
```txt
20260304 WallpaperComposer/
  Cargo.toml
  CHANGELOG.md
  README.md
  docs/
  packaging/
  assets/
  scripts/
  crates/
    wc-cli/
    wc-core/
    wc-gui/
    wc-render/
```

### 5. Instalacija i setup
Trenutno se projekat koristi iz source koda.

Linux/macOS setup:
```bash
brew install rustup-init
rustup default stable
rustup component add rustfmt clippy
cd "20260304 WallpaperComposer"
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Ako `cargo` pokazuje na Homebrew Rust, postavi rustup putanju pre toga u `PATH`.

### 6. Build i pokretanje
```bash
cargo build
cargo run -p wc-cli -- doctor
cargo run -p wc-cli -- init
cargo run -p wc-cli -- render-preview
cargo run -p wc-cli -- run --once
cargo run -p wc-gui
```

Alpha helper skripte:
```bash
./scripts/build-alpha-rpm.sh 2026.03.09-2
./scripts/build-alpha-deb.sh 2026.03.09-2
```

### 6.1 Pokretanje na svom sistemu (release)
Verzionisanje:
- format izdanja: `YYYY.MM.DD-N`
- primer: `2026.03.09-2`
- za vise buildova istog dana povecaj `N` (`.2`, `.3`, ...)

Imas dve opcije:
- `Option A`: prebuilt release artefakti sa GitHub Releases (tag `v...` ili numericki kao `2026.03.09-2`)
- `Option B`: lokalni build iz source koda

Fedora / RHEL (RPM):
```bash
# Option A
sudo dnf install ./wallpaper-composer-*.rpm

# Option B
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.09-2
sudo rpm -Uvh --replacepkgs ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.09-3-1*.rpm

wc-gui
wc-cli run --once
```

Ubuntu / Debian (DEB):
```bash
# Option A
sudo apt install ./wallpaper-composer_*_amd64.deb

# Option B
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.09-2
sudo apt install ./dist/le-compositeur_2026.03.09-3_amd64.deb

wc-gui
wc-cli run --once
```

Windows (ZIP):
```powershell
.\wallpaper-composer-windows-x86_64\bin\wc-gui.exe
```

macOS Intel / Apple Silicon (tar.gz):
```bash
./wallpaper-composer-macos-*/bin/wc-gui
```

### 6.1.1 GitHub Actions napomene (vazno)
- Trenutni workflow artefakti: `tar.gz` (Linux/macOS) i `zip` (Windows).
- U release-u su i direktni binarni fajlovi po platformi:
  - `wc-cli-<platform>` / `wc-gui-<platform>` (Linux/macOS)
  - `wc-cli-<platform>.exe` / `wc-gui-<platform>.exe` (Windows)
- Trenutno se ne objavljuju automatski nativni instaleri (`.rpm`, `.deb`, `.dmg`, `.msi/.exe installer`).
- Nativni Linux paketi (`.rpm`, `.deb`) nastaju lokalnim skriptama:
  - RPM izlaz: `~/rpmbuild/RPMS/x86_64/`
  - DEB izlaz: `./dist/`
- GitHub lokacije:
  - privremeno: `Actions -> Release Artifacts run -> Artifacts`
  - trajno: `Releases -> <tag> -> Assets`
- Poznat CI problem (2026-03-08):
  - `Build macos-x86_64` moze pasti sa: `The configuration 'macos-13-us-default' is not supported`
  - U tom slucaju ostavi `macos-arm64`, a Intel runner prebaci na podrzanu sliku za vas GitHub plan.

### 7. CLI referenca
`doctor`
- prikazuje dijagnostiku (projekat/profil/lokalno vreme)

`init [--config <PUTANJA>] [--force]`
- pravi pocetni config fajl
- podrazumevana putanja: `~/.config/wallpaper-composer/config.toml`
- `--force` pregazi postojeci fajl

`render-preview [--config <PUTANJA>]`
- renderuje sliku sa pozadinom, citatom i satom
- podrzava `local`, `preset`, `url` izvore

`run [--config <PUTANJA>] [--once]`
- pokrece ciklus
- `--once`: jedan prolaz
- bez `--once`: loop
- citat koristi isti master tajmer kao slike (`image_refresh_seconds`)

### 8. Config referenca
```toml
# Le Compositeur config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "local"
quote_source = "local"
image_order_mode = "sequential"
quote_order_mode = "random"
quote_avoid_repeat = true
quote_font_size = 36
quote_pos_x = 80
quote_pos_y = 860
clock_font_size = 44
clock_pos_x = 1600
clock_pos_y = 960
output_image = "/tmp/wallpaper-composer-current.png"
image_refresh_seconds = 300
quote_refresh_seconds = 300
time_format = "%H:%M"
```

Znacenje polja:
- `image_dir`: folder sa slikama
- `quotes_path`: fajl sa citatima (`.txt`/`.md`)
- `image_source` / `quote_source`: izvor (`local`, `preset`, `url`)
- `image_order_mode` / `quote_order_mode`: `sequential` ili `random`
- `quote_avoid_repeat`: smanjuje brzo ponavljanje
- `quote_font_size`, `quote_pos_x`, `quote_pos_y`: stil/pozicija citata
- `clock_font_size`, `clock_pos_x`, `clock_pos_y`: stil/pozicija sata
- `output_image`: izlazna slika
- `image_refresh_seconds`: master interval
- `quote_refresh_seconds`: uskladjuje se sa master tajmerom
- `time_format`: format vremena (`chrono`)

Lokalni format citata (preporucen block mode):
```txt
***
Nemacki red
English line
Srpski red
:
Ime autora
***
```

Napomene:
- podrazumevani fajl u repou: `assets/quotes/local/local-quotes.md`
- `wc-cli init` automatski pravi: `~/Documents/wallpaper-composer/quotes.md`
- instalirana putanja paketa (RPM/DEB): `/usr/share/wallpaper-composer/quotes/local-quotes.md`

### 9. Dalji razvoj
1. Prvo procitati `docs/PROJECT_PLAYBOOK.md`.
2. Pratiti `docs/ARCHITECTURE.md` i `docs/TEST_MATRIX.md`.
3. Raditi male tematske grane (`codex/<topic>`).
4. Pre svakog PR-a pokrenuti `fmt`, `clippy`, `test`.
5. Sve korisnicke promene upisati u README/CHANGELOG.

CI:
- `.github/workflows/ci.yml` (fmt/clippy/test)
- `.github/workflows/release-alpha.yml` (release artefakti za Linux/Windows/macOS)

### 10. Plan
MVP:
- prava preview slika (pozadina + citat + vreme)
- periodicno osvezavanje
- prvi backend za Linux pozadinu

Posle toga:
- backend apstrakcija po desktop/compositor okruzenju
- stabilno pakovanje za Fedora/Ubuntu + release upload
- alpha/beta testiranje na VM matrici

### 11. Licenca i odgovornost
Licenca: GPL-3.0-or-later.
Softver se isporucuje bez garancije.

---

## 中文
重要提示：本项目是兴趣项目，使用风险由你自行承担。欢迎提交 bug，但修复时间不作保证。

### 1. 项目目标
Le Compositeur 是一个面向 Linux 桌面环境的 Rust 开源项目。
目标是从以下内容生成动态壁纸：
- 图片目录，
- 来自 `.txt`/`.md` 的轮换语录，
- 当前时间叠加。

后续可选功能：登录界面背景与启动画面集成（取决于发行版和显示管理器）。

### 2. 当前实现状态（截至 2026-03-06）
已完成：
- Rust 工作区，包含：
  - `wc-cli`
  - `wc-core`
  - `wc-render`
  - `wc-gui`
- CLI 命令：
  - `doctor`
  - `init`
  - `render-preview`
  - `run`（`--once` 或循环）
- 主计时器：`image_refresh_seconds` 同时驱动图片与语录切换
- 每轮渲染动态读取当前屏幕分辨率（回退 `1920x1080`）
- `random` 模式下基于来源数量的历史去重，减少快速重复
- ImageMagick 渲染为分层流程（背景层 + 文本层）
- 初始配置文件生成（`init`）
- 基础测试与 lint/test 流程

尚未完成：
- 全平台生产级后端细节收敛
- 完整的打包发布自动化
- 登录/启动画面集成

### 3. 技术栈与版本
- 语言：Rust（edition `2024`）
- 项目版本：`2026.03.09-2`
- 许可证：`GPL-3.0-or-later`

当前依赖：
- `anyhow = 1.0`
- `clap = 4.5`（derive）
- `chrono = 0.4`

工具链基线：
- Rust stable（已在 `rustc 1.93.1` 验证）
- 使用 rustup 管理的 Cargo

### 4. 仓库结构
```txt
20260304 WallpaperComposer/
  Cargo.toml
  CHANGELOG.md
  README.md
  docs/
  packaging/
  assets/
  scripts/
  crates/
    wc-cli/
    wc-core/
    wc-gui/
    wc-render/
```

### 5. 安装与开发环境
当前阶段以源码方式使用。

Linux/macOS 开发环境示例：
```bash
brew install rustup-init
rustup default stable
rustup component add rustfmt clippy
cd "20260304 WallpaperComposer"
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

如果 `cargo` 指向 Homebrew Rust，请在 `PATH` 中优先使用 rustup 工具链。

### 6. 构建与运行
```bash
cargo build
cargo run -p wc-cli -- doctor
cargo run -p wc-cli -- init
cargo run -p wc-cli -- render-preview
cargo run -p wc-cli -- run --once
cargo run -p wc-gui
```

Alpha 打包脚本：
```bash
./scripts/build-alpha-rpm.sh 2026.03.09-2
./scripts/build-alpha-deb.sh 2026.03.09-2
```

### 6.1 在你的系统上启动（release）
版本规则：
- 发布格式：`YYYY.MM.DD-N`
- 示例：`2026.03.09-2`
- 同一天多次构建时递增 `N`（`.2`、`.3`）

可选两种方式：
- `Option A`：从 GitHub Releases 下载预构建产物（标签 `v...` 或数字标签如 `2026.03.09-2`）
- `Option B`：本地从源码构建

Fedora / RHEL（RPM）：
```bash
# Option A
sudo dnf install ./wallpaper-composer-*.rpm

# Option B
sudo dnf install -y rpm-build rpmdevtools rust cargo desktop-file-utils rsync
rpmdev-setuptree
./scripts/build-alpha-rpm.sh 2026.03.09-2
sudo rpm -Uvh --replacepkgs ~/rpmbuild/RPMS/x86_64/le-compositeur-2026.03.09-3-1*.rpm

wc-gui
wc-cli run --once
```

Ubuntu / Debian（DEB）：
```bash
# Option A
sudo apt install ./wallpaper-composer_*_amd64.deb

# Option B
sudo apt update
sudo apt install -y rustc cargo dpkg-dev
./scripts/build-alpha-deb.sh 2026.03.09-2
sudo apt install ./dist/le-compositeur_2026.03.09-3_amd64.deb

wc-gui
wc-cli run --once
```

Windows（ZIP）：
```powershell
.\wallpaper-composer-windows-x86_64\bin\wc-gui.exe
```

macOS Intel / Apple Silicon（tar.gz）：
```bash
./wallpaper-composer-macos-*/bin/wc-gui
```

### 6.1.1 GitHub Actions 说明（重要）
- 当前工作流产物类型：`tar.gz`（Linux/macOS）和 `zip`（Windows）。
- Release 也会附带各平台的直接二进制文件：
  - `wc-cli-<platform>` / `wc-gui-<platform>`（Linux/macOS）
  - `wc-cli-<platform>.exe` / `wc-gui-<platform>.exe`（Windows）
- 当前工作流不会自动生成原生安装包（`.rpm`、`.deb`、`.dmg`、`.msi/.exe installer`）。
- Linux 原生包（`.rpm`、`.deb`）需要本地脚本构建：
  - RPM 输出目录：`~/rpmbuild/RPMS/x86_64/`
  - DEB 输出目录：`./dist/`
- GitHub 下载位置：
  - 临时产物：`Actions -> Release Artifacts run -> Artifacts`
  - 长期下载：`Releases -> <tag> -> Assets`
- 已知 CI 问题（2026-03-08）：
  - `Build macos-x86_64` 可能报错：`The configuration 'macos-13-us-default' is not supported`
  - 这种情况下保留 `macos-arm64`，并将 Intel runner 改为你们 GitHub 计划支持的 macOS 镜像。

### 7. CLI 说明
`doctor`
- 输出本地诊断信息（项目/配置档位/本地时间）

`init [--config <路径>] [--force]`
- 创建初始配置文件
- 默认路径：`~/.config/wallpaper-composer/config.toml`
- 使用 `--force` 可覆盖已有文件

`render-preview [--config <路径>]`
- 渲染输出图像（背景 + 语录 + 时钟）
- 支持来源：`local`、`preset`、`url`

`run [--config <路径>] [--once]`
- 启动循环
- `--once` 执行一次
- 不加 `--once` 持续运行
- 语录切换跟随图片主计时器（`image_refresh_seconds`）

### 8. 配置说明
```toml
# Le Compositeur config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "local"
quote_source = "local"
image_order_mode = "sequential"
quote_order_mode = "random"
quote_avoid_repeat = true
quote_font_size = 36
quote_pos_x = 80
quote_pos_y = 860
clock_font_size = 44
clock_pos_x = 1600
clock_pos_y = 960
output_image = "/tmp/wallpaper-composer-current.png"
image_refresh_seconds = 300
quote_refresh_seconds = 300
time_format = "%H:%M"
```

字段含义：
- `image_dir`：壁纸图片目录
- `quotes_path`：语录来源文件（`.txt`/`.md`）
- `image_source` / `quote_source`：来源（`local`/`preset`/`url`）
- `image_order_mode` / `quote_order_mode`：顺序或随机
- `quote_avoid_repeat`：降低快速重复
- `quote_font_size`, `quote_pos_x`, `quote_pos_y`：语录样式/位置
- `clock_font_size`, `clock_pos_x`, `clock_pos_y`：时钟样式/位置
- `output_image`：渲染输出路径
- `image_refresh_seconds`：主刷新间隔
- `quote_refresh_seconds`：会与主计时器保持一致
- `time_format`：时间格式（`chrono` 格式）

本地语录文件格式（推荐区块模式）：
```txt
***
German line
English line
Serbian line
:
Author Name
***
```

说明：
- 仓库内默认文件：`assets/quotes/local/local-quotes.md`
- `wc-cli init` 会自动创建：`~/Documents/wallpaper-composer/quotes.md`
- RPM/DEB 安装后的路径：`/usr/share/wallpaper-composer/quotes/local-quotes.md`

### 9. 协作开发流程
1. 先阅读 `docs/PROJECT_PLAYBOOK.md`。
2. 参考 `docs/ARCHITECTURE.md` 与 `docs/TEST_MATRIX.md`。
3. 使用小而聚焦的分支（建议 `codex/<topic>`）。
4. 每次 PR 前运行 `fmt`、`clippy`、`test`。
5. 所有用户可见变更同步更新 README/CHANGELOG。

CI：
- `.github/workflows/ci.yml`（fmt/clippy/test）
- `.github/workflows/release-alpha.yml`（Linux/Windows/macOS release 构建产物）

### 10. 路线图
MVP：
- 真实预览输出（背景图 + 语录 + 时间）
- 周期性刷新
- 首个 Linux 壁纸后端集成

下一阶段：
- 按桌面/合成器抽象后端
- Fedora/Ubuntu 稳定打包与发布
- 多 VM 的 alpha/beta 测试矩阵

### 11. 许可证与免责声明
许可证：GPL-3.0-or-later。
软件按“现状”提供，不附带任何担保。
