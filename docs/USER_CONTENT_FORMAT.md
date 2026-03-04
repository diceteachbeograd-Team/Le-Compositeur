# User Content Format

This document defines how users can provide their own quote files.

## Supported file types
- `.txt`
- `.md`

## Default parsing mode
Current default mode is `quote_format = "lines"`:
- one non-empty line equals one quote
- markdown markers at line start are stripped (`#`, `-`, `*`, `>`)

Examples:

```txt
Stay focused.
Ship small increments.
Document every change.
```

```md
# Stay focused.
- Ship small increments.
> Document every change.
```

## Planned parsing modes
These modes are reserved for GUI settings and future parser upgrades:
- `lines`: one quote per line (current)
- `paragraphs`: one quote per paragraph (blank line separator)
- `markdown_blocks`: block-level markdown extraction

## Public source settings
Public sources are available in experimental mode.

Config keys:
- `image_source`: `local` or future remote source mode
- `image_source_preset`: preset id for public image provider
- `image_source_url`: custom URL for image source
- `quote_source`: `local` or remote source mode
- `quote_source_preset`: preset id for quote provider
- `quote_source_url`: custom URL for quote source

Supported source values:
- `local`
- `preset`
- `url`

Remote mode currently uses `curl` and cached downloads/response processing.

## Text and clock layout settings
These settings are already available for GUI mapping:
- `quote_font_size` (minimum 8)
- `quote_pos_x`, `quote_pos_y`
- `clock_font_size` (minimum 8)
- `clock_pos_x`, `clock_pos_y`

Current renderer mode writes these values into preview metadata sidecar.
In a later rendering phase, the same settings will directly control drawn text placement and size.
