# Product Decisions (2026-03-14)

## Decision 1: De-scope live video/cams from stable mode

### Context
- Fedora VM validation showed repeated instability with live video/cam overlays (black frames, stale panes, process/window coupling).
- Wallpaper refresh cadence and external player/window behavior were not reliable enough for a stable UX baseline.

### Decision
- Stable product path no longer depends on live video/cam rendering.
- `News`/`Cams` workspace flow is replaced by:
  1. `NewsTicker` tab
  2. `Static URL` tab

### Rationale
- Ticker + snapshot/static URL behavior is deterministic, testable, and lower risk on Wayland/Fedora.
- Avoiding live browser/video embedding reduces runtime and packaging complexity.

### Consequences
- Live media is treated as experimental/out-of-scope for stable release quality.
- Documentation and release notes must reflect "stable mode = ticker + static URL".

---

## Decision 2: Ticker must be independent from live-news layer

### Context
- Previous gating tied secondary ticker to `show_news_layer`, which made ticker unusable when live-news pane was disabled.

### Decision
- `news_ticker2` enablement is decoupled from `show_news_layer`.
- Overlay runtime includes dedicated `news_ticker2` ticker state/process entry.

### Rationale
- Product pivot requires ticker-first behavior without live video dependency.

---

## Decision 3: Unique package release version per VM test cycle

### Context
- Reusing identical RPM release strings led to stale binary installs and false-positive validation.

### Decision
- Every VM test build must use a new package version/release suffix.
- Post-install binary hash check is mandatory for critical runtime fixes.

### Rationale
- Ensures installed package actually contains the code under test.

---

## Answer to open architecture question

- "Is a browser in background as hard as video?"
  - For this project and target platform, yes: similarly hard or harder.
  - Therefore browser-embedded background rendering is not part of stable scope.

