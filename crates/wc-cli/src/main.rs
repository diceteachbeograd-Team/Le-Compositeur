use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use wc_backend::apply_wallpaper;
use wc_core::{
    AppConfig, build_doctor_report, builtin_image_presets, builtin_quote_presets, cycle_index,
    default_config_path, default_config_toml, expand_tilde, image_preset_endpoint, load_config,
    pick_background_image, pick_quote, presets_catalog_json, quote_preset_endpoint,
    settings_schema_json, settings_ui_blueprint_json, to_config_toml,
};
use wc_render::{PreviewText, render_preview_to_file};
use wc_source::{ImageProvider, QuoteProvider, fetch_remote_image, fetch_remote_quote};

#[derive(Debug, Parser)]
#[command(name = "wc-cli")]
#[command(about = "Wallpaper Composer CLI", version)]
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
            let cycle = determine_cycle(&cfg)?;
            run_cycle(&cfg, cycle)?;
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
        }
        Commands::Validate { config } => {
            let config_path = resolve_config_path(config)?;
            let cfg = load_config(&config_path)?;
            validate_config(&cfg)?;
            println!("config_valid: {}", config_path.display());
        }
        Commands::Run { config, once } => {
            let config_path = resolve_config_path(config)?;
            loop {
                let cfg = load_config(&config_path)?;
                validate_config(&cfg)?;
                let cycle = determine_cycle(&cfg)?;
                run_cycle(&cfg, cycle)?;

                if once {
                    break;
                }
                thread::sleep(Duration::from_secs(cfg.refresh_seconds.max(1)));
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

fn run_cycle(cfg: &AppConfig, cycle: u64) -> Result<()> {
    let output_path = expand_tilde(&cfg.output_image)?;
    let source_image = resolve_source_image(cfg, cycle)?;
    let quote = resolve_quote(cfg, cycle)?;
    let clock = chrono::Local::now().format(&cfg.time_format).to_string();

    ensure_parent_dir(&output_path)?;
    let render = render_preview_to_file(
        &source_image,
        &output_path,
        PreviewText {
            quote: &quote,
            clock: &clock,
            quote_font_size: cfg.quote_font_size,
            quote_pos_x: cfg.quote_pos_x,
            quote_pos_y: cfg.quote_pos_y,
            clock_font_size: cfg.clock_font_size,
            clock_pos_x: cfg.clock_pos_x,
            clock_pos_y: cfg.clock_pos_y,
        },
    )
    .map_err(anyhow::Error::msg)?;

    println!("cycle: {}", cycle);
    println!("source_image: {}", source_image.display());
    println!("quote: {}", quote);
    println!("clock: {}", clock);
    println!("preview_mode: {}", render.preview_mode);
    println!("preview_output: {}", output_path.display());
    println!("preview_metadata: {}", render.meta_path.display());

    let apply_status = apply_wallpaper(&cfg.wallpaper_backend, cfg.apply_wallpaper, &output_path)
        .map_err(anyhow::Error::msg)?;
    println!("wallpaper_apply: {}", apply_status);

    Ok(())
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
            if cfg
                .image_source_url
                .as_deref()
                .is_none_or(|v| v.trim().is_empty())
            {
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
            if cfg
                .quote_source_url
                .as_deref()
                .is_none_or(|v| v.trim().is_empty())
            {
                anyhow::bail!("quote_source_url is required for quote_source=url");
            }
        }
        other => anyhow::bail!("unsupported quote_source={other}; use local, preset, or url"),
    }

    if cfg.refresh_seconds == 0 {
        anyhow::bail!("refresh_seconds must be greater than 0");
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

    Ok(())
}

fn determine_cycle(cfg: &AppConfig) -> Result<u64> {
    let base_cycle = cycle_index(cfg.refresh_seconds);
    if !cfg.rotation_use_persistent_state {
        return Ok(base_cycle);
    }

    let state_path = expand_tilde(&cfg.rotation_state_file)?;
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let next_cycle = read_last_cycle(&state_path)
        .map_or(base_cycle, |last| last.saturating_add(1))
        .max(base_cycle);

    write_last_cycle(&state_path, next_cycle)?;
    Ok(next_cycle)
}

fn read_last_cycle(path: &Path) -> Option<u64> {
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse::<u64>().ok()
}

fn write_last_cycle(path: &Path, cycle: u64) -> Result<()> {
    fs::write(path, format!("{}\n", cycle))?;
    Ok(())
}

fn resolve_source_image(cfg: &AppConfig, cycle: u64) -> Result<PathBuf> {
    match cfg.image_source.trim().to_ascii_lowercase().as_str() {
        "local" => {
            let image_dir = expand_tilde(&cfg.image_dir)?;
            pick_background_image(&image_dir, cycle)
        }
        "preset" | "remote_preset" => {
            let (endpoint, provider) = resolve_image_endpoint_from_preset(cfg)?;
            fetch_remote_image(endpoint, provider)
                .map_err(|e| anyhow::anyhow!("failed to fetch preset image source: {e}"))
        }
        "url" | "remote_url" => {
            let endpoint = resolve_image_endpoint_from_url(cfg)?;
            fetch_remote_image(endpoint, ImageProvider::Generic)
                .map_err(|e| anyhow::anyhow!("failed to fetch custom image source: {e}"))
        }
        other => Err(anyhow::anyhow!(
            "unsupported image_source={other}; supported: local, preset, url"
        )),
    }
}

fn resolve_quote(cfg: &AppConfig, cycle: u64) -> Result<String> {
    match cfg.quote_source.trim().to_ascii_lowercase().as_str() {
        "local" => {
            let quotes_path = expand_tilde(&cfg.quotes_path)?;
            pick_quote(&quotes_path, cycle)
        }
        "preset" | "remote_preset" => {
            let (endpoint, provider) = resolve_quote_endpoint_from_preset(cfg)?;
            fetch_remote_quote(endpoint, provider)
                .map_err(|e| anyhow::anyhow!("failed to fetch preset quote source: {e}"))
        }
        "url" | "remote_url" => {
            let endpoint = resolve_quote_endpoint_from_url(cfg)?;
            fetch_remote_quote(endpoint, QuoteProvider::Generic)
                .map_err(|e| anyhow::anyhow!("failed to fetch custom quote source: {e}"))
        }
        other => Err(anyhow::anyhow!(
            "unsupported quote_source={other}; supported: local, preset, url"
        )),
    }
}

fn resolve_image_endpoint_from_preset(cfg: &AppConfig) -> Result<(String, ImageProvider)> {
    let id = cfg.image_source_preset.as_deref().ok_or_else(|| {
        anyhow::anyhow!("image_source_preset is required for image_source=preset")
    })?;
    let endpoint = image_preset_endpoint(id)
        .ok_or_else(|| anyhow::anyhow!("unknown image_source_preset: {id}"))?;

    let provider = match id {
        "nasa_apod" => ImageProvider::NasaApod,
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

fn resolve_image_endpoint_from_url(cfg: &AppConfig) -> Result<String> {
    cfg.image_source_url
        .clone()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("image_source_url is required for image_source=url"))
}

fn resolve_quote_endpoint_from_url(cfg: &AppConfig) -> Result<String> {
    cfg.quote_source_url
        .clone()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("quote_source_url is required for quote_source=url"))
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

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn backup_path_for(config_path: &Path) -> PathBuf {
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let name = config_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.toml");
    config_path.with_file_name(format!("{name}.bak-{ts}"))
}
