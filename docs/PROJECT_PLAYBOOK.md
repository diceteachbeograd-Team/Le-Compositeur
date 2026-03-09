# Le Compositeur Projekt-Playbook (Rust, Linux, Wayland)

Dieses Dokument ist die zentrale Projektakte und dient als **Agent-Handoff**.
Wenn ein neuer Agent startet, bekommt er dieses Dokument zuerst.

## 1) Projektziel
Wir entwickeln ein Open-Source-Linux-Programm (GPL), das aus:
- einem frei gewählten Bildordner,
- rotierenden Sprüchen aus `.md`/`.txt`,
- aktueller Uhrzeit,
ein Wallpaper rendert und zyklisch aktualisiert.

Später optional:
- Login-Hintergrund,
- Boot-Screen-Integration,
- mehrere Benutzerprofile / systemweite Profile.

## 2) Rollen im Projekt
- **Owner/Architekt (User):** Produktentscheidungen, Prioritäten, UI/Look, Freigaben.
- **AI-Team (Codex):** Architekturvorschläge, Implementierung, Refactoring, Tests, CI/CD, Doku.
- **Owner übernimmt Aufgaben, die AI nicht kann:** echte VM-Tests, manuelle Desktop-Checks, Secrets, Accounts, ggf. Signierung/Publishing.

## 3) Produkt-Scope
### MVP (zuerst)
- Konfigurierbarer Bildordner
- Sprüche aus Datei/Ordner laden (`.txt`, `.md`)
- Uhrzeit overlay
- Vorschau rendern (`render-preview`)
- Hintergrund zyklisch setzen (`run`)
- User-Config unter `~/.config/wallpaper-composer/config.toml`

### V2
- Desktop-spezifische Wayland/X11 Backends (GNOME/KDE/Sway)
- robustere Layout-Engine (Position, Rand, Textstil)
- Logging und Fehlerdiagnose

### V3 (optional, aufwendiger)
- Login-Hintergrund je nach Display Manager
- Boot-Screen/Plymouth Theme-Generator

## 4) Technische Leitplanken
- Sprache: **Rust stable**
- Lizenz: **GPLv3** (inkl. NO WARRANTY Hinweis)
- Zielplattformen: Fedora (Wayland first), Ubuntu LTS, Debian/openSUSE
- Architektur: Core + Backend-Adapter (pro Desktop/Compositor)

Empfohlene Crates (Start):
- CLI: `clap`
- Config: `serde`, `toml`
- Zeit: `chrono`
- Rendering: `image`, `ab_glyph` (oder `rusttype`)
- Fehler: `anyhow`, `thiserror`
- Logging: `tracing`, `tracing-subscriber`
- Tests: `cargo nextest` (optional), `assert_cmd` für CLI

## 5) Repository- und Arbeitsstruktur
Empfehlte Struktur:

```txt
wallpaper-composer/
  crates/
    wc-cli/
    wc-core/
    wc-render/
    wc-backend/
  assets/
    examples/
  docs/
    PROJECT_PLAYBOOK.md
    ARCHITECTURE.md
    TEST_MATRIX.md
  .github/workflows/
```

Branching:
- `main` stabil
- Feature-Branches: `codex/<topic>`
- Kleine PRs mit klarer Test-Notiz

## 6) Agent-Onboarding-Protokoll (immer gleich)
Jeder neue Agent soll diese Reihenfolge nutzen:
1. `docs/PROJECT_PLAYBOOK.md` lesen (dieses Dokument).
2. Aktuellen Stand in `README.md`, `CHANGELOG.md`, `docs/ARCHITECTURE.md` prüfen.
3. Offene Aufgaben priorisieren: Blocker -> MVP -> Nice-to-have.
4. Erst danach Code ändern.
5. Nach Änderung: `cargo fmt`, `cargo clippy`, `cargo test`.
6. Ergebnis + offene Risiken dokumentieren.

## 7) Standard-Kommandos
```bash
rustup update
cargo fmt --all
cargo clippy --all-targets --all-features -D warnings
cargo test --all
cargo run -p wc-cli -- render-preview --config ~/.config/wallpaper-composer/config.toml
cargo run -p wc-cli -- run --config ~/.config/wallpaper-composer/config.toml
```

## 8) Accounts & Organisation
Für dieses Projekt sinnvoll:
- Eigener Projekt-GitHub-Account oder GitHub-Organisation
- Eigene Projekt-E-Mail (z. B. `wallpaper-composer@...`)

Warum:
- Trennung von privaten Accounts
- saubere Ownership für Releases/Issues/Secrets
- einfacher Zugang für spätere Mitwirkende

Empfehlung:
- GitHub Org anlegen, Repo dort hosten.
- Projekt-Mail als Recovery + Benachrichtigungen nutzen.
- 2FA aktivieren (GitHub + Mail).

## 9) VM-Teststrategie (Owner stellt bereit)
Der Owner richtet VMs ein und gibt SSH/CCH-Zugänge.

### Pflicht-VMs für MVP
1. Fedora Workstation (GNOME, Wayland)
2. Fedora KDE Spin (Wayland)
3. Ubuntu LTS (GNOME, Wayland)
4. Debian oder openSUSE (zweite Vergleichsplattform)

### Empfohlene VM-Ressourcen
- 2 vCPU (besser 4)
- 4-8 GB RAM
- 40 GB Disk
- Snapshots vor jedem Testlauf
- sudo + SSH Zugriff

### Wann welche VM gebraucht wird
- **Sprint 1-2 (Core/Render):** primär 1 Fedora-VM
- **Sprint 3-4 (Wallpaper Backends):** alle 4 VMs
- **Alpha:** komplette Matrix + Regressionsrunde
- **Beta/Release:** vollständige Re-Tests + Paketinstallation frisch

## 10) Rust-Lernpfad (hands-on im Projekt)
### Lernmodus
- Jede Feature-Umsetzung in kleinen Schritten.
- Erst lauffähig, dann sauber machen.
- Pro PR 1-2 neue Rust-Konzepte bewusst lernen.

### Reihenfolge der Rust-Themen
1. `struct`, `enum`, `Result`, `Option`
2. Borrowing/Ownership im echten Code
3. Traits + Modulstruktur
4. Fehlerbehandlung (`anyhow`/`thiserror`)
5. Tests (Unit + CLI)
6. Concurrency/Async nur wenn nötig

## 11) Mac mini Setup (Rust + IDE + BBEdit)
### Toolchain
```bash
xcode-select --install
curl https://sh.rustup.rs -sSf | sh
rustup default stable
rustup component add rustfmt clippy
brew install pkg-config
```

### Empfohlene IDE/Editoren
- **RustRover** (sehr gut für Rust-Refactorings)
- **VS Code** (leichtgewichtig, große Extension-Auswahl)
- **BBEdit** (gut als schneller Text-/Code-Editor, in Kombination mit CLI)

### Wichtige Plugins (falls VS Code)
- `rust-analyzer`
- `CodeLLDB`
- `Even Better TOML`
- `EditorConfig` (optional)

### BBEdit sinnvoll nutzen
BBEdit hat keine so tiefe Rust-Semantik wie RustRover/VS Code, ist aber stark für schnelles Editieren.
Empfohlenes Setup:
- BBEdit für schnelle Dateiänderungen, Markdown, Doku
- Terminal für `cargo fmt`, `clippy`, `test`
- Bei komplexem Refactoring zusätzlich RustRover oder VS Code mit `rust-analyzer`

## 12) Qualitätsregeln
- Kein Feature ohne Beispiel-Config
- Keine neue Funktion ohne mindestens einen Test
- Jede Fehlermeldung soll für Endnutzer verständlich sein
- Dokumentation bei jeder User-sichtbaren Änderung anpassen

## 13) Lizenz & Disclaimer
- Lizenz: GPLv3
- Projekt liefert Software **ohne Gewährleistung**
- Klarer Haftungsausschluss in `README` und `LICENSE`-Kontext

## 14) Nächste konkrete Schritte (Start)
1. Repo initialisieren (GPLv3, README, dieses Playbook unter `docs/`)
2. Rust Workspace mit `wc-cli`, `wc-core`, `wc-render` anlegen
3. Kommando `render-preview` implementieren
4. Beispiel-Config + Beispiel-Assets bereitstellen
5. Erste Fedora-VM Tests durchführen

## 15) Agent-Start-Prompt (Copy/Paste)
Nutze diesen Prompt, wenn du einen neuen Agenten startest:

```txt
Lies zuerst docs/PROJECT_PLAYBOOK.md und arbeite strikt danach.
Ziel: MVP für wallpaper-composer in Rust (Bildordner + Sprüche + Uhrzeit + Wallpaper-Wechsel).
Ich bin Owner/Architekt, du bist Umsetzungsteam.
Arbeite in kleinen, testbaren Schritten; nach jedem Schritt: fmt, clippy, test und kurze Statusnotiz.
Wenn VM-Tests nötig sind, liste konkret welche VM und welchen Testfall ich ausführen soll.
```

## 16) Block 1: Rust-Grundlagen (für den Owner)
Ziel von Block 1: In kurzer Zeit genug Rust-Verständnis aufbauen, um aktiv am Projekt mitzuwirken.

### Empfohlene Quellen (offiziell/primär)
- Rust Learn Portal: https://www.rust-lang.org/learn
- The Rust Programming Language (Book): https://doc.rust-lang.org/stable/book/
- Rust By Example: https://doc.rust-lang.org/rust-by-example/
- Rustlings Übungen: https://github.com/rust-lang/rustlings
- Comprehensive Rust (Google, kostenlos): https://google.github.io/comprehensive-rust/

### 7-Tage Einstieg (parallel zum Projekt)
1. Tag 1: Kapitel 1-3 aus dem Book + `cargo`, `rustc`, erstes CLI-Projekt.
2. Tag 2: Ownership/Borrowing aus dem Book + 10-15 Rustlings Aufgaben.
3. Tag 3: `struct`, `enum`, `match`, `Result`, `Option` + kleine Parser-Übung.
4. Tag 4: Module, Crates, Fehlerbehandlung (`anyhow`/`thiserror`) im Projektkontext.
5. Tag 5: Rendering-Basics mit `image` crate nachvollziehen.
6. Tag 6: Tests schreiben (`cargo test`), `clippy`-Warnungen beheben.
7. Tag 7: Mini-Feature selbst implementieren (z. B. Zeitformat konfigurierbar).

### Was sich gegenüber C/Python/Java besonders ändert
- **Gegenüber C:** Ownership statt manuellem Memory-Management; Compiler verhindert viele UB-Klassen früh.
- **Gegenüber Python:** striktes Typ- und Fehlerhandling zur Compile-Zeit, deutlich weniger „Runtime-Überraschungen“.
- **Gegenüber Java:** kein GC im klassischen Sinn; Lifetimes/Borrowing statt Referenzmodell mit GC.
- **Übergreifend:** Compiler-Feedback ist Teil des Lernwegs; Fehlermeldungen aktiv durcharbeiten.
