use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PreviewText<'a> {
    pub quote: &'a str,
    pub clock: &'a str,
    pub quote_font_size: u32,
    pub quote_pos_x: i32,
    pub quote_pos_y: i32,
    pub quote_auto_fit: bool,
    pub quote_min_font_size: u32,
    pub font_family: &'a str,
    pub quote_color: &'a str,
    pub clock_font_size: u32,
    pub clock_pos_x: i32,
    pub clock_pos_y: i32,
    pub clock_color: &'a str,
    pub text_stroke_color: &'a str,
    pub text_stroke_width: u32,
    pub text_undercolor: &'a str,
    pub text_shadow_enabled: bool,
    pub text_shadow_color: &'a str,
    pub text_shadow_offset_x: i32,
    pub text_shadow_offset_y: i32,
    pub text_box_size: &'a str,
    pub text_box_width_pct: u32,
    pub text_box_height_pct: u32,
}

#[derive(Debug, Clone)]
pub struct RenderResult {
    pub meta_path: PathBuf,
    pub preview_mode: String,
}

pub fn render_preview_to_file(
    source_image: &Path,
    output_image: &Path,
    text: PreviewText<'_>,
) -> Result<RenderResult, String> {
    let render_mode = if render_with_imagemagick(source_image, output_image, &text)? {
        "imagemagick-overlay".to_string()
    } else if render_with_native_bmp(source_image, output_image, &text)? {
        "native-bmp-overlay".to_string()
    } else {
        fs::copy(source_image, output_image).map_err(|e| {
            format!(
                "failed to copy source image {} -> {}: {e}",
                source_image.display(),
                output_image.display()
            )
        })?;
        "copy-source".to_string()
    };

    let meta_path = metadata_path_for(output_image);
    let metadata = format!(
        "preview_mode = {:?}\nquote = {:?}\nclock = {:?}\nquote_font_size = {}\nquote_pos_x = {}\nquote_pos_y = {}\nquote_auto_fit = {}\nquote_min_font_size = {}\nfont_family = {:?}\nquote_color = {:?}\nclock_font_size = {}\nclock_pos_x = {}\nclock_pos_y = {}\nclock_color = {:?}\ntext_stroke_color = {:?}\ntext_stroke_width = {}\ntext_undercolor = {:?}\ntext_shadow_enabled = {}\ntext_shadow_color = {:?}\ntext_shadow_offset_x = {}\ntext_shadow_offset_y = {}\ntext_box_size = {:?}\ntext_box_width_pct = {}\ntext_box_height_pct = {}\nsource_image = {:?}\n",
        render_mode,
        text.quote,
        text.clock,
        text.quote_font_size,
        text.quote_pos_x,
        text.quote_pos_y,
        text.quote_auto_fit,
        text.quote_min_font_size,
        text.font_family,
        text.quote_color,
        text.clock_font_size,
        text.clock_pos_x,
        text.clock_pos_y,
        text.clock_color,
        text.text_stroke_color,
        text.text_stroke_width,
        text.text_undercolor,
        text.text_shadow_enabled,
        text.text_shadow_color,
        text.text_shadow_offset_x,
        text.text_shadow_offset_y,
        text.text_box_size,
        text.text_box_width_pct,
        text.text_box_height_pct,
        source_image.display().to_string()
    );

    fs::write(&meta_path, metadata)
        .map_err(|e| format!("failed to write metadata {}: {e}", meta_path.display()))?;
    Ok(RenderResult {
        meta_path,
        preview_mode: render_mode,
    })
}

fn metadata_path_for(output_image: &Path) -> PathBuf {
    let mut name = output_image
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("preview-output")
        .to_string();
    name.push_str(".meta.txt");
    output_image.with_file_name(name)
}

fn render_with_imagemagick(
    source_image: &Path,
    output_image: &Path,
    text: &PreviewText<'_>,
) -> Result<bool, String> {
    let tool = detect_imagemagick_tool();
    let Some((cmd, use_magick_subcommand)) = tool else {
        return Ok(false);
    };

    let mut args = Vec::<String>::new();
    if use_magick_subcommand {
        args.push("convert".to_string());
    }

    let (quote_body, author) = split_quote_and_author(text.quote);
    let rtl = is_rtl_text(&quote_body);
    let quote_gravity = if rtl { "East" } else { "West" };
    let author_gravity = if rtl { "West" } else { "East" };
    let (img_w, img_h) =
        detect_image_dimensions(&source_image.display().to_string()).unwrap_or((1920, 1080));
    let layout_w = 1920;
    let layout_h = 1080;

    // Build an explicit background layer first so image scaling/placement is independent
    // from the quote box/text layer rendered afterwards.
    args.push("(".to_string());
    args.push(source_image.display().to_string());
    args.push("-resize".to_string());
    args.push(format!("{img_w}x{img_h}^"));
    args.push("-gravity".to_string());
    args.push("Center".to_string());
    args.push("-extent".to_string());
    args.push(format!("{img_w}x{img_h}"));
    args.push(")".to_string());

    let (box_w_pct, box_h_pct) = resolve_text_box_pct(text);
    let box_w = ((layout_w * box_w_pct as i32) / 100).max(240);
    let box_h = ((layout_h * box_h_pct as i32) / 100).max(160);
    let effective_quote_size = resolve_quote_font_size(text.quote_font_size, text);
    let author_size = ((effective_quote_size as f32) * 0.85).round() as u32;
    let author_h = (author_size as i32 * 2).max(40);
    let quote_h = (box_h - author_h).max(80);
    let effective_stroke_width = text.text_stroke_width;
    let effective_undercolor = text.text_undercolor;
    let shadow_x = text.quote_pos_x.saturating_add(text.text_shadow_offset_x);
    let shadow_y = text.quote_pos_y.saturating_add(text.text_shadow_offset_y);

    if text.text_shadow_enabled {
        // Draw a shadow pass first, then overlay the main text on top.
        args.push("(".to_string());
        args.push("-background".to_string());
        args.push("none".to_string());
        args.push("-size".to_string());
        args.push(format!("{}x{}", box_w, quote_h));
        args.push("-fill".to_string());
        args.push(text.text_shadow_color.to_string());
        args.push("-stroke".to_string());
        args.push("none".to_string());
        args.push("-undercolor".to_string());
        args.push("none".to_string());
        args.push("-gravity".to_string());
        args.push(quote_gravity.to_string());
        args.push("-font".to_string());
        args.push(text.font_family.to_string());
        args.push("-pointsize".to_string());
        args.push(effective_quote_size.to_string());
        args.push(format!("caption:{quote_body}"));
        args.push(")".to_string());
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-geometry".to_string());
        args.push(format!("+{}+{}", shadow_x, shadow_y));
        args.push("-composite".to_string());
    }

    // Quote text in bounded box to avoid full-screen overlay.
    args.push("(".to_string());
    args.push("-background".to_string());
    args.push("none".to_string());
    args.push("-size".to_string());
    args.push(format!("{}x{}", box_w, quote_h));
    args.push("-fill".to_string());
    args.push(text.quote_color.to_string());
    args.push("-stroke".to_string());
    args.push(text.text_stroke_color.to_string());
    args.push("-strokewidth".to_string());
    args.push(effective_stroke_width.to_string());
    args.push("-undercolor".to_string());
    args.push(effective_undercolor.to_string());
    args.push("-gravity".to_string());
    args.push(quote_gravity.to_string());
    args.push("-font".to_string());
    args.push(text.font_family.to_string());
    args.push("-pointsize".to_string());
    args.push(effective_quote_size.to_string());
    args.push(format!("caption:{quote_body}"));
    args.push(")".to_string());
    args.push("-gravity".to_string());
    args.push("NorthWest".to_string());
    args.push("-geometry".to_string());
    args.push(format!("+{}+{}", text.quote_pos_x, text.quote_pos_y));
    args.push("-composite".to_string());

    // Optional author line in same bounded area at opposite edge.
    if let Some(author_text) = author {
        if text.text_shadow_enabled {
            args.push("(".to_string());
            args.push("-background".to_string());
            args.push("none".to_string());
            args.push("-size".to_string());
            args.push(format!("{}x{}", box_w, author_h));
            args.push("-fill".to_string());
            args.push(text.text_shadow_color.to_string());
            args.push("-stroke".to_string());
            args.push("none".to_string());
            args.push("-undercolor".to_string());
            args.push("none".to_string());
            args.push("-gravity".to_string());
            args.push(author_gravity.to_string());
            args.push("-font".to_string());
            args.push(text.font_family.to_string());
            args.push("-pointsize".to_string());
            args.push(author_size.to_string());
            args.push(format!("caption:- {author_text}"));
            args.push(")".to_string());
            args.push("-gravity".to_string());
            args.push("NorthWest".to_string());
            args.push("-geometry".to_string());
            args.push(format!(
                "+{}+{}",
                shadow_x,
                shadow_y.saturating_add(quote_h)
            ));
            args.push("-composite".to_string());
        }

        args.push("(".to_string());
        args.push("-background".to_string());
        args.push("none".to_string());
        args.push("-size".to_string());
        args.push(format!("{}x{}", box_w, author_h));
        args.push("-fill".to_string());
        args.push(text.quote_color.to_string());
        args.push("-stroke".to_string());
        args.push(text.text_stroke_color.to_string());
        args.push("-strokewidth".to_string());
        args.push(effective_stroke_width.to_string());
        args.push("-undercolor".to_string());
        args.push(effective_undercolor.to_string());
        args.push("-gravity".to_string());
        args.push(author_gravity.to_string());
        args.push("-font".to_string());
        args.push(text.font_family.to_string());
        args.push("-pointsize".to_string());
        args.push(author_size.to_string());
        args.push(format!("caption:- {author_text}"));
        args.push(")".to_string());
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-geometry".to_string());
        args.push(format!(
            "+{}+{}",
            text.quote_pos_x,
            text.quote_pos_y.saturating_add(quote_h)
        ));
        args.push("-composite".to_string());
    }

    // Clock styling and placement.
    if text.text_shadow_enabled {
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-fill".to_string());
        args.push(text.text_shadow_color.to_string());
        args.push("-stroke".to_string());
        args.push("none".to_string());
        args.push("-undercolor".to_string());
        args.push("none".to_string());
        args.push("-font".to_string());
        args.push(text.font_family.to_string());
        args.push("-pointsize".to_string());
        args.push(text.clock_font_size.to_string());
        args.push("-annotate".to_string());
        args.push(format!(
            "+{}+{}",
            text.clock_pos_x.saturating_add(text.text_shadow_offset_x),
            text.clock_pos_y.saturating_add(text.text_shadow_offset_y)
        ));
        args.push(text.clock.to_string());
    }

    args.push("-gravity".to_string());
    args.push("NorthWest".to_string());
    args.push("-fill".to_string());
    args.push(text.clock_color.to_string());
    args.push("-font".to_string());
    args.push(text.font_family.to_string());
    args.push("-pointsize".to_string());
    args.push(text.clock_font_size.to_string());
    args.push("-annotate".to_string());
    args.push(format!("+{}+{}", text.clock_pos_x, text.clock_pos_y));
    args.push(text.clock.to_string());
    args.push("-stroke".to_string());
    args.push("none".to_string());

    args.push(output_image.display().to_string());

    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;

    Ok(status.success())
}

fn split_quote_and_author(input: &str) -> (String, Option<String>) {
    let lines = input.lines().map(str::trim_end).collect::<Vec<_>>();
    if lines.len() < 2 {
        return (input.trim().to_string(), None);
    }

    let mut last_non_empty = None;
    for (idx, line) in lines.iter().enumerate().rev() {
        if !line.trim().is_empty() {
            last_non_empty = Some((idx, line.trim().to_string()));
            break;
        }
    }
    let Some((author_idx, author_line)) = last_non_empty else {
        return (input.trim().to_string(), None);
    };
    let Some(author) = author_line.strip_prefix("- ").map(|s| s.trim().to_string()) else {
        return (input.trim().to_string(), None);
    };
    if author.is_empty() {
        return (input.trim().to_string(), None);
    }

    let body = lines[..author_idx]
        .iter()
        .map(|s| s.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if body.is_empty() {
        return (input.trim().to_string(), None);
    }
    (body, Some(author))
}

fn is_rtl_text(input: &str) -> bool {
    input.chars().any(is_rtl_char)
}

fn is_rtl_char(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x05FF  // Hebrew
            | 0x0600..=0x06FF // Arabic
            | 0x0700..=0x074F // Syriac
            | 0x0750..=0x077F // Arabic Supplement
            | 0x0780..=0x07BF // Thaana
            | 0x08A0..=0x08FF // Arabic Extended-A
            | 0xFB50..=0xFDFF // Arabic Presentation Forms-A
            | 0xFE70..=0xFEFF // Arabic Presentation Forms-B
    )
}

fn detect_image_dimensions(path: &str) -> Option<(i32, i32)> {
    let out = Command::new("identify")
        .args(["-format", "%w %h", path])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    let mut parts = raw.split_whitespace();
    let w = parts.next()?.parse::<i32>().ok()?;
    let h = parts.next()?.parse::<i32>().ok()?;
    Some((w, h))
}

fn resolve_text_box_pct(text: &PreviewText<'_>) -> (u32, u32) {
    match text.text_box_size.trim().to_ascii_lowercase().as_str() {
        "third" => (58, 58),
        "half" => (70, 70),
        "full" => (100, 100),
        "custom" => (
            text.text_box_width_pct.clamp(10, 100),
            text.text_box_height_pct.clamp(10, 100),
        ),
        _ => (50, 50),
    }
}

fn resolve_quote_font_size(preferred: u32, text: &PreviewText<'_>) -> u32 {
    // Keep quote font stable as configured; image scaling and text box sizing
    // must not downscale the chosen font size.
    preferred.max(text.quote_min_font_size.max(8))
}

fn render_with_native_bmp(
    source_image: &Path,
    output_image: &Path,
    text: &PreviewText<'_>,
) -> Result<bool, String> {
    let mut bytes = fs::read(source_image).map_err(|e| {
        format!(
            "failed to read source image {}: {e}",
            source_image.display()
        )
    })?;
    let Some(mut bmp) = Bmp24::from_bytes(&mut bytes)? else {
        return Ok(false);
    };

    let quote_scale = (text.quote_font_size / 10).max(1);
    let clock_scale = (text.clock_font_size / 10).max(1);

    draw_text(
        &mut bmp,
        text.quote_pos_x,
        text.quote_pos_y,
        text.quote,
        quote_scale,
        Rgb {
            r: 255,
            g: 255,
            b: 255,
        },
    );
    draw_text(
        &mut bmp,
        text.clock_pos_x,
        text.clock_pos_y,
        text.clock,
        clock_scale,
        Rgb {
            r: 255,
            g: 215,
            b: 0,
        },
    );

    fs::write(output_image, &bytes).map_err(|e| {
        format!(
            "failed to write native bmp output {}: {e}",
            output_image.display()
        )
    })?;
    Ok(true)
}

fn detect_imagemagick_tool() -> Option<(&'static str, bool)> {
    if has_command("magick") {
        return Some(("magick", true));
    }
    if has_command("convert") {
        return Some(("convert", false));
    }
    None
}

fn has_command(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[derive(Clone, Copy)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

struct Bmp24<'a> {
    bytes: &'a mut [u8],
    width: usize,
    height: usize,
    row_stride: usize,
    data_offset: usize,
    top_down: bool,
}

impl<'a> Bmp24<'a> {
    fn from_bytes(bytes: &'a mut [u8]) -> Result<Option<Self>, String> {
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return Ok(None);
        }

        let data_offset = le_u32(bytes, 10) as usize;
        let dib_size = le_u32(bytes, 14);
        if dib_size < 40 || bytes.len() < data_offset {
            return Ok(None);
        }

        let width = le_i32(bytes, 18);
        let height = le_i32(bytes, 22);
        let planes = le_u16(bytes, 26);
        let bpp = le_u16(bytes, 28);
        let compression = le_u32(bytes, 30);

        if width <= 0 || height == 0 || planes != 1 || bpp != 24 || compression != 0 {
            return Ok(None);
        }

        let width_u = width as usize;
        let height_u = height.unsigned_abs() as usize;
        let row_stride = (width_u * 3).div_ceil(4) * 4;
        let required = data_offset.saturating_add(row_stride.saturating_mul(height_u));
        if required > bytes.len() {
            return Ok(None);
        }

        Ok(Some(Self {
            bytes,
            width: width_u,
            height: height_u,
            row_stride,
            data_offset,
            top_down: height < 0,
        }))
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: Rgb) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return;
        }

        let row = if self.top_down {
            y
        } else {
            self.height - 1 - y
        };
        let idx = self.data_offset + row * self.row_stride + x * 3;
        if idx + 2 >= self.bytes.len() {
            return;
        }
        self.bytes[idx] = color.b;
        self.bytes[idx + 1] = color.g;
        self.bytes[idx + 2] = color.r;
    }
}

fn le_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn le_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn draw_text(bmp: &mut Bmp24<'_>, x: i32, y: i32, text: &str, scale: u32, color: Rgb) {
    let mut cursor = x;
    for ch in text.chars() {
        draw_glyph(bmp, cursor, y, ch, scale, color);
        cursor += (6 * scale) as i32;
    }
}

fn draw_glyph(bmp: &mut Bmp24<'_>, x: i32, y: i32, ch: char, scale: u32, color: Rgb) {
    let glyph = glyph_bits(ch);
    for (row_idx, row) in glyph.iter().enumerate() {
        for col_idx in 0_i32..5_i32 {
            if (row >> (4 - col_idx as usize)) & 1 == 0 {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x + (col_idx * scale as i32) + sx as i32;
                    let py = y + (row_idx as i32 * scale as i32) + sy as i32;
                    bmp.set_pixel(px, py, color);
                }
            }
        }
    }
}

fn glyph_bits(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10011, 0b10101, 0b10101, 0b10101, 0b11001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b10100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b01110, 0b00000, 0b00000, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000,
        ],
        '!' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
        '?' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100,
        ],
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [
            0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111,
        ],
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        PreviewText, is_rtl_text, render_preview_to_file, render_with_native_bmp,
        split_quote_and_author,
    };
    use std::fs;

    #[test]
    fn render_preview_copies_image_and_writes_metadata() {
        let src = std::env::temp_dir().join("wc-render-src.bin");
        let dst = std::env::temp_dir().join("wc-render-dst.bin");
        fs::write(&src, vec![1_u8, 2, 3, 4]).expect("source should be writable");

        let result = render_preview_to_file(
            &src,
            &dst,
            PreviewText {
                quote: "Focus.",
                clock: "13:37",
                quote_font_size: 24,
                quote_pos_x: 10,
                quote_pos_y: 20,
                quote_auto_fit: true,
                quote_min_font_size: 14,
                font_family: "DejaVu-Sans",
                quote_color: "#FFFFFF",
                clock_font_size: 28,
                clock_pos_x: 300,
                clock_pos_y: 180,
                clock_color: "#FFD700",
                text_stroke_color: "#000000",
                text_stroke_width: 2,
                text_undercolor: "#00000066",
                text_shadow_enabled: true,
                text_shadow_color: "#00000099",
                text_shadow_offset_x: 3,
                text_shadow_offset_y: 3,
                text_box_size: "quarter",
                text_box_width_pct: 50,
                text_box_height_pct: 50,
            },
        )
        .expect("render should succeed");

        assert!(dst.exists(), "copied output should exist");
        assert!(result.meta_path.exists(), "metadata file should exist");
        assert!(
            ["copy-source", "imagemagick-overlay"].contains(&result.preview_mode.as_str()),
            "preview mode should be known"
        );

        let _ = fs::remove_file(src);
        let _ = fs::remove_file(dst);
        let _ = fs::remove_file(result.meta_path);
    }

    #[test]
    fn native_bmp_overlay_renders_when_source_is_bmp24() {
        let src = std::env::temp_dir().join("wc-render-native-src.bmp");
        let dst = std::env::temp_dir().join("wc-render-native-dst.bmp");
        fs::write(&src, bmp24_solid(64, 48, 20, 30, 40)).expect("bmp source should be writable");

        let ok = render_with_native_bmp(
            &src,
            &dst,
            &PreviewText {
                quote: "HELLO",
                clock: "12:34",
                quote_font_size: 24,
                quote_pos_x: 2,
                quote_pos_y: 2,
                quote_auto_fit: true,
                quote_min_font_size: 14,
                font_family: "DejaVu-Sans",
                quote_color: "#FFFFFF",
                clock_font_size: 24,
                clock_pos_x: 2,
                clock_pos_y: 20,
                clock_color: "#FFD700",
                text_stroke_color: "#000000",
                text_stroke_width: 1,
                text_undercolor: "#00000066",
                text_shadow_enabled: true,
                text_shadow_color: "#00000099",
                text_shadow_offset_x: 2,
                text_shadow_offset_y: 2,
                text_box_size: "quarter",
                text_box_width_pct: 50,
                text_box_height_pct: 50,
            },
        )
        .expect("native bmp render should return status");

        assert!(ok, "native renderer should handle 24-bit BMP");
        assert!(dst.exists(), "output bmp should exist");

        let _ = fs::remove_file(src);
        let _ = fs::remove_file(dst);
    }

    #[test]
    fn split_quote_and_author_extracts_signature_line() {
        let (body, author) = split_quote_and_author("Line one\nLine two\n- Boris");
        assert_eq!(body, "Line one\nLine two");
        assert_eq!(author.as_deref(), Some("Boris"));
    }

    #[test]
    fn rtl_detection_finds_arabic_scripts() {
        assert!(is_rtl_text("مرحبا"));
        assert!(is_rtl_text("ܫܠܡܐ"));
        assert!(!is_rtl_text("Hello world"));
    }

    fn bmp24_solid(width: usize, height: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        let row_stride = (width * 3).div_ceil(4) * 4;
        let pixel_bytes = row_stride * height;
        let file_size = 54 + pixel_bytes;

        let mut out = vec![0_u8; file_size];
        out[0..2].copy_from_slice(b"BM");
        out[2..6].copy_from_slice(&(file_size as u32).to_le_bytes());
        out[10..14].copy_from_slice(&54_u32.to_le_bytes());
        out[14..18].copy_from_slice(&40_u32.to_le_bytes());
        out[18..22].copy_from_slice(&(width as i32).to_le_bytes());
        out[22..26].copy_from_slice(&(height as i32).to_le_bytes());
        out[26..28].copy_from_slice(&1_u16.to_le_bytes());
        out[28..30].copy_from_slice(&24_u16.to_le_bytes());
        out[34..38].copy_from_slice(&(pixel_bytes as u32).to_le_bytes());

        for y in 0..height {
            for x in 0..width {
                let idx = 54 + y * row_stride + x * 3;
                out[idx] = b;
                out[idx + 1] = g;
                out[idx + 2] = r;
            }
        }
        out
    }
}
