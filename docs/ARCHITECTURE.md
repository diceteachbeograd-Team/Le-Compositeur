# Architektur (Draft)

- `wc-cli`: Kommandozeileninterface und User-Interaktion.
- `wc-core`: Domänenlogik (Config, Auswahl, Scheduling-Logik).
- `wc-render`: Bild/Overlay Rendering.
- `wc-backend`: Wallpaper-Anwendung pro Desktop/Compositor (`auto`, `gnome`, `sway`, `feh`, `noop`).
- `wc-source`: Remote-Quellen (Preset/URL), Fetching, Caching und Provider-spezifisches Parsing.

Geplante Ergänzung:
- `wc-backend`: Desktop-/Compositor-Adapter (GNOME, KDE, Sway).

Source strategy:
- Local sources are active now (`image_dir`, `quotes_path`).
- Public source presets and URLs are modeled in config and available in experimental CLI mode:
  - image presets (for example NASA APOD)
  - quote presets (for example public quote APIs)
- Current remote fetch path uses `curl` + local cache + lightweight payload parsing.
- Provider adapters currently include:
  - NASA APOD image URL parsing (`media_type=image`, `hdurl/url`)
  - ZenQuotes parsing (`q`, `a`)
  - Quotable parsing (`content`, `author`)
- Planned next step: move remote fetching into dedicated integration layer with strict parser/provider adapters.

Rotation strategy:
- Time-based cycle index remains baseline.
- Shared timer strategy: one cycle timer (`refresh_seconds`) drives both image and quote updates.
- Optional persistent state file stores last used cycle so sequence continues across restarts.
- Local no-repeat history stores a rolling pick history and applies source-count-aware windows (especially in `random`) to avoid short repeat loops.

Render layering strategy:
- Background image is prepared first as independent layer.
- Quote/author/clock overlays are composed afterwards.
- Text box sizes (`quarter/third/half/full/custom`) are resolved against current image dimensions.
- Quote font size follows configured value and is not downscaled by image-size heuristics.
- Canvas size is auto-detected from current desktop resolution each cycle (fallback `1920x1080`).

GUI contract strategy:
- `export-schema` provides a machine-readable contract for settings UI generation.
- Contract includes group definitions and field metadata (labels/descriptions/options/defaults).
- Contract also includes conditional hints (`visible_when`, `enabled_when`) for dynamic form behavior.
- `ui-blueprint` provides section-level layout and field ordering for direct form scaffolding.
- Contract includes `ui_widget` hints (for example `directory-picker`, `file-picker`, `file-save`).
- `preset-catalog` provides GUI-ready provider metadata for dropdowns and helper text.

Distribution strategy:
- CI quality gate in GitHub Actions (`fmt`, `clippy`, `test`).
- Packaging scaffolds for RPM and DEB are tracked under `packaging/`.
