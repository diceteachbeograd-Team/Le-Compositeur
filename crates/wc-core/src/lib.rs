use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub project: String,
    pub profile: String,
    pub local_time: String,
}

pub fn build_doctor_report() -> DoctorReport {
    let local_time = Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();

    DoctorReport {
        project: "wallpaper-composer".to_string(),
        profile: "dev".to_string(),
        local_time,
    }
}

pub const DEFAULT_CONFIG_RELATIVE_PATH: &str = ".config/wallpaper-composer/config.toml";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub config_version: u32,
    pub image_dir: String,
    pub quotes_path: String,
    pub image_source: String,
    pub image_source_url: Option<String>,
    pub image_source_preset: Option<String>,
    pub quote_source: String,
    pub quote_source_url: Option<String>,
    pub quote_source_preset: Option<String>,
    pub quote_format: String,
    pub image_order_mode: String,
    pub image_avoid_repeat: bool,
    pub quote_order_mode: String,
    pub quote_avoid_repeat: bool,
    pub quote_font_size: u32,
    pub quote_pos_x: i32,
    pub quote_pos_y: i32,
    pub quote_auto_fit: bool,
    pub quote_min_font_size: u32,
    pub font_family: String,
    pub quote_color: String,
    pub clock_font_size: u32,
    pub clock_pos_x: i32,
    pub clock_pos_y: i32,
    pub clock_color: String,
    pub text_stroke_color: String,
    pub text_stroke_width: u32,
    pub text_undercolor: String,
    pub text_shadow_enabled: bool,
    pub text_shadow_color: String,
    pub text_shadow_offset_x: i32,
    pub text_shadow_offset_y: i32,
    pub text_box_size: String,
    pub text_box_width_pct: u32,
    pub text_box_height_pct: u32,
    pub rotation_use_persistent_state: bool,
    pub rotation_state_file: String,
    pub output_image: String,
    pub refresh_seconds: u64,
    pub image_refresh_seconds: u64,
    pub quote_refresh_seconds: u64,
    pub time_format: String,
    pub apply_wallpaper: bool,
    pub wallpaper_backend: String,
    pub wallpaper_fit_mode: String,
}

pub fn default_config_toml() -> String {
    r##"# Wallpaper Composer config
config_version = 1
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "local"
image_source_url = ""
image_source_preset = "nasa_apod"
quote_source = "local"
quote_source_url = ""
quote_source_preset = "zenquotes_daily"
quote_format = "lines"
image_order_mode = "sequential"
image_avoid_repeat = true
quote_order_mode = "sequential"
quote_avoid_repeat = true
quote_font_size = 36
quote_pos_x = 80
quote_pos_y = 860
quote_auto_fit = true
quote_min_font_size = 18
font_family = "DejaVu-Sans"
quote_color = "#FFFFFF"
clock_font_size = 44
clock_pos_x = 1600
clock_pos_y = 960
clock_color = "#FFD700"
text_stroke_color = "#000000"
text_stroke_width = 2
text_undercolor = "#00000066"
text_shadow_enabled = true
text_shadow_color = "#00000099"
text_shadow_offset_x = 3
text_shadow_offset_y = 3
text_box_size = "quarter"
text_box_width_pct = 50
text_box_height_pct = 50
rotation_use_persistent_state = true
rotation_state_file = "~/.local/state/wallpaper-composer/rotation.state"
output_image = "/tmp/wallpaper-composer-current.png"
refresh_seconds = 300
image_refresh_seconds = 300
quote_refresh_seconds = 300
time_format = "%H:%M"
apply_wallpaper = false
wallpaper_backend = "auto"
wallpaper_fit_mode = "zoom"
"##
    .to_string()
}

pub fn to_config_toml(cfg: &AppConfig) -> String {
    format!(
        "# Wallpaper Composer config\nconfig_version = {}\nimage_dir = {:?}\nquotes_path = {:?}\nimage_source = {:?}\nimage_source_url = {:?}\nimage_source_preset = {:?}\nquote_source = {:?}\nquote_source_url = {:?}\nquote_source_preset = {:?}\nquote_format = {:?}\nimage_order_mode = {:?}\nimage_avoid_repeat = {}\nquote_order_mode = {:?}\nquote_avoid_repeat = {}\nquote_font_size = {}\nquote_pos_x = {}\nquote_pos_y = {}\nquote_auto_fit = {}\nquote_min_font_size = {}\nfont_family = {:?}\nquote_color = {:?}\nclock_font_size = {}\nclock_pos_x = {}\nclock_pos_y = {}\nclock_color = {:?}\ntext_stroke_color = {:?}\ntext_stroke_width = {}\ntext_undercolor = {:?}\ntext_shadow_enabled = {}\ntext_shadow_color = {:?}\ntext_shadow_offset_x = {}\ntext_shadow_offset_y = {}\ntext_box_size = {:?}\ntext_box_width_pct = {}\ntext_box_height_pct = {}\nrotation_use_persistent_state = {}\nrotation_state_file = {:?}\noutput_image = {:?}\nrefresh_seconds = {}\nimage_refresh_seconds = {}\nquote_refresh_seconds = {}\ntime_format = {:?}\napply_wallpaper = {}\nwallpaper_backend = {:?}\nwallpaper_fit_mode = {:?}\n",
        cfg.config_version,
        cfg.image_dir,
        cfg.quotes_path,
        cfg.image_source,
        cfg.image_source_url.clone().unwrap_or_default(),
        cfg.image_source_preset.clone().unwrap_or_default(),
        cfg.quote_source,
        cfg.quote_source_url.clone().unwrap_or_default(),
        cfg.quote_source_preset.clone().unwrap_or_default(),
        cfg.quote_format,
        cfg.image_order_mode,
        cfg.image_avoid_repeat,
        cfg.quote_order_mode,
        cfg.quote_avoid_repeat,
        cfg.quote_font_size,
        cfg.quote_pos_x,
        cfg.quote_pos_y,
        cfg.quote_auto_fit,
        cfg.quote_min_font_size,
        cfg.font_family,
        cfg.quote_color,
        cfg.clock_font_size,
        cfg.clock_pos_x,
        cfg.clock_pos_y,
        cfg.clock_color,
        cfg.text_stroke_color,
        cfg.text_stroke_width,
        cfg.text_undercolor,
        cfg.text_shadow_enabled,
        cfg.text_shadow_color,
        cfg.text_shadow_offset_x,
        cfg.text_shadow_offset_y,
        cfg.text_box_size,
        cfg.text_box_width_pct,
        cfg.text_box_height_pct,
        cfg.rotation_use_persistent_state,
        cfg.rotation_state_file,
        cfg.output_image,
        cfg.refresh_seconds,
        cfg.image_refresh_seconds,
        cfg.quote_refresh_seconds,
        cfg.time_format,
        cfg.apply_wallpaper,
        cfg.wallpaper_backend,
        cfg.wallpaper_fit_mode
    )
}

pub fn settings_schema_json() -> &'static str {
    r##"{
  "schema_version": 1,
  "config_key": "config_version",
  "ui_contract_version": 1,
  "groups": [
    {"id":"general","label":"General"},
    {"id":"sources","label":"Sources"},
    {"id":"layout","label":"Layout"},
    {"id":"rotation","label":"Rotation"},
    {"id":"wallpaper","label":"Wallpaper"}
  ],
  "fields": [
    {"key":"config_version","group":"general","label":"Config Version","type":"u32","required":false,"default":1},
    {"key":"output_image","group":"general","label":"Output Image","type":"string","required":true,"description":"Rendered wallpaper output path.","ui_widget":"file-save"},
    {"key":"refresh_seconds","group":"general","label":"Runner Tick Seconds","type":"u64","required":true,"description":"Main loop tick interval in seconds."},
    {"key":"image_refresh_seconds","group":"general","label":"Image Change Seconds","type":"u64","required":false,"default":300,"description":"How often the background image changes."},
    {"key":"quote_refresh_seconds","group":"general","label":"Quote Change Seconds","type":"u64","required":false,"default":300,"description":"How often the quote changes."},
    {"key":"time_format","group":"general","label":"Time Format","type":"string","required":true,"description":"Clock format using chrono syntax."},

    {"key":"image_dir","group":"sources","label":"Image Directory","type":"string","required":true,"ui_widget":"directory-picker"},
    {"key":"quotes_path","group":"sources","label":"Quotes File","type":"string","required":true,"ui_widget":"file-picker"},
    {"key":"image_source","group":"sources","label":"Image Source Mode","type":"enum","required":false,"default":"local","options":["local","preset","url"]},
    {"key":"image_source_url","group":"sources","label":"Image Source URL","type":"string","required":false,"default":"","visible_when":{"field":"image_source","equals":"url"},"enabled_when":{"field":"image_source","equals":"url"}},
    {"key":"image_source_preset","group":"sources","label":"Image Source Preset","type":"string","required":false,"default":"nasa_apod","visible_when":{"field":"image_source","equals":"preset"},"enabled_when":{"field":"image_source","equals":"preset"}},
    {"key":"quote_source","group":"sources","label":"Quote Source Mode","type":"enum","required":false,"default":"local","options":["local","preset","url"]},
    {"key":"quote_source_url","group":"sources","label":"Quote Source URL","type":"string","required":false,"default":"","visible_when":{"field":"quote_source","equals":"url"},"enabled_when":{"field":"quote_source","equals":"url"}},
    {"key":"quote_source_preset","group":"sources","label":"Quote Source Preset","type":"string","required":false,"default":"zenquotes_daily","visible_when":{"field":"quote_source","equals":"preset"},"enabled_when":{"field":"quote_source","equals":"preset"}},
    {"key":"quote_format","group":"sources","label":"Quote Format","type":"enum","required":false,"default":"lines","options":["lines","paragraphs","markdown_blocks"]},
    {"key":"image_order_mode","group":"sources","label":"Image Order","type":"enum","required":false,"default":"sequential","options":["sequential","random"]},
    {"key":"image_avoid_repeat","group":"sources","label":"Avoid Immediate Repeat","type":"bool","required":false,"default":true},
    {"key":"quote_order_mode","group":"sources","label":"Quote Order","type":"enum","required":false,"default":"sequential","options":["sequential","random"]},
    {"key":"quote_avoid_repeat","group":"sources","label":"Avoid Immediate Quote Repeat","type":"bool","required":false,"default":true},

    {"key":"quote_font_size","group":"layout","label":"Quote Font Size","type":"u32","required":false,"default":36,"min":8},
    {"key":"quote_pos_x","group":"layout","label":"Quote X","type":"i32","required":false,"default":80},
    {"key":"quote_pos_y","group":"layout","label":"Quote Y","type":"i32","required":false,"default":860},
    {"key":"quote_auto_fit","group":"layout","label":"Auto Fit Quote","type":"bool","required":false,"default":true},
    {"key":"quote_min_font_size","group":"layout","label":"Quote Min Font Size","type":"u32","required":false,"default":18,"min":8},
    {"key":"font_family","group":"layout","label":"Font Family","type":"enum","required":false,"default":"DejaVu-Sans","options":["DejaVu-Sans","Noto-Sans","Liberation-Sans","Serif","Monospace"]},
    {"key":"quote_color","group":"layout","label":"Quote Color","type":"string","required":false,"default":"#FFFFFF"},
    {"key":"clock_font_size","group":"layout","label":"Clock Font Size","type":"u32","required":false,"default":44,"min":8},
    {"key":"clock_pos_x","group":"layout","label":"Clock X","type":"i32","required":false,"default":1600},
    {"key":"clock_pos_y","group":"layout","label":"Clock Y","type":"i32","required":false,"default":960},
    {"key":"clock_color","group":"layout","label":"Clock Color","type":"string","required":false,"default":"#FFD700"},
    {"key":"text_stroke_color","group":"layout","label":"Text Stroke Color","type":"string","required":false,"default":"#000000"},
    {"key":"text_stroke_width","group":"layout","label":"Text Stroke Width","type":"u32","required":false,"default":2},
    {"key":"text_undercolor","group":"layout","label":"Text Undercolor","type":"string","required":false,"default":"#00000066"},
    {"key":"text_shadow_enabled","group":"layout","label":"Text Shadow","type":"bool","required":false,"default":true},
    {"key":"text_shadow_color","group":"layout","label":"Shadow Color","type":"string","required":false,"default":"#00000099","visible_when":{"field":"text_shadow_enabled","equals":true},"enabled_when":{"field":"text_shadow_enabled","equals":true}},
    {"key":"text_shadow_offset_x","group":"layout","label":"Shadow Offset X","type":"i32","required":false,"default":3,"visible_when":{"field":"text_shadow_enabled","equals":true},"enabled_when":{"field":"text_shadow_enabled","equals":true}},
    {"key":"text_shadow_offset_y","group":"layout","label":"Shadow Offset Y","type":"i32","required":false,"default":3,"visible_when":{"field":"text_shadow_enabled","equals":true},"enabled_when":{"field":"text_shadow_enabled","equals":true}},
    {"key":"text_box_size","group":"layout","label":"Text Box Size","type":"enum","required":false,"default":"quarter","options":["quarter","third","half","full","custom"]},
    {"key":"text_box_width_pct","group":"layout","label":"Text Box Width %","type":"u32","required":false,"default":50,"min":10,"max":100,"visible_when":{"field":"text_box_size","equals":"custom"},"enabled_when":{"field":"text_box_size","equals":"custom"}},
    {"key":"text_box_height_pct","group":"layout","label":"Text Box Height %","type":"u32","required":false,"default":50,"min":10,"max":100,"visible_when":{"field":"text_box_size","equals":"custom"},"enabled_when":{"field":"text_box_size","equals":"custom"}},

    {"key":"rotation_use_persistent_state","group":"rotation","label":"Use Persistent Rotation State","type":"bool","required":false,"default":true},
    {"key":"rotation_state_file","group":"rotation","label":"Rotation State File","type":"string","required":false,"default":"~/.local/state/wallpaper-composer/rotation.state","ui_widget":"file-save","visible_when":{"field":"rotation_use_persistent_state","equals":true},"enabled_when":{"field":"rotation_use_persistent_state","equals":true}},

    {"key":"apply_wallpaper","group":"wallpaper","label":"Apply Wallpaper","type":"bool","required":false,"default":false},
    {"key":"wallpaper_backend","group":"wallpaper","label":"Wallpaper Backend","type":"enum","required":false,"default":"auto","options":["auto","noop","gnome","sway","feh"],"visible_when":{"field":"apply_wallpaper","equals":true},"enabled_when":{"field":"apply_wallpaper","equals":true}},
    {"key":"wallpaper_fit_mode","group":"wallpaper","label":"Wallpaper Fit Mode","type":"enum","required":false,"default":"zoom","options":["zoom","scaled","stretched","spanned","centered","wallpaper"],"visible_when":{"field":"apply_wallpaper","equals":true},"enabled_when":{"field":"apply_wallpaper","equals":true}}
  ]
}"##
}

pub fn settings_ui_blueprint_json() -> &'static str {
    r#"{
  "blueprint_version": 1,
  "form": {
    "id": "wallpaper-composer-settings",
    "title": "Wallpaper Composer Settings",
    "sections": [
      {
        "id": "general",
        "title": "General",
        "fields": ["output_image", "refresh_seconds", "image_refresh_seconds", "quote_refresh_seconds", "time_format"]
      },
      {
        "id": "sources",
        "title": "Sources",
        "fields": [
          "image_source",
          "image_dir",
          "image_source_preset",
          "image_source_url",
          "quote_source",
          "quotes_path",
          "quote_source_preset",
          "quote_source_url",
          "quote_format",
          "image_order_mode",
          "image_avoid_repeat",
          "quote_order_mode",
          "quote_avoid_repeat"
        ]
      },
      {
        "id": "layout",
        "title": "Layout",
        "fields": [
          "quote_font_size",
          "quote_pos_x",
          "quote_pos_y",
          "quote_auto_fit",
          "quote_min_font_size",
          "font_family",
          "quote_color",
          "clock_font_size",
          "clock_pos_x",
          "clock_pos_y",
          "clock_color",
          "text_stroke_color",
          "text_stroke_width",
          "text_undercolor",
          "text_shadow_enabled",
          "text_shadow_color",
          "text_shadow_offset_x",
          "text_shadow_offset_y",
          "text_box_size",
          "text_box_width_pct",
          "text_box_height_pct"
        ]
      },
      {
        "id": "rotation",
        "title": "Rotation",
        "fields": ["rotation_use_persistent_state", "rotation_state_file"]
      },
      {
        "id": "wallpaper",
        "title": "Wallpaper",
        "fields": ["apply_wallpaper", "wallpaper_backend", "wallpaper_fit_mode"]
      }
    ]
  },
  "conditions": [
    {"field":"image_source_url","visible_when":{"field":"image_source","equals":"url"},"enabled_when":{"field":"image_source","equals":"url"}},
    {"field":"image_source_preset","visible_when":{"field":"image_source","equals":"preset"},"enabled_when":{"field":"image_source","equals":"preset"}},
    {"field":"quote_source_url","visible_when":{"field":"quote_source","equals":"url"},"enabled_when":{"field":"quote_source","equals":"url"}},
    {"field":"quote_source_preset","visible_when":{"field":"quote_source","equals":"preset"},"enabled_when":{"field":"quote_source","equals":"preset"}},
    {"field":"text_shadow_color","visible_when":{"field":"text_shadow_enabled","equals":true},"enabled_when":{"field":"text_shadow_enabled","equals":true}},
    {"field":"text_shadow_offset_x","visible_when":{"field":"text_shadow_enabled","equals":true},"enabled_when":{"field":"text_shadow_enabled","equals":true}},
    {"field":"text_shadow_offset_y","visible_when":{"field":"text_shadow_enabled","equals":true},"enabled_when":{"field":"text_shadow_enabled","equals":true}},
    {"field":"text_box_width_pct","visible_when":{"field":"text_box_size","equals":"custom"},"enabled_when":{"field":"text_box_size","equals":"custom"}},
    {"field":"text_box_height_pct","visible_when":{"field":"text_box_size","equals":"custom"},"enabled_when":{"field":"text_box_size","equals":"custom"}},
    {"field":"rotation_state_file","visible_when":{"field":"rotation_use_persistent_state","equals":true},"enabled_when":{"field":"rotation_use_persistent_state","equals":true}},
    {"field":"wallpaper_backend","visible_when":{"field":"apply_wallpaper","equals":true},"enabled_when":{"field":"apply_wallpaper","equals":true}},
    {"field":"wallpaper_fit_mode","visible_when":{"field":"apply_wallpaper","equals":true},"enabled_when":{"field":"apply_wallpaper","equals":true}}
  ]
}"#
}

pub fn default_config_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(DEFAULT_CONFIG_RELATIVE_PATH))
}

pub fn load_config(config_path: &Path) -> Result<AppConfig> {
    let raw = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config at {}", config_path.display()))?;
    let cfg = parse_config_toml_like(&raw)
        .with_context(|| format!("failed to parse config at {}", config_path.display()))?;
    Ok(cfg)
}

fn parse_config_toml_like(raw: &str) -> Result<AppConfig> {
    let mut config_version = None::<u32>;
    let mut image_dir = None::<String>;
    let mut quotes_path = None::<String>;
    let mut image_source = None::<String>;
    let mut image_source_url = None::<String>;
    let mut image_source_preset = None::<String>;
    let mut quote_source = None::<String>;
    let mut quote_source_url = None::<String>;
    let mut quote_source_preset = None::<String>;
    let mut quote_format = None::<String>;
    let mut image_order_mode = None::<String>;
    let mut image_avoid_repeat = None::<bool>;
    let mut quote_order_mode = None::<String>;
    let mut quote_avoid_repeat = None::<bool>;
    let mut quote_font_size = None::<u32>;
    let mut quote_pos_x = None::<i32>;
    let mut quote_pos_y = None::<i32>;
    let mut quote_auto_fit = None::<bool>;
    let mut quote_min_font_size = None::<u32>;
    let mut font_family = None::<String>;
    let mut quote_color = None::<String>;
    let mut clock_font_size = None::<u32>;
    let mut clock_pos_x = None::<i32>;
    let mut clock_pos_y = None::<i32>;
    let mut clock_color = None::<String>;
    let mut text_stroke_color = None::<String>;
    let mut text_stroke_width = None::<u32>;
    let mut text_undercolor = None::<String>;
    let mut text_shadow_enabled = None::<bool>;
    let mut text_shadow_color = None::<String>;
    let mut text_shadow_offset_x = None::<i32>;
    let mut text_shadow_offset_y = None::<i32>;
    let mut text_box_size = None::<String>;
    let mut text_box_width_pct = None::<u32>;
    let mut text_box_height_pct = None::<u32>;
    let mut rotation_use_persistent_state = None::<bool>;
    let mut rotation_state_file = None::<String>;
    let mut output_image = None::<String>;
    let mut refresh_seconds = None::<u64>;
    let mut image_refresh_seconds = None::<u64>;
    let mut quote_refresh_seconds = None::<u64>;
    let mut time_format = None::<String>;
    let mut apply_wallpaper = None::<bool>;
    let mut wallpaper_backend = None::<String>;
    let mut wallpaper_fit_mode = None::<String>;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value_raw)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value_raw.trim();

        match key {
            "config_version" => config_version = parse_u32(value),
            "image_dir" => image_dir = parse_string(value),
            "quotes_path" => quotes_path = parse_string(value),
            "image_source" => image_source = parse_string(value),
            "image_source_url" => image_source_url = parse_string(value),
            "image_source_preset" => image_source_preset = parse_string(value),
            "quote_source" => quote_source = parse_string(value),
            "quote_source_url" => quote_source_url = parse_string(value),
            "quote_source_preset" => quote_source_preset = parse_string(value),
            "quote_format" => quote_format = parse_string(value),
            "image_order_mode" => image_order_mode = parse_string(value),
            "image_avoid_repeat" => image_avoid_repeat = parse_bool(value),
            "quote_order_mode" => quote_order_mode = parse_string(value),
            "quote_avoid_repeat" => quote_avoid_repeat = parse_bool(value),
            "quote_font_size" => quote_font_size = parse_u32(value),
            "quote_pos_x" => quote_pos_x = parse_i32(value),
            "quote_pos_y" => quote_pos_y = parse_i32(value),
            "quote_auto_fit" => quote_auto_fit = parse_bool(value),
            "quote_min_font_size" => quote_min_font_size = parse_u32(value),
            "font_family" => font_family = parse_string(value),
            "quote_color" => quote_color = parse_string(value),
            "clock_font_size" => clock_font_size = parse_u32(value),
            "clock_pos_x" => clock_pos_x = parse_i32(value),
            "clock_pos_y" => clock_pos_y = parse_i32(value),
            "clock_color" => clock_color = parse_string(value),
            "text_stroke_color" => text_stroke_color = parse_string(value),
            "text_stroke_width" => text_stroke_width = parse_u32(value),
            "text_undercolor" => text_undercolor = parse_string(value),
            "text_shadow_enabled" => text_shadow_enabled = parse_bool(value),
            "text_shadow_color" => text_shadow_color = parse_string(value),
            "text_shadow_offset_x" => text_shadow_offset_x = parse_i32(value),
            "text_shadow_offset_y" => text_shadow_offset_y = parse_i32(value),
            "text_box_size" => text_box_size = parse_string(value),
            "text_box_width_pct" => text_box_width_pct = parse_u32(value),
            "text_box_height_pct" => text_box_height_pct = parse_u32(value),
            "rotation_use_persistent_state" => rotation_use_persistent_state = parse_bool(value),
            "rotation_state_file" => rotation_state_file = parse_string(value),
            "output_image" => output_image = parse_string(value),
            "refresh_seconds" => refresh_seconds = value.parse::<u64>().ok(),
            "image_refresh_seconds" => image_refresh_seconds = value.parse::<u64>().ok(),
            "quote_refresh_seconds" => quote_refresh_seconds = value.parse::<u64>().ok(),
            "time_format" => time_format = parse_string(value),
            "apply_wallpaper" => apply_wallpaper = parse_bool(value),
            "wallpaper_backend" => wallpaper_backend = parse_string(value),
            "wallpaper_fit_mode" => wallpaper_fit_mode = parse_string(value),
            _ => {}
        }
    }

    Ok(AppConfig {
        config_version: config_version.unwrap_or(1),
        image_dir: image_dir.ok_or_else(|| anyhow::anyhow!("missing key: image_dir"))?,
        quotes_path: quotes_path.ok_or_else(|| anyhow::anyhow!("missing key: quotes_path"))?,
        image_source: image_source.unwrap_or_else(|| "local".to_string()),
        image_source_url: sanitize_optional_string(image_source_url),
        image_source_preset: sanitize_optional_string(image_source_preset),
        quote_source: quote_source.unwrap_or_else(|| "local".to_string()),
        quote_source_url: sanitize_optional_string(quote_source_url),
        quote_source_preset: sanitize_optional_string(quote_source_preset),
        quote_format: quote_format.unwrap_or_else(|| "lines".to_string()),
        image_order_mode: image_order_mode.unwrap_or_else(|| "sequential".to_string()),
        image_avoid_repeat: image_avoid_repeat.unwrap_or(true),
        quote_order_mode: quote_order_mode.unwrap_or_else(|| "sequential".to_string()),
        quote_avoid_repeat: quote_avoid_repeat.unwrap_or(true),
        quote_font_size: quote_font_size.unwrap_or(36).max(8),
        quote_pos_x: quote_pos_x.unwrap_or(80),
        quote_pos_y: quote_pos_y.unwrap_or(860),
        quote_auto_fit: quote_auto_fit.unwrap_or(true),
        quote_min_font_size: quote_min_font_size.unwrap_or(18).max(8),
        font_family: font_family.unwrap_or_else(|| "DejaVu-Sans".to_string()),
        quote_color: quote_color.unwrap_or_else(|| "#FFFFFF".to_string()),
        clock_font_size: clock_font_size.unwrap_or(44).max(8),
        clock_pos_x: clock_pos_x.unwrap_or(1600),
        clock_pos_y: clock_pos_y.unwrap_or(960),
        clock_color: clock_color.unwrap_or_else(|| "#FFD700".to_string()),
        text_stroke_color: text_stroke_color.unwrap_or_else(|| "#000000".to_string()),
        text_stroke_width: text_stroke_width.unwrap_or(2),
        text_undercolor: text_undercolor.unwrap_or_else(|| "#00000066".to_string()),
        text_shadow_enabled: text_shadow_enabled.unwrap_or(true),
        text_shadow_color: text_shadow_color.unwrap_or_else(|| "#00000099".to_string()),
        text_shadow_offset_x: text_shadow_offset_x.unwrap_or(3),
        text_shadow_offset_y: text_shadow_offset_y.unwrap_or(3),
        text_box_size: text_box_size.unwrap_or_else(|| "quarter".to_string()),
        text_box_width_pct: text_box_width_pct.unwrap_or(50).clamp(10, 100),
        text_box_height_pct: text_box_height_pct.unwrap_or(50).clamp(10, 100),
        rotation_use_persistent_state: rotation_use_persistent_state.unwrap_or(true),
        rotation_state_file: rotation_state_file
            .unwrap_or_else(|| "~/.local/state/wallpaper-composer/rotation.state".to_string()),
        output_image: output_image.ok_or_else(|| anyhow::anyhow!("missing key: output_image"))?,
        refresh_seconds: refresh_seconds
            .ok_or_else(|| anyhow::anyhow!("missing/invalid key: refresh_seconds"))?
            .max(1),
        image_refresh_seconds: image_refresh_seconds
            .unwrap_or_else(|| refresh_seconds.unwrap_or(300))
            .max(1),
        quote_refresh_seconds: quote_refresh_seconds
            .unwrap_or_else(|| refresh_seconds.unwrap_or(300))
            .max(1),
        time_format: time_format.ok_or_else(|| anyhow::anyhow!("missing key: time_format"))?,
        apply_wallpaper: apply_wallpaper.unwrap_or(false),
        wallpaper_backend: wallpaper_backend.unwrap_or_else(|| "auto".to_string()),
        wallpaper_fit_mode: wallpaper_fit_mode.unwrap_or_else(|| "zoom".to_string()),
    })
}

fn sanitize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_string(value: &str) -> Option<String> {
    let v = value.trim();
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        return Some(v[1..v.len() - 1].to_string());
    }
    None
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok()
}

pub fn expand_tilde(input: &str) -> Result<PathBuf> {
    if let Some(rest) = input.strip_prefix("~/") {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| anyhow::anyhow!("HOME is not set; cannot expand ~/ paths"))?;
        return Ok(PathBuf::from(home).join(rest));
    }
    Ok(PathBuf::from(input))
}

pub fn list_background_images(image_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(image_dir)
        .with_context(|| format!("failed to read image_dir {}", image_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_l = ext.to_ascii_lowercase();
            if ["png", "jpg", "jpeg", "webp", "bmp"].contains(&ext_l.as_str()) {
                candidates.push(path);
            }
        }
    }

    candidates.sort();
    Ok(candidates)
}

pub fn pick_background_image(image_dir: &Path, index: u64) -> Result<PathBuf> {
    let candidates = list_background_images(image_dir)?;
    pick_by_index(&candidates, index, "images", image_dir)
}

pub fn pick_background_image_with_mode(
    image_dir: &Path,
    index: u64,
    mode: &str,
    avoid_repeat: bool,
    previous_index: Option<usize>,
) -> Result<(PathBuf, usize)> {
    let candidates = list_background_images(image_dir)?;
    if candidates.is_empty() {
        return Err(anyhow::anyhow!(
            "no images available in {}",
            image_dir.display()
        ));
    }

    let idx = pick_index_with_mode(candidates.len(), index, mode, avoid_repeat, previous_index);

    Ok((candidates[idx].clone(), idx))
}

fn pseudo_random_index(seed: u64, modulo: usize) -> usize {
    if modulo == 0 {
        return 0;
    }
    let x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((x >> 16) as usize) % modulo
}

pub fn load_quotes(quotes_path: &Path) -> Result<Vec<String>> {
    let raw = fs::read_to_string(quotes_path)
        .with_context(|| format!("failed to read quotes file {}", quotes_path.display()))?;

    let mut quotes = if raw.contains("***") {
        parse_star_delimited_quotes(&raw)
    } else {
        raw.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(strip_simple_markdown_prefix)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    };

    if quotes.is_empty() {
        quotes.push("Stay focused. Build step by step.".to_string());
    }

    Ok(quotes)
}

fn parse_star_delimited_quotes(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    let mut buf = String::new();

    for chunk in raw.split("***") {
        if inside {
            let text = chunk.trim();
            if !text.is_empty() {
                out.push(normalize_block_quote(text));
            }
        } else if !chunk.trim().is_empty() {
            // Tolerate files not starting with delimiter by treating plain text as fallback quote.
            buf.push_str(chunk.trim());
        }
        inside = !inside;
    }

    if out.is_empty() && !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }

    out
}

fn normalize_block_quote(input: &str) -> String {
    let mut lines = input
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>();

    if lines.len() >= 2 {
        let header = lines[0].trim();
        let second = lines[1].trim();
        if second != ":" && is_optional_block_label(header) {
            lines = lines[1..].to_vec();
        }
    }

    format_quote_with_optional_author(&lines)
}

fn is_optional_block_label(line: &str) -> bool {
    let upper = line.trim().to_ascii_uppercase();
    if upper.is_empty() || upper.len() > 16 {
        return false;
    }
    if let Some(rest) = upper.strip_prefix("TEXT") {
        let rest = rest.trim();
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '_');
    }
    if let Some(rest) = upper.strip_prefix('T') {
        if rest.is_empty() {
            return true;
        }
        return rest.chars().all(|c| c.is_ascii_digit() || c == '_');
    }
    false
}

fn format_quote_with_optional_author(lines: &[&str]) -> String {
    let mut split_index = None;
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == ":" {
            split_index = Some(i);
            break;
        }
    }

    let Some(idx) = split_index else {
        // Support inline style like "***Text:Author***" as requested by users.
        if lines.len() == 1
            && let Some((body, author)) = lines[0].split_once(':')
        {
            let body = body.trim();
            let author = author.trim();
            if !body.is_empty()
                && !author.is_empty()
                && !body.contains("://")
                && !author.contains("://")
            {
                return format!("{body}\n- {author}");
            }
        }
        return lines.join("\n").trim().to_string();
    };

    let body = lines[..idx]
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let author = lines[idx + 1..]
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if body.is_empty() {
        return author;
    }
    if author.is_empty() {
        return body;
    }
    format!("{body}\n- {author}")
}

pub fn pick_quote(quotes_path: &Path, index: u64) -> Result<String> {
    let quotes = load_quotes(quotes_path)?;
    pick_by_index(&quotes, index, "quotes", quotes_path)
}

pub fn pick_quote_with_mode(
    quotes_path: &Path,
    index: u64,
    mode: &str,
    avoid_repeat: bool,
    previous_index: Option<usize>,
) -> Result<(String, usize)> {
    let quotes = load_quotes(quotes_path)?;
    if quotes.is_empty() {
        return Err(anyhow::anyhow!(
            "no quotes available in {}",
            quotes_path.display()
        ));
    }
    let idx = pick_index_with_mode(quotes.len(), index, mode, avoid_repeat, previous_index);
    Ok((quotes[idx].clone(), idx))
}

pub fn cycle_index(refresh_seconds: u64) -> u64 {
    let seconds = refresh_seconds.max(1);
    let now = Local::now().timestamp().max(0) as u64;
    now / seconds
}

fn pick_by_index<T: Clone>(items: &[T], index: u64, label: &str, context: &Path) -> Result<T> {
    if items.is_empty() {
        return Err(anyhow::anyhow!(
            "no {label} available in {}",
            context.display()
        ));
    }
    let idx = (index as usize) % items.len();
    Ok(items[idx].clone())
}

fn pick_index_with_mode(
    len: usize,
    index: u64,
    mode: &str,
    avoid_repeat: bool,
    previous_index: Option<usize>,
) -> usize {
    let mut idx = match mode.trim().to_ascii_lowercase().as_str() {
        "random" => pseudo_random_index(index, len),
        _ => (index as usize) % len,
    };

    if avoid_repeat && len > 1 && previous_index == Some(idx) {
        idx = (idx + 1) % len;
    }
    idx
}

fn strip_simple_markdown_prefix(line: &str) -> &str {
    line.trim_start_matches(['#', '-', '*', '>', ' '])
}

#[derive(Debug, Clone)]
pub struct SourcePreset {
    pub id: &'static str,
    pub display_label: &'static str,
    pub name: &'static str,
    pub endpoint: &'static str,
    pub category: &'static str,
    pub auth: &'static str,
    pub rate_limit: &'static str,
    pub notes: &'static str,
}

pub fn builtin_image_presets() -> Vec<SourcePreset> {
    vec![
        SourcePreset {
            id: "nasa_apod",
            display_label: "NASA APOD (Daily Space Image)",
            name: "NASA APOD",
            endpoint: "https://api.nasa.gov/planetary/apod",
            category: "space",
            auth: "api-key",
            rate_limit: "provider-defined",
            notes: "Public NASA image metadata API; API key required for production usage.",
        },
        SourcePreset {
            id: "wikimedia_featured",
            display_label: "Wikimedia Featured Images",
            name: "Wikimedia Featured",
            endpoint: "https://commons.wikimedia.org/wiki/Commons:Featured_pictures",
            category: "public-archive",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Curated public-domain/CC media hub; scraping strategy must respect terms.",
        },
    ]
}

pub fn builtin_quote_presets() -> Vec<SourcePreset> {
    vec![
        SourcePreset {
            id: "zenquotes_daily",
            display_label: "ZenQuotes Daily",
            name: "ZenQuotes Daily",
            endpoint: "https://zenquotes.io/api/today",
            category: "quotes",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Public quote API; verify rate limits and attribution before shipping.",
        },
        SourcePreset {
            id: "quotable_random",
            display_label: "Quotable Random",
            name: "Quotable Random",
            endpoint: "https://api.quotable.io/random",
            category: "quotes",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Public quote endpoint; availability can change over time.",
        },
    ]
}

pub fn image_preset_endpoint(id: &str) -> Option<&'static str> {
    preset_endpoint(&builtin_image_presets(), id)
}

pub fn quote_preset_endpoint(id: &str) -> Option<&'static str> {
    preset_endpoint(&builtin_quote_presets(), id)
}

fn preset_endpoint(presets: &[SourcePreset], id: &str) -> Option<&'static str> {
    presets.iter().find(|p| p.id == id).map(|p| p.endpoint)
}

pub fn presets_catalog_json() -> String {
    let images = builtin_image_presets()
        .into_iter()
        .map(preset_to_json)
        .collect::<Vec<_>>()
        .join(",\n");
    let quotes = builtin_quote_presets()
        .into_iter()
        .map(preset_to_json)
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "{{\n  \"catalog_version\": 1,\n  \"image_presets\": [\n{}\n  ],\n  \"quote_presets\": [\n{}\n  ]\n}}",
        indent_block(&images, 4),
        indent_block(&quotes, 4)
    )
}

fn preset_to_json(p: SourcePreset) -> String {
    format!(
        "{{\"id\":{:?},\"display_label\":{:?},\"name\":{:?},\"endpoint\":{:?},\"category\":{:?},\"auth\":{:?},\"rate_limit\":{:?},\"notes\":{:?}}}",
        p.id, p.display_label, p.name, p.endpoint, p.category, p.auth, p.rate_limit, p.notes
    )
}

fn indent_block(input: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    input
        .lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use crate::{
        build_doctor_report, builtin_image_presets, builtin_quote_presets, default_config_toml,
        expand_tilde, load_config, load_quotes, parse_bool, parse_i32, parse_u32,
        presets_catalog_json, sanitize_optional_string, settings_schema_json,
        settings_ui_blueprint_json, to_config_toml,
    };
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn doctor_report_has_project_name() {
        let report = build_doctor_report();
        assert_eq!(report.project, "wallpaper-composer");
    }

    #[test]
    fn default_config_contains_required_keys() {
        let cfg = default_config_toml();
        assert!(cfg.contains("image_dir"));
        assert!(cfg.contains("quotes_path"));
        assert!(cfg.contains("refresh_seconds"));
    }

    #[test]
    fn expand_tilde_leaves_regular_path() {
        let p = expand_tilde("/tmp/demo").expect("path expansion should work");
        assert_eq!(p, PathBuf::from("/tmp/demo"));
    }

    #[test]
    fn load_config_reads_expected_values() {
        let cfg_path = std::env::temp_dir().join("wc-core-config-test.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let cfg = load_config(&cfg_path).expect("config should parse");
        assert_eq!(cfg.time_format, "%H:%M");
        assert_eq!(cfg.image_source, "local");
        assert_eq!(cfg.config_version, 1);
        assert_eq!(cfg.quote_source, "local");
        assert_eq!(cfg.quote_format, "lines");
        assert_eq!(cfg.image_order_mode, "sequential");
        assert!(cfg.image_avoid_repeat);
        assert_eq!(cfg.quote_order_mode, "sequential");
        assert!(cfg.quote_avoid_repeat);
        assert_eq!(cfg.quote_font_size, 36);
        assert_eq!(cfg.quote_pos_x, 80);
        assert_eq!(cfg.quote_pos_y, 860);
        assert!(cfg.quote_auto_fit);
        assert_eq!(cfg.quote_min_font_size, 18);
        assert_eq!(cfg.font_family, "DejaVu-Sans");
        assert_eq!(cfg.quote_color, "#FFFFFF");
        assert_eq!(cfg.clock_font_size, 44);
        assert_eq!(cfg.clock_pos_x, 1600);
        assert_eq!(cfg.clock_pos_y, 960);
        assert_eq!(cfg.clock_color, "#FFD700");
        assert_eq!(cfg.text_stroke_color, "#000000");
        assert_eq!(cfg.text_stroke_width, 2);
        assert_eq!(cfg.text_undercolor, "#00000066");
        assert!(cfg.text_shadow_enabled);
        assert_eq!(cfg.text_shadow_color, "#00000099");
        assert_eq!(cfg.text_shadow_offset_x, 3);
        assert_eq!(cfg.text_shadow_offset_y, 3);
        assert_eq!(cfg.text_box_size, "quarter");
        assert_eq!(cfg.text_box_width_pct, 50);
        assert_eq!(cfg.text_box_height_pct, 50);
        assert!(cfg.rotation_use_persistent_state);
        assert_eq!(
            cfg.rotation_state_file,
            "~/.local/state/wallpaper-composer/rotation.state"
        );
        assert!(!cfg.apply_wallpaper);
        assert_eq!(cfg.wallpaper_backend, "auto");
        assert_eq!(cfg.wallpaper_fit_mode, "zoom");
        assert_eq!(cfg.image_refresh_seconds, 300);
        assert_eq!(cfg.quote_refresh_seconds, 300);
        let _ = fs::remove_file(cfg_path);
    }

    #[test]
    fn parse_bool_reads_true_false() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("1"), None);
    }

    #[test]
    fn parse_integer_helpers_work() {
        assert_eq!(parse_u32("12"), Some(12));
        assert_eq!(parse_u32("-1"), None);
        assert_eq!(parse_i32("-3"), Some(-3));
        assert_eq!(parse_i32("x"), None);
    }

    #[test]
    fn load_quotes_skips_empty_and_markdown_prefix() {
        let quotes_path = std::env::temp_dir().join("wc-core-quotes-test.md");
        fs::write(&quotes_path, "# Hello\n\n- World\n").expect("quotes should be writable");
        let quotes = load_quotes(&quotes_path).expect("quotes should parse");
        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0], "Hello");
        assert_eq!(quotes[1], "World");
        let _ = fs::remove_file(quotes_path);
    }

    #[test]
    fn load_quotes_supports_star_delimited_blocks() {
        let quotes_path = std::env::temp_dir().join("wc-core-quotes-block-test.md");
        let sample = "*** Text 1\nBlabla\nBlabal***\n***T2\nweiter zweiter anzeigetext\n***\n***T\nDritter anzeigetext***";
        fs::write(&quotes_path, sample).expect("quotes block file should be writable");
        let quotes = load_quotes(&quotes_path).expect("block quotes should parse");
        assert_eq!(quotes.len(), 3);
        assert_eq!(quotes[0], "Blabla\nBlabal");
        assert_eq!(quotes[1], "weiter zweiter anzeigetext");
        assert_eq!(quotes[2], "Dritter anzeigetext");
        let _ = fs::remove_file(quotes_path);
    }

    #[test]
    fn load_quotes_supports_author_separator_in_block() {
        let quotes_path = std::env::temp_dir().join("wc-core-quotes-author-block-test.md");
        let sample = "***\nText1\n:\nVerfasser\n***";
        fs::write(&quotes_path, sample).expect("quotes author block file should be writable");
        let quotes = load_quotes(&quotes_path).expect("author block should parse");
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0], "Text1\n- Verfasser");
        let _ = fs::remove_file(quotes_path);
    }

    #[test]
    fn load_quotes_supports_inline_text_author_block() {
        let quotes_path = std::env::temp_dir().join("wc-core-quotes-inline-author-test.md");
        let sample = "***Text1:Verfasser***";
        fs::write(&quotes_path, sample).expect("inline author block file should be writable");
        let quotes = load_quotes(&quotes_path).expect("inline author block should parse");
        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0], "Text1\n- Verfasser");
        let _ = fs::remove_file(quotes_path);
    }

    #[test]
    fn load_quotes_supports_multiple_inline_text_author_blocks() {
        let quotes_path = std::env::temp_dir().join("wc-core-quotes-inline-author-multi-test.md");
        let sample = "***Text eins:Autor A***\n***Text zwei:Autor B***";
        fs::write(&quotes_path, sample).expect("inline author blocks file should be writable");
        let quotes = load_quotes(&quotes_path).expect("inline author blocks should parse");
        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0], "Text eins\n- Autor A");
        assert_eq!(quotes[1], "Text zwei\n- Autor B");
        let _ = fs::remove_file(quotes_path);
    }

    #[test]
    fn sanitize_optional_string_handles_empty() {
        assert_eq!(sanitize_optional_string(Some("".to_string())), None);
        assert_eq!(sanitize_optional_string(Some("  ".to_string())), None);
        assert_eq!(
            sanitize_optional_string(Some("x".to_string())),
            Some("x".to_string())
        );
    }

    #[test]
    fn builtin_presets_are_available() {
        assert!(!builtin_image_presets().is_empty());
        assert!(!builtin_quote_presets().is_empty());
        assert!(builtin_image_presets()[0].display_label.contains("NASA"));
    }

    #[test]
    fn preset_endpoint_lookup_works() {
        assert!(super::image_preset_endpoint("nasa_apod").is_some());
        assert!(super::quote_preset_endpoint("zenquotes_daily").is_some());
        assert!(super::quote_preset_endpoint("missing").is_none());
    }

    #[test]
    fn schema_mentions_config_version() {
        assert!(settings_schema_json().contains("\"config_version\""));
    }

    #[test]
    fn schema_contains_ui_contract_groups() {
        let schema = settings_schema_json();
        assert!(schema.contains("\"ui_contract_version\""));
        assert!(schema.contains("\"groups\""));
        assert!(schema.contains("\"layout\""));
        assert!(schema.contains("\"visible_when\""));
        assert!(schema.contains("\"enabled_when\""));
        assert!(schema.contains("\"ui_widget\""));
    }

    #[test]
    fn ui_blueprint_contains_sections_and_conditions() {
        let blueprint = settings_ui_blueprint_json();
        assert!(blueprint.contains("\"sections\""));
        assert!(blueprint.contains("\"conditions\""));
        assert!(blueprint.contains("\"wallpaper_backend\""));
        assert!(blueprint.contains("\"wallpaper_fit_mode\""));
    }

    #[test]
    fn presets_catalog_contains_expected_keys() {
        let json = presets_catalog_json();
        assert!(json.contains("\"image_presets\""));
        assert!(json.contains("\"quote_presets\""));
        assert!(json.contains("\"display_label\""));
        assert!(json.contains("\"rate_limit\""));
    }

    #[test]
    fn to_config_toml_writes_version() {
        let cfg_path = std::env::temp_dir().join("wc-core-config-format-test.toml");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");
        let cfg = load_config(&cfg_path).expect("config should parse");
        let out = to_config_toml(&cfg);
        assert!(out.contains("config_version = 1"));
        let _ = fs::remove_file(cfg_path);
    }
}
