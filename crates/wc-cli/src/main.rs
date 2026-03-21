use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use image::imageops;
use image::{DynamicImage, Rgba, RgbaImage};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};
use wc_backend::apply_wallpaper;
use wc_core::{
    AppConfig, BUILTIN_WIDGET_TYPE_IDS, WidgetInstanceConfig, WidgetPlugin, WidgetRegistry,
    WidgetResolvedPayload, WidgetRuntimeContext, build_doctor_report, builtin_image_presets,
    builtin_news_source, builtin_news_source_is_live_video, builtin_quote_presets, cycle_index,
    default_config_path, default_config_toml, ensure_local_quotes_file, expand_tilde,
    image_preset_endpoint, list_background_images, load_config, load_quotes,
    pick_background_image_with_mode, pick_quote_with_mode, presets_catalog_json,
    quote_preset_endpoint, settings_schema_json, settings_ui_blueprint_json, to_config_toml,
};
use wc_render::{PreviewText, render_preview_to_file};
use wc_source::{ImageProvider, QuoteProvider, fetch_remote_image, fetch_remote_quote};

const MAX_STORED_HISTORY: usize = 64;
const WEATHER_GEO_CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;
const WEATHER_PAYLOAD_CACHE_MAX_AGE_SECS: u64 = 6 * 60 * 60;
const NEWS_WIDGET_CACHE_MAX_AGE_SECS: u64 = 24 * 60 * 60;
const CAMS_WIDGET_CACHE_MAX_AGE_SECS: u64 = 24 * 60 * 60;
const WIDGET_REGISTRY_STAGE_B_ENV: &str = "WC_WIDGET_REGISTRY_STAGE_B";
const OVERLAY_HELPERS_DISABLED_ENV: &str = "WC_DISABLE_OVERLAY_HELPERS";
const MIN_SMOOTH_VIDEO_FPS: f32 = 15.0;
const TICKER_MIN_PASS_SECS: f64 = 14.0;
const TICKER_MAX_PASS_SECS: f64 = 42.0;
const TICKER_READING_CHARS_PER_SEC: f64 = 7.5;
const TICKER_MIN_SHIFT_MS: f64 = 120.0;
const TICKER_MAX_SHIFT_MS: f64 = 650.0;
const WEATHER_MAP_TILE_SIZE: u32 = 256;
const WEATHER_MAP_TARGET_RADIUS_KM: f64 = 24.0;
const WEATHER_POINTER_CENTER_X: i32 = 548;
const WEATHER_POINTER_CENTER_Y: i32 = 258;
const WEATHER_POINTER_RING_RADIUS: i32 = 62;
const STREAM_CAPTURE_TIMEOUT_SECS: u64 = 4;
const YOUTUBE_PAGE_TIMEOUT_SECS: u64 = 4;
const LIVE_MEDIA_EXPERIMENTAL_ENABLED: bool = cfg!(target_os = "linux");

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
    /// Stop spawned overlay helper processes for a config.
    OverlayStop {
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
            let cfg = load_config_with_quote_recovery(&config_path)?;
            validate_config(&cfg)?;
            let cycle = determine_cycle(&cfg, master_rotation_interval(&cfg), "rotation")?;
            run_cycle(&config_path, &cfg, cycle, cycle, false)?;
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
            match ensure_default_local_quotes(&config_path) {
                Ok(Some(quotes_path)) => {
                    println!("created local quotes: {}", quotes_path.display());
                }
                Ok(None) => {}
                Err(err) => {
                    eprintln!("warning: could not create local quotes automatically: {err}");
                }
            }
        }
        Commands::Validate { config } => {
            let config_path = resolve_config_path(config)?;
            let cfg = load_config_with_quote_recovery(&config_path)?;
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
                let cfg = load_config_with_quote_recovery(&config_path)?;
                validate_config(&cfg)?;
                let cycle = determine_cycle(&cfg, master_rotation_interval(&cfg), "rotation")?;
                run_cycle(&config_path, &cfg, cycle, cycle, true)?;

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
            let cfg = load_config_with_quote_recovery(&config_path)?;
            let backup_path = backup_path_for(&config_path);
            fs::copy(&config_path, &backup_path)?;
            fs::write(&config_path, to_config_toml(&cfg))?;
            println!("migrated_config: {}", config_path.display());
            println!("backup_created: {}", backup_path.display());
        }
        Commands::OverlayStop { config } => {
            let config_path = resolve_config_path(config)?;
            stop_overlay_runtime(&config_path)?;
            println!("overlay_runtime: stopped");
        }
    }

    Ok(())
}

fn run_cycle(
    config_path: &Path,
    cfg: &AppConfig,
    image_cycle: u64,
    quote_cycle: u64,
    sync_overlay: bool,
) -> Result<()> {
    let overlay_windows_supported = supports_positioned_overlay_windows();
    let output_path = expand_tilde(&cfg.output_image)?;
    let overlay_status = if sync_overlay {
        sync_overlay_runtime(config_path, cfg, image_cycle)?
    } else {
        "preview-disabled".to_string()
    };
    let source_image = resolve_source_image(cfg, image_cycle)?;
    let widget_bundle = resolve_widgets_for_cycle(cfg, image_cycle, quote_cycle)?;
    // In live loop mode, overlay-rendered media should not be baked into the wallpaper frame.
    // Otherwise users see stale standbilder until the next background rewrite.
    let render_news_into_wallpaper = !(sync_overlay && news_overlay_enabled(cfg));
    let render_cams_into_wallpaper = !(sync_overlay && cams_overlay_enabled(cfg));
    let render_news_ticker2_into_wallpaper =
        !(sync_overlay && news_ticker2_enabled(cfg) && overlay_windows_supported);

    let quote = widget_bundle.quote;
    let clock = widget_bundle.clock;
    let weather_payload = widget_bundle.weather;
    let weather = weather_payload.text.clone();
    let news_payload = if render_news_into_wallpaper {
        widget_bundle.news.clone()
    } else {
        NewsWidgetPayload {
            text: String::new(),
            preview_image: None,
        }
    };
    let news = news_payload.text.clone();
    let news_ticker2 = if render_news_ticker2_into_wallpaper {
        widget_bundle.news_ticker2
    } else {
        String::new()
    };
    let cams_payload = if render_cams_into_wallpaper {
        widget_bundle.cams.clone()
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
            news_ticker2: &news_ticker2,
            news_ticker2_pos_x: cfg.news_ticker2_pos_x,
            news_ticker2_pos_y: cfg.news_ticker2_pos_y,
            news_ticker2_width: cfg.news_ticker2_width,
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
    println!("news_ticker2: {}", news_ticker2);
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
    println!("overlay_runtime: {}", overlay_status);

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
    if !cfg.news_render_mode.trim().eq_ignore_ascii_case("overlay") {
        anyhow::bail!("news_render_mode must be overlay");
    }
    if !cfg.cams_render_mode.trim().eq_ignore_ascii_case("overlay") {
        anyhow::bail!("cams_render_mode must be overlay");
    }
    if !(0.05..=30.0).contains(&cfg.news_fps) {
        anyhow::bail!("news_fps must be between 0.05 and 30.0");
    }
    if cfg.news_refresh_seconds < 10 {
        anyhow::bail!("news_refresh_seconds must be >= 10");
    }
    if news_overlay_enabled(cfg)
        && cfg.news_source.trim().eq_ignore_ascii_case("custom")
        && cfg.news_custom_url.trim().is_empty()
    {
        anyhow::bail!("news_custom_url is required when news_source=custom");
    }
    if news_ticker2_enabled(cfg)
        && cfg
            .news_ticker2_source
            .trim()
            .eq_ignore_ascii_case("custom")
        && cfg.news_ticker2_custom_url.trim().is_empty()
    {
        anyhow::bail!("news_ticker2_custom_url is required when news_ticker2_source=custom");
    }
    if news_widget_enabled(cfg)
        && is_camera_like_url(cfg.news_custom_url.trim())
        && cfg.news_fps > 1.0
    {
        anyhow::bail!("camera-like custom news URLs are limited to max 1.0 FPS");
    }
    if news_ticker2_enabled(cfg)
        && is_camera_like_url(cfg.news_ticker2_custom_url.trim())
        && cfg.news_ticker2_fps > 1.0
    {
        anyhow::bail!("camera-like custom news ticker2 URLs are limited to max 1.0 FPS");
    }
    if !(220..=1920).contains(&cfg.news_ticker2_width) {
        anyhow::bail!("news_ticker2_width must be between 220 and 1920");
    }
    if !(0.05..=30.0).contains(&cfg.news_ticker2_fps) {
        anyhow::bail!("news_ticker2_fps must be between 0.05 and 30.0");
    }
    if cfg.news_ticker2_refresh_seconds < 10 {
        anyhow::bail!("news_ticker2_refresh_seconds must be >= 10");
    }
    if cfg.cams_refresh_seconds < 10 {
        anyhow::bail!("cams_refresh_seconds must be >= 10");
    }
    if !(0.05..=10.0).contains(&cfg.cams_fps) {
        anyhow::bail!("cams_fps must be between 0.05 and 10.0");
    }
    if news_overlay_video_enabled(cfg) && overlay_video_player().is_none() {
        anyhow::bail!(
            "news overlay requires mpv or ffplay on PATH; packaged builds should install mpv"
        );
    }
    if cams_overlay_enabled(cfg) && overlay_video_player().is_none() {
        anyhow::bail!(
            "cams overlay requires mpv or ffplay on PATH; packaged builds should install mpv"
        );
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
    if cfg.overlay_script_ticker_enabled && cfg.overlay_script_ticker_command.trim().is_empty() {
        anyhow::bail!(
            "overlay_script_ticker_command is required when overlay_script_ticker_enabled=true"
        );
    }
    if !(220..=1920).contains(&cfg.overlay_script_ticker_width) {
        anyhow::bail!("overlay_script_ticker_width must be between 220 and 1920");
    }
    if !(32..=240).contains(&cfg.overlay_script_ticker_height) {
        anyhow::bail!("overlay_script_ticker_height must be between 32 and 240");
    }
    if !(10..=120).contains(&cfg.overlay_script_ticker_font_size) {
        anyhow::bail!("overlay_script_ticker_font_size must be between 10 and 120");
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
            let candidates = list_background_images(&image_dir)?;
            if candidates.is_empty() {
                anyhow::bail!("no images available in {}", image_dir.display());
            }
            if let Some((last_cycle, last_idx)) = read_cycle_pick_state(&state_path)
                && last_cycle == cycle
                && last_idx < candidates.len()
            {
                return Ok(candidates[last_idx].clone());
            }
            let recent_indices = read_recent_indices(&state_path);
            let (picked, picked_idx) = pick_background_image_with_mode(
                &image_dir,
                cycle,
                &cfg.image_order_mode,
                cfg.image_avoid_repeat,
                &recent_indices,
            )?;
            write_recent_indices(&state_path, &recent_indices, picked_idx)?;
            write_cycle_pick_state(&state_path, cycle, picked_idx)?;
            Ok(picked)
        }
        "preset" | "remote_preset" => {
            let (endpoint, provider) = resolve_image_endpoint_from_preset(cfg)?;
            resolve_remote_image_for_cycle(
                cfg,
                cycle,
                &format!(
                    "preset:{}:{endpoint}",
                    cfg.image_source_preset.as_deref().unwrap_or("")
                ),
                || fetch_remote_image(endpoint.clone(), provider),
            )
            .map_err(|e| anyhow::anyhow!("failed to fetch preset image source: {e}"))
        }
        "url" | "remote_url" => {
            let endpoint = resolve_image_endpoint_from_url(cfg, cycle)?;
            if stream_like_endpoint(&endpoint) {
                let bg_fps = (1.0 / cfg.image_refresh_seconds.max(1) as f32).clamp(0.05, 30.0);
                if let Some(frame) = capture_stream_preview(&endpoint, bg_fps) {
                    return Ok(frame);
                }
            }
            resolve_remote_image_for_cycle(cfg, cycle, &format!("url:{endpoint}"), || {
                fetch_remote_image(endpoint.clone(), ImageProvider::Generic)
            })
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
            let quotes = load_quotes(&quotes_path)?;
            if let Some((last_cycle, last_idx)) = read_cycle_pick_state(&state_path)
                && last_cycle == cycle
                && last_idx < quotes.len()
            {
                return Ok(strip_project_line_suffix(&quotes[last_idx]));
            }
            let recent_indices = read_recent_indices(&state_path);
            let (picked, picked_idx) = pick_quote_with_mode(
                &quotes_path,
                cycle,
                &cfg.quote_order_mode,
                cfg.quote_avoid_repeat,
                &recent_indices,
            )?;
            write_recent_indices(&state_path, &recent_indices, picked_idx)?;
            write_cycle_pick_state(&state_path, cycle, picked_idx)?;
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
    let (_, wind_dir) = compass_arrow(wind_deg);
    let temp_unit = match units {
        UnitSystem::Metric => "C",
        UnitSystem::Imperial => "F",
    };
    let wind_unit = match units {
        UnitSystem::Metric => "km/h",
        UnitSystem::Imperial => "mph",
    };
    let minimap_image = resolve_weather_minimap(geo.lat, geo.lon, wind_deg, wind, wind_unit)
        .or_else(|| fallback_weather_minimap(wind_deg, wind, wind_unit));
    let text = format_weather_compact(&WeatherCompactInput {
        location_label: &geo.label,
        condition_label: weather_code_label(code),
        temp: t,
        feels,
        rain_prob,
        wind_dir,
        wind_speed: wind,
        humidity,
        temp_unit,
        wind_unit,
    });

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
    let area_name = area
        .and_then(|a| a.get("areaName"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
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
    let wind_deg = current
        .get("winddirDegree")
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or_else(|| compass_degrees_for_name(wind_dir));
    let _wind_arrow = compass_arrow_for_name(wind_dir);
    let area_lat = area
        .and_then(|a| a.get("latitude"))
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok());
    let area_lon = area
        .and_then(|a| a.get("longitude"))
        .and_then(Value::as_str)
        .and_then(|v| v.parse::<f64>().ok());
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
    let minimap_image = area_lat
        .zip(area_lon)
        .and_then(|(lat, lon)| resolve_weather_minimap(lat, lon, wind_deg, wind, wind_unit))
        .or_else(|| fallback_weather_minimap(wind_deg, wind, wind_unit));
    Ok(WeatherWidgetPayload {
        text: format_weather_compact(&WeatherCompactInput {
            location_label: &format!("{area_name}, {country}"),
            condition_label: compact_condition_label(desc),
            temp,
            feels,
            rain_prob: rain,
            wind_dir,
            wind_speed: wind,
            humidity,
            temp_unit,
            wind_unit,
        }),
        minimap_image,
    })
}

#[derive(Debug, Clone)]
struct NewsWidgetPayload {
    text: String,
    preview_image: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct NewsCachedPayload {
    raw_line: String,
    preview_image: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CamsWidgetPayload {
    text: String,
    preview_image: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CamsCachedPayload {
    base_line: String,
    preview_image: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct CamSourceEntry {
    label: String,
    url: String,
}

#[derive(Debug, Clone)]
struct WidgetRenderBundle {
    quote: String,
    clock: String,
    weather: WeatherWidgetPayload,
    news: NewsWidgetPayload,
    news_ticker2: String,
    cams: CamsWidgetPayload,
}

fn widget_registry_stage_b_enabled() -> bool {
    std::env::var(WIDGET_REGISTRY_STAGE_B_ENV).is_ok_and(|raw| {
        matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        )
    })
}

fn weather_unavailable_payload(err: impl std::fmt::Display) -> WeatherWidgetPayload {
    WeatherWidgetPayload {
        text: format!(
            "⚠ {}",
            compact_news_line(&format!("weather unavailable ({err})"))
        ),
        minimap_image: fallback_weather_minimap(0.0, 0.0, "km/h"),
    }
}

fn news_unavailable_payload(err: impl std::fmt::Display) -> NewsWidgetPayload {
    NewsWidgetPayload {
        text: format!("News unavailable ({err})"),
        preview_image: None,
    }
}

fn cams_unavailable_payload(err: impl std::fmt::Display) -> CamsWidgetPayload {
    CamsWidgetPayload {
        text: format!("CAMS ◆ unavailable ({err})"),
        preview_image: None,
    }
}

fn resolve_widgets_for_cycle(
    cfg: &AppConfig,
    image_cycle: u64,
    quote_cycle: u64,
) -> Result<WidgetRenderBundle> {
    if widget_registry_stage_b_enabled() {
        match resolve_widgets_via_registry(cfg, image_cycle, quote_cycle) {
            Ok(bundle) => Ok(bundle),
            Err(err) => {
                eprintln!(
                    "warning: widget registry stage-B path failed, falling back to legacy path: {err}"
                );
                resolve_widgets_legacy(cfg, image_cycle, quote_cycle)
            }
        }
    } else {
        resolve_widgets_legacy(cfg, image_cycle, quote_cycle)
    }
}

fn resolve_widgets_legacy(
    cfg: &AppConfig,
    image_cycle: u64,
    quote_cycle: u64,
) -> Result<WidgetRenderBundle> {
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
    let weather = if cfg.show_weather_layer {
        resolve_weather_widget(cfg).unwrap_or_else(weather_unavailable_payload)
    } else {
        WeatherWidgetPayload {
            text: String::new(),
            minimap_image: None,
        }
    };
    let news = if news_widget_enabled(cfg) {
        resolve_news_widget(cfg, image_cycle).unwrap_or_else(news_unavailable_payload)
    } else {
        NewsWidgetPayload {
            text: String::new(),
            preview_image: None,
        }
    };
    let news_ticker2 = if news_ticker2_enabled(cfg) {
        resolve_secondary_news_ticker(cfg)
    } else {
        String::new()
    };
    let cams = if cams_widget_enabled(cfg) {
        resolve_cams_widget(cfg).unwrap_or_else(cams_unavailable_payload)
    } else {
        CamsWidgetPayload {
            text: String::new(),
            preview_image: None,
        }
    };

    Ok(WidgetRenderBundle {
        quote,
        clock,
        weather,
        news,
        news_ticker2,
        cams,
    })
}

fn resolve_widgets_via_registry(
    cfg: &AppConfig,
    image_cycle: u64,
    quote_cycle: u64,
) -> Result<WidgetRenderBundle> {
    let registry = build_builtin_widget_registry(cfg, image_cycle, quote_cycle)?;
    let cache_dir =
        expand_tilde("~/.cache/wallpaper-composer").unwrap_or_else(|_| std::env::temp_dir());
    let mut resolved = BTreeMap::<String, WidgetResolvedPayload>::new();

    for plugin in registry.all() {
        let instance = widget_instance_from_config(cfg, plugin.type_id())?;
        if !instance.enabled {
            continue;
        }
        plugin.validate(&instance)?;
        let payload = plugin.resolve(
            &instance,
            &WidgetRuntimeContext {
                cycle: image_cycle,
                cache_dir: cache_dir.clone(),
                now_unix: now_epoch_seconds(),
            },
        )?;
        resolved.insert(plugin.type_id().to_string(), payload);
    }

    Ok(WidgetRenderBundle {
        quote: resolved_text(&resolved, "quote"),
        clock: resolved_text(&resolved, "clock"),
        weather: WeatherWidgetPayload {
            text: resolved_text(&resolved, "weather"),
            minimap_image: resolved_image(&resolved, "weather"),
        },
        news: NewsWidgetPayload {
            text: resolved_text(&resolved, "news"),
            preview_image: resolved_image(&resolved, "news"),
        },
        news_ticker2: resolved_text(&resolved, "news_ticker2"),
        cams: CamsWidgetPayload {
            text: resolved_text(&resolved, "cams"),
            preview_image: resolved_image(&resolved, "cams"),
        },
    })
}

fn resolved_text(map: &BTreeMap<String, WidgetResolvedPayload>, type_id: &str) -> String {
    map.get(type_id).map(|p| p.text.clone()).unwrap_or_default()
}

fn resolved_image(map: &BTreeMap<String, WidgetResolvedPayload>, type_id: &str) -> Option<PathBuf> {
    map.get(type_id).and_then(|p| p.image_path.clone())
}

fn build_builtin_widget_registry(
    cfg: &AppConfig,
    image_cycle: u64,
    quote_cycle: u64,
) -> Result<WidgetRegistry> {
    let mut registry = WidgetRegistry::new();
    registry.register(Box::new(QuoteWidgetPlugin {
        cfg: cfg.clone(),
        quote_cycle,
    }))?;
    registry.register(Box::new(ClockWidgetPlugin { cfg: cfg.clone() }))?;
    registry.register(Box::new(WeatherWidgetPlugin { cfg: cfg.clone() }))?;
    registry.register(Box::new(NewsWidgetPlugin {
        cfg: cfg.clone(),
        image_cycle,
    }))?;
    registry.register(Box::new(NewsTicker2WidgetPlugin { cfg: cfg.clone() }))?;
    registry.register(Box::new(CamsWidgetPlugin { cfg: cfg.clone() }))?;

    if registry.len() != BUILTIN_WIDGET_TYPE_IDS.len() {
        anyhow::bail!(
            "builtin widget registry mismatch: registered={}, expected={}",
            registry.len(),
            BUILTIN_WIDGET_TYPE_IDS.len()
        );
    }
    Ok(registry)
}

fn widget_instance_from_config(cfg: &AppConfig, widget_type: &str) -> Result<WidgetInstanceConfig> {
    match widget_type {
        "quote" => {
            let mut instance = WidgetInstanceConfig::new("quote", "quote_main");
            instance.enabled = cfg.show_quote_layer;
            instance.layer_z = cfg.layer_z_quote;
            instance.pos_x = cfg.quote_pos_x;
            instance.pos_y = cfg.quote_pos_y;
            instance.width =
                ((1920_u64 * cfg.text_box_width_pct.min(100) as u64) / 100).max(1) as u32;
            instance.height =
                ((1080_u64 * cfg.text_box_height_pct.min(100) as u64) / 100).max(1) as u32;
            instance.refresh_seconds = cfg.quote_refresh_seconds.max(1);
            instance.fps_cap = 1.0;
            Ok(instance)
        }
        "clock" => {
            let mut instance = WidgetInstanceConfig::new("clock", "clock_main");
            instance.enabled = cfg.show_clock_layer;
            instance.layer_z = cfg.layer_z_clock;
            instance.pos_x = cfg.clock_pos_x;
            instance.pos_y = cfg.clock_pos_y;
            instance.width = 180;
            instance.height = 64;
            instance.refresh_seconds = 1;
            instance.fps_cap = 1.0;
            Ok(instance)
        }
        "weather" => {
            let mut instance = WidgetInstanceConfig::new("weather", "weather_main");
            instance.enabled = cfg.show_weather_layer;
            instance.layer_z = cfg.layer_z_weather;
            instance.pos_x = cfg.weather_pos_x;
            instance.pos_y = cfg.weather_pos_y;
            instance.width = cfg.weather_widget_width;
            instance.height = cfg.weather_widget_height;
            instance.refresh_seconds = cfg.weather_refresh_seconds.max(60);
            instance.fps_cap = 1.0;
            Ok(instance)
        }
        "news" => {
            let mut instance = WidgetInstanceConfig::new("news", "news_main");
            instance.enabled = news_widget_enabled(cfg);
            instance.layer_z = cfg.layer_z_news;
            instance.pos_x = cfg.news_pos_x;
            instance.pos_y = cfg.news_pos_y;
            instance.width = cfg.news_widget_width;
            instance.height = cfg.news_widget_height;
            instance.refresh_seconds = cfg.news_refresh_seconds.max(10);
            instance.fps_cap = cfg.news_fps;
            Ok(instance)
        }
        "news_ticker2" => {
            let mut instance = WidgetInstanceConfig::new("news_ticker2", "news_ticker2_main");
            instance.enabled = news_ticker2_enabled(cfg);
            instance.layer_z = cfg.layer_z_news;
            instance.pos_x = cfg.news_ticker2_pos_x;
            instance.pos_y = cfg.news_ticker2_pos_y;
            instance.width = cfg.news_ticker2_width;
            instance.height = 56;
            instance.refresh_seconds = cfg.news_ticker2_refresh_seconds.max(10);
            instance.fps_cap = cfg.news_ticker2_fps;
            Ok(instance)
        }
        "cams" => {
            let mut instance = WidgetInstanceConfig::new("cams", "cams_main");
            instance.enabled = cams_widget_enabled(cfg);
            instance.layer_z = cfg.layer_z_cams;
            instance.pos_x = cfg.cams_pos_x;
            instance.pos_y = cfg.cams_pos_y;
            instance.width = cfg.cams_widget_width;
            instance.height = cfg.cams_widget_height;
            instance.refresh_seconds = cfg.cams_refresh_seconds.max(10);
            instance.fps_cap = cfg.cams_fps;
            Ok(instance)
        }
        other => anyhow::bail!("unsupported widget type for stage-B registry: {other}"),
    }
}

struct QuoteWidgetPlugin {
    cfg: AppConfig,
    quote_cycle: u64,
}

impl WidgetPlugin for QuoteWidgetPlugin {
    fn type_id(&self) -> &'static str {
        "quote"
    }

    fn display_name(&self) -> &'static str {
        "Quote"
    }

    fn default_instance(&self) -> WidgetInstanceConfig {
        widget_instance_from_config(&self.cfg, "quote")
            .unwrap_or_else(|_| WidgetInstanceConfig::new("quote", "quote_main"))
    }

    fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
        Ok(())
    }

    fn resolve(
        &self,
        _instance: &WidgetInstanceConfig,
        _ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload> {
        Ok(WidgetResolvedPayload {
            text: resolve_quote(&self.cfg, self.quote_cycle)?,
            image_path: None,
        })
    }
}

struct ClockWidgetPlugin {
    cfg: AppConfig,
}

impl WidgetPlugin for ClockWidgetPlugin {
    fn type_id(&self) -> &'static str {
        "clock"
    }

    fn display_name(&self) -> &'static str {
        "Clock"
    }

    fn default_instance(&self) -> WidgetInstanceConfig {
        widget_instance_from_config(&self.cfg, "clock")
            .unwrap_or_else(|_| WidgetInstanceConfig::new("clock", "clock_main"))
    }

    fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
        Ok(())
    }

    fn resolve(
        &self,
        _instance: &WidgetInstanceConfig,
        _ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload> {
        Ok(WidgetResolvedPayload {
            text: chrono::Local::now()
                .format(&self.cfg.time_format)
                .to_string(),
            image_path: None,
        })
    }
}

struct WeatherWidgetPlugin {
    cfg: AppConfig,
}

impl WidgetPlugin for WeatherWidgetPlugin {
    fn type_id(&self) -> &'static str {
        "weather"
    }

    fn display_name(&self) -> &'static str {
        "Weather"
    }

    fn default_instance(&self) -> WidgetInstanceConfig {
        widget_instance_from_config(&self.cfg, "weather")
            .unwrap_or_else(|_| WidgetInstanceConfig::new("weather", "weather_main"))
    }

    fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
        Ok(())
    }

    fn resolve(
        &self,
        _instance: &WidgetInstanceConfig,
        _ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload> {
        let payload = resolve_weather_widget(&self.cfg).unwrap_or_else(weather_unavailable_payload);
        Ok(WidgetResolvedPayload {
            text: payload.text,
            image_path: payload.minimap_image,
        })
    }
}

struct NewsWidgetPlugin {
    cfg: AppConfig,
    image_cycle: u64,
}

impl WidgetPlugin for NewsWidgetPlugin {
    fn type_id(&self) -> &'static str {
        "news"
    }

    fn display_name(&self) -> &'static str {
        "News"
    }

    fn default_instance(&self) -> WidgetInstanceConfig {
        widget_instance_from_config(&self.cfg, "news")
            .unwrap_or_else(|_| WidgetInstanceConfig::new("news", "news_main"))
    }

    fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
        Ok(())
    }

    fn resolve(
        &self,
        _instance: &WidgetInstanceConfig,
        _ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload> {
        let payload = resolve_news_widget(&self.cfg, self.image_cycle)
            .unwrap_or_else(news_unavailable_payload);
        Ok(WidgetResolvedPayload {
            text: payload.text,
            image_path: payload.preview_image,
        })
    }
}

struct NewsTicker2WidgetPlugin {
    cfg: AppConfig,
}

impl WidgetPlugin for NewsTicker2WidgetPlugin {
    fn type_id(&self) -> &'static str {
        "news_ticker2"
    }

    fn display_name(&self) -> &'static str {
        "News"
    }

    fn default_instance(&self) -> WidgetInstanceConfig {
        widget_instance_from_config(&self.cfg, "news_ticker2")
            .unwrap_or_else(|_| WidgetInstanceConfig::new("news_ticker2", "news_ticker2_main"))
    }

    fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
        Ok(())
    }

    fn resolve(
        &self,
        _instance: &WidgetInstanceConfig,
        _ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload> {
        Ok(WidgetResolvedPayload {
            text: resolve_secondary_news_ticker(&self.cfg),
            image_path: None,
        })
    }
}

struct CamsWidgetPlugin {
    cfg: AppConfig,
}

impl WidgetPlugin for CamsWidgetPlugin {
    fn type_id(&self) -> &'static str {
        "cams"
    }

    fn display_name(&self) -> &'static str {
        "Cams"
    }

    fn default_instance(&self) -> WidgetInstanceConfig {
        widget_instance_from_config(&self.cfg, "cams")
            .unwrap_or_else(|_| WidgetInstanceConfig::new("cams", "cams_main"))
    }

    fn validate(&self, _instance: &WidgetInstanceConfig) -> Result<()> {
        Ok(())
    }

    fn resolve(
        &self,
        _instance: &WidgetInstanceConfig,
        _ctx: &WidgetRuntimeContext,
    ) -> Result<WidgetResolvedPayload> {
        let payload = resolve_cams_widget(&self.cfg).unwrap_or_else(cams_unavailable_payload);
        Ok(WidgetResolvedPayload {
            text: payload.text,
            image_path: payload.preview_image,
        })
    }
}

fn resolve_news_widget(cfg: &AppConfig, cycle: u64) -> Result<NewsWidgetPayload> {
    let (label, stream_url, feed_url) = news_source_profile(cfg);
    let preview_image = resolve_news_preview_image(cfg, &stream_url, cycle);
    let cache_id = format!(
        "news-main-{}",
        stable_hash(&format!(
            "{}|{}|{}|{}",
            cfg.news_source, cfg.news_custom_url, cfg.news_refresh_seconds, cfg.news_fps
        ))
    );
    let cached = load_cached_news_payload(&cache_id, cfg.news_refresh_seconds)?;
    let payload = if let Some(fresh) = cached {
        fresh
    } else {
        match fetch_news_payload(label, &stream_url, feed_url, Some((cfg, cycle))) {
            Ok(fetched) => {
                store_cached_news_payload(&cache_id, &fetched)?;
                fetched
            }
            Err(e) => {
                if let Some(stale) =
                    load_cached_news_payload(&cache_id, NEWS_WIDGET_CACHE_MAX_AGE_SECS)?
                {
                    stale
                } else {
                    return Err(e);
                }
            }
        }
    };
    let line = compact_news_line(&payload.raw_line);
    Ok(NewsWidgetPayload {
        text: line,
        preview_image: preview_image.or(payload.preview_image),
    })
}

fn resolve_secondary_news_ticker(cfg: &AppConfig) -> String {
    let (label, stream_url, feed_url) =
        news_source_profile_raw(&cfg.news_ticker2_source, &cfg.news_ticker2_custom_url);
    let cache_id = format!(
        "news-ticker2-{}",
        stable_hash(&format!(
            "{}|{}|{}|{}",
            cfg.news_ticker2_source,
            cfg.news_ticker2_custom_url,
            cfg.news_ticker2_refresh_seconds,
            cfg.news_ticker2_fps
        ))
    );
    let payload = load_cached_news_payload(&cache_id, cfg.news_ticker2_refresh_seconds)
        .ok()
        .flatten()
        .or_else(|| {
            fetch_news_payload(label, &stream_url, feed_url, None)
                .ok()
                .inspect(|fetched| {
                    let _ = store_cached_news_payload(&cache_id, fetched);
                })
        })
        .or_else(|| {
            load_cached_news_payload(&cache_id, NEWS_WIDGET_CACHE_MAX_AGE_SECS)
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| NewsCachedPayload {
            raw_line: compact_news_line("Custom ◆ live source"),
            preview_image: None,
        });

    news_ticker_frame(&payload.raw_line)
}

fn resolve_cams_widget(cfg: &AppConfig) -> Result<CamsWidgetPayload> {
    let source_cycle = now_epoch_seconds() / cfg.cams_refresh_seconds.max(10);
    let mut sources = cams_source_entries(cfg, source_cycle);
    if sources.is_empty() {
        anyhow::bail!("no camera URLs available");
    }

    let count = cfg.cams_count.clamp(1, 5) as usize;
    if sources.len() > count {
        sources.truncate(count);
    }
    let source_fingerprint = sources
        .iter()
        .map(|entry| format!("{}=>{}", entry.label, entry.url))
        .collect::<Vec<_>>()
        .join("|");

    let cache_id = format!(
        "cams-{}",
        stable_hash(&format!(
            "{}|{}|{}|{}|{}|{}|{}",
            cfg.cams_source,
            cfg.cams_custom_urls,
            cfg.cams_count,
            cfg.cams_columns,
            cfg.cams_refresh_seconds,
            source_cycle,
            source_fingerprint
        ))
    );
    let preview_image = resolve_cams_preview_image(cfg, &sources);
    let cached = load_cached_cams_payload(&cache_id, cfg.cams_refresh_seconds)?;
    let payload = if let Some(fresh) = cached {
        fresh
    } else {
        let short = sources
            .iter()
            .map(cam_source_display_label)
            .collect::<Vec<_>>()
            .join(" ◆ ");
        let built = CamsCachedPayload {
            base_line: compact_news_line(&format!("CAMS ◆ {short}")),
            preview_image: preview_image.clone(),
        };
        store_cached_cams_payload(&cache_id, &built)?;
        if built.preview_image.is_some() {
            built
        } else if let Some(stale) =
            load_cached_cams_payload(&cache_id, CAMS_WIDGET_CACHE_MAX_AGE_SECS)?
        {
            stale
        } else {
            built
        }
    };
    let text = news_ticker_frame(&payload.base_line);

    Ok(CamsWidgetPayload {
        text,
        preview_image: preview_image.or(payload.preview_image),
    })
}

fn resolve_cams_preview_image(cfg: &AppConfig, sources: &[CamSourceEntry]) -> Option<PathBuf> {
    let mut frames = Vec::<PathBuf>::new();
    for source in sources {
        if let Some(frame) =
            resolve_cam_source_preview(&source.url, effective_video_fps(cfg.cams_fps))
        {
            frames.push(frame);
        }
    }
    compose_cams_grid(&frames, cfg.cams_columns.clamp(1, 4)).or_else(|| frames.first().cloned())
}

fn resolve_cam_source_preview(url: &str, fps: f32) -> Option<PathBuf> {
    if let Some(frame) = capture_stream_preview(url, fps) {
        return Some(frame);
    }
    let lower = url.trim().to_ascii_lowercase();
    if lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
    {
        return fetch_remote_image(url.to_string(), ImageProvider::Generic).ok();
    }
    None
}

fn fetch_news_payload(
    label: &str,
    stream_url: &str,
    feed_url: Option<&str>,
    with_preview: Option<(&AppConfig, u64)>,
) -> Result<NewsCachedPayload> {
    let headline = if let Some(feed) = feed_url {
        fetch_rss_ticker(feed).unwrap_or_else(|| "live feed".to_string())
    } else {
        "live source".to_string()
    };
    let subtitle_hint = fetch_stream_title_hint(stream_url).unwrap_or_default();
    let raw_line = if subtitle_hint.is_empty() {
        compact_news_line(&format!("{label} ◆ {headline}"))
    } else {
        compact_news_line(&format!("{label} ◆ {headline} ◆ {subtitle_hint}"))
    };
    let preview_image =
        with_preview.and_then(|(cfg, cycle)| resolve_news_preview_image(cfg, stream_url, cycle));
    Ok(NewsCachedPayload {
        raw_line,
        preview_image,
    })
}

fn cams_source_entries(cfg: &AppConfig, cycle: u64) -> Vec<CamSourceEntry> {
    match cfg.cams_source.trim().to_ascii_lowercase().as_str() {
        "custom" => {
            let mut entries = parse_cam_source_list(&cfg.cams_custom_urls);
            entries.retain(|entry| looks_like_endpoint(&entry.url));
            if entries.is_empty() {
                auto_local_cam_entries(cycle)
            } else {
                rotate_cam_entries(entries, cycle)
            }
        }
        "city_public" => city_public_cam_entries(cycle),
        _ => auto_local_cam_entries(cycle),
    }
}

fn auto_local_cam_entries(cycle: u64) -> Vec<CamSourceEntry> {
    rotate_cam_entries(
        vec![
            CamSourceEntry {
                label: "Belgrade - Knez Mihailova".to_string(),
                url: "https://stream.uzivobeograd.rs/live/cam_20.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Berlin - Funkturm".to_string(),
                url: "https://www.berlin.de/webcams/fsz/webcam.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Paris - Skyline".to_string(),
                url: "https://images.webcamgalore.com/33422-current-webcam-Paris.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Rome - Panorama".to_string(),
                url: "https://images.webcamgalore.com/4513-current-webcam-Rome.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Euronews Live".to_string(),
                url: "https://www.youtube.com/watch?v=pykpO5kQJ98".to_string(),
            },
            CamSourceEntry {
                label: "Earth Orbit - NASA".to_string(),
                url: "https://www.youtube.com/watch?v=21X5lGlDOfg".to_string(),
            },
        ],
        cycle,
    )
}

fn city_public_cam_entries(cycle: u64) -> Vec<CamSourceEntry> {
    rotate_cam_entries(
        vec![
            CamSourceEntry {
                label: "Belgrade".to_string(),
                url: "https://stream.uzivobeograd.rs/live/cam_20.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Berlin".to_string(),
                url: "https://www.berlin.de/webcams/fsz/webcam.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Paris".to_string(),
                url: "https://images.webcamgalore.com/33422-current-webcam-Paris.jpg".to_string(),
            },
            CamSourceEntry {
                label: "Rome".to_string(),
                url: "https://images.webcamgalore.com/4513-current-webcam-Rome.jpg".to_string(),
            },
        ],
        cycle,
    )
}

fn rotate_cam_entries(entries: Vec<CamSourceEntry>, cycle: u64) -> Vec<CamSourceEntry> {
    if entries.is_empty() {
        return entries;
    }
    let shift = (cycle as usize) % entries.len();
    entries[shift..]
        .iter()
        .chain(entries[..shift].iter())
        .cloned()
        .collect()
}

fn cam_source_display_label(source: &CamSourceEntry) -> String {
    let label = compact_news_line(source.label.trim());
    if !label.is_empty() {
        return label;
    }
    summarize_source_label(&source.url)
}

fn summarize_source_label(url: &str) -> String {
    if let Some(id) = extract_youtube_video_id(url) {
        if let Some(label) = known_youtube_source_label(&id) {
            return label.to_string();
        }
        let short = id.chars().take(6).collect::<String>();
        return format!("YouTube:{short}");
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

fn known_youtube_source_label(id: &str) -> Option<&'static str> {
    match id {
        "1-iS7LArMPA" => Some("New York"),
        "GE_SfNVNyqk" => Some("Berlin"),
        "l8PMl7tUDIE" => Some("Paris"),
        "gCNeDWCI0vo" => Some("Doha"),
        "pykpO5kQJ98" => Some("Euronews"),
        "21X5lGlDOfg" => Some("NASA"),
        "dp8PhLsUcFE" => Some("Bloomberg"),
        "9Auq9mYxFEE" => Some("Yahoo"),
        _ => None,
    }
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

fn news_source_profile(cfg: &AppConfig) -> (&'static str, String, Option<&'static str>) {
    news_source_profile_raw(&cfg.news_source, &cfg.news_custom_url)
}

fn news_source_supports_live_video_source(source: &str, custom_url: &str) -> bool {
    if source == "custom" {
        return stream_like_endpoint(custom_url);
    }
    builtin_news_source_is_live_video(source)
}

fn news_overlay_video_enabled(cfg: &AppConfig) -> bool {
    news_overlay_enabled(cfg)
        && news_source_supports_live_video_source(&cfg.news_source, &cfg.news_custom_url)
}

fn news_source_profile_raw(
    source: &str,
    custom_url: &str,
) -> (&'static str, String, Option<&'static str>) {
    if let Some(source) = builtin_news_source(source) {
        return (
            source.name,
            source.stream_url.to_string(),
            source.ticker_url,
        );
    }
    ("Custom", custom_url.trim().to_string(), None)
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
        && let Some(single) =
            extract_first_rss_item_title(&body).or_else(|| extract_first_xml_tag(&body, "title"))
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
    let effective_fps = effective_video_fps(cfg.news_fps);
    if let Some(path) = capture_stream_preview(stream_url, effective_fps) {
        return Some(path);
    }

    if cfg.news_source.eq_ignore_ascii_case("custom")
        && is_camera_like_url(stream_url)
        && let Some(path) = capture_camera_frame(stream_url, effective_fps)
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

struct WeatherCompactInput<'a> {
    location_label: &'a str,
    condition_label: &'a str,
    temp: f64,
    feels: f64,
    rain_prob: f64,
    wind_dir: &'a str,
    wind_speed: f64,
    humidity: f64,
    temp_unit: &'a str,
    wind_unit: &'a str,
}

fn format_weather_compact(input: &WeatherCompactInput<'_>) -> String {
    format!(
        "{}\n{}  {:.1}{}  feels {:.1}{}\nRain {:.0}%  Wind {} {:.1} {}\nHumidity {:.0}%",
        input.location_label,
        input.condition_label,
        input.temp,
        input.temp_unit,
        input.feels,
        input.temp_unit,
        input.rain_prob,
        input.wind_dir,
        input.wind_speed,
        input.wind_unit,
        input.humidity
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

fn resolve_weather_minimap(
    lat: f64,
    lon: f64,
    wind_deg: f64,
    wind_speed: f64,
    wind_unit: &str,
) -> Option<PathBuf> {
    let raw_map = resolve_weather_minimap_raw(lat, lon)?;
    stylize_weather_minimap(&raw_map, lat, lon, wind_deg, wind_speed, wind_unit)
}

fn resolve_weather_minimap_raw(lat: f64, lon: f64) -> Option<PathBuf> {
    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let zoom = weather_map_zoom_for_lat(lat);
    let weather_map_source_fingerprint = "carto-light+osm-fallback";
    let output = cache_dir.join(format!(
        "weather-map-base-{}.png",
        stable_hash(&format!(
            "{lat:.4}:{lon:.4}:z{zoom}:{weather_map_source_fingerprint}"
        ))
    ));
    if output.exists() {
        return Some(output);
    }

    let (tile_x, tile_y) = lat_lon_to_tile(lat, lon, zoom);
    let base_x = tile_x.floor() as i64 - 1;
    let base_y = tile_y.floor() as i64 - 1;
    let tile_span = (1_i64) << zoom;
    let mut mosaic = RgbaImage::from_pixel(
        WEATHER_MAP_TILE_SIZE * 3,
        WEATHER_MAP_TILE_SIZE * 3,
        Rgba([18, 22, 28, 255]),
    );
    let mut downloaded_any = false;

    for oy in 0..3_i64 {
        for ox in 0..3_i64 {
            let wrapped_x = (base_x + ox).rem_euclid(tile_span);
            let clamped_y = (base_y + oy).clamp(0, tile_span.saturating_sub(1));
            for url in weather_tile_urls(zoom, wrapped_x, clamped_y) {
                if let Ok(path) = fetch_remote_image(url, ImageProvider::Generic)
                    && let Ok(tile) = image::open(path)
                {
                    let tile_rgba = tile.to_rgba8();
                    imageops::overlay(
                        &mut mosaic,
                        &tile_rgba,
                        ox * WEATHER_MAP_TILE_SIZE as i64,
                        oy * WEATHER_MAP_TILE_SIZE as i64,
                    );
                    downloaded_any = true;
                    break;
                }
            }
        }
    }

    if !downloaded_any {
        return None;
    }

    let center_px_x = ((tile_x - base_x as f64) * WEATHER_MAP_TILE_SIZE as f64).round() as i64;
    let center_px_y = ((tile_y - base_y as f64) * WEATHER_MAP_TILE_SIZE as f64).round() as i64;
    let max_crop_x = (mosaic.width().saturating_sub(640)) as i64;
    let max_crop_y = (mosaic.height().saturating_sub(360)) as i64;
    let crop_x = (center_px_x - 320).clamp(0, max_crop_x) as u32;
    let crop_y = (center_px_y - 180).clamp(0, max_crop_y) as u32;
    let cropped = imageops::crop_imm(&mosaic, crop_x, crop_y, 640, 360).to_image();
    write_weather_image(&output, &cropped)
}

fn weather_tile_urls(zoom: u32, tile_x: i64, tile_y: i64) -> [String; 3] {
    [
        format!("https://a.basemaps.cartocdn.com/light_all/{zoom}/{tile_x}/{tile_y}.png"),
        format!("https://b.basemaps.cartocdn.com/light_all/{zoom}/{tile_x}/{tile_y}.png"),
        format!("https://tile.openstreetmap.org/{zoom}/{tile_x}/{tile_y}.png"),
    ]
}

fn weather_map_zoom_for_lat(lat: f64) -> u32 {
    let lat_cos = lat.to_radians().cos().abs().max(0.2);
    let target_width_meters = WEATHER_MAP_TARGET_RADIUS_KM * 2.0 * 1000.0;
    let meters_per_pixel = target_width_meters / 640.0;
    let world_meters = 40_075_016.686_f64 * lat_cos;
    let zoom = (world_meters / (WEATHER_MAP_TILE_SIZE as f64 * meters_per_pixel)).log2();
    zoom.round().clamp(8.0, 12.0) as u32
}

fn lat_lon_to_tile(lat: f64, lon: f64, zoom: u32) -> (f64, f64) {
    let lat_rad = lat.to_radians();
    let n = 2.0_f64.powi(zoom as i32);
    let x = (lon + 180.0) / 360.0 * n;
    let y = (1.0 - ((lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f64::consts::PI)) / 2.0 * n;
    (x, y)
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

fn compass_degrees_for_name(dir: &str) -> f64 {
    match dir.trim().to_ascii_uppercase().as_str() {
        "N" => 0.0,
        "NNE" => 22.5,
        "NE" => 45.0,
        "ENE" => 67.5,
        "E" => 90.0,
        "ESE" => 112.5,
        "SE" => 135.0,
        "SSE" => 157.5,
        "S" => 180.0,
        "SSW" => 202.5,
        "SW" => 225.0,
        "WSW" => 247.5,
        "W" => 270.0,
        "WNW" => 292.5,
        "NW" => 315.0,
        "NNW" => 337.5,
        _ => 0.0,
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

fn ticker_shift_millis_for_len(char_count: usize) -> u64 {
    let chars = char_count.max(1) as f64;
    let pass_seconds =
        (chars / TICKER_READING_CHARS_PER_SEC).clamp(TICKER_MIN_PASS_SECS, TICKER_MAX_PASS_SECS);
    ((pass_seconds * 1000.0) / chars)
        .clamp(TICKER_MIN_SHIFT_MS, TICKER_MAX_SHIFT_MS)
        .round() as u64
}

fn news_ticker_frame(input: &str) -> String {
    let clean = compact_news_line(input);
    let parts = clean
        .split('◆')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(compact_news_line)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "LIVE ◆ no data".to_string();
    }
    let source = parts.first().cloned().unwrap_or_else(|| "LIVE".to_string());
    let headlines = if parts.len() > 1 {
        parts[1..].to_vec()
    } else {
        vec![source.clone()]
    };
    let visible = headlines
        .into_iter()
        .take(4)
        .map(|item| compact_news_line(&item))
        .collect::<Vec<_>>()
        .join("   |   ");
    format!("▮ {source} ▮ {visible} ▮")
}

#[derive(Debug, Clone)]
struct OverlayVideoWindow {
    label: String,
    url: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    audio: bool,
}

#[derive(Debug, Clone)]
struct OverlayTickerWindow {
    id: String,
    label: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    font_size: u32,
    refresh_seconds: u64,
    text: String,
    command: String,
}

#[derive(Debug, Clone)]
struct OverlayRuntimePlan {
    fingerprint: String,
    videos: Vec<OverlayVideoWindow>,
    tickers: Vec<OverlayTickerWindow>,
}

fn sync_overlay_runtime(config_path: &Path, cfg: &AppConfig, image_cycle: u64) -> Result<String> {
    let plan = build_overlay_runtime_plan(cfg, image_cycle)?;
    if plan.videos.is_empty() && plan.tickers.is_empty() {
        stop_overlay_runtime(config_path)?;
        return Ok("disabled".to_string());
    }
    let has_video_windows = !plan.videos.is_empty();
    let has_video_player = overlay_video_player().is_some();

    for ticker in &plan.tickers {
        write_overlay_ticker_state(config_path, ticker)?;
    }

    if overlay_helpers_disabled() {
        stop_overlay_runtime(config_path)?;
        return Ok("suppressed-by-env".to_string());
    }

    let state_path = overlay_runtime_state_path(config_path)?;
    if let Some((fingerprint, pids)) = load_overlay_runtime_state(&state_path)?
        && fingerprint == plan.fingerprint
        && !pids.is_empty()
        && pids.iter().all(|pid| process_alive(*pid))
    {
        return Ok(format!("active ({})", pids.len()));
    }

    stop_overlay_runtime(config_path)?;

    let mut pids = Vec::<u32>::new();
    for ticker in &plan.tickers {
        if let Some(pid) =
            spawn_overlay_ticker_process(&overlay_ticker_state_path(config_path, &ticker.id)?)
        {
            pids.push(pid);
        }
    }
    for video in &plan.videos {
        if let Some(pid) = spawn_overlay_video_process(video) {
            pids.push(pid);
        }
    }

    store_overlay_runtime_state(&state_path, &plan.fingerprint, &pids)?;
    Ok(if pids.is_empty() {
        if has_video_windows && !has_video_player {
            "no video helper available (install mpv)".to_string()
        } else {
            "requested (no helpers available)".to_string()
        }
    } else if has_video_windows && !has_video_player {
        format!(
            "started ({}) + tickers only (install mpv for live video)",
            pids.len()
        )
    } else if news_overlay_enabled(cfg) && !news_overlay_video_enabled(cfg) {
        format!("started ({}) + news ticker-only source", pids.len())
    } else {
        format!("started ({})", pids.len())
    })
}

fn build_overlay_runtime_plan(cfg: &AppConfig, _image_cycle: u64) -> Result<OverlayRuntimePlan> {
    let overlay_windows_supported = supports_positioned_overlay_windows();
    let mut videos = Vec::<OverlayVideoWindow>::new();
    let mut tickers = Vec::<OverlayTickerWindow>::new();

    if news_overlay_enabled(cfg) {
        let (label, stream_url, _feed_url) = news_source_profile(cfg);
        let has_live_video =
            news_source_supports_live_video_source(&cfg.news_source, &cfg.news_custom_url);
        if has_live_video {
            videos.push(OverlayVideoWindow {
                label: label.to_string(),
                url: stream_url.clone(),
                x: cfg.news_pos_x,
                y: cfg.news_pos_y,
                width: cfg.news_widget_width,
                height: cfg.news_widget_height,
                audio: cfg.news_audio_enabled,
            });
        }
    }

    if news_ticker2_enabled(cfg) && overlay_windows_supported {
        tickers.push(OverlayTickerWindow {
            id: "news_ticker2".to_string(),
            label: "news-ticker2".to_string(),
            x: cfg.news_ticker2_pos_x,
            y: cfg.news_ticker2_pos_y,
            width: cfg.news_ticker2_width,
            height: 56,
            font_size: 28,
            refresh_seconds: cfg.news_ticker2_refresh_seconds.max(10),
            text: resolve_secondary_news_ticker(cfg),
            command: String::new(),
        });
    }

    if cams_overlay_enabled(cfg) {
        let source_cycle = now_epoch_seconds() / cfg.cams_refresh_seconds.max(10);
        let mut sources = cams_source_entries(cfg, source_cycle);
        let count = cfg.cams_count.clamp(1, 9) as usize;
        if sources.len() > count {
            sources.truncate(count);
        }
        let cols = cfg.cams_columns.clamp(1, 4).min(count.max(1) as u32);
        let rows = ((sources.len().max(1) as u32).div_ceil(cols)).max(1);
        let gap = 8_i32;
        let total_gap_w = gap * cols.saturating_sub(1) as i32;
        let total_gap_h = gap * rows.saturating_sub(1) as i32;
        let cell_w = ((cfg.cams_widget_width as i32 - total_gap_w) / cols as i32).max(160) as u32;
        let cell_h = ((cfg.cams_widget_height as i32 - total_gap_h) / rows as i32).max(96) as u32;
        for (idx, source) in sources.iter().enumerate() {
            let col = idx as i32 % cols as i32;
            let row = idx as i32 / cols as i32;
            videos.push(OverlayVideoWindow {
                label: cam_source_display_label(source),
                url: source.url.clone(),
                x: cfg.cams_pos_x + col * (cell_w as i32 + gap),
                y: cfg.cams_pos_y + row * (cell_h as i32 + gap),
                width: cell_w,
                height: cell_h,
                audio: false,
            });
        }
        let labels = sources
            .iter()
            .map(cam_source_display_label)
            .collect::<Vec<_>>()
            .join("  ◆  ");
        tickers.push(OverlayTickerWindow {
            id: "cams".to_string(),
            label: "cams".to_string(),
            x: cfg.cams_pos_x,
            y: cfg
                .cams_pos_y
                .saturating_add(cfg.cams_widget_height as i32)
                .saturating_add(8),
            width: cfg.cams_widget_width,
            height: 56,
            font_size: 28,
            refresh_seconds: cfg.cams_refresh_seconds.max(10),
            text: compact_news_line(&format!("CAMS ◆ {labels}")),
            command: String::new(),
        });
    }

    if cfg.overlay_script_ticker_enabled {
        tickers.push(OverlayTickerWindow {
            id: "script".to_string(),
            label: "script".to_string(),
            x: cfg.overlay_script_ticker_pos_x,
            y: cfg.overlay_script_ticker_pos_y,
            width: cfg.overlay_script_ticker_width,
            height: cfg.overlay_script_ticker_height,
            font_size: cfg.overlay_script_ticker_font_size,
            refresh_seconds: cfg.overlay_script_ticker_refresh_seconds.max(1),
            text: String::new(),
            command: cfg.overlay_script_ticker_command.clone(),
        });
    }

    let fingerprint = stable_hash(&format!(
        "news_overlay={}#{}#{}#{}#{}#{}|cams_overlay={}#{}#{}#{}#{}#{}#{}|script={}#{}#{}#{}#{}#{}",
        news_overlay_enabled(cfg),
        cfg.news_render_mode,
        cfg.news_source,
        cfg.news_custom_url,
        cfg.news_pos_x,
        cfg.news_pos_y,
        cams_overlay_enabled(cfg),
        cfg.cams_render_mode,
        cfg.cams_source,
        cfg.cams_custom_urls,
        cfg.cams_pos_x,
        cfg.cams_pos_y,
        cfg.cams_count,
        cfg.overlay_script_ticker_enabled,
        cfg.overlay_script_ticker_command,
        cfg.overlay_script_ticker_pos_x,
        cfg.overlay_script_ticker_pos_y,
        cfg.overlay_script_ticker_width,
        cfg.overlay_script_ticker_height,
    ));

    Ok(OverlayRuntimePlan {
        fingerprint,
        videos,
        tickers,
    })
}

fn overlay_runtime_dir(config_path: &Path) -> Result<PathBuf> {
    let base = expand_tilde("~/.local/state/wallpaper-composer/overlay")?;
    let dir = base.join(stable_hash(&config_path.display().to_string()));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn overlay_runtime_state_path(config_path: &Path) -> Result<PathBuf> {
    Ok(overlay_runtime_dir(config_path)?.join("runtime.json"))
}

fn overlay_ticker_state_path(config_path: &Path, id: &str) -> Result<PathBuf> {
    Ok(overlay_runtime_dir(config_path)?.join(format!("ticker-{id}.json")))
}

fn write_overlay_ticker_state(config_path: &Path, ticker: &OverlayTickerWindow) -> Result<()> {
    let path = overlay_ticker_state_path(config_path, &ticker.id)?;
    let payload = serde_json::json!({
        "label": ticker.label,
        "x": ticker.x,
        "y": ticker.y,
        "width": ticker.width,
        "height": ticker.height,
        "font_size": ticker.font_size,
        "refresh_seconds": ticker.refresh_seconds,
        "text": ticker.text,
        "command": ticker.command,
    });
    fs::write(path, payload.to_string())?;
    Ok(())
}

fn load_overlay_runtime_state(path: &Path) -> Result<Option<(String, Vec<u32>)>> {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let value = serde_json::from_str::<Value>(&raw)?;
    let fingerprint = value
        .get("fingerprint")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let pids = value
        .get("pids")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_u64)
                .map(|pid| pid as u32)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if fingerprint.is_empty() {
        Ok(None)
    } else {
        Ok(Some((fingerprint, pids)))
    }
}

fn store_overlay_runtime_state(path: &Path, fingerprint: &str, pids: &[u32]) -> Result<()> {
    let payload = serde_json::json!({
        "fingerprint": fingerprint,
        "pids": pids,
        "updated_unix": now_epoch_seconds(),
    });
    fs::write(path, payload.to_string())?;
    Ok(())
}

fn stop_overlay_runtime(config_path: &Path) -> Result<()> {
    let state_path = overlay_runtime_state_path(config_path)?;
    if let Some((_, pids)) = load_overlay_runtime_state(&state_path)? {
        for pid in pids {
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
    let _ = fs::remove_file(&state_path);
    Ok(())
}

fn process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn supports_positioned_overlay_windows() -> bool {
    if !LIVE_MEDIA_EXPERIMENTAL_ENABLED {
        return false;
    }
    let session = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if session == "wayland" {
        return false;
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return false;
    }
    true
}

fn overlay_helpers_disabled() -> bool {
    std::env::var(OVERLAY_HELPERS_DISABLED_ENV)
        .map(|raw| {
            let trimmed = raw.trim();
            trimmed == "1"
                || trimmed.eq_ignore_ascii_case("true")
                || trimmed.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false)
}

fn find_overlay_gui_binary() -> Option<PathBuf> {
    if let Ok(current) = std::env::current_exe() {
        for candidate in [
            current.with_file_name("le-compositeur"),
            current.with_file_name("wc-gui"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    for cmd in ["le-compositeur", "wc-gui"] {
        if command_exists(cmd) {
            return Some(PathBuf::from(cmd));
        }
    }
    None
}

fn overlay_video_player() -> Option<&'static str> {
    if command_exists("mpv") {
        Some("mpv")
    } else if command_exists("ffplay") {
        Some("ffplay")
    } else {
        None
    }
}

fn spawn_overlay_ticker_process(state_path: &Path) -> Option<u32> {
    let bin = find_overlay_gui_binary()?;
    let child = Command::new(bin)
        .arg("--overlay-state")
        .arg(state_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    Some(child.id())
}

fn spawn_overlay_video_process(window: &OverlayVideoWindow) -> Option<u32> {
    let player = overlay_video_player()?;

    let resolved = if is_youtube_url(&window.url) {
        resolve_youtube_playback_url(&window.url).unwrap_or_else(|| window.url.clone())
    } else {
        window.url.clone()
    };

    let title = format!("Le Compositeur Overlay - {}", window.label);
    let child = if player == "mpv" {
        let mut cmd = Command::new("mpv");
        cmd.args([
            "--no-terminal",
            "--really-quiet",
            "--force-window=yes",
            "--no-border",
            "--profile=low-latency",
            "--cache=no",
            "--geometry",
            &format!(
                "{}x{}+{}+{}",
                window.width, window.height, window.x, window.y
            ),
            "--title",
            &title,
        ]);
        if !window.audio {
            cmd.arg("--mute=yes");
        }
        if is_static_image_url(&resolved) {
            cmd.arg("--image-display-duration=inf");
        }
        cmd.arg(&resolved)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?
    } else {
        let mut cmd = Command::new("ffplay");
        cmd.env("SDL_VIDEO_WINDOW_POS", format!("{},{}", window.x, window.y));
        cmd.args([
            "-loglevel",
            "error",
            "-noborder",
            "-window_title",
            &title,
            "-x",
            &window.width.to_string(),
            "-y",
            &window.height.to_string(),
        ]);
        if !window.audio {
            cmd.arg("-an");
        }
        cmd.arg(&resolved)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?
    };
    Some(child.id())
}

fn is_static_image_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
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
    if is_youtube_url(raw)
        && let Some(video_id) = extract_youtube_video_id(raw)
    {
        let ts = now_epoch_seconds() / 2;
        for still in [
            format!("https://img.youtube.com/vi/{video_id}/maxresdefault_live.jpg?t={ts}"),
            format!("https://img.youtube.com/vi/{video_id}/hqdefault_live.jpg?t={ts}"),
            format!("https://img.youtube.com/vi/{video_id}/maxresdefault.jpg?t={ts}"),
            format!("https://img.youtube.com/vi/{video_id}/hqdefault.jpg?t={ts}"),
        ] {
            if let Ok(path) = fetch_remote_image(still, ImageProvider::Generic) {
                return Some(path);
            }
        }
    }

    if !is_youtube_url(stream_url) && stream_like_endpoint(stream_url) {
        return capture_news_frame(raw, raw, fps);
    }
    None
}

fn weather_code_label(code: i64) -> &'static str {
    match code {
        0 => "Clear",
        1..=3 => "Clouds",
        45 | 48 => "Fog",
        51..=57 => "Drizzle",
        61..=67 => "Rain",
        71..=77 => "Snow",
        80..=86 => "Showers",
        95..=99 => "Storm",
        _ => "Weather",
    }
}

fn compact_condition_label(desc: &str) -> &'static str {
    let l = desc.to_ascii_lowercase();
    if l.contains("thunder") {
        return "Storm";
    }
    if l.contains("snow") {
        return "Snow";
    }
    if l.contains("rain") || l.contains("drizzle") || l.contains("shower") {
        return "Rain";
    }
    if l.contains("fog") || l.contains("mist") {
        return "Fog";
    }
    if l.contains("cloud") {
        return "Clouds";
    }
    if l.contains("clear") || l.contains("sun") {
        return "Clear";
    }
    "Weather"
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

fn stream_like_endpoint(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    is_youtube_url(url)
        || is_camera_like_url(url)
        || lower.ends_with(".mp4")
        || lower.ends_with(".webm")
        || lower.ends_with(".mkv")
        || lower.ends_with(".mpd")
        || lower.contains("stream")
}

fn is_youtube_url(url: &str) -> bool {
    let l = url.to_ascii_lowercase();
    l.contains("youtube.com") || l.contains("youtu.be")
}

fn news_widget_enabled(cfg: &AppConfig) -> bool {
    cfg.show_news_layer
        && cfg.news_render_mode.trim().eq_ignore_ascii_case("overlay")
        && LIVE_MEDIA_EXPERIMENTAL_ENABLED
}

fn news_ticker2_enabled(cfg: &AppConfig) -> bool {
    cfg.show_news_ticker2 && LIVE_MEDIA_EXPERIMENTAL_ENABLED
}

fn news_overlay_enabled(cfg: &AppConfig) -> bool {
    cfg.show_news_layer
        && cfg.news_render_mode.trim().eq_ignore_ascii_case("overlay")
        && news_source_supports_live_video_source(&cfg.news_source, &cfg.news_custom_url)
        && LIVE_MEDIA_EXPERIMENTAL_ENABLED
}

fn cams_widget_enabled(cfg: &AppConfig) -> bool {
    cfg.show_cams_layer
        && cfg.cams_render_mode.trim().eq_ignore_ascii_case("overlay")
        && LIVE_MEDIA_EXPERIMENTAL_ENABLED
}

fn cams_overlay_enabled(cfg: &AppConfig) -> bool {
    cfg.show_cams_layer
        && cfg.cams_render_mode.trim().eq_ignore_ascii_case("overlay")
        && LIVE_MEDIA_EXPERIMENTAL_ENABLED
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
    let now_ms = now_epoch_millis();
    let fps = fps.clamp(0.05, 30.0);
    let min_interval = if enforce_camera_floor {
        1.0_f32.max(1.0 / fps)
    } else {
        (1.0 / fps).max(0.05)
    };
    let min_ms = (min_interval * 1000.0).ceil() as u64;
    let last_ms = fs::read_to_string(&stamp)
        .ok()
        .and_then(|v| parse_cache_stamp_millis(v.trim()))
        .unwrap_or(0);
    if now_ms.saturating_sub(last_ms) < min_ms && target.exists() {
        return Some(target);
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "error", "-nostdin"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    if url.to_ascii_lowercase().starts_with("rtsp://") {
        cmd.args(["-rtsp_transport", "tcp"]);
    }
    let mut child = cmd
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
        .spawn()
        .ok()?;
    let started = Instant::now();
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break Some(status);
        }
        if started.elapsed() >= Duration::from_secs(STREAM_CAPTURE_TIMEOUT_SECS) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(100));
    };
    if status.is_none() {
        let _ = fs::remove_file(&target);
        return None;
    }
    if !status.is_some_and(|s| s.success()) || !target.exists() {
        let _ = fs::remove_file(&target);
        return None;
    }
    let _ = fs::write(stamp, format!("{now_ms}\n"));
    Some(target)
}

fn parse_cache_stamp_millis(raw: &str) -> Option<u64> {
    let parsed = raw.trim().parse::<u64>().ok()?;
    // Backward compatibility for old second-based stamps.
    if parsed < 10_000_000_000 {
        Some(parsed.saturating_mul(1000))
    } else {
        Some(parsed)
    }
}

fn resolve_youtube_playback_url(url: &str) -> Option<String> {
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

    if command_exists("yt-dlp") {
        let output = Command::new("yt-dlp")
            .args(["--no-warnings", "--no-playlist", "-g", url])
            .output()
            .ok()?;
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout).ok()?;
            let mut candidates = stdout
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with("http"))
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            if !candidates.is_empty() {
                candidates.sort_by_key(|l| if l.contains(".m3u8") { 0 } else { 1 });
                let picked = candidates[0].clone();
                let _ = fs::write(&cache_file, format!("{picked}\n"));
                return Some(picked);
            }
        }
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(YOUTUBE_PAGE_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
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
    let picked = extract_youtube_manifest_url(&body)?;
    let _ = fs::write(&cache_file, format!("{picked}\n"));
    Some(picked)
}

fn extract_youtube_manifest_url(body: &str) -> Option<String> {
    for needle in ["hlsManifestUrl\":\"", "dashManifestUrl\":\""] {
        let Some(start) = body.find(needle) else {
            continue;
        };
        let rest = &body[start + needle.len()..];
        let end = rest.find("\",").or_else(|| rest.find('"'))?;
        let raw = rest[..end].trim();
        let val = raw
            .replace("\\u0026", "&")
            .replace("\\/", "/")
            .replace("&amp;", "&");
        if val.starts_with("http") {
            return Some(val);
        }
    }
    None
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
            .replace("&gt;", ">")
            .replace("&apos;", "'")
            .replace("&quot;", "\"");
        let value = value
            .replace("<![CDATA[", "")
            .replace("]]>", "")
            .replace("<![cdata[", "")
            .replace("]]&gt;", "")
            .replace("&apos;", "'");
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

fn cycle_pick_state_path(path: &Path) -> PathBuf {
    let mut raw = path.as_os_str().to_os_string();
    raw.push(".cycle");
    PathBuf::from(raw)
}

fn read_cycle_pick_state(path: &Path) -> Option<(u64, usize)> {
    let raw = fs::read_to_string(cycle_pick_state_path(path)).ok()?;
    let (cycle_raw, idx_raw) = raw.trim().split_once(',')?;
    let cycle = cycle_raw.trim().parse::<u64>().ok()?;
    let idx = idx_raw.trim().parse::<usize>().ok()?;
    Some((cycle, idx))
}

fn write_cycle_pick_state(path: &Path, cycle: u64, idx: usize) -> Result<()> {
    fs::write(cycle_pick_state_path(path), format!("{cycle},{idx}\n"))?;
    Ok(())
}

fn remote_image_cycle_state_path(cfg: &AppConfig, key: &str) -> Result<PathBuf> {
    let path = expand_tilde(&format!(
        "{}.remote-image-{}",
        cfg.rotation_state_file,
        stable_hash(key)
    ))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn read_cycle_image_state(path: &Path) -> Option<(u64, PathBuf)> {
    let raw = fs::read_to_string(path).ok()?;
    let (cycle_raw, image_raw) = raw.trim().split_once('|')?;
    let cycle = cycle_raw.trim().parse::<u64>().ok()?;
    let image_path = PathBuf::from(image_raw.trim());
    if !image_path.exists() {
        return None;
    }
    Some((cycle, image_path))
}

fn write_cycle_image_state(path: &Path, cycle: u64, image_path: &Path) -> Result<()> {
    fs::write(path, format!("{cycle}|{}\n", image_path.display()))?;
    Ok(())
}

fn resolve_remote_image_for_cycle<F>(
    cfg: &AppConfig,
    cycle: u64,
    key: &str,
    fetcher: F,
) -> Result<PathBuf, String>
where
    F: FnOnce() -> Result<PathBuf, String>,
{
    let state_path =
        remote_image_cycle_state_path(cfg, key).map_err(|e| format!("state path failed: {e}"))?;
    if let Some((last_cycle, cached)) = read_cycle_image_state(&state_path)
        && last_cycle == cycle
    {
        return Ok(cached);
    }
    let fetched = fetcher()?;
    write_cycle_image_state(&state_path, cycle, &fetched)
        .map_err(|e| format!("state write failed: {e}"))?;
    Ok(fetched)
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

fn parse_cam_source_list(raw: &str) -> Vec<CamSourceEntry> {
    let mut out = Vec::<CamSourceEntry>::new();

    for piece in raw.split(['\n', ';']) {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((left, right)) = trimmed.split_once("=>") {
            let url = right.trim();
            if url.is_empty() {
                continue;
            }
            let label = compact_news_line(left.trim());
            out.push(CamSourceEntry {
                label: if label.is_empty() {
                    summarize_source_label(url)
                } else {
                    label
                },
                url: url.to_string(),
            });
            continue;
        }

        if let Some((left, right)) = trimmed.split_once('|') {
            let url = right.trim();
            if looks_like_endpoint(url) {
                let label = compact_news_line(left.trim());
                out.push(CamSourceEntry {
                    label: if label.is_empty() {
                        summarize_source_label(url)
                    } else {
                        label
                    },
                    url: url.to_string(),
                });
                continue;
            }
        }

        out.push(CamSourceEntry {
            label: summarize_source_label(trimmed),
            url: trimmed.to_string(),
        });
    }

    if out.is_empty() {
        out.extend(
            parse_endpoint_list(raw)
                .into_iter()
                .map(|url| CamSourceEntry {
                    label: summarize_source_label(&url),
                    url,
                }),
        );
    }

    out
}

fn looks_like_endpoint(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    v.starts_with("https://")
        || v.starts_with("http://")
        || v.starts_with("file://")
        || v.starts_with("rtsp://")
        || v.starts_with("rtmp://")
        || v.starts_with("mms://")
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

fn load_config_with_quote_recovery(config_path: &Path) -> Result<AppConfig> {
    let mut cfg = load_config(config_path)?;
    let recovered = ensure_local_quotes_file(&mut cfg)?;
    if let Some(path) = recovered {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(config_path, to_config_toml(&cfg))?;
        println!("recovered_local_quotes: {}", path.display());
    }
    Ok(cfg)
}

fn ensure_default_local_quotes(config_path: &Path) -> Result<Option<PathBuf>> {
    let mut cfg = load_config(config_path)?;
    let recovered = ensure_local_quotes_file(&mut cfg)?;
    if recovered.is_some() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(config_path, to_config_toml(&cfg))?;
    }
    Ok(recovered)
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

fn effective_video_fps(requested_fps: f32) -> f32 {
    requested_fps.clamp(0.05, 30.0).max(MIN_SMOOTH_VIDEO_FPS)
}

fn loop_tick_duration(cfg: &AppConfig) -> Duration {
    // Base loop from user-selected background interval, but keep clock updates within one minute.
    // Overlay video/ticker animation is handled by helper processes; CLI only refreshes data/state.
    let mut seconds = master_rotation_interval(cfg).min(60) as f64;
    if news_overlay_enabled(cfg) {
        seconds = seconds.min(cfg.news_refresh_seconds.max(10) as f64);
    }
    if news_ticker2_enabled(cfg) {
        seconds = seconds.min(cfg.news_ticker2_refresh_seconds.max(10) as f64);
        seconds = seconds.min((1.0 / cfg.news_ticker2_fps.max(0.05)) as f64);
    }
    if cams_overlay_enabled(cfg) {
        seconds = seconds.min(cfg.cams_refresh_seconds.max(10) as f64);
    }
    if cfg.overlay_script_ticker_enabled {
        seconds = seconds.min(cfg.overlay_script_ticker_refresh_seconds.max(1) as f64);
    }
    let millis = (seconds * 1000.0).round().clamp(250.0, 60_000.0) as u64;
    Duration::from_millis(millis)
}

fn master_rotation_interval(cfg: &AppConfig) -> u64 {
    cfg.image_refresh_seconds.max(1)
}

fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn widget_payload_cache_path(name: &str) -> Option<PathBuf> {
    let p = expand_tilde(&format!("~/.cache/wallpaper-composer/{name}.json")).ok()?;
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    Some(p)
}

fn load_cached_widget_json(path: &Path, max_age_secs: u64) -> Result<Option<Value>> {
    let modified = match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .ok()
        .map(|d| d.as_secs())
        .unwrap_or(max_age_secs.saturating_add(1));
    if age > max_age_secs {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let parsed = serde_json::from_str::<Value>(&raw)?;
    Ok(Some(parsed))
}

fn load_cached_news_payload(
    cache_id: &str,
    max_age_secs: u64,
) -> Result<Option<NewsCachedPayload>> {
    let Some(path) = widget_payload_cache_path(&format!("news-{cache_id}")) else {
        return Ok(None);
    };
    let Some(payload) = load_cached_widget_json(&path, max_age_secs)? else {
        return Ok(None);
    };
    let raw_line = payload
        .get("raw_line")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if raw_line.is_empty() {
        return Ok(None);
    }
    let preview_image = payload
        .get("preview_image")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .filter(|p| p.exists());
    Ok(Some(NewsCachedPayload {
        raw_line,
        preview_image,
    }))
}

fn store_cached_news_payload(cache_id: &str, payload: &NewsCachedPayload) -> Result<()> {
    let Some(path) = widget_payload_cache_path(&format!("news-{cache_id}")) else {
        return Ok(());
    };
    let json = serde_json::json!({
        "raw_line": payload.raw_line,
        "preview_image": payload.preview_image.as_ref().map(|p| p.display().to_string()),
        "updated_unix": now_epoch_seconds(),
    });
    fs::write(path, json.to_string())?;
    Ok(())
}

fn load_cached_cams_payload(
    cache_id: &str,
    max_age_secs: u64,
) -> Result<Option<CamsCachedPayload>> {
    let Some(path) = widget_payload_cache_path(&format!("cams-{cache_id}")) else {
        return Ok(None);
    };
    let Some(payload) = load_cached_widget_json(&path, max_age_secs)? else {
        return Ok(None);
    };
    let base_line = payload
        .get("base_line")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if base_line.is_empty() {
        return Ok(None);
    }
    let preview_image = payload
        .get("preview_image")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .filter(|p| p.exists());
    Ok(Some(CamsCachedPayload {
        base_line,
        preview_image,
    }))
}

fn store_cached_cams_payload(cache_id: &str, payload: &CamsCachedPayload) -> Result<()> {
    let Some(path) = widget_payload_cache_path(&format!("cams-{cache_id}")) else {
        return Ok(());
    };
    let json = serde_json::json!({
        "base_line": payload.base_line,
        "preview_image": payload.preview_image.as_ref().map(|p| p.display().to_string()),
        "updated_unix": now_epoch_seconds(),
    });
    fs::write(path, json.to_string())?;
    Ok(())
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

fn stylize_weather_minimap(
    input: &Path,
    _lat: f64,
    _lon: f64,
    wind_deg: f64,
    wind_speed: f64,
    _wind_unit: &str,
) -> Option<PathBuf> {
    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let hash = stable_hash(&format!(
        "{}-{}-{}",
        input.display(),
        (wind_deg / 5.0).round() as i32,
        (wind_speed * 2.0).round() as i32
    ));
    let output = cache_dir.join(format!("weather-map-{hash}.png"));
    if output.exists() {
        return Some(output);
    }

    let mut img = image::open(input).ok()?.to_rgba8();
    if img.width() != 640 || img.height() != 360 {
        img = imageops::resize(&img, 640, 360, imageops::FilterType::CatmullRom);
    }
    tint_weather_map(&mut img);
    draw_weather_frame(&mut img);
    draw_weather_pointer(
        &mut img,
        WEATHER_POINTER_CENTER_X,
        WEATHER_POINTER_CENTER_Y,
        wind_deg,
        wind_speed,
    );
    write_weather_image(&output, &img)
}

fn fallback_weather_minimap(wind_deg: f64, wind_speed: f64, wind_unit: &str) -> Option<PathBuf> {
    let cache_dir = expand_tilde("~/.cache/wallpaper-composer/images").ok()?;
    fs::create_dir_all(&cache_dir).ok()?;
    let hash = stable_hash(&format!(
        "weather-fallback-{}-{}-{wind_unit}",
        (wind_deg / 5.0).round() as i32,
        (wind_speed * 2.0).round() as i32
    ));
    let output = cache_dir.join(format!("weather-map-fallback-{hash}.png"));
    if output.exists() {
        return Some(output);
    }

    let mut img = RgbaImage::from_pixel(640, 360, Rgba([20, 26, 33, 255]));
    for x in (16..640).step_by(32) {
        draw_line(&mut img, x, 12, x, 348, Rgba([46, 58, 69, 140]), 1);
    }
    for y in (16..360).step_by(32) {
        draw_line(&mut img, 12, y, 628, y, Rgba([46, 58, 69, 140]), 1);
    }
    draw_weather_frame(&mut img);
    draw_weather_pointer(
        &mut img,
        WEATHER_POINTER_CENTER_X,
        WEATHER_POINTER_CENTER_Y,
        wind_deg,
        wind_speed,
    );
    write_weather_image(&output, &img)
}

fn write_weather_image(path: &Path, img: &RgbaImage) -> Option<PathBuf> {
    DynamicImage::ImageRgba8(img.clone()).save(path).ok()?;
    Some(path.to_path_buf())
}

fn tint_weather_map(img: &mut RgbaImage) {
    for pixel in img.pixels_mut() {
        let src = pixel.0;
        let lum = (src[0] as f32 * 0.299) + (src[1] as f32 * 0.587) + (src[2] as f32 * 0.114);
        let boosted = ((lum - 128.0) * 1.35 + 152.0).clamp(0.0, 255.0) as u8;
        *pixel = Rgba([
            boosted.saturating_sub(10),
            boosted.saturating_sub(4),
            boosted,
            255,
        ]);
    }
}

fn draw_weather_frame(img: &mut RgbaImage) {
    draw_rect_outline(img, 6, 6, 628, 348, Rgba([48, 61, 74, 220]), 2);
    draw_rect_outline(img, 14, 14, 612, 332, Rgba([28, 36, 45, 220]), 1);
    draw_circle_outline(
        img,
        WEATHER_POINTER_CENTER_X,
        WEATHER_POINTER_CENTER_Y,
        WEATHER_POINTER_RING_RADIUS,
        Rgba([93, 107, 122, 170]),
        2,
    );
    draw_filled_circle(
        img,
        WEATHER_POINTER_CENTER_X,
        WEATHER_POINTER_CENTER_Y,
        4,
        Rgba([255, 107, 107, 255]),
    );
}

fn draw_weather_pointer(img: &mut RgbaImage, cx: i32, cy: i32, wind_deg: f64, wind_speed: f64) {
    let radius = (WEATHER_POINTER_RING_RADIUS + 8) as f64;
    let tip_offset = 18.0_f64;
    let rad = wind_deg.to_radians();
    let dx = rad.sin();
    let dy = -rad.cos();
    let sx = cx as f64 + dx * radius;
    let sy = cy as f64 + dy * radius;
    let ex = cx as f64 + dx * tip_offset;
    let ey = cy as f64 + dy * tip_offset;
    let lx = ex + (dx * 14.0) - (dy * 10.0);
    let ly = ey + (dy * 14.0) + (dx * 10.0);
    let rx = ex + (dx * 14.0) + (dy * 10.0);
    let ry = ey + (dy * 14.0) - (dx * 10.0);
    let stroke = if wind_speed >= 35.0 {
        5
    } else if wind_speed >= 18.0 {
        4
    } else {
        3
    };
    let red = Rgba([255, 76, 76, 235]);
    draw_line(
        img,
        sx.round() as i32,
        sy.round() as i32,
        ex.round() as i32,
        ey.round() as i32,
        red,
        stroke,
    );
    draw_line(
        img,
        ex.round() as i32,
        ey.round() as i32,
        lx.round() as i32,
        ly.round() as i32,
        red,
        stroke,
    );
    draw_line(
        img,
        ex.round() as i32,
        ey.round() as i32,
        rx.round() as i32,
        ry.round() as i32,
        red,
        stroke,
    );
    draw_filled_circle(img, cx, cy, 5, Rgba([255, 96, 96, 255]));
}

fn draw_rect_outline(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Rgba<u8>,
    stroke: i32,
) {
    for offset in 0..stroke {
        draw_line(
            img,
            x + offset,
            y + offset,
            x + w - offset,
            y + offset,
            color,
            1,
        );
        draw_line(
            img,
            x + offset,
            y + h - offset,
            x + w - offset,
            y + h - offset,
            color,
            1,
        );
        draw_line(
            img,
            x + offset,
            y + offset,
            x + offset,
            y + h - offset,
            color,
            1,
        );
        draw_line(
            img,
            x + w - offset,
            y + offset,
            x + w - offset,
            y + h - offset,
            color,
            1,
        );
    }
}

fn draw_circle_outline(
    img: &mut RgbaImage,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Rgba<u8>,
    stroke: i32,
) {
    for angle in 0..360 {
        let rad = (angle as f64).to_radians();
        let x = cx + (rad.cos() * radius as f64).round() as i32;
        let y = cy + (rad.sin() * radius as f64).round() as i32;
        draw_filled_circle(img, x, y, stroke.max(1) - 1, color);
    }
}

fn draw_filled_circle(img: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let r2 = radius * radius;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= r2 {
                put_pixel_safe(img, cx + dx, cy + dy, color);
            }
        }
    }
}

fn draw_line(
    img: &mut RgbaImage,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Rgba<u8>,
    stroke: i32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        for ox in -(stroke / 2)..=(stroke / 2) {
            for oy in -(stroke / 2)..=(stroke / 2) {
                put_pixel_safe(img, x0 + ox, y0 + oy, color);
            }
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn put_pixel_safe(img: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x < 0 || y < 0 {
        return;
    }
    let (w, h) = img.dimensions();
    if x as u32 >= w || y as u32 >= h {
        return;
    }
    img.put_pixel(x as u32, y as u32, color);
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
    use super::{
        LIVE_MEDIA_EXPERIMENTAL_ENABLED, OVERLAY_HELPERS_DISABLED_ENV,
        build_builtin_widget_registry, build_overlay_runtime_plan, cycle_pick_state_path,
        determine_cycle, loop_tick_duration, overlay_helpers_disabled, read_cycle_pick_state,
        read_recent_indices, ticker_shift_millis_for_len, widget_instance_from_config,
        write_cycle_pick_state, write_recent_indices,
    };
    use std::fs;
    use std::time::Duration;
    use wc_core::{BUILTIN_WIDGET_TYPE_IDS, default_config_toml, load_config};

    #[test]
    fn loop_tick_follows_master_interval_when_live_media_is_overlay_only() {
        let cfg_path = std::env::temp_dir().join("wc-cli-loop-tick-test.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let mut cfg = load_config(&cfg_path).expect("default config should parse");
        cfg.image_refresh_seconds = 300;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(60));

        cfg.image_refresh_seconds = 15;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(15));

        cfg.show_news_layer = true;
        cfg.news_fps = 10.0;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(15));

        cfg.show_news_layer = false;
        cfg.show_news_ticker2 = true;
        cfg.news_ticker2_fps = 12.0;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(15));

        cfg.show_news_ticker2 = false;
        cfg.show_cams_layer = true;
        cfg.cams_fps = 9.0;
        assert_eq!(loop_tick_duration(&cfg), Duration::from_secs(15));
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

    #[test]
    fn cycle_pick_state_roundtrip() {
        let state_path = std::env::temp_dir().join("wc-cli-cycle-pick.state");
        let _ = fs::remove_file(&state_path);
        let _ = fs::remove_file(cycle_pick_state_path(&state_path));

        write_cycle_pick_state(&state_path, 42, 7).expect("cycle pick should write");
        assert_eq!(read_cycle_pick_state(&state_path), Some((42, 7)));

        let _ = fs::remove_file(cycle_pick_state_path(&state_path));
        let _ = fs::remove_file(state_path);
    }

    #[test]
    fn ticker_shift_scales_with_text_length() {
        let short_ms = ticker_shift_millis_for_len(24);
        let long_ms = ticker_shift_millis_for_len(96);
        assert!(short_ms > long_ms);
        assert!(short_ms >= 300);
        assert!(long_ms <= 180);
    }

    #[test]
    fn stage_b_registry_builds_all_builtin_plugins() {
        let cfg_path = std::env::temp_dir().join("wc-cli-stage-b-registry.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let cfg = load_config(&cfg_path).expect("default config should parse");

        let registry =
            build_builtin_widget_registry(&cfg, 3, 3).expect("registry should build cleanly");
        assert_eq!(registry.len(), BUILTIN_WIDGET_TYPE_IDS.len());
        for type_id in BUILTIN_WIDGET_TYPE_IDS {
            assert!(
                registry.get(type_id).is_some(),
                "registry should contain builtin widget type {type_id}"
            );
        }
    }

    #[test]
    fn stage_b_instance_mapping_reflects_widget_caps() {
        let cfg_path = std::env::temp_dir().join("wc-cli-stage-b-instance.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let mut cfg = load_config(&cfg_path).expect("default config should parse");
        cfg.news_refresh_seconds = 123;
        cfg.news_fps = 7.5;
        cfg.cams_refresh_seconds = 75;
        cfg.cams_fps = 1.25;

        let news = widget_instance_from_config(&cfg, "news").expect("news instance");
        assert_eq!(news.refresh_seconds, 123);
        assert!((news.fps_cap - 7.5).abs() < f32::EPSILON);

        cfg.show_news_layer = false;
        cfg.show_news_ticker2 = true;
        let ticker2 =
            widget_instance_from_config(&cfg, "news_ticker2").expect("news ticker2 instance");
        assert_eq!(ticker2.enabled, LIVE_MEDIA_EXPERIMENTAL_ENABLED);

        let cams = widget_instance_from_config(&cfg, "cams").expect("cams instance");
        assert_eq!(cams.refresh_seconds, 75);
        assert!((cams.fps_cap - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn overlay_runtime_plan_follows_live_media_feature_gate() {
        let cfg_path = std::env::temp_dir().join("wc-cli-overlay-plan.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let mut cfg = load_config(&cfg_path).expect("default config should parse");
        cfg.show_news_layer = true;
        cfg.news_render_mode = "overlay".to_string();
        cfg.show_cams_layer = true;
        cfg.cams_source = "custom".to_string();
        cfg.cams_render_mode = "overlay".to_string();
        cfg.cams_count = 2;
        cfg.cams_columns = 2;
        cfg.cams_custom_urls =
            "Berlin => https://example.com/berlin.m3u8\nParis => https://example.com/paris.m3u8"
                .to_string();
        cfg.overlay_script_ticker_enabled = true;
        cfg.overlay_script_ticker_command = "printf 'dynamic headline\\n'".to_string();

        let news = widget_instance_from_config(&cfg, "news").expect("news instance");
        assert_eq!(news.enabled, LIVE_MEDIA_EXPERIMENTAL_ENABLED);
        let cams = widget_instance_from_config(&cfg, "cams").expect("cams instance");
        assert_eq!(cams.enabled, LIVE_MEDIA_EXPERIMENTAL_ENABLED);

        let plan = build_overlay_runtime_plan(&cfg, 0).expect("overlay plan");
        if LIVE_MEDIA_EXPERIMENTAL_ENABLED {
            assert!(!plan.videos.is_empty());
            assert!(plan.tickers.iter().any(|ticker| ticker.id == "script"));
        } else {
            assert!(plan.videos.is_empty());
            assert_eq!(plan.tickers.len(), 1);
            assert!(
                plan.tickers
                    .iter()
                    .any(|ticker| ticker.id == "script" && !ticker.command.is_empty())
            );
        }

        let _ = fs::remove_file(cfg_path);
    }

    #[test]
    fn overlay_runtime_plan_respects_live_media_feature_gate() {
        let cfg_path = std::env::temp_dir().join("wc-cli-overlay-feed-only.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let mut cfg = load_config(&cfg_path).expect("default config should parse");
        cfg.show_news_layer = true;
        cfg.news_render_mode = "overlay".to_string();
        cfg.news_source = "google_world_en".to_string();

        let news = widget_instance_from_config(&cfg, "news").expect("news instance");
        assert_eq!(news.enabled, LIVE_MEDIA_EXPERIMENTAL_ENABLED);

        let plan = build_overlay_runtime_plan(&cfg, 0).expect("overlay plan");
        if LIVE_MEDIA_EXPERIMENTAL_ENABLED {
            assert!(!plan.videos.is_empty() || !plan.tickers.is_empty());
        } else {
            assert!(plan.videos.is_empty());
            assert!(plan.tickers.is_empty());
        }

        let _ = fs::remove_file(cfg_path);
    }

    #[test]
    fn overlay_helpers_disabled_env_is_parsed_leniently() {
        unsafe { std::env::remove_var(OVERLAY_HELPERS_DISABLED_ENV) };
        assert!(!overlay_helpers_disabled());

        unsafe { std::env::set_var(OVERLAY_HELPERS_DISABLED_ENV, "true") };
        assert!(overlay_helpers_disabled());

        unsafe { std::env::set_var(OVERLAY_HELPERS_DISABLED_ENV, "1") };
        assert!(overlay_helpers_disabled());

        unsafe { std::env::remove_var(OVERLAY_HELPERS_DISABLED_ENV) };
    }
}
