use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use reqwest::blocking::Client;
use serde_json::Value;
use std::fs;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};
use wc_backend::apply_wallpaper;
use wc_core::{
    AppConfig, build_doctor_report, builtin_image_presets, builtin_quote_presets, cycle_index,
    default_config_path, default_config_toml, expand_tilde, image_preset_endpoint,
    list_background_images, load_config, load_quotes, pick_background_image_with_mode,
    pick_quote_with_mode, presets_catalog_json, quote_preset_endpoint, settings_schema_json,
    settings_ui_blueprint_json, to_config_toml,
};
use wc_render::{PreviewText, render_preview_to_file};
use wc_source::{ImageProvider, QuoteProvider, fetch_remote_image, fetch_remote_quote};

const MAX_STORED_HISTORY: usize = 64;
const BUNDLED_LOCAL_QUOTES: &str = include_str!("../../../assets/quotes/local/local-quotes.md");
const WEATHER_GEO_CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
const WEATHER_PAYLOAD_CACHE_MAX_AGE_SECS: u64 = 6 * 60 * 60;

#[derive(Debug, Parser)]
#[command(name = "wc-cli")]
#[command(about = "Le Compositeur CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print local environment data for debugging.
    Doctor,
    /// Render a preview image from config and save it to output_image.
    RenderPreview {
        /// Config path. Defaults to ~/.config/wallpaper-composer/config.toml
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Create a starter config file.
    Init {
        /// Config target path. Defaults to ~/.config/wallpaper-composer/config.toml
        #[arg(long)]
        config: Option<PathBuf>,
        /// Overwrite an existing config file.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Validate config values and source/backend requirements.
    Validate {
        /// Config path. Defaults to ~/.config/wallpaper-composer/config.toml
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Run preview generation in a loop using refresh_seconds from config.
    Run {
        /// Config path. Defaults to ~/.config/wallpaper-composer/config.toml
        #[arg(long)]
        config: Option<PathBuf>,
        /// Run exactly one cycle and exit.
        #[arg(long, default_value_t = false)]
        once: bool,
        /// Replace an already running loop process for the same config path.
        #[arg(long, default_value_t = false)]
        replace_existing: bool,
    },
    /// Show built-in public source presets for future GUI/settings integration.
    Presets,
    /// Print structured preset catalog JSON for GUI dropdown population.
    PresetCatalog,
    /// Print machine-readable settings schema (for GUI generators).
    ExportSchema,
    /// Print a UI-oriented settings blueprint (sections + ordering + conditions).
    UiBlueprint,
    /// Rewrite config to the latest normalized format and create a backup.
    Migrate {
        /// Config path. Defaults to ~/.config/wallpaper-composer/config.toml
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => {
            let report = build_doctor_report();
            println!("project: {}", report.project);
            println!("profile: {}", report.profile);
            println!("local_time: {}", report.local_time);
        }
        Commands::RenderPreview { config } => {
            let config_path = resolve_config_path(config)?;
            let cfg = load_config(&config_path)?;
            validate_config(&cfg)?;
            let cycle = determine_cycle(&cfg, master_rotation_interval(&cfg), "rotation")?;
            run_cycle(&cfg, cycle, cycle)?;
        }
        Commands::Init { config, force } => {
            let config_path = resolve_config_path(config)?;
            if config_path.exists() && !force {
                anyhow::bail!(
                    "config already exists at {} (use --force to overwrite)",
                    config_path.display()
                );
            }

            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&config_path, default_config_toml())?;
            println!("created config: {}", config_path.display());
            if let Some(quotes_path) = ensure_default_local_quotes(&config_path)? {
                println!("created local quotes: {}", quotes_path.display());
            }
        }
        Commands::Validate { config } => {
            let config_path = resolve_config_path(config)?;
            let cfg = load_config(&config_path)?;
            validate_config(&cfg)?;
            println!("config_valid: {}", config_path.display());
        }
        Commands::Run {
            config,
            once,
            replace_existing,
        } => {
            let config_path = resolve_config_path(config)?;
            let _run_lock = if once {
                None
            } else {
                Some(acquire_run_lock(&config_path, replace_existing)?)
            };
            loop {
                let tick_started = Instant::now();
                let cfg = load_config(&config_path)?;
                validate_config(&cfg)?;
                let cycle = determine_cycle(&cfg, master_rotation_interval(&cfg), "rotation")?;
                run_cycle(&cfg, cycle, cycle)?;

                if once {
                    break;
                }
                let desired_tick = loop_tick_duration(&cfg);
                let spent = tick_started.elapsed();
                if spent < desired_tick {
                    thread::sleep(desired_tick - spent);
                }
            }
        }
        Commands::Presets => {
            print_presets();
        }
        Commands::PresetCatalog => {
            println!("{}", presets_catalog_json());
        }
        Commands::ExportSchema => {
            println!("{}", settings_schema_json());
        }
        Commands::UiBlueprint => {
            println!("{}", settings_ui_blueprint_json());
        }
        Commands::Migrate { config } => {
            let config_path = resolve_config_path(config)?;
            let cfg = load_config(&config_path)?;
            let backup_path = backup_path_for(&config_path);
            fs::copy(&config_path, &backup_path)?;
            fs::write(&config_path, to_config_toml(&cfg))?;
            println!("migrated_config: {}", config_path.display());
            println!("backup_created: {}", backup_path.display());
        }
    }

    Ok(())
}

fn run_cycle(cfg: &AppConfig, image_cycle: u64, quote_cycle: u64) -> Result<()> {
    let output_path = expand_tilde(&cfg.output_image)?;
    let source_image = resolve_source_image(cfg, image_cycle)?;
    let quote = if cfg.show_quote_layer {
        resolve_quote(cfg, quote_cycle)?
    } else {
        String::new()
    };
    let clock = if cfg.show_clock_layer {
        chrono::Local::now().format(&cfg.time_format).to_string()
    } else {
        String::new()
    };
    let weather_payload = if cfg.show_weather_layer {
        resolve_weather_widget(cfg).unwrap_or_else(|e| WeatherWidgetPayload {
            text: format!(
                "⚠ {}",
                compact_news_line(&format!("weather unavailable ({e})"))
            ),
            minimap_image: None,
        })
    } else {
        WeatherWidgetPayload {
            text: String::new(),
            minimap_image: None,
        }
    };
    let weather = weather_payload.text.clone();
    let news_payload = if cfg.show_news_layer {
        resolve_news_widget(cfg, image_cycle).unwrap_or_else(|e| NewsWidgetPayload {
            text: format!("News unavailable ({e})"),
            preview_image: None,
        })
    } else {
        NewsWidgetPayload {
            text: String::new(),
            preview_image: None,
        }
    };
    let news = news_payload.text.clone();
    let cams_payload = if cfg.show_cams_layer {
        resolve_cams_widget(cfg, image_cycle).unwrap_or_else(|e| CamsWidgetPayload {
            text: format!("CAMS ◆ unavailable ({e})"),
            preview_image: None,
        })
    } else {
        CamsWidgetPayload {
            text: String::new(),
            preview_image: None,
        }
    };
    let cams = cams_payload.text.clone();
    let (canvas_width, canvas_height) = detect_canvas_size();
    let (image_pool_size, quote_pool_size) = detect_local_pool_sizes(cfg);

    ensure_parent_dir(&output_path)?;
    let render = render_preview_to_file(
        &source_image,
        &output_path,
        PreviewText {
            quote: &quote,
            clock: &clock,
            weather: &weather,
            weather_map_image: weather_payload.minimap_image.as_deref(),
            news: &news,
            news_image: news_payload.preview_image.as_deref(),
            cams: &cams,
            cams_image: cams_payload.preview_image.as_deref(),
            quote_font_size: cfg.quote_font_size,
            quote_pos_x: cfg.quote_pos_x,
            quote_pos_y: cfg.quote_pos_y,
            quote_auto_fit: cfg.quote_auto_fit,
            quote_min_font_size: cfg.quote_min_font_size,
            font_family: &cfg.font_family,
            quote_color: &cfg.quote_color,
            clock_font_size: cfg.clock_font_size,
            clock_pos_x: cfg.clock_pos_x,
            clock_pos_y: cfg.clock_pos_y,
            clock_color: &cfg.clock_color,
            weather_pos_x: cfg.weather_pos_x,
            weather_pos_y: cfg.weather_pos_y,
            weather_width: cfg.weather_widget_width,
            weather_height: cfg.weather_widget_height,
            weather_font_size: cfg.weather_font_size,
            weather_font_family: &cfg.weather_font_family,
            weather_color: &cfg.weather_color,
            weather_undercolor: &cfg.weather_undercolor,
            weather_stroke_color: &cfg.weather_stroke_color,
            weather_stroke_width: cfg.weather_stroke_width,
            news_pos_x: cfg.news_pos_x,
            news_pos_y: cfg.news_pos_y,
            news_width: cfg.news_widget_width,
            news_height: cfg.news_widget_height,
            cams_pos_x: cfg.cams_pos_x,
            cams_pos_y: cfg.cams_pos_y,
            cams_width: cfg.cams_widget_width,
            cams_height: cfg.cams_widget_height,
            text_stroke_color: &cfg.text_stroke_color,
            text_stroke_width: cfg.text_stroke_width,
            text_undercolor: &cfg.text_undercolor,
            text_shadow_enabled: cfg.text_shadow_enabled,
            text_shadow_color: &cfg.text_shadow_color,
            text_shadow_offset_x: cfg.text_shadow_offset_x,
            text_shadow_offset_y: cfg.text_shadow_offset_y,
            text_box_size: &cfg.text_box_size,
            text_box_width_pct: cfg.text_box_width_pct,
            text_box_height_pct: cfg.text_box_height_pct,
            canvas_width,
            canvas_height,
        },
    )
    .map_err(anyhow::Error::msg)?;

    println!("image_cycle: {}", image_cycle);
    println!("quote_cycle: {}", quote_cycle);
    println!("source_image: {}", source_image.display());
    if let Some(count) = image_pool_size {
        println!("image_pool_size: {}", count);
    }
    if let Some(count) = quote_pool_size {
        println!("quote_pool_size: {}", count);
    }
    println!("quote: {}", quote);
    println!("clock: {}", clock);
    println!("weather: {}", weather);
    println!("news: {}", news);
    println!("cams: {}", cams);
    if let Some(path) = &news_payload.preview_image {
        println!("news_preview_image: {}", path.display());
    }
    if let Some(path) = &cams_payload.preview_image {
        println!("cams_preview_image: {}", path.display());
    }
    println!("canvas: {}x{}", canvas_width, canvas_height);
    println!("preview_mode: {}", render.preview_mode);
    println!("preview_output: {}", output_path.display());
    println!("preview_metadata: {}", render.meta_path.display());

    let effective_apply_wallpaper = cfg.apply_wallpaper && cfg.show_background_layer;
    let wallpaper_target = if effective_apply_wallpaper {
        prepare_wallpaper_apply_target(cfg, &output_path)?
    } else {
        output_path.clone()
    };
    let apply_status = apply_wallpaper(
        &cfg.wallpaper_backend,
        &cfg.wallpaper_fit_mode,
        effective_apply_wallpaper,
        &wallpaper_target,
    )
    .map_err(anyhow::Error::msg)?;
    if effective_apply_wallpaper {
        println!("wallpaper_target: {}", wallpaper_target.display());
    }
    println!("wallpaper_apply: {}", apply_status);

    Ok(())
}

fn detect_local_pool_sizes(cfg: &AppConfig) -> (Option<usize>, Option<usize>) {
    let image_count = if cfg.image_source.trim().eq_ignore_ascii_case("local") {
        expand_tilde(&cfg.image_dir)
            .ok()
            .and_then(|dir| list_background_images(&dir).ok().map(|v| v.len()))
    } else {
        None
    };
    let quote_count = if cfg.quote_source.trim().eq_ignore_ascii_case("local") {
        expand_tilde(&cfg.quotes_path)
            .ok()
            .and_then(|path| load_quotes(&path).ok().map(|v| v.len()))
    } else {
        None
    };
    (image_count, quote_count)
}

fn validate_config(cfg: &AppConfig) -> Result<()> {
    match cfg.image_source.trim().to_ascii_lowercase().as_str() {
        "local" => {
            let image_dir = expand_tilde(&cfg.image_dir)?;
            if !image_dir.exists() {
                anyhow::bail!("image_dir does not exist: {}", image_dir.display());
            }
            if !image_dir.is_dir() {
                anyhow::bail!("image_dir is not a directory: {}", image_dir.display());
            }
        }
        "preset" | "remote_preset" => {
            let id = cfg.image_source_preset.as_deref().ok_or_else(|| {
                anyhow::anyhow!("image_source_preset is required for image_source=preset")
            })?;
            if image_preset_endpoint(id).is_none() {
                anyhow::bail!("unknown image_source_preset: {id}");
            }
        }
        "url" | "remote_url" => {
            if cfg.image_source_url.as_deref().is_none_or(|v| {
                !parse_endpoint_list(v)
                    .into_iter()
                    .any(|u| looks_like_endpoint(&u))
            }) {
                anyhow::bail!("image_source_url is required for image_source=url");
            }
        }
        other => anyhow::bail!("unsupported image_source={other}; use local, preset, or url"),
    }

    match cfg.quote_source.trim().to_ascii_lowercase().as_str() {
        "local" => {
            let quotes_path = expand_tilde(&cfg.quotes_path)?;
            if !quotes_path.exists() {
                anyhow::bail!("quotes_path does not exist: {}", quotes_path.display());
            }
            if !quotes_path.is_file() {
                anyhow::bail!("quotes_path is not a file: {}", quotes_path.display());
            }
        }
        "preset" | "remote_preset" => {
            let id = cfg.quote_source_preset.as_deref().ok_or_else(|| {
                anyhow::anyhow!("quote_source_preset is required for quote_source=preset")
            })?;
            if quote_preset_endpoint(id).is_none() {
                anyhow::bail!("unknown quote_source_preset: {id}");
            }
        }
        "url" | "remote_url" => {
            if cfg.quote_source_url.as_deref().is_none_or(|v| {
                !parse_endpoint_list(v)
                    .into_iter()
                    .any(|u| looks_like_endpoint(&u))
            }) {
                anyhow::bail!("quote_source_url is required for quote_source=url");
            }
        }
        other => anyhow::bail!("unsupported quote_source={other}; use local, preset, or url"),
    }

    if cfg.image_refresh_seconds == 0 {
        anyhow::bail!("image_refresh_seconds must be greater than 0");
    }
    if cfg.weather_refresh_seconds < 60 {
        anyhow::bail!("weather_refresh_seconds must be >= 60");
    }
    if !(0.05..=30.0).contains(&cfg.news_fps) {
        anyhow::bail!("news_fps must be between 0.05 and 30.0");
    }
    if cfg.news_source.trim().eq_ignore_ascii_case("custom")
        && cfg.news_custom_url.trim().is_empty()
    {
        anyhow::bail!("news_custom_url is required when news_source=custom");
    }
    if is_camera_like_url(cfg.news_custom_url.trim()) && cfg.news_fps > 1.0 {
        anyhow::bail!("camera-like custom news URLs are limited to max 1.0 FPS");
    }
    if !(120..=1920).contains(&cfg.weather_widget_width) {
        anyhow::bail!("weather_widget_width must be between 120 and 1920");
    }
    if !(80..=1080).contains(&cfg.weather_widget_height) {
        anyhow::bail!("weather_widget_height must be between 80 and 1080");
    }
    if !(10..=220).contains(&cfg.weather_font_size) {
        anyhow::bail!("weather_font_size must be between 10 and 220");
    }
    if cfg.weather_stroke_width > 20 {
        anyhow::bail!("weather_stroke_width must be <= 20");
    }
    if cfg.weather_font_family.trim().is_empty() {
        anyhow::bail!("weather_font_family must be non-empty");
    }
    if !(180..=1920).contains(&cfg.news_widget_width) {
        anyhow::bail!("news_widget_width must be between 180 and 1920");
    }
    if !(120..=1080).contains(&cfg.news_widget_height) {
        anyhow::bail!("news_widget_height must be between 120 and 1080");
    }

    let backend = cfg.wallpaper_backend.trim().to_ascii_lowercase();
    if !["auto", "noop", "gnome", "sway", "feh"].contains(&backend.as_str()) {
        anyhow::bail!(
            "unsupported wallpaper_backend={}; use auto, noop, gnome, sway, or feh",
            cfg.wallpaper_backend
        );
    }

    if cfg.quote_font_size < 8 || cfg.clock_font_size < 8 {
        anyhow::bail!("quote_font_size and clock_font_size must be >= 8");
    }
    if cfg.quote_min_font_size < 8 {
        anyhow::bail!("quote_min_font_size must be >= 8");
    }
    if cfg.quote_min_font_size > cfg.quote_font_size {
        anyhow::bail!("quote_min_font_size must be <= quote_font_size");
    }
    if cfg.text_stroke_width > 20 {
        anyhow::bail!("text_stroke_width must be <= 20");
    }
    let text_box_size = cfg.text_box_size.trim().to_ascii_lowercase();
    if !["quarter", "third", "half", "full", "custom"].contains(&text_box_size.as_str()) {
        anyhow::bail!(
            "unsupported text_box_size={}; use quarter, third, half, full, or custom",
            cfg.text_box_size
        );
    }
    if cfg.text_box_width_pct < 10 || cfg.text_box_width_pct > 100 {
        anyhow::bail!("text_box_width_pct must be between 10 and 100");
    }
    if cfg.text_box_height_pct < 10 || cfg.text_box_height_pct > 100 {
        anyhow::bail!("text_box_height_pct must be between 10 and 100");
    }
    if cfg.quote_color.trim().is_empty()
        || cfg.clock_color.trim().is_empty()
        || cfg.weather_color.trim().is_empty()
        || cfg.weather_undercolor.trim().is_empty()
        || cfg.weather_stroke_color.trim().is_empty()
        || cfg.text_stroke_color.trim().is_empty()
        || cfg.text_undercolor.trim().is_empty()
        || cfg.text_shadow_color.trim().is_empty()
        || cfg.font_family.trim().is_empty()
    {
        anyhow::bail!(
            "font_family, quote_color, clock_color, weather style colors, text_stroke_color, text_undercolor and text_shadow_color must be non-empty"
        );
    }
    let fit_mode = cfg.wallpaper_fit_mode.trim().to_ascii_lowercase();
    if ![
        "zoom",
        "scaled",
        "stretched",
        "spanned",
        "centered",
        "wallpaper",
        "tiled",
    ]
    .contains(&fit_mode.as_str())
    {
        anyhow::bail!(
            "unsupported wallpaper_fit_mode={}; use zoom, scaled, stretched, spanned, centered, wallpaper, or tiled",
            cfg.wallpaper_fit_mode
        );
    }
    let image_order_mode = cfg.image_order_mode.trim().to_ascii_lowercase();
    if !["sequential", "random"].contains(&image_order_mode.as_str()) {
        anyhow::bail!(
            "unsupported image_order_mode={}; use sequential or random",
            cfg.image_order_mode
        );
    }
    let quote_order_mode = cfg.quote_order_mode.trim().to_ascii_lowercase();
    if !["sequential", "random"].contains(&quote_order_mode.as_str()) {
        anyhow::bail!(
            "unsupported quote_order_mode={}; use sequential or random",
            cfg.quote_order_mode
        );
    }

    Ok(())
}

fn determine_cycle(cfg: &AppConfig, interval_seconds: u64, stream: &str) -> Result<u64> {
    let base_cycle = cycle_index(interval_seconds);
    if !cfg.rotation_use_persistent_state {
        return Ok(base_cycle);
    }

    let state_path = expand_tilde(&format!("{}.{}", cfg.rotation_state_file, stream))?;
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let now = now_epoch_seconds();
    let (mut current_cycle, mut last_ts) = read_cycle_state(&state_path)
        .map(|(cycle, ts)| (cycle.max(base_cycle), ts))
        .unwrap_or((base_cycle, now));

    if now > last_ts {
        let elapsed = now - last_ts;
        let steps = elapsed / interval_seconds.max(1);
        if steps > 0 {
            current_cycle = current_cycle.saturating_add(steps).max(base_cycle);
            last_ts = last_ts.saturating_add(steps * interval_seconds.max(1));
        }
    }

    write_cycle_state(&state_path, current_cycle, last_ts)?;
    Ok(current_cycle.max(base_cycle))
}

fn read_last_cycle(path: &Path) -> Option<u64> {
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse::<u64>().ok()
}

fn read_cycle_state(path: &Path) -> Option<(u64, u64)> {
    let raw = fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    let Some((cycle, ts)) = trimmed.split_once(',') else {
        return read_last_cycle(path).map(|cycle| (cycle, now_epoch_seconds()));
    };
    let cycle = cycle.trim().parse::<u64>().ok()?;
    let ts = ts.trim().parse::<u64>().ok()?;
    Some((cycle, ts))
}

fn write_cycle_state(path: &Path, cycle: u64, epoch_ts: u64) -> Result<()> {
    fs::write(path, format!("{cycle},{epoch_ts}\n"))?;
    Ok(())
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn detect_canvas_size() -> (u32, u32) {
    if let Some(size) = detect_resolution_via_xrandr() {
        return size;
    }
    if let Some(size) = detect_resolution_via_xdpyinfo() {
        return size;
    }
    (1920, 1080)
}

fn detect_resolution_via_xrandr() -> Option<(u32, u32)> {
    let out = Command::new("xrandr").arg("--current").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    for line in raw.lines() {
        if !line.contains('*') {
            continue;
        }
        for token in line.split_whitespace() {
            if let Some((w, h)) = parse_resolution_token(token) {
                return Some((w, h));
            }
        }
    }
    None
}

fn detect_resolution_via_xdpyinfo() -> Option<(u32, u32)> {
    let out = Command::new("xdpyinfo").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("dimensions:") {
            continue;
        }
        for token in trimmed.split_whitespace() {
            if let Some((w, h)) = parse_resolution_token(token) {
                return Some((w, h));
            }
        }
    }
    None
}

fn parse_resolution_token(token: &str) -> Option<(u32, u32)> {
    let clean = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != 'x');
    let (w_raw, h_raw) = clean.split_once('x')?;
    let w = w_raw.parse::<u32>().ok()?;
    let h = h_raw.parse::<u32>().ok()?;
    if w >= 320 && h >= 200 {
        Some((w, h))
    } else {
        None
    }
}

fn resolve_source_image(cfg: &AppConfig, cycle: u64) -> Result<PathBuf> {
    match cfg.image_source.trim().to_ascii_lowercase().as_str() {
        "local" => {
            let image_dir = expand_tilde(&cfg.image_dir)?;
            let state_path = image_pick_state_path(cfg)?;
            let recent_indices = read_recent_indices(&state_path);
            let (picked, picked_idx) = pick_background_image_with_mode(
                &image_dir,
                cycle,
                &cfg.image_order_mode,
                cfg.image_avoid_repeat,
                &recent_indices,
            )?;
            write_recent_indices(&state_path, &recent_indices, picked_idx)?;
            Ok(picked)
        }
        "preset" | "remote_preset" => {
            let (endpoint, provider) = resolve_image_endpoint_from_preset(cfg)?;
            fetch_remote_image(endpoint, provider)
                .map_err(|e| anyhow::anyhow!("failed to fetch preset image source: {e}"))
        }
        "url" | "remote_url" => {
            let endpoint = resolve_image_endpoint_from_url(cfg, cycle)?;
            fetch_remote_image(endpoint, ImageProvider::Generic)
                .map_err(|e| anyhow::anyhow!("failed to fetch custom image source: {e}"))
        }
        other => Err(anyhow::anyhow!(
            "unsupported image_source={other}; supported: local, preset, url"
        )),
    }
}

fn image_pick_state_path(cfg: &AppConfig) -> Result<PathBuf> {
    let path = expand_tilde(&format!("{}.image_pick", cfg.rotation_state_file))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn resolve_quote(cfg: &AppConfig, cycle: u64) -> Result<String> {
    let raw = match cfg.quote_source.trim().to_ascii_lowercase().as_str() {
        "local" => {
            let quotes_path = expand_tilde(&cfg.quotes_path)?;
            let state_path = quote_pick_state_path(cfg)?;
            let recent_indices = read_recent_indices(&state_path);
            let (picked, picked_idx) = pick_quote_with_mode(
                &quotes_path,
                cycle,
                &cfg.quote_order_mode,
                cfg.quote_avoid_repeat,
                &recent_indices,
            )?;
            write_recent_indices(&state_path, &recent_indices, picked_idx)?;
            Ok(picked)
        }
        "preset" | "remote_preset" => {
            let (endpoint, provider) = resolve_quote_endpoint_from_preset(cfg)?;
            fetch_remote_quote(endpoint, provider)
                .map_err(|e| anyhow::anyhow!("failed to fetch preset quote source: {e}"))
        }
        "url" | "remote_url" => {
            let endpoint = resolve_quote_endpoint_from_url(cfg, cycle)?;
            fetch_remote_quote(endpoint, QuoteProvider::Generic)
                .map_err(|e| anyhow::anyhow!("failed to fetch custom quote source: {e}"))
        }
        other => Err(anyhow::anyhow!(
            "unsupported quote_source={other}; supported: local, preset, url"
        )),
    }?;
    Ok(strip_project_line_suffix(&raw))
}

#[derive(Debug, Clone)]
struct GeoLocation {
    lat: f64,
    lon: f64,
    label: String,
    country_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitSystem {
    Metric,
    Imperial,
}

#[derive(Debug, Clone)]
struct WeatherWidgetPayload {
    text: String,
    minimap_image: Option<PathBuf>,
}

fn resolve_weather_widget(cfg: &AppConfig) -> Result<WeatherWidgetPayload> {
    if let Some(cached) = load_cached_weather_widget(cfg.weather_refresh_seconds)? {
        return Ok(cached);
    }

    let client = Client::builder().timeout(Duration::from_secs(8)).build()?;

    let geo = if cfg.weather_use_system_location {
        match lookup_system_location(&client) {
            Ok(v) => v,
            Err(primary_err) => {
                let query = cfg.weather_location_override.trim();
                if query.is_empty() {
                    return resolve_weather_widget_wttr(&client)
                        .map_err(|wttr_err| anyhow::anyhow!("{primary_err}; {wttr_err}"));
                }
                geocode_location(&client, query)?
            }
        }
    } else {
        let query = cfg.weather_location_override.trim();
        if query.is_empty() {
            anyhow::bail!("set weather location override");
        }
        geocode_location(&client, query)?
    };

    let units = unit_system_for_country(geo.country_code.as_deref());
    let unit_query = match units {
        UnitSystem::Metric => "",
        UnitSystem::Imperial => "&temperature_unit=fahrenheit&windspeed_unit=mph",
    };
    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m,relative_humidity_2m&hourly=precipitation_probability&forecast_days=1&timezone=auto{}",
        geo.lat, geo.lon, unit_query
    );
    let payload = client
        .get(weather_url)
        .send()?
        .error_for_status()?
        .json::<Value>()?;
    let current = payload
        .get("current")
        .ok_or_else(|| anyhow::anyhow!("missing current weather"))?;
    let t = current
        .get("temperature_2m")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("missing temperature"))?;
    let feels = current
        .get("apparent_temperature")
        .and_then(Value::as_f64)
        .unwrap_or(t);
    let wind = current
        .get("wind_speed_10m")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let wind_deg = current
        .get("wind_direction_10m")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let humidity = current
        .get("relative_humidity_2m")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let code = current
        .get("weather_code")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    let current_time = current.get("time").and_then(Value::as_str);
    let rain_prob = current_time
        .and_then(|time| precipitation_probability(&payload, time))
        .unwrap_or(0.0);
    let (wind_arrow, wind_dir) = compass_arrow(wind_deg);
    let temp_unit = match units {
        UnitSystem::Metric => "C",
        UnitSystem::Imperial => "F",
    };
    let wind_unit = match units {
        UnitSystem::Metric => "km/h",
        UnitSystem::Imperial => "mph",
    };
    let minimap_image = resolve_weather_minimap(geo.lat, geo.lon, wind_deg)
        .or_else(|| resolve_weather_minimap_raw(geo.lat, geo.lon));
    let text = format_weather_compact(
        weather_code_icon(code),
        t,
        feels,
        rain_prob,
        wind_arrow,
        wind_dir,
        wind,
        humidity,
        temp_unit,
        wind_unit,
    );

    let payload = WeatherWidgetPayload {
        text,
        minimap_image,
    };
    store_cached_weather_widget(&payload)?;
    Ok(payload)
}

fn resolve_weather_widget_wttr(client: &Client) -> Result<WeatherWidgetPayload> {
    let payload = client
        .get("https://wttr.in/?format=j1")
        .send()?
        .error_for_status()?
        .json::<Value>()?;

    let current = payload
        .get("current_condition")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .ok_or_else(|| anyhow::anyhow!("wttr missing current_condition"))?;

    let area = payload
        .get("nearest_area")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first());
    let country = area
        .and_then(|a| a.get("country"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    let desc = current
        .get("weatherDesc")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    let temp = current
        .get("temp_C")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let humidity = current
        .get("humidity")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let rain = current
        .get("chanceofrain")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    let wind = current
        .get("windspeedKmph")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);

    let wind_dir = current
        .get("winddir16Point")
        .and_then(Value::as_str)
        .unwrap_or("N");
    let wind_arrow = compass_arrow_for_name(wind_dir);
    let units = unit_system_for_country(match country {
        "United States" | "Liberia" | "Myanmar" => Some("US"),
        _ => None,
    });
    let (temp, feels, wind, temp_unit, wind_unit) = match units {
        UnitSystem::Imperial => (
            current
                .get("temp_F")
                .and_then(Value::as_str)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(temp * 9.0 / 5.0 + 32.0),
            current
                .get("FeelsLikeF")
                .and_then(Value::as_str)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(temp * 9.0 / 5.0 + 32.0),
            current
                .get("windspeedMiles")
                .and_then(Value::as_str)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(wind / 1.609_34),
            "F",
            "mph",
        ),
        UnitSystem::Metric => (
            temp,
            current
                .get("FeelsLikeC")
                .and_then(Value::as_str)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(temp),
            wind,
            "C",
            "km/h",
        ),
    };
    Ok(WeatherWidgetPayload {
        text: format_weather_compact(
            compact_condition_symbol(desc),
            temp,
            feels,
            rain,
            wind_arrow,
            wind_dir,
            wind,
            humidity,
            temp_unit,
            wind_unit,
        ),
        minimap_image: None,
    })
}

#[derive(Debug, Clone)]
struct NewsWidgetPayload {
    text: String,
    preview_image: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CamsWidgetPayload {
    text: String,
    preview_image: Option<PathBuf>,
}

fn resolve_news_widget(cfg: &AppConfig, cycle: u64) -> Result<NewsWidgetPayload> {
    let (label, stream_url, feed_url) = news_source_profile(cfg);
    let headline = if let Some(feed) = feed_url {
        fetch_rss_ticker(feed).unwrap_or_else(|| "live feed".to_string())
    } else {
        "live source".to_string()
    };
    let subtitle_hint = fetch_stream_title_hint(&stream_url).unwrap_or_default();
    let preview_image = resolve_news_preview_image(cfg, &stream_url, cycle);
    let raw_line = if subtitle_hint.is_empty() {
        compact_news_line(&format!("{label} ◆ {headline}"))
    } else {
        compact_news_line(&format!("{label} ◆ {headline} ◆ {subtitle_hint}"))
    };
    let line = news_ticker_frame(&raw_line, ticker_cycle_for_fps(cfg.news_fps.max(8.0)));
    Ok(NewsWidgetPayload {
        text: line,
        preview_image,
    })
}

fn resolve_cams_widget(cfg: &AppConfig, cycle: u64) -> Result<CamsWidgetPayload> {
    let mut urls = cams_source_urls(cfg, cycle);
    if urls.is_empty() {
        anyhow::bail!("no camera URLs available");
    }

    let count = cfg.cams_count.clamp(1, 5) as usize;
    if urls.len() > count {
        urls.truncate(count);
    }

    let mut frames = Vec::<PathBuf>::new();
    for url in &urls {
        if let Some(frame) = capture_stream_preview(url, 1.0) {
            frames.push(frame);
        }
    }
    let preview_image = compose_cams_grid(&frames, cfg.cams_columns.clamp(1, 4))
        .or_else(|| frames.first().cloned());

    let short = urls
        .iter()
        .map(|u| summarize_source_label(u))
        .collect::<Vec<_>>()
        .join(" ◆ ");
    let text = news_ticker_frame(
        &compact_news_line(&format!("CAMS ◆ {short}")),
        ticker_cycle_for_fps(1.2),
    );

    Ok(CamsWidgetPayload {
        text,
        preview_image,
    })
}

fn cams_source_urls(cfg: &AppConfig, cycle: u64) -> Vec<String> {
    match cfg.cams_source.trim().to_ascii_lowercase().as_str() {
        "custom" => {
            let mut urls = parse_endpoint_list(&cfg.cams_custom_urls);
            urls.retain(|u| looks_like_endpoint(u));
            if urls.is_empty() {
                auto_local_cam_urls(cycle)
            } else {
                rotate_urls(urls, cycle)
            }
        }
        "city_public" => city_public_cam_urls(cycle),
        _ => auto_local_cam_urls(cycle),
    }
}

fn auto_local_cam_urls(cycle: u64) -> Vec<String> {
    // Public live-cam feeds (YouTube) used as default sources.
    // We rotate candidates so users in different regions don't always see the same feed.
    let mut base = vec![
        "https://www.youtube.com/watch?v=1-iS7LArMPA".to_string(), // Times Square
        "https://www.youtube.com/watch?v=AdUw5RdyZxI".to_string(), // Earth/City cam
        "https://www.youtube.com/watch?v=wccRif2DaGs".to_string(), // City stream
        "https://www.youtube.com/watch?v=21X5lGlDOfg".to_string(), // NASA live earth view
        "https://www.youtube.com/watch?v=GE_SfNVNyqk".to_string(), // DW live feed
    ];
    base = rotate_urls(base, cycle);
    base
}

fn city_public_cam_urls(cycle: u64) -> Vec<String> {
    let mut city_urls = vec![
        "https://www.youtube.com/watch?v=1-iS7LArMPA".to_string(),
        "https://www.youtube.com/watch?v=wccRif2DaGs".to_string(),
        "https://www.youtube.com/watch?v=AdUw5RdyZxI".to_string(),
        "https://www.youtube.com/watch?v=21X5lGlDOfg".to_string(),
    ];
    city_urls = rotate_urls(city_urls, cycle);
    city_urls
}

fn rotate_urls(urls: Vec<String>, cycle: u64) -> Vec<String> {
    if urls.is_empty() {
        return urls;
    }
    let shift = (cycle as usize) % urls.len();
    urls[shift..]
        .iter()
        .chain(urls[..shift].iter())
        .cloned()
        .collect::<Vec<_>>()
}

fn summarize_source_label(url: &str) -> String {
    if let Some(id) = extract_youtube_video_id(url) {
        return format!("YT:{id}");
    }
    let trimmed = url.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

fn compose_cams_grid(frames: &[PathBuf], columns: u32) -> Option<PathBuf> {
    if frames.is_empty() {
        return None;
    }
    if frames.len() == 1 {
        return Some(frames[0].clone());
    }

    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let key = frames
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("|");
    let target = cache_dir.join(format!("cams-grid-{}.jpg", stable_hash(&key)));

    let cols = columns.clamp(1, 4).min(frames.len() as u32);
    if command_exists("magick") {
        let mut cmd = Command::new("magick");
        cmd.arg("montage");
        for frame in frames {
            cmd.arg(frame);
        }
        let status = cmd
            .args([
                "-tile",
                &format!("{cols}x"),
                "-geometry",
                "640x360+2+2",
                "-background",
                "#01070A",
            ])
            .arg(&target)
            .status()
            .ok()?;
        if status.success() && target.exists() {
            return Some(target);
        }
    } else if command_exists("montage") {
        let mut cmd = Command::new("montage");
        for frame in frames {
            cmd.arg(frame);
        }
        let status = cmd
            .args([
                "-tile",
                &format!("{cols}x"),
                "-geometry",
                "640x360+2+2",
                "-background",
                "#01070A",
            ])
            .arg(&target)
            .status()
            .ok()?;
        if status.success() && target.exists() {
            return Some(target);
        }
    }

    frames.first().cloned()
}

fn news_source_profile(cfg: &AppConfig) -> (&str, String, Option<&'static str>) {
    match cfg.news_source.as_str() {
        "euronews" => (
            "Euronews",
            "https://www.youtube.com/watch?v=pykpO5kQJ98".to_string(),
            Some("https://www.euronews.com/rss"),
        ),
        "aljazeera" => (
            "Al Jazeera English",
            "https://www.youtube.com/watch?v=gCNeDWCI0vo".to_string(),
            Some("https://www.aljazeera.com/xml/rss/all.xml"),
        ),
        "france24" => (
            "France 24",
            "https://www.youtube.com/watch?v=l8PMl7tUDIE".to_string(),
            Some("https://www.france24.com/en/rss"),
        ),
        "dw" => (
            "DW News",
            "https://www.youtube.com/watch?v=GE_SfNVNyqk".to_string(),
            Some("https://rss.dw.com/rdf/rss-en-all"),
        ),
        "yahoo_finance" => (
            "Yahoo Finance",
            "https://www.youtube.com/watch?v=9Auq9mYxFEE".to_string(),
            Some("https://finance.yahoo.com/news/rssindex"),
        ),
        "bloomberg_tv" => (
            "Bloomberg TV",
            "https://www.youtube.com/watch?v=dp8PhLsUcFE".to_string(),
            Some("https://feeds.bloomberg.com/markets/news.rss"),
        ),
        "techcrunch" => (
            "TechCrunch",
            "https://techcrunch.com/".to_string(),
            Some("https://techcrunch.com/feed/"),
        ),
        "theverge" => (
            "The Verge",
            "https://www.theverge.com/tech".to_string(),
            Some("https://www.theverge.com/rss/index.xml"),
        ),
        "nasa_tv" => (
            "NASA TV",
            "https://www.youtube.com/watch?v=21X5lGlDOfg".to_string(),
            Some("https://www.nasa.gov/rss/dyn/breaking_news.rss"),
        ),
        "documentary_heaven" => (
            "DocumentaryHeaven",
            "https://documentaryheaven.com/".to_string(),
            Some("https://documentaryheaven.com/feed/"),
        ),
        _ => ("Custom", cfg.news_custom_url.trim().to_string(), None),
    }
}

fn fetch_rss_ticker(url: &str) -> Option<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .ok()?;
    let body = client
        .get(url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;
    let mut titles = extract_rss_item_titles(&body, 4);
    if titles.is_empty()
        && let Some(single) = extract_first_rss_item_title(&body).or_else(|| extract_first_xml_tag(&body, "title"))
    {
        titles.push(single);
    }
    if titles.is_empty() {
        None
    } else {
        Some(compact_news_line(&titles.join(" ◆ ")))
    }
}

fn resolve_news_preview_image(cfg: &AppConfig, stream_url: &str, cycle: u64) -> Option<PathBuf> {
    if let Some(path) = capture_stream_preview(stream_url, cfg.news_fps) {
        return Some(path);
    }

    if cfg.news_source.eq_ignore_ascii_case("custom")
        && is_camera_like_url(stream_url)
        && let Some(path) = capture_camera_frame(stream_url, cfg.news_fps)
    {
        return Some(path);
    }

    let mut candidates = news_preview_candidates(cfg, stream_url, cycle);
    candidates.dedup();
    for endpoint in candidates {
        if let Ok(path) = fetch_remote_image(endpoint, ImageProvider::Generic) {
            return Some(path);
        }
    }
    None
}

fn format_weather_compact(
    condition_icon: &str,
    temp: f64,
    feels: f64,
    rain_prob: f64,
    wind_arrow: &str,
    wind_dir: &str,
    wind_speed: f64,
    humidity: f64,
    temp_unit: &str,
    wind_unit: &str,
) -> String {
    format!(
        "{}  ◉ {:.1}{}  ◇  ◍ {:.1}{}  ◇  ☂ {:.0}%\n⌖ {} {}  ◇  ➤ {:.1} {}  ◇  ◒ {:.0}%",
        condition_icon,
        temp,
        temp_unit,
        feels,
        temp_unit,
        rain_prob,
        wind_arrow,
        wind_dir,
        wind_speed,
        wind_unit,
        humidity
    )
}

fn unit_system_for_country(country_code: Option<&str>) -> UnitSystem {
    match country_code
        .unwrap_or_default()
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "US" | "LR" | "MM" => UnitSystem::Imperial,
        _ => UnitSystem::Metric,
    }
}

fn resolve_weather_minimap(lat: f64, lon: f64, wind_deg: f64) -> Option<PathBuf> {
    let raw_map = resolve_weather_minimap_raw(lat, lon)?;
    stylize_weather_minimap(&raw_map, lat, lon, wind_deg)
}

fn resolve_weather_minimap_raw(lat: f64, lon: f64) -> Option<PathBuf> {
    let endpoint = format!(
        "https://staticmap.openstreetmap.de/staticmap.php?center={lat:.4},{lon:.4}&zoom=7&size=640x360&maptype=mapnik&markers={lat:.4},{lon:.4},lightblue1"
    );
    fetch_remote_image(endpoint, ImageProvider::Generic).ok()
}

fn precipitation_probability(payload: &Value, current_time: &str) -> Option<f64> {
    let hourly = payload.get("hourly")?;
    let times = hourly.get("time")?.as_array()?;
    let probs = hourly.get("precipitation_probability")?.as_array()?;
    let idx = times
        .iter()
        .position(|t| t.as_str().unwrap_or_default() == current_time)?;
    probs.get(idx).and_then(Value::as_f64)
}

fn compass_arrow(deg: f64) -> (&'static str, &'static str) {
    let mut d = deg % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    match ((d + 22.5) / 45.0).floor() as usize % 8 {
        0 => ("↑", "N"),
        1 => ("↗", "NE"),
        2 => ("→", "E"),
        3 => ("↘", "SE"),
        4 => ("↓", "S"),
        5 => ("↙", "SW"),
        6 => ("←", "W"),
        _ => ("↖", "NW"),
    }
}

fn compass_arrow_for_name(dir: &str) -> &'static str {
    match dir.trim().to_ascii_uppercase().as_str() {
        "N" => "↑",
        "NNE" | "NE" | "ENE" => "↗",
        "E" => "→",
        "ESE" | "SE" | "SSE" => "↘",
        "S" => "↓",
        "SSW" | "SW" | "WSW" => "↙",
        "W" => "←",
        "WNW" | "NW" | "NNW" => "↖",
        _ => "•",
    }
}

fn compact_news_line(input: &str) -> String {
    let line = input.replace('\n', " ").replace("  ", " ");
    let mut out = String::new();
    for c in line.chars() {
        if c.is_control() {
            continue;
        }
        out.push(c);
        if out.chars().count() >= 96 {
            out.push('…');
            break;
        }
    }
    out
}

fn news_ticker_frame(input: &str, cycle: u64) -> String {
    let clean = compact_news_line(input);
    let tape = format!("   {clean}   ◆   ");
    let chars = tape.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return "LIVE ◆ no data".to_string();
    }
    let shift = (cycle as usize) % chars.len();
    let ordered_full = chars[shift..]
        .iter()
        .chain(chars[..shift].iter())
        .collect::<String>();
    let visible = ordered_full.chars().take(92).collect::<String>();
    format!("▮ {visible} ▮")
}

fn extract_first_rss_item_title(raw: &str) -> Option<String> {
    let item_start = raw.find("<item")?;
    let sub = &raw[item_start..];
    extract_first_xml_tag(sub, "title")
}

fn capture_stream_preview(stream_url: &str, fps: f32) -> Option<PathBuf> {
    let raw = stream_url.trim();
    if raw.is_empty() {
        return None;
    }
    if is_youtube_url(raw)
        && let Some(resolved) = resolve_youtube_playback_url(raw)
        && let Some(path) = capture_news_frame(&resolved, raw, fps)
    {
        return Some(path);
    }

    let lower = raw.to_ascii_lowercase();
    if is_camera_like_url(stream_url)
        || lower.ends_with(".m3u8")
        || lower.ends_with(".mp4")
        || lower.ends_with(".webm")
        || lower.ends_with(".mkv")
        || lower.contains("stream")
    {
        return capture_news_frame(raw, raw, fps);
    }
    None
}

fn weather_code_icon(code: i64) -> &'static str {
    match code {
        0 => "☀",
        1..=3 => "⛅",
        45 | 48 => "🌫",
        51..=57 => "🌦",
        61..=67 => "🌧",
        71..=77 => "❄",
        80..=86 => "🌧",
        95..=99 => "⚡",
        _ => "•",
    }
}

fn compact_condition_symbol(desc: &str) -> &'static str {
    let l = desc.to_ascii_lowercase();
    if l.contains("thunder") {
        return "⚡";
    }
    if l.contains("snow") {
        return "❄";
    }
    if l.contains("rain") || l.contains("drizzle") || l.contains("shower") {
        return "🌧";
    }
    if l.contains("fog") || l.contains("mist") {
        return "🌫";
    }
    if l.contains("cloud") {
        return "☁";
    }
    if l.contains("clear") || l.contains("sun") {
        return "☀";
    }
    "•"
}

fn news_preview_candidates(cfg: &AppConfig, stream_url: &str, cycle: u64) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let ts = now_epoch_seconds();
    if let Some(video_id) = extract_youtube_video_id(stream_url) {
        out.push(format!(
            "https://img.youtube.com/vi/{video_id}/maxresdefault_live.jpg?t={}",
            ts / 2
        ));
        out.push(format!(
            "https://img.youtube.com/vi/{video_id}/hqdefault_live.jpg?t={}",
            ts / 2
        ));
        out.push(format!(
            "https://img.youtube.com/vi/{video_id}/maxresdefault.jpg?t={}",
            ts / 8
        ));
        out.push(format!(
            "https://img.youtube.com/vi/{video_id}/hqdefault.jpg?t={}",
            ts / 8
        ));
    }
    let stream_lower = stream_url.to_ascii_lowercase();
    if stream_lower.ends_with(".jpg")
        || stream_lower.ends_with(".jpeg")
        || stream_lower.ends_with(".png")
        || stream_lower.ends_with(".webp")
    {
        out.push(stream_url.to_string());
    }
    out.push(format!(
        "https://picsum.photos/seed/news-{}-{cycle}-preview/1280/720.jpg",
        cfg.news_source.replace(' ', "-"),
    ));
    out
}

fn extract_rss_item_titles(raw: &str, max_items: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut cursor = 0usize;
    while out.len() < max_items {
        let Some(item_rel) = raw[cursor..].find("<item") else {
            break;
        };
        let item_start = cursor + item_rel;
        let item_slice = &raw[item_start..];
        let Some(title) = extract_first_xml_tag(item_slice, "title") else {
            cursor = item_start.saturating_add(5);
            continue;
        };
        if !title.trim().is_empty() {
            out.push(title);
        }
        if let Some(end_rel) = item_slice.find("</item>") {
            cursor = item_start + end_rel + "</item>".len();
        } else {
            break;
        }
    }
    out
}

fn extract_youtube_video_id(url: &str) -> Option<String> {
    if let Some(idx) = url.find("youtu.be/") {
        let tail = &url[idx + "youtu.be/".len()..];
        let id = tail
            .split(['?', '&', '/', '#'])
            .next()
            .unwrap_or_default()
            .trim();
        if is_valid_youtube_id(id) {
            return Some(id.to_string());
        }
    }
    if let Some(idx) = url.find("watch?v=") {
        let tail = &url[idx + "watch?v=".len()..];
        let id = tail
            .split(['&', '#', '/'])
            .next()
            .unwrap_or_default()
            .trim();
        if is_valid_youtube_id(id) {
            return Some(id.to_string());
        }
    }
    if let Some(idx) = url.find("/live/") {
        let tail = &url[idx + "/live/".len()..];
        let id = tail
            .split(['?', '&', '/', '#'])
            .next()
            .unwrap_or_default()
            .trim();
        if is_valid_youtube_id(id) {
            return Some(id.to_string());
        }
    }
    None
}

fn is_valid_youtube_id(id: &str) -> bool {
    if id.len() < 6 {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn geocode_location(client: &Client, query: &str) -> Result<GeoLocation> {
    let search_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        query.replace(' ', "+")
    );
    let geo = client
        .get(search_url)
        .send()?
        .error_for_status()?
        .json::<Value>()?;
    let first = geo
        .get("results")
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .ok_or_else(|| anyhow::anyhow!("location not found"))?;
    let lat = first
        .get("latitude")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("missing latitude"))?;
    let lon = first
        .get("longitude")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("missing longitude"))?;
    let name = first
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(query)
        .to_string();
    let country = first
        .get("country")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    let country_code = first
        .get("country_code")
        .and_then(Value::as_str)
        .map(|s| s.to_string());
    Ok(GeoLocation {
        lat,
        lon,
        label: format!("{name}, {country}"),
        country_code,
    })
}

fn weather_geo_cache_path() -> Option<PathBuf> {
    let p = expand_tilde("~/.cache/wallpaper-composer/weather-geo.json").ok()?;
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    Some(p)
}

fn load_cached_geo_location(max_age_secs: Option<u64>) -> Option<GeoLocation> {
    let path = weather_geo_cache_path()?;
    if let Some(max_age) = max_age_secs {
        let modified = fs::metadata(&path).ok()?.modified().ok()?;
        let age = SystemTime::now()
            .duration_since(modified)
            .ok()
            .map(|d| d.as_secs())
            .unwrap_or(max_age.saturating_add(1));
        if age > max_age {
            return None;
        }
    }
    let raw = fs::read_to_string(path).ok()?;
    let payload = serde_json::from_str::<Value>(&raw).ok()?;
    Some(GeoLocation {
        lat: payload.get("lat")?.as_f64()?,
        lon: payload.get("lon")?.as_f64()?,
        label: payload.get("label")?.as_str()?.to_string(),
        country_code: payload
            .get("country_code")
            .and_then(Value::as_str)
            .map(|v| v.to_string()),
    })
}

fn store_cached_geo_location(geo: &GeoLocation) {
    let Some(path) = weather_geo_cache_path() else {
        return;
    };
    let payload = serde_json::json!({
        "lat": geo.lat,
        "lon": geo.lon,
        "label": geo.label,
        "country_code": geo.country_code,
        "updated_unix": SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    });
    let _ = fs::write(path, payload.to_string());
}

fn lookup_system_location(client: &Client) -> Result<GeoLocation> {
    if let Some(cached) = load_cached_geo_location(Some(WEATHER_GEO_CACHE_MAX_AGE_SECS)) {
        return Ok(cached);
    }

    let mut errors = Vec::<String>::new();

    let ipapi = client
        .get("https://ipapi.co/json/")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json::<Value>());
    match ipapi {
        Ok(geo) => {
            let lat = geo
                .get("latitude")
                .and_then(Value::as_f64)
                .ok_or_else(|| anyhow::anyhow!("ipapi missing latitude"))?;
            let lon = geo
                .get("longitude")
                .and_then(Value::as_f64)
                .ok_or_else(|| anyhow::anyhow!("ipapi missing longitude"))?;
            let city = geo.get("city").and_then(Value::as_str).unwrap_or("Unknown");
            let country = geo
                .get("country_name")
                .and_then(Value::as_str)
                .unwrap_or("Unknown");
            let country_code = geo
                .get("country_code")
                .and_then(Value::as_str)
                .map(|v| v.to_string());
            let geo_loc = GeoLocation {
                lat,
                lon,
                label: format!("{city}, {country}"),
                country_code,
            };
            store_cached_geo_location(&geo_loc);
            return Ok(geo_loc);
        }
        Err(e) => errors.push(format!("ipapi: {e}")),
    }

    let ipwho = client
        .get("https://ipwho.is/")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json::<Value>());
    match ipwho {
        Ok(geo) => {
            let success = geo.get("success").and_then(Value::as_bool).unwrap_or(true);
            if !success {
                errors.push("ipwho: success=false".to_string());
            } else {
                let lat = geo
                    .get("latitude")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| anyhow::anyhow!("ipwho missing latitude"))?;
                let lon = geo
                    .get("longitude")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| anyhow::anyhow!("ipwho missing longitude"))?;
                let city = geo.get("city").and_then(Value::as_str).unwrap_or("Unknown");
                let country = geo
                    .get("country")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown");
                let country_code = geo
                    .get("country_code")
                    .and_then(Value::as_str)
                    .map(|v| v.to_string());
                let geo_loc = GeoLocation {
                    lat,
                    lon,
                    label: format!("{city}, {country}"),
                    country_code,
                };
                store_cached_geo_location(&geo_loc);
                return Ok(geo_loc);
            }
        }
        Err(e) => errors.push(format!("ipwho: {e}")),
    }

    let ipinfo = client
        .get("https://ipinfo.io/json")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json::<Value>());
    match ipinfo {
        Ok(geo) => {
            if let Some((lat, lon)) = geo
                .get("loc")
                .and_then(Value::as_str)
                .and_then(parse_lat_lon_pair)
            {
                let city = geo.get("city").and_then(Value::as_str).unwrap_or("Unknown");
                let country = geo
                    .get("country")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown");
                let country_code = geo
                    .get("country")
                    .and_then(Value::as_str)
                    .map(|v| v.to_string());
                let geo_loc = GeoLocation {
                    lat,
                    lon,
                    label: format!("{city}, {country}"),
                    country_code,
                };
                store_cached_geo_location(&geo_loc);
                return Ok(geo_loc);
            }
            errors.push("ipinfo: missing loc".to_string());
        }
        Err(e) => errors.push(format!("ipinfo: {e}")),
    }

    if let Some(stale_cached) = load_cached_geo_location(None) {
        return Ok(stale_cached);
    }

    anyhow::bail!(
        "location lookup failed across providers; {}. Set weather_location_override to bypass geolocation.",
        errors.join(" | ")
    )
}

fn parse_lat_lon_pair(raw: &str) -> Option<(f64, f64)> {
    let mut parts = raw.split(',');
    let lat = parts.next()?.trim().parse::<f64>().ok()?;
    let lon = parts.next()?.trim().parse::<f64>().ok()?;
    Some((lat, lon))
}

fn is_camera_like_url(url: &str) -> bool {
    let l = url.trim().to_ascii_lowercase();
    l.starts_with("rtsp://")
        || l.ends_with(".m3u8")
        || l.ends_with(".mpd")
        || l.contains("mjpeg")
        || l.contains("snapshot")
        || l.contains("camera")
        || l.contains("webcam")
}

fn is_youtube_url(url: &str) -> bool {
    let l = url.to_ascii_lowercase();
    l.contains("youtube.com") || l.contains("youtu.be")
}

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn capture_camera_frame(url: &str, fps: f32) -> Option<PathBuf> {
    capture_frame_with_ffmpeg(url, url, fps, true, "camera")
}

fn capture_news_frame(url: &str, cache_key: &str, fps: f32) -> Option<PathBuf> {
    capture_frame_with_ffmpeg(url, cache_key, fps, false, "news")
}

fn capture_frame_with_ffmpeg(
    url: &str,
    cache_key: &str,
    fps: f32,
    enforce_camera_floor: bool,
    prefix: &str,
) -> Option<PathBuf> {
    if !command_exists("ffmpeg") {
        return None;
    }
    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let hash = stable_hash(cache_key);
    let target = cache_dir.join(format!("{prefix}-{hash}.jpg"));
    let stamp = cache_dir.join(format!("{prefix}-{hash}.stamp"));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let fps = fps.clamp(0.05, 30.0);
    let min_interval = if enforce_camera_floor {
        1.0_f32.max(1.0 / fps)
    } else {
        (1.0 / fps).max(0.05)
    };
    let min_secs = min_interval.ceil() as u64;
    let last = fs::read_to_string(&stamp)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);
    if now.saturating_sub(last) < min_secs && target.exists() {
        return Some(target);
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "error", "-nostdin"]);
    if url.to_ascii_lowercase().starts_with("rtsp://") {
        cmd.args(["-rtsp_transport", "tcp"]);
    }
    let status = cmd
        .args([
            "-i",
            url,
            "-an",
            "-sn",
            "-dn",
            "-frames:v",
            "1",
            "-q:v",
            "4",
        ])
        .arg(&target)
        .status()
        .ok()?;
    if !status.success() || !target.exists() {
        return None;
    }
    let _ = fs::write(stamp, format!("{now}\n"));
    Some(target)
}

fn resolve_youtube_playback_url(url: &str) -> Option<String> {
    if !command_exists("yt-dlp") {
        return None;
    }
    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let hash = stable_hash(url);
    let cache_file = cache_dir.join(format!("yt-dlp-{hash}.url"));
    let ttl_secs = 10 * 60;

    if let Ok(meta) = fs::metadata(&cache_file)
        && let Ok(modified) = meta.modified()
        && let Ok(age) = SystemTime::now().duration_since(modified)
        && age.as_secs() < ttl_secs
        && let Ok(cached) = fs::read_to_string(&cache_file)
    {
        let val = cached.trim();
        if val.starts_with("http") {
            return Some(val.to_string());
        }
    }

    let output = Command::new("yt-dlp")
        .args(["--no-warnings", "--no-playlist", "-g", url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut candidates = stdout
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("http"))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by_key(|l| if l.contains(".m3u8") { 0 } else { 1 });
    let picked = candidates[0].clone();
    let _ = fs::write(&cache_file, format!("{picked}\n"));
    Some(picked)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn extract_first_xml_tag(raw: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut search_start = 0usize;
    while let Some(start_rel) = raw[search_start..].find(&open) {
        let start = search_start + start_rel + open.len();
        let end_rel = raw[start..].find(&close)?;
        let end = start + end_rel;
        let value = raw[start..end]
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">");
        let decoded = decode_numeric_entities(&value);
        let cleaned = decoded.trim();
        if !cleaned.is_empty() && !cleaned.to_ascii_lowercase().contains("rss") {
            return Some(cleaned.to_string());
        }
        search_start = end + close.len();
    }
    None
}

fn decode_numeric_entities(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(idx) = rest.find("&#") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + 2..];
        if let Some(end) = after.find(';') {
            let num = &after[..end];
            if let Ok(code) = num.parse::<u32>()
                && let Some(ch) = char::from_u32(code)
            {
                out.push(ch);
                rest = &after[end + 1..];
                continue;
            }
            out.push_str("&#");
            out.push_str(num);
            out.push(';');
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[idx..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

fn strip_project_line_suffix(input: &str) -> String {
    let mut out = Vec::<String>::new();
    for line in input.lines() {
        let mut cleaned = line.to_string();
        if cleaned.trim_start().starts_with('-') {
            cleaned = cleaned
                .replace("(project line)", "")
                .replace("(projectline)", "");
            cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
            cleaned = cleaned.trim_end().to_string();
        }
        out.push(cleaned);
    }
    out.join("\n")
}

fn quote_pick_state_path(cfg: &AppConfig) -> Result<PathBuf> {
    let path = expand_tilde(&format!("{}.quote_pick", cfg.rotation_state_file))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn read_recent_indices(path: &Path) -> Vec<usize> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    raw.split(',')
        .filter_map(|part| part.trim().parse::<usize>().ok())
        .take(MAX_STORED_HISTORY)
        .collect::<Vec<_>>()
}

fn write_recent_indices(path: &Path, previous: &[usize], next_idx: usize) -> Result<()> {
    let mut merged = vec![next_idx];
    for idx in previous.iter().copied() {
        if idx != next_idx && merged.len() < MAX_STORED_HISTORY {
            merged.push(idx);
        }
    }
    let serialized = merged
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    fs::write(path, format!("{serialized}\n"))?;
    Ok(())
}

fn resolve_image_endpoint_from_preset(cfg: &AppConfig) -> Result<(String, ImageProvider)> {
    let id = cfg.image_source_preset.as_deref().ok_or_else(|| {
        anyhow::anyhow!("image_source_preset is required for image_source=preset")
    })?;
    let endpoint = image_preset_endpoint(id)
        .ok_or_else(|| anyhow::anyhow!("unknown image_source_preset: {id}"))?;

    let provider = match id {
        "wallhaven_random_4k" => ImageProvider::WallhavenApi,
        _ => ImageProvider::Generic,
    };
    Ok((with_nasa_demo_key(endpoint), provider))
}

fn resolve_quote_endpoint_from_preset(cfg: &AppConfig) -> Result<(String, QuoteProvider)> {
    let id = cfg.quote_source_preset.as_deref().ok_or_else(|| {
        anyhow::anyhow!("quote_source_preset is required for quote_source=preset")
    })?;
    let endpoint = quote_preset_endpoint(id)
        .ok_or_else(|| anyhow::anyhow!("unknown quote_source_preset: {id}"))?;

    let provider = match id {
        "zenquotes_daily" => QuoteProvider::ZenQuotes,
        "quotable_random" => QuoteProvider::Quotable,
        _ => QuoteProvider::Generic,
    };

    Ok((endpoint.to_string(), provider))
}

fn resolve_image_endpoint_from_url(cfg: &AppConfig, cycle: u64) -> Result<String> {
    let raw = cfg
        .image_source_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("image_source_url is required for image_source=url"))?;
    pick_endpoint_from_list(raw, cycle)
        .ok_or_else(|| anyhow::anyhow!("image_source_url must contain at least one valid URL"))
}

fn resolve_quote_endpoint_from_url(cfg: &AppConfig, cycle: u64) -> Result<String> {
    let raw = cfg
        .quote_source_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("quote_source_url is required for quote_source=url"))?;
    pick_endpoint_from_list(raw, cycle)
        .ok_or_else(|| anyhow::anyhow!("quote_source_url must contain at least one valid URL"))
}

fn with_nasa_demo_key(endpoint: &str) -> String {
    if !endpoint.contains("api.nasa.gov/planetary/apod") || endpoint.contains("api_key=") {
        return endpoint.to_string();
    }
    if endpoint.contains('?') {
        format!("{endpoint}&api_key=DEMO_KEY")
    } else {
        format!("{endpoint}?api_key=DEMO_KEY")
    }
}

fn pick_endpoint_from_list(raw: &str, cycle: u64) -> Option<String> {
    let urls = parse_endpoint_list(raw)
        .into_iter()
        .filter(|u| looks_like_endpoint(u))
        .collect::<Vec<_>>();
    if urls.is_empty() {
        return None;
    }
    let idx = (cycle as usize) % urls.len();
    urls.get(idx).cloned()
}

fn parse_endpoint_list(raw: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for piece in raw.split(['\n', ';', '|']) {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push(trimmed.to_string());
    }
    if out.is_empty() && !raw.trim().is_empty() {
        out.push(raw.trim().to_string());
    }
    out
}

fn looks_like_endpoint(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    v.starts_with("https://") || v.starts_with("http://") || v.starts_with("file://")
}

fn print_presets() {
    println!("image_presets:");
    for p in builtin_image_presets() {
        println!(
            "  - id={} label={} endpoint={}",
            p.id, p.display_label, p.endpoint
        );
        println!(
            "    category={} auth={} rate_limit={}",
            p.category, p.auth, p.rate_limit
        );
        println!("    notes={}", p.notes);
    }
    println!("quote_presets:");
    for p in builtin_quote_presets() {
        println!(
            "  - id={} label={} endpoint={}",
            p.id, p.display_label, p.endpoint
        );
        println!(
            "    category={} auth={} rate_limit={}",
            p.category, p.auth, p.rate_limit
        );
        println!("    notes={}", p.notes);
    }
}

fn resolve_config_path(config: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = config {
        return Ok(path);
    }

    default_config_path().map_err(|_| anyhow::anyhow!("HOME is not set; pass --config explicitly"))
}

fn ensure_default_local_quotes(config_path: &Path) -> Result<Option<PathBuf>> {
    let cfg = load_config(config_path)?;
    let quotes_path = expand_tilde(&cfg.quotes_path)?;
    if quotes_path.exists() {
        return Ok(None);
    }
    if let Some(parent) = quotes_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&quotes_path, BUNDLED_LOCAL_QUOTES)?;
    Ok(Some(quotes_path))
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn prepare_wallpaper_apply_target(cfg: &AppConfig, rendered_output: &Path) -> Result<PathBuf> {
    if !is_temp_output_path(&cfg.output_image) {
        return Ok(rendered_output.to_path_buf());
    }
    let target = expand_tilde("~/.local/state/wallpaper-composer/current.png")?;
    ensure_parent_dir(&target)?;
    fs::copy(rendered_output, &target).with_context(|| {
        format!(
            "failed to persist rendered wallpaper {} -> {}",
            rendered_output.display(),
            target.display()
        )
    })?;
    Ok(target)
}

fn is_temp_output_path(raw: &str) -> bool {
    let normalized = raw.trim().to_ascii_lowercase();
    normalized.starts_with("/tmp/")
        || normalized.starts_with("/var/tmp/")
        || normalized == "/tmp/wallpaper-composer-current.png"
}

fn backup_path_for(config_path: &Path) -> PathBuf {
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let name = config_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.toml");
    config_path.with_file_name(format!("{name}.bak-{ts}"))
}

struct RunLockGuard {
    _file: fs::File,
}

fn acquire_run_lock(config_path: &Path, replace_existing: bool) -> Result<RunLockGuard> {
    let lock_path = run_lock_path(config_path);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .with_context(|| format!("failed to open lock file {}", lock_path.display()))?;

    if lock_file.try_lock_exclusive().is_err() {
        if !replace_existing {
            anyhow::bail!(
                "another le-compositeur loop is already running (lock: {}). Use --replace-existing to restart it.",
                lock_path.display()
            );
        }
        if let Some(old_pid) = read_lock_pid(&lock_path)
            && old_pid > 1
            && old_pid != std::process::id()
        {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(old_pid.to_string())
                .status();
            thread::sleep(Duration::from_millis(500));
        }

        lock_file.try_lock_exclusive().with_context(|| {
            format!(
                "failed to acquire loop lock {} after replace attempt",
                lock_path.display()
            )
        })?;
    }

    lock_file.set_len(0)?;
    lock_file.seek(SeekFrom::Start(0))?;
    writeln!(lock_file, "{}", std::process::id())?;
    lock_file.flush()?;

    Ok(RunLockGuard { _file: lock_file })
}

fn run_lock_path(config_path: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    config_path.display().to_string().hash(&mut hasher);
    let cfg_hash = format!("{:016x}", hasher.finish());
    let base = expand_tilde("~/.local/state/wallpaper-composer")
        .unwrap_or_else(|_| std::env::temp_dir().join("wallpaper-composer"));
    base.join(format!("wc-cli-{}.lock", cfg_hash))
}

fn read_lock_pid(path: &Path) -> Option<u32> {
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn loop_tick_duration(cfg: &AppConfig) -> Duration {
    // Keep the clock fresh and allow smoother news frames when configured.
    let mut seconds = master_rotation_interval(cfg) as f64;
    seconds = seconds.min(1.0);
    if cfg.show_news_layer {
        let news_step = 1.0 / cfg.news_fps.clamp(0.05, 30.0) as f64;
        seconds = seconds.min(news_step.max(0.033));
    }
    let millis = (seconds * 1000.0).round().clamp(33.0, 60_000.0) as u64;
    Duration::from_millis(millis)
}

fn master_rotation_interval(cfg: &AppConfig) -> u64 {
    cfg.image_refresh_seconds.max(1)
}

fn ticker_cycle_for_fps(fps: f32) -> u64 {
    let frame_ms = (1000.0 / fps.clamp(0.05, 30.0) as f64).max(33.0) as u64;
    now_epoch_millis() / frame_ms.max(1)
}

fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn weather_payload_cache_path() -> Option<PathBuf> {
    let p = expand_tilde("~/.cache/wallpaper-composer/weather-payload.json").ok()?;
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    Some(p)
}

fn load_cached_weather_widget(refresh_seconds: u64) -> Result<Option<WeatherWidgetPayload>> {
    let Some(path) = weather_payload_cache_path() else {
        return Ok(None);
    };
    let max_age = refresh_seconds
        .clamp(60, WEATHER_PAYLOAD_CACHE_MAX_AGE_SECS)
        .max(60);
    let modified = match fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .ok()
        .map(|d| d.as_secs())
        .unwrap_or(max_age.saturating_add(1));
    if age > max_age {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)?;
    let payload = serde_json::from_str::<Value>(&raw)?;
    let text = payload
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if text.trim().is_empty() {
        return Ok(None);
    }
    let minimap_image = payload
        .get("minimap_image")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .filter(|p| p.exists());
    Ok(Some(WeatherWidgetPayload {
        text,
        minimap_image,
    }))
}

fn store_cached_weather_widget(payload: &WeatherWidgetPayload) -> Result<()> {
    let Some(path) = weather_payload_cache_path() else {
        return Ok(());
    };
    let json = serde_json::json!({
        "text": payload.text,
        "minimap_image": payload.minimap_image.as_ref().map(|p| p.display().to_string()),
        "updated_unix": now_epoch_seconds(),
    });
    fs::write(path, json.to_string())?;
    Ok(())
}

fn stylize_weather_minimap(input: &Path, lat: f64, lon: f64, wind_deg: f64) -> Option<PathBuf> {
    let (cmd, use_magick_subcommand) = if command_exists("magick") {
        ("magick", true)
    } else if command_exists("convert") {
        ("convert", false)
    } else {
        return Some(input.to_path_buf());
    };

    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let hash = stable_hash(&format!(
        "{}-{lat:.3}-{lon:.3}-{}",
        input.display(),
        (wind_deg / 5.0).round() as i32
    ));
    let output = cache_dir.join(format!("weather-map-{hash}.png"));
    if output.exists() {
        return Some(output);
    }

    let center_x = 320.0_f64;
    let center_y = 180.0_f64;
    let radius = 82.0_f64;
    let rad = wind_deg.to_radians();
    let dx = rad.sin();
    let dy = -rad.cos();
    let ex = center_x + dx * radius;
    let ey = center_y + dy * radius;
    let left_x = ex - (dx * 10.0) - (dy * 8.0);
    let left_y = ey - (dy * 10.0) + (dx * 8.0);
    let right_x = ex - (dx * 10.0) + (dy * 8.0);
    let right_y = ey - (dy * 10.0) - (dx * 8.0);

    let mut args = Vec::<String>::new();
    if use_magick_subcommand {
        args.push("convert".to_string());
    }
    args.extend([
        input.display().to_string(),
        "-auto-orient".to_string(),
        "-resize".to_string(),
        "640x360^".to_string(),
        "-gravity".to_string(),
        "Center".to_string(),
        "-extent".to_string(),
        "640x360".to_string(),
        "-colorspace".to_string(),
        "Gray".to_string(),
        "-fill".to_string(),
        "#00111A66".to_string(),
        "-colorize".to_string(),
        "22".to_string(),
        "-stroke".to_string(),
        "#007C8A88".to_string(),
        "-strokewidth".to_string(),
        "2".to_string(),
        "-fill".to_string(),
        "none".to_string(),
        "-draw".to_string(),
        format!(
            "circle {center_x:.1},{center_y:.1} {center_x:.1},{:.1}",
            center_y - 92.0
        ),
        "-stroke".to_string(),
        "#00E7FFDD".to_string(),
        "-strokewidth".to_string(),
        "4".to_string(),
        "-draw".to_string(),
        format!("line {center_x:.1},{center_y:.1} {ex:.1},{ey:.1}"),
        "-fill".to_string(),
        "#00F5FFEE".to_string(),
        "-stroke".to_string(),
        "#003640".to_string(),
        "-strokewidth".to_string(),
        "1".to_string(),
        "-draw".to_string(),
        format!("polygon {ex:.1},{ey:.1} {left_x:.1},{left_y:.1} {right_x:.1},{right_y:.1}"),
        "-fill".to_string(),
        "#00F5FF".to_string(),
        "-stroke".to_string(),
        "none".to_string(),
        "-draw".to_string(),
        format!(
            "circle {center_x:.1},{center_y:.1} {center_x:.1},{:.1}",
            center_y - 4.0
        ),
        output.display().to_string(),
    ]);

    let ok = Command::new(cmd)
        .args(args)
        .status()
        .ok()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok && output.exists() {
        Some(output)
    } else {
        Some(input.to_path_buf())
    }
}

fn fetch_stream_title_hint(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    if url.contains("youtube.com") || url.contains("youtu.be") {
        let escaped = url.replace(' ', "%20");
        let oembed = format!("https://www.youtube.com/oembed?url={escaped}&format=json");
        if let Ok(v) = client
            .get(oembed)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json::<Value>())
            && let Some(t) = v.get("title").and_then(Value::as_str)
        {
            let cleaned = compact_news_line(t);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }

    let body = client
        .get(url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;
    extract_first_xml_tag(&body, "title").map(|t| compact_news_line(&t))
}

#[cfg(test)]
mod tests {
    use super::{determine_cycle, loop_tick_duration, read_recent_indices, write_recent_indices};
    use std::fs;
    use std::time::Duration;
    use wc_core::{default_config_toml, load_config};

    #[test]
    fn loop_tick_is_capped_to_keep_clock_current() {
        let cfg_path = std::env::temp_dir().join("wc-cli-loop-tick-test.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let mut cfg = load_config(&cfg_path).expect("default config should parse");
        cfg.image_refresh_seconds = 300;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(1));

        cfg.image_refresh_seconds = 15;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(1));

        cfg.show_news_layer = true;
        cfg.news_fps = 10.0;
        assert!(loop_tick_duration(&cfg) <= Duration::from_millis(100));
    }

    #[test]
    fn single_cycle_timer_is_used_for_both_streams() {
        let cfg_path = std::env::temp_dir().join("wc-cli-cycle-test.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let mut cfg = load_config(&cfg_path).expect("default config should parse");
        cfg.rotation_use_persistent_state = false;
        cfg.image_refresh_seconds = 45;

        let image_cycle =
            determine_cycle(&cfg, cfg.image_refresh_seconds, "rotation").expect("cycle ok");
        let quote_cycle =
            determine_cycle(&cfg, cfg.image_refresh_seconds, "rotation").expect("cycle ok");

        assert_eq!(image_cycle, quote_cycle);
    }

    #[test]
    fn recent_history_keeps_last_three_distinct_entries() {
        let state_path = std::env::temp_dir().join("wc-cli-recent-history-test.state");
        let _ = fs::remove_file(&state_path);

        write_recent_indices(&state_path, &[], 5).expect("state write should work");
        write_recent_indices(&state_path, &read_recent_indices(&state_path), 2)
            .expect("state write should work");
        write_recent_indices(&state_path, &read_recent_indices(&state_path), 7)
            .expect("state write should work");
        write_recent_indices(&state_path, &read_recent_indices(&state_path), 2)
            .expect("state write should work");

        let got = read_recent_indices(&state_path);
        assert_eq!(got, vec![2, 7, 5]);
        let _ = fs::remove_file(state_path);
    }
}
