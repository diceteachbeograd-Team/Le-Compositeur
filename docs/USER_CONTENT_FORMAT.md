# User Content Format

This document defines how users can provide their own quote files.

## Supported file types
- `.txt`
- `.md`

## Default parsing mode
Current default mode is `quote_format = "lines"`:
- one non-empty line equals one quote
- markdown markers at line start are stripped (`#`, `-`, `*`, `>`)

Alternative block format (recommended for multi-line quotes):
- wrap each quote block between `***` delimiters
- delimiter markers are never displayed
- optional short first line header inside a block (for example `T1`, `Text 1`) is treated as a label and hidden
- optional author separator line `:` inside a block: content after `:` is displayed as author line (`- Author`)
- this format is recommended for reliable rotation in loop mode
- rendering alignment is automatic by script direction:
  - LTR: quote left-aligned, author right-aligned
  - RTL (Arabic/Hebrew/Syriac etc.): quote right-aligned, author left-aligned

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

```txt
*** Text 1
Blabla
Blabal
***
***T2
weiter zweiter anzeigetext
***
***T
Dritter anzeigetext
***
```

```txt
***
Text1
:
Verfasser
***
```

Repository sample file:
- `assets/examples/quotes.md` (10 English quotes, block format)
- `assets/quotes/local/local-quotes.md` (packaged multilingual default quotes)

`wc-cli init` bootstrap behavior:
- creates config at `~/.config/wallpaper-composer/config.toml`
- auto-creates local quotes file at `~/Documents/wallpaper-composer/quotes.md` if missing

Installed package path (RPM/DEB):
- `/usr/share/wallpaper-composer/quotes/local-quotes.md`

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

Current renderer applies these values directly:
- text box size presets are resolved against current image dimensions
- quote font size stays at configured value (no auto downscale from image size)
- quote/clock are rendered on top of a prepared background layer
