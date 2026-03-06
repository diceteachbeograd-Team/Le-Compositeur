# Wallpaper Composer

EN | DE | SR | 中文

Project status: active early-stage prototype.
Documentation note: the English section is the canonical up-to-date reference during rapid iteration.

## English

### 1. What this project is
Wallpaper Composer is an open-source Rust application for Linux desktop environments.
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
- no-repeat history for local sources (last 3 image/quote picks)
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
- Workspace version: `0.1.0`
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

`run [--config <PATH>] [--once]`
- runs the generation cycle
- with `--once`, executes exactly one cycle and exits
- without `--once`, loops using `refresh_seconds` from config
- image and quote changes use the same cycle timer (`refresh_seconds`)

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
# Wallpaper Composer config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "local"
image_source_url = ""
image_source_preset = "nasa_apod"
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
output_image = "/tmp/wallpaper-composer-current.png"
refresh_seconds = 300
time_format = "%H:%M"
apply_wallpaper = false
wallpaper_backend = "auto"
```

Local quote template:
- `assets/examples/quotes.md`

Field meanings:
- `image_dir`: source folder for wallpaper images
- `quotes_path`: quote source file (`.txt` or `.md`)
- `image_source`: `local` today; remote/public modes are prepared for GUI phase
- `image_source`: `local`, `preset`, `url`
- `image_source_preset`: built-in public image source id
- `image_source_url`: custom public image endpoint
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
- presets are starter integrations and may require provider-specific API keys/rate-limit handling for production
- provider-specific parsing exists for NASA APOD, ZenQuotes, and Quotable presets
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

### 1. Projektziel
Wallpaper Composer ist eine Open-Source-Anwendung in Rust fur Linux-Desktops.
Das Programm soll dynamische Hintergrunde erzeugen aus:
- einem Bildordner,
- rotierenden Spruchen aus `.txt`/`.md`,
- eingeblendeter Uhrzeit.

Spater optional: Login-Screen- und Boot-Screen-Integration (je nach Distro/Display-Manager unterschiedlich aufwendig).

### 2. Aktueller Stand (2026-03-04)
Bereits umgesetzt:
- Rust-Workspace mit:
  - `wc-cli`
  - `wc-core`
  - `wc-render`
- CLI-Kommandos:
  - `doctor`
  - `init`
  - `render-preview` (derzeit Platzhalter)
- Erzeugung einer Starter-Konfiguration (`init`)
- Basis-Tests und Lint/Test-Workflow

Noch offen:
- echte Bild-Komposition
- Wallpaper-Backends fur GNOME/KDE/Sway
- Paketierung (`rpm`, `deb`)
- Login/Boot-Integration

### 3. Technologien und Versionen
- Sprache: Rust (Edition `2024`)
- Projektversion: `0.1.0`
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
  crates/
    wc-cli/
    wc-core/
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
```

### 7. CLI-Referenz
`doctor`
- zeigt lokale Diagnosewerte (Projekt/Profil/Uhrzeit)

`init [--config <PFAD>] [--force]`
- legt eine Start-Konfiguration an
- Standardpfad: `~/.config/wallpaper-composer/config.toml`
- `--force` uberschreibt vorhandene Datei

`render-preview`
- Platzhalter fur die kommende Rendering-Pipeline

### 8. Konfigurationsreferenz
```toml
# Wallpaper Composer config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
output_image = "/tmp/wallpaper-composer-current.png"
refresh_seconds = 300
time_format = "%H:%M"
```

Bedeutung:
- `image_dir`: Quellordner fur Bilder
- `quotes_path`: Datei mit Spruchen (`.txt`/`.md`)
- `output_image`: Zielbild
- `refresh_seconds`: Aktualisierungsintervall
- `time_format`: Uhrzeitformat (chrono-Stil)

### 9. Beitragen/Weiterentwickeln
1. Zuerst `docs/PROJECT_PLAYBOOK.md` lesen.
2. An `docs/ARCHITECTURE.md` und `docs/TEST_MATRIX.md` orientieren.
3. Kleine fokussierte Branches (`codex/<topic>`).
4. Vor jedem PR: `fmt`, `clippy`, `test`.
5. Nutzerrelevante Anderungen in README/CHANGELOG dokumentieren.

### 10. Roadmap
MVP:
- echte Preview-Ausgabe (Bild + Spruch + Uhrzeit)
- periodische Aktualisierung
- erstes Wallpaper-Backend

Danach:
- Backend-Abstraktion pro Desktop/Compositor
- Paketierung fur Fedora/Ubuntu
- Alpha/Beta auf VM-Matrix

### 11. Lizenz und Haftung
Lizenz: GPL-3.0-or-later.
Bereitstellung ohne Gewahrleistung.

---

## Srpski

### 1. Cilj projekta
Wallpaper Composer je open-source aplikacija u Rust-u za Linux desktop okruzenja.
Program treba da pravi dinamicne pozadine iz:
- foldera sa slikama,
- rotirajucih citata iz `.txt`/`.md`,
- prikaza trenutnog vremena.

Kasnije opcionalno: integracija login ekrana i boot ekrana (zavisno od distribucije i display manager-a).

### 2. Trenutni status (2026-03-04)
Uradjeno:
- Rust workspace sa paketima:
  - `wc-cli`
  - `wc-core`
  - `wc-render`
- CLI komande:
  - `doctor`
  - `init`
  - `render-preview` (trenutno placeholder)
- generisanje pocetnog config fajla (`init`)
- osnovni test/lint workflow

Nije jos uradjeno:
- prava kompozicija slike
- backend za postavljanje pozadine na GNOME/KDE/Sway
- pakovanje (`rpm`, `deb`)
- login/boot integracija

### 3. Tehnologije i verzije
- Jezik: Rust (edition `2024`)
- Verzija projekta: `0.1.0`
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
  crates/
    wc-cli/
    wc-core/
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
```

### 7. CLI referenca
`doctor`
- prikazuje dijagnostiku (projekat/profil/lokalno vreme)

`init [--config <PUTANJA>] [--force]`
- pravi pocetni config fajl
- podrazumevana putanja: `~/.config/wallpaper-composer/config.toml`
- `--force` pregazi postojeci fajl

`render-preview`
- placeholder za buduci render pipeline

### 8. Config referenca
```toml
# Wallpaper Composer config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
output_image = "/tmp/wallpaper-composer-current.png"
refresh_seconds = 300
time_format = "%H:%M"
```

Znacenje polja:
- `image_dir`: folder sa slikama
- `quotes_path`: fajl sa citatima (`.txt`/`.md`)
- `output_image`: izlazna slika
- `refresh_seconds`: interval osvezavanja
- `time_format`: format vremena (`chrono`)

### 9. Dalji razvoj
1. Prvo procitati `docs/PROJECT_PLAYBOOK.md`.
2. Pratiti `docs/ARCHITECTURE.md` i `docs/TEST_MATRIX.md`.
3. Raditi male tematske grane (`codex/<topic>`).
4. Pre svakog PR-a pokrenuti `fmt`, `clippy`, `test`.
5. Sve korisnicke promene upisati u README/CHANGELOG.

### 10. Plan
MVP:
- prava preview slika (pozadina + citat + vreme)
- periodicno osvezavanje
- prvi backend za Linux pozadinu

Posle toga:
- backend apstrakcija po desktop/compositor okruzenju
- pakovanje za Fedora/Ubuntu
- alpha/beta testiranje na VM matrici

### 11. Licenca i odgovornost
Licenca: GPL-3.0-or-later.
Softver se isporucuje bez garancije.

---

## 中文

### 1. 项目目标
Wallpaper Composer 是一个面向 Linux 桌面环境的 Rust 开源项目。
目标是从以下内容生成动态壁纸：
- 图片目录，
- 来自 `.txt`/`.md` 的轮换语录，
- 当前时间叠加。

后续可选功能：登录界面背景与启动画面集成（取决于发行版和显示管理器）。

### 2. 当前实现状态（截至 2026-03-04）
已完成：
- Rust 工作区，包含：
  - `wc-cli`
  - `wc-core`
  - `wc-render`
- CLI 命令：
  - `doctor`
  - `init`
  - `render-preview`（目前为占位实现）
- 初始配置文件生成（`init`）
- 基础测试与 lint/test 流程

尚未完成：
- 真正的图像合成渲染
- GNOME/KDE/Sway 壁纸设置后端
- 软件打包（`rpm`、`deb`）
- 登录/启动画面集成

### 3. 技术栈与版本
- 语言：Rust（edition `2024`）
- 项目版本：`0.1.0`
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
  crates/
    wc-cli/
    wc-core/
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
```

### 7. CLI 说明
`doctor`
- 输出本地诊断信息（项目/配置档位/本地时间）

`init [--config <路径>] [--force]`
- 创建初始配置文件
- 默认路径：`~/.config/wallpaper-composer/config.toml`
- 使用 `--force` 可覆盖已有文件

`render-preview`
- 后续渲染流程的占位命令

### 8. 配置说明
```toml
# Wallpaper Composer config
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
output_image = "/tmp/wallpaper-composer-current.png"
refresh_seconds = 300
time_format = "%H:%M"
```

字段含义：
- `image_dir`：壁纸图片目录
- `quotes_path`：语录来源文件（`.txt`/`.md`）
- `output_image`：渲染输出路径
- `refresh_seconds`：刷新间隔（秒）
- `time_format`：时间格式（`chrono` 格式）

### 9. 协作开发流程
1. 先阅读 `docs/PROJECT_PLAYBOOK.md`。
2. 参考 `docs/ARCHITECTURE.md` 与 `docs/TEST_MATRIX.md`。
3. 使用小而聚焦的分支（建议 `codex/<topic>`）。
4. 每次 PR 前运行 `fmt`、`clippy`、`test`。
5. 所有用户可见变更同步更新 README/CHANGELOG。

### 10. 路线图
MVP：
- 真实预览输出（背景图 + 语录 + 时间）
- 周期性刷新
- 首个 Linux 壁纸后端集成

下一阶段：
- 按桌面/合成器抽象后端
- Fedora/Ubuntu 打包
- 多 VM 的 alpha/beta 测试矩阵

### 11. 许可证与免责声明
许可证：GPL-3.0-or-later。
软件按“现状”提供，不附带任何担保。
