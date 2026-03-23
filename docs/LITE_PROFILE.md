# Lite Profile (Small Version)

Starting with release line `2026.03.23-3`, Linux defaults to a memory-safe Lite profile.

## Why this exists
On smaller VMs, startup/render spikes could trigger heavy RAM + swap pressure and freeze the desktop session.
The Lite profile intentionally trades feature breadth for stability.

## What we drop in Lite (and why)
- `Weather` source + weather layer
Reason: provider fetch + rendering path can create transient memory spikes.

- `News` source + ticker/overlay paths (`show_news_layer`, `show_news_ticker2`)
Reason: mixed network/feed/image processing path is expensive under constrained memory.

- `Static URL` source panel
Reason: snapshot/render conversion path is one of the highest memory-risk areas.

- `Script Ticker`
Reason: dynamic command-driven overlays increase runtime variability and cache pressure.

- `Cams`
Reason: already non-final in stable path and not suitable for low-memory baseline.

## Additional Lite defaults
- Lower canvas cap by default: `1280x720` (override via `WC_MAX_CANVAS_WIDTH/HEIGHT`).
- Low-memory rendering mode enabled by default (`WC_LOW_MEMORY_MODE=1`).
- Stronger ImageMagick limits (memory/map/disk/thread) in renderer.
- GUI starts with local image source and avoids unnecessary preview thumbnail loading.
- Refresh defaults are more conservative (`600s`) to reduce churn.

## Full mode (opt-in)
If your machine has enough RAM and you accept higher risk of memory spikes:

```bash
export WC_LITE_PROFILE=0
le-compositeur
```

## Tuning knobs
- `WC_LITE_PROFILE` (`1`/`0`)
- `WC_LOW_MEMORY_MODE`
- `WC_MAGICK_MEMORY_MB`
- `WC_MAGICK_MAP_MB`
- `WC_MAGICK_DISK_MB`
- `WC_MAGICK_THREAD_LIMIT`
- `WC_MAX_CANVAS_WIDTH`
- `WC_MAX_CANVAS_HEIGHT`
- `WC_GUI_MAX_PREVIEW_PIXELS`

## Scope note
Lite is currently default on Linux because Linux VM usage is the primary affected deployment target.
