use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PreviewText<'a> {
    pub quote: &'a str,
    pub clock: &'a str,
    pub weather: &'a str,
    pub weather_map_image: Option<&'a Path>,
    pub news: &'a str,
    pub news_image: Option<&'a Path>,
    pub news_ticker2: &'a str,
    pub news_ticker2_pos_x: i32,
    pub news_ticker2_pos_y: i32,
    pub news_ticker2_width: u32,
    pub cams: &'a str,
    pub cams_image: Option<&'a Path>,
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
    pub weather_pos_x: i32,
    pub weather_pos_y: i32,
    pub weather_width: u32,
    pub weather_height: u32,
    pub weather_font_size: u32,
    pub weather_font_family: &'a str,
    pub weather_color: &'a str,
    pub weather_undercolor: &'a str,
    pub weather_stroke_color: &'a str,
    pub weather_stroke_width: u32,
    pub news_pos_x: i32,
    pub news_pos_y: i32,
    pub news_width: u32,
    pub news_height: u32,
    pub cams_pos_x: i32,
    pub cams_pos_y: i32,
    pub cams_width: u32,
    pub cams_height: u32,
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
    pub canvas_width: u32,
    pub canvas_height: u32,
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
        "preview_mode = {:?}\nquote = {:?}\nclock = {:?}\nweather = {:?}\nweather_map_image = {:?}\nnews = {:?}\nnews_image = {:?}\nnews_ticker2 = {:?}\nnews_ticker2_pos_x = {}\nnews_ticker2_pos_y = {}\nnews_ticker2_width = {}\ncams = {:?}\ncams_image = {:?}\nquote_font_size = {}\nquote_pos_x = {}\nquote_pos_y = {}\nquote_auto_fit = {}\nquote_min_font_size = {}\nfont_family = {:?}\nquote_color = {:?}\nclock_font_size = {}\nclock_pos_x = {}\nclock_pos_y = {}\nclock_color = {:?}\nweather_pos_x = {}\nweather_pos_y = {}\nweather_width = {}\nweather_height = {}\nweather_font_size = {}\nweather_font_family = {:?}\nweather_color = {:?}\nweather_undercolor = {:?}\nweather_stroke_color = {:?}\nweather_stroke_width = {}\nnews_pos_x = {}\nnews_pos_y = {}\nnews_width = {}\nnews_height = {}\ncams_pos_x = {}\ncams_pos_y = {}\ncams_width = {}\ncams_height = {}\ntext_stroke_color = {:?}\ntext_stroke_width = {}\ntext_undercolor = {:?}\ntext_shadow_enabled = {}\ntext_shadow_color = {:?}\ntext_shadow_offset_x = {}\ntext_shadow_offset_y = {}\ntext_box_size = {:?}\ntext_box_width_pct = {}\ntext_box_height_pct = {}\ncanvas_width = {}\ncanvas_height = {}\nsource_image = {:?}\n",
        render_mode,
        text.quote,
        text.clock,
        text.weather,
        text.weather_map_image
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        text.news,
        text.news_image
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        text.news_ticker2,
        text.news_ticker2_pos_x,
        text.news_ticker2_pos_y,
        text.news_ticker2_width,
        text.cams,
        text.cams_image
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
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
        text.weather_pos_x,
        text.weather_pos_y,
        text.weather_width,
        text.weather_height,
        text.weather_font_size,
        text.weather_font_family,
        text.weather_color,
        text.weather_undercolor,
        text.weather_stroke_color,
        text.weather_stroke_width,
        text.news_pos_x,
        text.news_pos_y,
        text.news_width,
        text.news_height,
        text.cams_pos_x,
        text.cams_pos_y,
        text.cams_width,
        text.cams_height,
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
        text.canvas_width,
        text.canvas_height,
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
    let canvas_w = text.canvas_width.max(1);
    let canvas_h = text.canvas_height.max(1);

    // Build an explicit background layer first so image scaling/placement is independent
    // from the quote box/text layer rendered afterwards.
    args.push("(".to_string());
    args.push(source_image.display().to_string());
    args.push("-resize".to_string());
    args.push(format!("{canvas_w}x{canvas_h}^"));
    args.push("-gravity".to_string());
    args.push("Center".to_string());
    args.push("-extent".to_string());
    args.push(format!("{canvas_w}x{canvas_h}"));
    args.push(")".to_string());

    let (box_w_pct, box_h_pct) = resolve_text_box_pct(text);
    let box_w = (((canvas_w as i32) * box_w_pct as i32) / 100).max(240);
    let box_h = (((canvas_h as i32) * box_h_pct as i32) / 100).max(160);
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

    if !text.weather.trim().is_empty() {
        let weather_size = text.weather_font_size.clamp(10, 220);
        let weather_box_w = text.weather_width.clamp(120, canvas_w.max(120));
        let weather_box_h = text.weather_height.clamp(80, canvas_h.max(80));
        let weather_panel = parse_weather_panel_text(text.weather);

        // Cyber panel background behind weather minimap + metrics.
        args.push("(".to_string());
        args.push("-size".to_string());
        args.push(format!("{}x{}", weather_box_w, weather_box_h));
        args.push("xc:#02131CB8".to_string());
        args.push(")".to_string());
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-geometry".to_string());
        args.push(format!("+{}+{}", text.weather_pos_x, text.weather_pos_y));
        args.push("-composite".to_string());

        let mut weather_text_x = text.weather_pos_x.saturating_add(8);
        let mut weather_text_w = weather_box_w.saturating_sub(16);
        if let Some(map_img) = text.weather_map_image
            && map_img.exists()
        {
            let map_w = (weather_box_w.saturating_mul(48) / 100)
                .max(120)
                .min(weather_box_w.saturating_sub(80));
            let map_h = weather_box_h.saturating_sub(16).max(64);

            args.push("(".to_string());
            args.push(map_img.display().to_string());
            args.push("-auto-orient".to_string());
            args.push("-resize".to_string());
            args.push(format!("{map_w}x{map_h}^"));
            args.push("-gravity".to_string());
            args.push("Center".to_string());
            args.push("-extent".to_string());
            args.push(format!("{map_w}x{map_h}"));
            args.push("-modulate".to_string());
            args.push("110,95,105".to_string());
            args.push(")".to_string());
            args.push("-gravity".to_string());
            args.push("NorthWest".to_string());
            args.push("-geometry".to_string());
            args.push(format!(
                "+{}+{}",
                text.weather_pos_x.saturating_add(8),
                text.weather_pos_y.saturating_add(8)
            ));
            args.push("-composite".to_string());

            weather_text_x = text
                .weather_pos_x
                .saturating_add(map_w as i32)
                .saturating_add(16);
            weather_text_w = weather_box_w.saturating_sub(map_w).saturating_sub(22);

            push_weather_badge(
                &mut args,
                text.weather_pos_x.saturating_add(16),
                text.weather_pos_y
                    .saturating_add(weather_box_h as i32)
                    .saturating_sub(42),
                map_w.saturating_sub(16).max(96),
                26,
                &weather_panel.location,
                "DejaVu-Sans-Mono",
                15,
                "#FF6565",
                "#08161EE6",
            );
        }

        let inner_h = weather_box_h.saturating_sub(16);
        let gap = 8_i32;
        let location_h = ((inner_h as f32) * 0.20).round() as u32;
        let row1_h = ((inner_h as f32) * 0.28).round() as u32;
        let row2_h = ((inner_h as f32) * 0.24).round() as u32;
        let location_h = location_h.clamp(26, 42);
        let row1_h = row1_h.clamp(34, 64);
        let row2_h = row2_h.clamp(30, 56);
        let used_h = location_h
            .saturating_add(row1_h)
            .saturating_add(row2_h)
            .saturating_add((gap * 3) as u32);
        let row3_h = inner_h.saturating_sub(used_h).max(26);
        let col_gap = 8_u32;
        let col_w = weather_text_w
            .saturating_sub(col_gap)
            .saturating_div(2)
            .max(88);
        let right_col_w = weather_text_w.saturating_sub(col_w).saturating_sub(col_gap);
        let tile_bg = "#081A24D8";
        let tile_border = "#133040DD";
        let title_size = ((weather_size as f32) * 0.42).round() as u32;
        let title_size = title_size.clamp(10, 20);
        let value_large = weather_size.clamp(16, 64);
        let value_medium = ((weather_size as f32) * 0.72).round() as u32;
        let value_medium = value_medium.clamp(14, 40);
        let value_small = ((weather_size as f32) * 0.62).round() as u32;
        let value_small = value_small.clamp(12, 32);
        let mut row_y = text.weather_pos_y.saturating_add(8);

        push_weather_badge(
            &mut args,
            weather_text_x,
            row_y,
            weather_text_w,
            location_h,
            &weather_panel.location,
            text.weather_font_family,
            value_small,
            "#DCEBFF",
            "#071B25D9",
        );
        row_y = row_y.saturating_add(location_h as i32).saturating_add(gap);

        push_weather_tile(
            &mut args,
            weather_text_x,
            row_y,
            col_w,
            row1_h,
            &weather_panel.condition,
            &weather_panel.temp,
            text.weather_font_family,
            title_size,
            value_large,
            text.weather_color,
            text.weather_stroke_color,
            text.weather_stroke_width.min(20),
            tile_bg,
            tile_border,
        );
        push_weather_tile(
            &mut args,
            weather_text_x
                .saturating_add(col_w as i32)
                .saturating_add(col_gap as i32),
            row_y,
            right_col_w,
            row1_h,
            "FEELS",
            &weather_panel.feels,
            text.weather_font_family,
            title_size,
            value_large,
            "#FFD88A",
            text.weather_stroke_color,
            text.weather_stroke_width.min(20),
            tile_bg,
            tile_border,
        );
        row_y = row_y.saturating_add(row1_h as i32).saturating_add(gap);

        push_weather_tile(
            &mut args,
            weather_text_x,
            row_y,
            col_w,
            row2_h,
            "RAIN",
            &weather_panel.rain,
            text.weather_font_family,
            title_size,
            value_medium,
            "#8DE6FF",
            text.weather_stroke_color,
            text.weather_stroke_width.min(20),
            tile_bg,
            tile_border,
        );
        push_weather_tile(
            &mut args,
            weather_text_x
                .saturating_add(col_w as i32)
                .saturating_add(col_gap as i32),
            row_y,
            right_col_w,
            row2_h,
            "WIND",
            &weather_panel.wind,
            text.weather_font_family,
            title_size,
            value_medium,
            "#FF7A7A",
            text.weather_stroke_color,
            text.weather_stroke_width.min(20),
            tile_bg,
            tile_border,
        );
        row_y = row_y.saturating_add(row2_h as i32).saturating_add(gap);

        push_weather_tile(
            &mut args,
            weather_text_x,
            row_y,
            weather_text_w,
            row3_h,
            "HUMIDITY",
            &weather_panel.humidity,
            text.weather_font_family,
            title_size,
            value_medium,
            "#9EFFF1",
            text.weather_stroke_color,
            text.weather_stroke_width.min(20),
            tile_bg,
            tile_border,
        );
    }

    if !text.news.trim().is_empty() {
        let news_box_w = text.news_width.clamp(320, canvas_w.max(320));
        let news_box_h = news_box_w.saturating_mul(9) / 16;
        let news_text_h = (text.clock_font_size.saturating_mul(7) / 5).clamp(44, 92);
        if let Some(news_image) = text.news_image
            && news_image.exists()
        {
            args.push("(".to_string());
            args.push(news_image.display().to_string());
            args.push("-auto-orient".to_string());
            args.push("-resize".to_string());
            args.push(format!("{news_box_w}x{news_box_h}^"));
            args.push("-gravity".to_string());
            args.push("Center".to_string());
            args.push("-extent".to_string());
            args.push(format!("{news_box_w}x{news_box_h}"));
            args.push(")".to_string());
            args.push("-gravity".to_string());
            args.push("NorthWest".to_string());
            args.push("-geometry".to_string());
            args.push(format!("+{}+{}", text.news_pos_x, text.news_pos_y));
            args.push("-composite".to_string());
        }

        let news_size = (text.clock_font_size.saturating_mul(58) / 100).max(14);
        let news_line = text.news.replace(['\n', '\r'], " ");
        args.push("(".to_string());
        args.push("-size".to_string());
        args.push(format!("{news_box_w}x{news_text_h}"));
        args.push("xc:none".to_string());
        args.push("-background".to_string());
        args.push("none".to_string());
        args.push("-fill".to_string());
        args.push("#001108D9".to_string());
        args.push("-stroke".to_string());
        args.push("none".to_string());
        args.push("-draw".to_string());
        args.push(format!(
            "rectangle 0,0 {},{}",
            news_box_w.saturating_sub(1),
            news_text_h.saturating_sub(1)
        ));
        args.push("-fill".to_string());
        args.push("#39FF14".to_string());
        args.push("-stroke".to_string());
        args.push("#062200".to_string());
        args.push("-strokewidth".to_string());
        args.push("1".to_string());
        args.push("-undercolor".to_string());
        args.push("#001108D9".to_string());
        args.push("-gravity".to_string());
        args.push("West".to_string());
        args.push("-font".to_string());
        args.push("DejaVu-Sans-Mono".to_string());
        args.push("-pointsize".to_string());
        args.push(news_size.to_string());
        args.push("-annotate".to_string());
        args.push("+12+0".to_string());
        args.push(news_line);
        args.push(")".to_string());
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-geometry".to_string());
        let news_text_y = if text.news_image.is_some() {
            text.news_pos_y
                .saturating_add(news_box_h as i32)
                .saturating_add(8)
        } else {
            text.news_pos_y
        };
        args.push(format!("+{}+{}", text.news_pos_x, news_text_y));
        args.push("-composite".to_string());
    }

    if !text.news_ticker2.trim().is_empty() {
        let ticker_w = text.news_ticker2_width.clamp(220, canvas_w.max(220));
        let ticker_h = (text.clock_font_size.saturating_mul(7) / 5).clamp(44, 92);
        let ticker_size = (text.clock_font_size.saturating_mul(58) / 100).max(14);
        let ticker_line = text.news_ticker2.replace(['\n', '\r'], " ");
        args.push("(".to_string());
        args.push("-size".to_string());
        args.push(format!("{ticker_w}x{ticker_h}"));
        args.push("xc:none".to_string());
        args.push("-background".to_string());
        args.push("none".to_string());
        args.push("-fill".to_string());
        args.push("#001108D9".to_string());
        args.push("-stroke".to_string());
        args.push("none".to_string());
        args.push("-draw".to_string());
        args.push(format!(
            "rectangle 0,0 {},{}",
            ticker_w.saturating_sub(1),
            ticker_h.saturating_sub(1)
        ));
        args.push("-fill".to_string());
        args.push("#39FF14".to_string());
        args.push("-stroke".to_string());
        args.push("#062200".to_string());
        args.push("-strokewidth".to_string());
        args.push("1".to_string());
        args.push("-undercolor".to_string());
        args.push("#001108D9".to_string());
        args.push("-gravity".to_string());
        args.push("West".to_string());
        args.push("-font".to_string());
        args.push("DejaVu-Sans-Mono".to_string());
        args.push("-pointsize".to_string());
        args.push(ticker_size.to_string());
        args.push("-annotate".to_string());
        args.push("+12+0".to_string());
        args.push(ticker_line);
        args.push(")".to_string());
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-geometry".to_string());
        args.push(format!(
            "+{}+{}",
            text.news_ticker2_pos_x, text.news_ticker2_pos_y
        ));
        args.push("-composite".to_string());
    }

    if !text.cams.trim().is_empty() {
        let cams_box_w = text.cams_width.clamp(240, canvas_w.max(240));
        let cams_box_h = text.cams_height.clamp(140, canvas_h.max(140));
        let cams_text_h = (text.clock_font_size.saturating_mul(11) / 10).clamp(36, 88);

        if let Some(cams_image) = text.cams_image
            && cams_image.exists()
        {
            args.push("(".to_string());
            args.push(cams_image.display().to_string());
            args.push("-auto-orient".to_string());
            args.push("-resize".to_string());
            args.push(format!("{cams_box_w}x{cams_box_h}^"));
            args.push("-gravity".to_string());
            args.push("Center".to_string());
            args.push("-extent".to_string());
            args.push(format!("{cams_box_w}x{cams_box_h}"));
            args.push(")".to_string());
            args.push("-gravity".to_string());
            args.push("NorthWest".to_string());
            args.push("-geometry".to_string());
            args.push(format!("+{}+{}", text.cams_pos_x, text.cams_pos_y));
            args.push("-composite".to_string());
        }

        let cams_size = (text.clock_font_size.saturating_mul(55) / 100).max(12);
        let cams_line = text.cams.replace(['\n', '\r'], " ");
        args.push("(".to_string());
        args.push("-size".to_string());
        args.push(format!("{cams_box_w}x{cams_text_h}"));
        args.push("xc:none".to_string());
        args.push("-background".to_string());
        args.push("none".to_string());
        args.push("-fill".to_string());
        args.push("#001108D9".to_string());
        args.push("-stroke".to_string());
        args.push("none".to_string());
        args.push("-draw".to_string());
        args.push(format!(
            "rectangle 0,0 {},{}",
            cams_box_w.saturating_sub(1),
            cams_text_h.saturating_sub(1)
        ));
        args.push("-fill".to_string());
        args.push("#37FF12".to_string());
        args.push("-stroke".to_string());
        args.push("#062200".to_string());
        args.push("-strokewidth".to_string());
        args.push("1".to_string());
        args.push("-undercolor".to_string());
        args.push("#001108D9".to_string());
        args.push("-gravity".to_string());
        args.push("West".to_string());
        args.push("-font".to_string());
        args.push("DejaVu-Sans-Mono".to_string());
        args.push("-pointsize".to_string());
        args.push(cams_size.to_string());
        args.push("-annotate".to_string());
        args.push("+10+0".to_string());
        args.push(cams_line);
        args.push(")".to_string());
        args.push("-gravity".to_string());
        args.push("NorthWest".to_string());
        args.push("-geometry".to_string());
        args.push(format!(
            "+{}+{}",
            text.cams_pos_x,
            text.cams_pos_y
                .saturating_add(cams_box_h as i32)
                .saturating_add(8)
        ));
        args.push("-composite".to_string());
    }

    args.push(output_image.display().to_string());

    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;

    Ok(status.success())
}

#[derive(Debug, Default, Clone)]
struct WeatherPanelText {
    location: String,
    condition: String,
    temp: String,
    feels: String,
    rain: String,
    wind: String,
    humidity: String,
}

fn parse_weather_panel_text(input: &str) -> WeatherPanelText {
    let lines = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let location = lines.first().copied().unwrap_or("Weather").to_string();
    let condition_line = lines.get(1).copied().unwrap_or_default();
    let wind_line = lines.get(2).copied().unwrap_or_default();
    let humidity_line = lines.get(3).copied().unwrap_or_default();

    let (condition, temp, feels) = parse_condition_line(condition_line);
    let (rain, wind) = parse_wind_line(wind_line);
    let humidity = humidity_line
        .strip_prefix("Humidity ")
        .unwrap_or(humidity_line)
        .trim()
        .to_string();

    WeatherPanelText {
        location,
        condition,
        temp,
        feels,
        rain,
        wind,
        humidity: if humidity.is_empty() {
            "--".to_string()
        } else {
            humidity
        },
    }
}

fn parse_condition_line(line: &str) -> (String, String, String) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ("NOW".to_string(), "--".to_string(), "--".to_string());
    }

    let (left, feels) = if let Some((lhs, rhs)) = trimmed.split_once(" feels ") {
        (lhs.trim(), rhs.trim())
    } else {
        (trimmed, "--")
    };
    let mut tokens = left.split_whitespace().collect::<Vec<_>>();
    let temp = tokens.pop().unwrap_or("--").to_string();
    let condition = if tokens.is_empty() {
        "NOW".to_string()
    } else {
        tokens.join(" ")
    };
    let feels = if feels.is_empty() {
        "--".to_string()
    } else {
        feels.to_string()
    };
    (condition, temp, feels)
}

fn parse_wind_line(line: &str) -> (String, String) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ("--".to_string(), "--".to_string());
    }
    if let Some((rain_part, wind_part)) = trimmed.split_once("Wind ") {
        let rain = rain_part
            .trim()
            .strip_prefix("Rain ")
            .unwrap_or(rain_part.trim())
            .trim();
        let wind = wind_part.trim();
        return (
            if rain.is_empty() {
                "--".to_string()
            } else {
                rain.to_string()
            },
            if wind.is_empty() {
                "--".to_string()
            } else {
                wind.to_string()
            },
        );
    }
    ("--".to_string(), trimmed.to_string())
}

#[allow(clippy::too_many_arguments)]
fn push_weather_badge(
    args: &mut Vec<String>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    value: &str,
    font: &str,
    size: u32,
    fill: &str,
    background: &str,
) {
    if value.trim().is_empty() {
        return;
    }
    args.push("(".to_string());
    args.push("-size".to_string());
    args.push(format!("{}x{}", w.max(32), h.max(18)));
    args.push("xc:none".to_string());
    args.push("-background".to_string());
    args.push("none".to_string());
    args.push("-fill".to_string());
    args.push(background.to_string());
    args.push("-stroke".to_string());
    args.push("none".to_string());
    args.push("-draw".to_string());
    args.push(format!(
        "roundrectangle 0,0 {},{} 10,10",
        w.saturating_sub(1),
        h.saturating_sub(1)
    ));
    args.push("-fill".to_string());
    args.push(fill.to_string());
    args.push("-stroke".to_string());
    args.push("none".to_string());
    args.push("-gravity".to_string());
    args.push("West".to_string());
    args.push("-font".to_string());
    args.push(font.to_string());
    args.push("-pointsize".to_string());
    args.push(size.max(10).to_string());
    args.push("-annotate".to_string());
    args.push("+12+0".to_string());
    args.push(value.replace(['\n', '\r'], " "));
    args.push(")".to_string());
    args.push("-gravity".to_string());
    args.push("NorthWest".to_string());
    args.push("-geometry".to_string());
    args.push(format!("+{}+{}", x, y));
    args.push("-composite".to_string());
}

#[allow(clippy::too_many_arguments)]
fn push_weather_tile(
    args: &mut Vec<String>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    title: &str,
    value: &str,
    font: &str,
    title_size: u32,
    value_size: u32,
    fill: &str,
    stroke: &str,
    stroke_width: u32,
    background: &str,
    border: &str,
) {
    let clean_value = if value.trim().is_empty() {
        "--".to_string()
    } else {
        value.replace(['\n', '\r'], " ")
    };
    args.push("(".to_string());
    args.push("-size".to_string());
    args.push(format!("{}x{}", w.max(48), h.max(24)));
    args.push("xc:none".to_string());
    args.push("-background".to_string());
    args.push("none".to_string());
    args.push("-fill".to_string());
    args.push(background.to_string());
    args.push("-stroke".to_string());
    args.push(border.to_string());
    args.push("-strokewidth".to_string());
    args.push("1".to_string());
    args.push("-draw".to_string());
    args.push(format!(
        "roundrectangle 0,0 {},{} 12,12",
        w.saturating_sub(1),
        h.saturating_sub(1)
    ));
    args.push("-fill".to_string());
    args.push("#93A9BA".to_string());
    args.push("-stroke".to_string());
    args.push("none".to_string());
    args.push("-gravity".to_string());
    args.push("NorthWest".to_string());
    args.push("-font".to_string());
    args.push("DejaVu-Sans-Mono".to_string());
    args.push("-pointsize".to_string());
    args.push(title_size.max(10).to_string());
    args.push("-annotate".to_string());
    args.push("+12+10".to_string());
    args.push(title.to_string());
    args.push("-fill".to_string());
    args.push(fill.to_string());
    args.push("-stroke".to_string());
    args.push(stroke.to_string());
    args.push("-strokewidth".to_string());
    args.push(stroke_width.max(1).to_string());
    args.push("-gravity".to_string());
    args.push("SouthWest".to_string());
    args.push("-font".to_string());
    args.push(font.to_string());
    args.push("-pointsize".to_string());
    args.push(value_size.max(12).to_string());
    args.push("-annotate".to_string());
    args.push("+12+10".to_string());
    args.push(clean_value);
    args.push(")".to_string());
    args.push("-gravity".to_string());
    args.push("NorthWest".to_string());
    args.push("-geometry".to_string());
    args.push(format!("+{}+{}", x, y));
    args.push("-composite".to_string());
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
    draw_text(
        &mut bmp,
        text.weather_pos_x,
        text.weather_pos_y,
        text.weather,
        (clock_scale / 2).max(1),
        Rgb {
            r: 0,
            g: 245,
            b: 255,
        },
    );
    draw_text(
        &mut bmp,
        text.news_pos_x,
        text.news_pos_y,
        text.news,
        (clock_scale / 2).max(1),
        Rgb {
            r: 255,
            g: 92,
            b: 243,
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
                weather: "Belgrade 13C clear",
                weather_map_image: None,
                news: "LIVE: Euronews",
                news_image: None,
                news_ticker2: "▮ second ticker ▮",
                news_ticker2_pos_x: 30,
                news_ticker2_pos_y: 960,
                news_ticker2_width: 1200,
                cams: "CAMS ◆ Downtown",
                cams_image: None,
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
                weather_pos_x: 40,
                weather_pos_y: 40,
                weather_width: 640,
                weather_height: 180,
                weather_font_size: 30,
                weather_font_family: "DejaVu-Sans",
                weather_color: "#00F5FF",
                weather_undercolor: "#0B0014B3",
                weather_stroke_color: "#001A22",
                weather_stroke_width: 1,
                news_pos_x: 40,
                news_pos_y: 90,
                news_width: 760,
                news_height: 240,
                cams_pos_x: 980,
                cams_pos_y: 640,
                cams_width: 760,
                cams_height: 428,
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
                canvas_width: 1920,
                canvas_height: 1080,
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
                weather: "Rain 4C",
                weather_map_image: None,
                news: "LIVE",
                news_image: None,
                news_ticker2: "T2",
                news_ticker2_pos_x: 2,
                news_ticker2_pos_y: 98,
                news_ticker2_width: 300,
                cams: "CAMS",
                cams_image: None,
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
                weather_pos_x: 2,
                weather_pos_y: 40,
                weather_width: 640,
                weather_height: 180,
                weather_font_size: 30,
                weather_font_family: "DejaVu-Sans",
                weather_color: "#00F5FF",
                weather_undercolor: "#0B0014B3",
                weather_stroke_color: "#001A22",
                weather_stroke_width: 1,
                news_pos_x: 2,
                news_pos_y: 58,
                news_width: 760,
                news_height: 240,
                cams_pos_x: 2,
                cams_pos_y: 90,
                cams_width: 320,
                cams_height: 180,
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
                canvas_width: 1920,
                canvas_height: 1080,
            },
        )
        .expect("native bmp render should return status");

        assert!(ok, "native renderer should handle 24-bit BMP");
        assert!(dst.exists(), "output bmp should exist");

        let _ = fs::remove_file(src);
        let _ = fs::remove_file(dst);
    }

    #[test]
    fn native_bmp_overlay_output_hash_is_stable() {
        let src = std::env::temp_dir().join("wc-render-native-hash-src.bmp");
        let dst = std::env::temp_dir().join("wc-render-native-hash-dst.bmp");
        fs::write(&src, bmp24_solid(64, 48, 12, 22, 32)).expect("bmp source should be writable");

        let ok = render_with_native_bmp(
            &src,
            &dst,
            &PreviewText {
                quote: "SNAPSHOT",
                clock: "09:42",
                weather: "☀ 14C",
                weather_map_image: None,
                news: "NEWS-LINE",
                news_image: None,
                news_ticker2: "T2",
                news_ticker2_pos_x: 4,
                news_ticker2_pos_y: 96,
                news_ticker2_width: 320,
                cams: "CAMS",
                cams_image: None,
                quote_font_size: 22,
                quote_pos_x: 3,
                quote_pos_y: 3,
                quote_auto_fit: true,
                quote_min_font_size: 14,
                font_family: "DejaVu-Sans",
                quote_color: "#FFFFFF",
                clock_font_size: 22,
                clock_pos_x: 3,
                clock_pos_y: 22,
                clock_color: "#FFD700",
                weather_pos_x: 3,
                weather_pos_y: 41,
                weather_width: 640,
                weather_height: 180,
                weather_font_size: 30,
                weather_font_family: "DejaVu-Sans",
                weather_color: "#00F5FF",
                weather_undercolor: "#0B0014B3",
                weather_stroke_color: "#001A22",
                weather_stroke_width: 1,
                news_pos_x: 3,
                news_pos_y: 58,
                news_width: 760,
                news_height: 240,
                cams_pos_x: 3,
                cams_pos_y: 90,
                cams_width: 320,
                cams_height: 180,
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
                canvas_width: 1920,
                canvas_height: 1080,
            },
        )
        .expect("native bmp render should return status");

        assert!(ok, "native renderer should handle 24-bit BMP");
        let bytes = fs::read(&dst).expect("output bmp should be readable");
        let hash = fnv1a64(&bytes);
        assert_eq!(
            hash, 0x4a45_10d8_14e1_50fb,
            "native bmp output changed; update baseline intentionally if expected"
        );

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

    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for b in bytes {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}
