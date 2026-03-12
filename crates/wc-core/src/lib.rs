use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

mod news_catalog;
pub mod widget_registry;

pub use news_catalog::{
    NewsSourcePreset, builtin_news_source, builtin_news_source_is_live_video,
    builtin_news_source_label, builtin_news_source_name, builtin_news_source_stream_url,
    builtin_news_source_ticker_url, builtin_news_sources,
};
pub use widget_registry::{
    BUILTIN_WIDGET_TYPE_IDS, WidgetInstanceConfig, WidgetPlugin, WidgetRegistry,
    WidgetResolvedPayload, WidgetRuntimeContext, WidgetTypeId,
};

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
pub const DEFAULT_LOCAL_QUOTES_PATH: &str = "~/Documents/wallpaper-composer/quotes.md";
const PACKAGED_LOCAL_QUOTES_PATHS: [&str; 2] = [
    "/usr/share/le-compositeur/quotes/local-quotes.md",
    "/usr/share/wallpaper-composer/quotes/local-quotes.md",
];
const BUNDLED_LOCAL_QUOTES: &str = include_str!("../../../assets/quotes/local/local-quotes.md");

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
    pub show_background_layer: bool,
    pub show_quote_layer: bool,
    pub show_clock_layer: bool,
    pub show_weather_layer: bool,
    pub show_news_layer: bool,
    pub show_cams_layer: bool,
    pub layer_z_quote: i32,
    pub layer_z_clock: i32,
    pub layer_z_weather: i32,
    pub layer_z_news: i32,
    pub layer_z_cams: i32,
    pub weather_pos_x: i32,
    pub weather_pos_y: i32,
    pub weather_widget_width: u32,
    pub weather_widget_height: u32,
    pub weather_font_size: u32,
    pub weather_font_family: String,
    pub weather_color: String,
    pub weather_undercolor: String,
    pub weather_stroke_color: String,
    pub weather_stroke_width: u32,
    pub news_pos_x: i32,
    pub news_pos_y: i32,
    pub news_widget_width: u32,
    pub news_widget_height: u32,
    pub weather_refresh_seconds: u64,
    pub weather_use_system_location: bool,
    pub weather_location_override: String,
    pub news_source: String,
    pub news_custom_url: String,
    pub news_render_mode: String,
    pub news_fps: f32,
    pub news_refresh_seconds: u64,
    pub news_audio_enabled: bool,
    pub show_news_ticker2: bool,
    pub news_ticker2_pos_x: i32,
    pub news_ticker2_pos_y: i32,
    pub news_ticker2_width: u32,
    pub news_ticker2_source: String,
    pub news_ticker2_custom_url: String,
    pub news_ticker2_fps: f32,
    pub news_ticker2_refresh_seconds: u64,
    pub cams_pos_x: i32,
    pub cams_pos_y: i32,
    pub cams_widget_width: u32,
    pub cams_widget_height: u32,
    pub cams_source: String,
    pub cams_render_mode: String,
    pub cams_custom_urls: String,
    pub cams_refresh_seconds: u64,
    pub cams_fps: f32,
    pub cams_count: u32,
    pub cams_columns: u32,
    pub overlay_script_ticker_enabled: bool,
    pub overlay_script_ticker_command: String,
    pub overlay_script_ticker_refresh_seconds: u64,
    pub overlay_script_ticker_pos_x: i32,
    pub overlay_script_ticker_pos_y: i32,
    pub overlay_script_ticker_width: u32,
    pub overlay_script_ticker_height: u32,
    pub overlay_script_ticker_font_size: u32,
    pub login_screen_integration: bool,
    pub boot_screen_integration: bool,
}

pub fn default_config_toml() -> String {
    r##"# Le Compositeur config
config_version = 1
image_dir = "~/Pictures/Wallpapers"
quotes_path = "~/Documents/wallpaper-composer/quotes.md"
image_source = "preset"
image_source_url = ""
image_source_preset = "placecats_1920_1080"
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
output_image = "~/.local/state/wallpaper-composer/current.png"
refresh_seconds = 300
image_refresh_seconds = 300
quote_refresh_seconds = 300
time_format = "%H:%M"
apply_wallpaper = false
wallpaper_backend = "auto"
wallpaper_fit_mode = "zoom"
show_background_layer = true
show_quote_layer = true
show_clock_layer = true
show_weather_layer = false
show_news_layer = false
show_cams_layer = false
layer_z_quote = 40
layer_z_clock = 50
layer_z_weather = 60
layer_z_news = 70
layer_z_cams = 80
weather_pos_x = 120
weather_pos_y = 120
weather_widget_width = 640
weather_widget_height = 180
weather_font_size = 30
weather_font_family = "DejaVu-Sans"
weather_color = "#00F5FF"
weather_undercolor = "#0B0014B3"
weather_stroke_color = "#001A22"
weather_stroke_width = 1
news_pos_x = 980
news_pos_y = 180
news_widget_width = 760
news_widget_height = 240
weather_refresh_seconds = 600
weather_use_system_location = true
weather_location_override = ""
news_source = "euronews"
news_custom_url = ""
news_render_mode = "overlay"
news_fps = 1.0
news_refresh_seconds = 90
news_audio_enabled = false
show_news_ticker2 = false
news_ticker2_pos_x = 120
news_ticker2_pos_y = 980
news_ticker2_width = 1280
news_ticker2_source = "google_world_en"
news_ticker2_custom_url = ""
news_ticker2_fps = 1.2
news_ticker2_refresh_seconds = 120
cams_pos_x = 980
cams_pos_y = 640
cams_widget_width = 760
cams_widget_height = 428
cams_source = "auto_local"
cams_render_mode = "overlay"
cams_custom_urls = ""
cams_refresh_seconds = 75
cams_fps = 1.0
cams_count = 2
cams_columns = 2
overlay_script_ticker_enabled = false
overlay_script_ticker_command = ""
overlay_script_ticker_refresh_seconds = 30
overlay_script_ticker_pos_x = 120
overlay_script_ticker_pos_y = 920
overlay_script_ticker_width = 1280
overlay_script_ticker_height = 56
overlay_script_ticker_font_size = 30
login_screen_integration = false
boot_screen_integration = false
"##
    .to_string()
}

pub fn to_config_toml(cfg: &AppConfig) -> String {
    format!(
        "# Le Compositeur config\nconfig_version = {}\nimage_dir = {:?}\nquotes_path = {:?}\nimage_source = {:?}\nimage_source_url = {:?}\nimage_source_preset = {:?}\nquote_source = {:?}\nquote_source_url = {:?}\nquote_source_preset = {:?}\nquote_format = {:?}\nimage_order_mode = {:?}\nimage_avoid_repeat = {}\nquote_order_mode = {:?}\nquote_avoid_repeat = {}\nquote_font_size = {}\nquote_pos_x = {}\nquote_pos_y = {}\nquote_auto_fit = {}\nquote_min_font_size = {}\nfont_family = {:?}\nquote_color = {:?}\nclock_font_size = {}\nclock_pos_x = {}\nclock_pos_y = {}\nclock_color = {:?}\ntext_stroke_color = {:?}\ntext_stroke_width = {}\ntext_undercolor = {:?}\ntext_shadow_enabled = {}\ntext_shadow_color = {:?}\ntext_shadow_offset_x = {}\ntext_shadow_offset_y = {}\ntext_box_size = {:?}\ntext_box_width_pct = {}\ntext_box_height_pct = {}\nrotation_use_persistent_state = {}\nrotation_state_file = {:?}\noutput_image = {:?}\nrefresh_seconds = {}\nimage_refresh_seconds = {}\nquote_refresh_seconds = {}\ntime_format = {:?}\napply_wallpaper = {}\nwallpaper_backend = {:?}\nwallpaper_fit_mode = {:?}\nshow_background_layer = {}\nshow_quote_layer = {}\nshow_clock_layer = {}\nshow_weather_layer = {}\nshow_news_layer = {}\nshow_cams_layer = {}\nlayer_z_quote = {}\nlayer_z_clock = {}\nlayer_z_weather = {}\nlayer_z_news = {}\nlayer_z_cams = {}\nweather_pos_x = {}\nweather_pos_y = {}\nweather_widget_width = {}\nweather_widget_height = {}\nweather_font_size = {}\nweather_font_family = {:?}\nweather_color = {:?}\nweather_undercolor = {:?}\nweather_stroke_color = {:?}\nweather_stroke_width = {}\nnews_pos_x = {}\nnews_pos_y = {}\nnews_widget_width = {}\nnews_widget_height = {}\nweather_refresh_seconds = {}\nweather_use_system_location = {}\nweather_location_override = {:?}\nnews_source = {:?}\nnews_custom_url = {:?}\nnews_render_mode = {:?}\nnews_fps = {}\nnews_refresh_seconds = {}\nnews_audio_enabled = {}\nshow_news_ticker2 = {}\nnews_ticker2_pos_x = {}\nnews_ticker2_pos_y = {}\nnews_ticker2_width = {}\nnews_ticker2_source = {:?}\nnews_ticker2_custom_url = {:?}\nnews_ticker2_fps = {}\nnews_ticker2_refresh_seconds = {}\ncams_pos_x = {}\ncams_pos_y = {}\ncams_widget_width = {}\ncams_widget_height = {}\ncams_source = {:?}\ncams_render_mode = {:?}\ncams_custom_urls = {:?}\ncams_refresh_seconds = {}\ncams_fps = {}\ncams_count = {}\ncams_columns = {}\noverlay_script_ticker_enabled = {}\noverlay_script_ticker_command = {:?}\noverlay_script_ticker_refresh_seconds = {}\noverlay_script_ticker_pos_x = {}\noverlay_script_ticker_pos_y = {}\noverlay_script_ticker_width = {}\noverlay_script_ticker_height = {}\noverlay_script_ticker_font_size = {}\nlogin_screen_integration = {}\nboot_screen_integration = {}\n",
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
        cfg.wallpaper_fit_mode,
        cfg.show_background_layer,
        cfg.show_quote_layer,
        cfg.show_clock_layer,
        cfg.show_weather_layer,
        cfg.show_news_layer,
        cfg.show_cams_layer,
        cfg.layer_z_quote,
        cfg.layer_z_clock,
        cfg.layer_z_weather,
        cfg.layer_z_news,
        cfg.layer_z_cams,
        cfg.weather_pos_x,
        cfg.weather_pos_y,
        cfg.weather_widget_width,
        cfg.weather_widget_height,
        cfg.weather_font_size,
        cfg.weather_font_family,
        cfg.weather_color,
        cfg.weather_undercolor,
        cfg.weather_stroke_color,
        cfg.weather_stroke_width,
        cfg.news_pos_x,
        cfg.news_pos_y,
        cfg.news_widget_width,
        cfg.news_widget_height,
        cfg.weather_refresh_seconds,
        cfg.weather_use_system_location,
        cfg.weather_location_override,
        cfg.news_source,
        cfg.news_custom_url,
        cfg.news_render_mode,
        cfg.news_fps,
        cfg.news_refresh_seconds,
        cfg.news_audio_enabled,
        cfg.show_news_ticker2,
        cfg.news_ticker2_pos_x,
        cfg.news_ticker2_pos_y,
        cfg.news_ticker2_width,
        cfg.news_ticker2_source,
        cfg.news_ticker2_custom_url,
        cfg.news_ticker2_fps,
        cfg.news_ticker2_refresh_seconds,
        cfg.cams_pos_x,
        cfg.cams_pos_y,
        cfg.cams_widget_width,
        cfg.cams_widget_height,
        cfg.cams_source,
        cfg.cams_render_mode,
        cfg.cams_custom_urls,
        cfg.cams_refresh_seconds,
        cfg.cams_fps,
        cfg.cams_count,
        cfg.cams_columns,
        cfg.overlay_script_ticker_enabled,
        cfg.overlay_script_ticker_command,
        cfg.overlay_script_ticker_refresh_seconds,
        cfg.overlay_script_ticker_pos_x,
        cfg.overlay_script_ticker_pos_y,
        cfg.overlay_script_ticker_width,
        cfg.overlay_script_ticker_height,
        cfg.overlay_script_ticker_font_size,
        cfg.login_screen_integration,
        cfg.boot_screen_integration
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
    {"key":"image_source","group":"sources","label":"Image Source Mode","type":"enum","required":false,"default":"preset","options":["local","preset","url"]},
    {"key":"image_source_url","group":"sources","label":"Image Source URL","type":"string","required":false,"default":"","visible_when":{"field":"image_source","equals":"url"},"enabled_when":{"field":"image_source","equals":"url"}},
    {"key":"image_source_preset","group":"sources","label":"Image Source Preset","type":"string","required":false,"default":"placecats_1920_1080","visible_when":{"field":"image_source","equals":"preset"},"enabled_when":{"field":"image_source","equals":"preset"}},
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
    {"key":"wallpaper_fit_mode","group":"wallpaper","label":"Wallpaper Fit Mode","type":"enum","required":false,"default":"zoom","options":["zoom","scaled","stretched","spanned","centered","wallpaper"],"visible_when":{"field":"apply_wallpaper","equals":true},"enabled_when":{"field":"apply_wallpaper","equals":true}},
    {"key":"show_background_layer","group":"wallpaper","label":"Show Background Layer","type":"bool","required":false,"default":true},
    {"key":"show_quote_layer","group":"wallpaper","label":"Show Quote Layer","type":"bool","required":false,"default":true},
    {"key":"show_clock_layer","group":"wallpaper","label":"Show Clock Layer","type":"bool","required":false,"default":true},
    {"key":"show_weather_layer","group":"wallpaper","label":"Show Weather Layer","type":"bool","required":false,"default":false},
    {"key":"show_news_layer","group":"wallpaper","label":"Show News Layer","type":"bool","required":false,"default":false},
    {"key":"show_cams_layer","group":"wallpaper","label":"Show Cams Layer","type":"bool","required":false,"default":false},
    {"key":"layer_z_quote","group":"wallpaper","label":"Layer Z Quote","type":"i32","required":false,"default":40},
    {"key":"layer_z_clock","group":"wallpaper","label":"Layer Z Clock","type":"i32","required":false,"default":50},
    {"key":"layer_z_weather","group":"wallpaper","label":"Layer Z Weather","type":"i32","required":false,"default":60},
    {"key":"layer_z_news","group":"wallpaper","label":"Layer Z News","type":"i32","required":false,"default":70},
    {"key":"layer_z_cams","group":"wallpaper","label":"Layer Z Cams","type":"i32","required":false,"default":80},
    {"key":"weather_pos_x","group":"wallpaper","label":"Weather X","type":"i32","required":false,"default":120},
    {"key":"weather_pos_y","group":"wallpaper","label":"Weather Y","type":"i32","required":false,"default":120},
    {"key":"weather_widget_width","group":"wallpaper","label":"Weather Widget Width","type":"u32","required":false,"default":640},
    {"key":"weather_widget_height","group":"wallpaper","label":"Weather Widget Height","type":"u32","required":false,"default":180},
    {"key":"weather_font_size","group":"wallpaper","label":"Weather Font Size","type":"u32","required":false,"default":30},
    {"key":"weather_font_family","group":"wallpaper","label":"Weather Font Family","type":"enum","required":false,"default":"DejaVu-Sans","options":["DejaVu-Sans","Noto-Sans","Liberation-Sans","Serif","Monospace"]},
    {"key":"weather_color","group":"wallpaper","label":"Weather Color","type":"string","required":false,"default":"#00F5FF"},
    {"key":"weather_undercolor","group":"wallpaper","label":"Weather Undercolor","type":"string","required":false,"default":"#0B0014B3"},
    {"key":"weather_stroke_color","group":"wallpaper","label":"Weather Stroke Color","type":"string","required":false,"default":"#001A22"},
    {"key":"weather_stroke_width","group":"wallpaper","label":"Weather Stroke Width","type":"u32","required":false,"default":1},
    {"key":"news_pos_x","group":"wallpaper","label":"News X","type":"i32","required":false,"default":980},
    {"key":"news_pos_y","group":"wallpaper","label":"News Y","type":"i32","required":false,"default":180},
    {"key":"news_widget_width","group":"wallpaper","label":"News Widget Width","type":"u32","required":false,"default":760},
    {"key":"news_widget_height","group":"wallpaper","label":"News Widget Height","type":"u32","required":false,"default":240},
    {"key":"weather_refresh_seconds","group":"wallpaper","label":"Weather Refresh Seconds","type":"u64","required":false,"default":600},
    {"key":"weather_use_system_location","group":"wallpaper","label":"Weather Use System Location","type":"bool","required":false,"default":true},
    {"key":"weather_location_override","group":"wallpaper","label":"Weather Location Override","type":"string","required":false,"default":"","visible_when":{"field":"weather_use_system_location","equals":false},"enabled_when":{"field":"weather_use_system_location","equals":false}},
    {"key":"news_source","group":"wallpaper","label":"News Source","type":"string","required":false,"default":"euronews"},
    {"key":"news_custom_url","group":"wallpaper","label":"News Custom URL","type":"string","required":false,"default":"","visible_when":{"field":"news_source","equals":"custom"},"enabled_when":{"field":"news_source","equals":"custom"}},
    {"key":"news_render_mode","group":"wallpaper","label":"News Render Mode","type":"enum","required":false,"default":"overlay","options":["overlay"]},
    {"key":"news_fps","group":"wallpaper","label":"News FPS","type":"f32","required":false,"default":1.0},
    {"key":"news_refresh_seconds","group":"wallpaper","label":"News Refresh Seconds","type":"u64","required":false,"default":90},
    {"key":"news_audio_enabled","group":"wallpaper","label":"News Audio Enabled","type":"bool","required":false,"default":false},
    {"key":"show_news_ticker2","group":"wallpaper","label":"Show Secondary News Ticker","type":"bool","required":false,"default":false},
    {"key":"news_ticker2_pos_x","group":"wallpaper","label":"News Ticker 2 X","type":"i32","required":false,"default":120},
    {"key":"news_ticker2_pos_y","group":"wallpaper","label":"News Ticker 2 Y","type":"i32","required":false,"default":980},
    {"key":"news_ticker2_width","group":"wallpaper","label":"News Ticker 2 Width","type":"u32","required":false,"default":1280},
    {"key":"news_ticker2_source","group":"wallpaper","label":"News Ticker 2 Source","type":"string","required":false,"default":"techcrunch"},
    {"key":"news_ticker2_custom_url","group":"wallpaper","label":"News Ticker 2 Custom URL","type":"string","required":false,"default":"","visible_when":{"field":"news_ticker2_source","equals":"custom"},"enabled_when":{"field":"news_ticker2_source","equals":"custom"}},
    {"key":"news_ticker2_fps","group":"wallpaper","label":"News Ticker 2 FPS","type":"f32","required":false,"default":1.2},
    {"key":"news_ticker2_refresh_seconds","group":"wallpaper","label":"News Ticker 2 Refresh Seconds","type":"u64","required":false,"default":120},
    {"key":"cams_pos_x","group":"wallpaper","label":"Cams X","type":"i32","required":false,"default":980},
    {"key":"cams_pos_y","group":"wallpaper","label":"Cams Y","type":"i32","required":false,"default":640},
    {"key":"cams_widget_width","group":"wallpaper","label":"Cams Widget Width","type":"u32","required":false,"default":760},
    {"key":"cams_widget_height","group":"wallpaper","label":"Cams Widget Height","type":"u32","required":false,"default":428},
    {"key":"cams_source","group":"wallpaper","label":"Cams Source","type":"enum","required":false,"default":"auto_local","options":["auto_local","city_public","custom"]},
    {"key":"cams_render_mode","group":"wallpaper","label":"Cams Render Mode","type":"enum","required":false,"default":"overlay","options":["overlay"]},
    {"key":"cams_custom_urls","group":"wallpaper","label":"Cams Custom URLs","type":"string","required":false,"default":"","visible_when":{"field":"cams_source","equals":"custom"},"enabled_when":{"field":"cams_source","equals":"custom"}},
    {"key":"cams_refresh_seconds","group":"wallpaper","label":"Cams Refresh Seconds","type":"u64","required":false,"default":75},
    {"key":"cams_fps","group":"wallpaper","label":"Cams FPS","type":"f32","required":false,"default":1.0},
    {"key":"cams_count","group":"wallpaper","label":"Cams Count","type":"u32","required":false,"default":2,"min":1,"max":9},
    {"key":"cams_columns","group":"wallpaper","label":"Cams Columns","type":"u32","required":false,"default":2,"min":1,"max":4},
    {"key":"overlay_script_ticker_enabled","group":"wallpaper","label":"Overlay Script Ticker Enabled","type":"bool","required":false,"default":false},
    {"key":"overlay_script_ticker_command","group":"wallpaper","label":"Overlay Script Ticker Command","type":"string","required":false,"default":"","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"overlay_script_ticker_refresh_seconds","group":"wallpaper","label":"Overlay Script Ticker Refresh Seconds","type":"u64","required":false,"default":30,"visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"overlay_script_ticker_pos_x","group":"wallpaper","label":"Overlay Script Ticker X","type":"i32","required":false,"default":120,"visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"overlay_script_ticker_pos_y","group":"wallpaper","label":"Overlay Script Ticker Y","type":"i32","required":false,"default":920,"visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"overlay_script_ticker_width","group":"wallpaper","label":"Overlay Script Ticker Width","type":"u32","required":false,"default":1280,"min":220,"max":1920,"visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"overlay_script_ticker_height","group":"wallpaper","label":"Overlay Script Ticker Height","type":"u32","required":false,"default":56,"min":32,"max":240,"visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"overlay_script_ticker_font_size","group":"wallpaper","label":"Overlay Script Ticker Font Size","type":"u32","required":false,"default":30,"min":10,"max":120,"visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"key":"login_screen_integration","group":"wallpaper","label":"Login Screen Integration","type":"bool","required":false,"default":false},
    {"key":"boot_screen_integration","group":"wallpaper","label":"Boot Screen Integration","type":"bool","required":false,"default":false}
  ]
}"##
}

pub fn settings_ui_blueprint_json() -> &'static str {
    r#"{
  "blueprint_version": 1,
  "form": {
    "id": "wallpaper-composer-settings",
    "title": "Le Compositeur Settings",
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
        "fields": ["apply_wallpaper", "wallpaper_backend", "wallpaper_fit_mode", "show_background_layer", "show_quote_layer", "show_clock_layer", "show_weather_layer", "show_news_layer", "show_cams_layer", "layer_z_quote", "layer_z_clock", "layer_z_weather", "layer_z_news", "layer_z_cams", "weather_pos_x", "weather_pos_y", "weather_widget_width", "weather_widget_height", "weather_font_size", "weather_font_family", "weather_color", "weather_undercolor", "weather_stroke_color", "weather_stroke_width", "news_pos_x", "news_pos_y", "news_widget_width", "news_widget_height", "weather_refresh_seconds", "weather_use_system_location", "weather_location_override", "news_source", "news_custom_url", "news_render_mode", "news_fps", "news_refresh_seconds", "news_audio_enabled", "show_news_ticker2", "news_ticker2_pos_x", "news_ticker2_pos_y", "news_ticker2_width", "news_ticker2_source", "news_ticker2_custom_url", "news_ticker2_fps", "news_ticker2_refresh_seconds", "cams_pos_x", "cams_pos_y", "cams_widget_width", "cams_widget_height", "cams_source", "cams_render_mode", "cams_custom_urls", "cams_refresh_seconds", "cams_fps", "cams_count", "cams_columns", "overlay_script_ticker_enabled", "overlay_script_ticker_command", "overlay_script_ticker_refresh_seconds", "overlay_script_ticker_pos_x", "overlay_script_ticker_pos_y", "overlay_script_ticker_width", "overlay_script_ticker_height", "overlay_script_ticker_font_size", "login_screen_integration", "boot_screen_integration"]
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
    {"field":"wallpaper_fit_mode","visible_when":{"field":"apply_wallpaper","equals":true},"enabled_when":{"field":"apply_wallpaper","equals":true}},
    {"field":"show_background_layer"},
    {"field":"show_quote_layer"},
    {"field":"show_clock_layer"},
    {"field":"show_weather_layer"},
    {"field":"show_news_layer"},
    {"field":"show_cams_layer"},
    {"field":"weather_location_override","visible_when":{"field":"weather_use_system_location","equals":false},"enabled_when":{"field":"weather_use_system_location","equals":false}},
    {"field":"news_custom_url","visible_when":{"field":"news_source","equals":"custom"},"enabled_when":{"field":"news_source","equals":"custom"}},
    {"field":"news_ticker2_custom_url","visible_when":{"field":"news_ticker2_source","equals":"custom"},"enabled_when":{"field":"news_ticker2_source","equals":"custom"}},
    {"field":"cams_custom_urls","visible_when":{"field":"cams_source","equals":"custom"},"enabled_when":{"field":"cams_source","equals":"custom"}},
    {"field":"overlay_script_ticker_command","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"overlay_script_ticker_refresh_seconds","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"overlay_script_ticker_pos_x","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"overlay_script_ticker_pos_y","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"overlay_script_ticker_width","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"overlay_script_ticker_height","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"overlay_script_ticker_font_size","visible_when":{"field":"overlay_script_ticker_enabled","equals":true},"enabled_when":{"field":"overlay_script_ticker_enabled","equals":true}},
    {"field":"login_screen_integration"},
    {"field":"boot_screen_integration"}
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
    let mut show_background_layer = None::<bool>;
    let mut show_quote_layer = None::<bool>;
    let mut show_clock_layer = None::<bool>;
    let mut show_weather_layer = None::<bool>;
    let mut show_news_layer = None::<bool>;
    let mut show_cams_layer = None::<bool>;
    let mut layer_z_quote = None::<i32>;
    let mut layer_z_clock = None::<i32>;
    let mut layer_z_weather = None::<i32>;
    let mut layer_z_news = None::<i32>;
    let mut layer_z_cams = None::<i32>;
    let mut weather_pos_x = None::<i32>;
    let mut weather_pos_y = None::<i32>;
    let mut weather_widget_width = None::<u32>;
    let mut weather_widget_height = None::<u32>;
    let mut weather_font_size = None::<u32>;
    let mut weather_font_family = None::<String>;
    let mut weather_color = None::<String>;
    let mut weather_undercolor = None::<String>;
    let mut weather_stroke_color = None::<String>;
    let mut weather_stroke_width = None::<u32>;
    let mut news_pos_x = None::<i32>;
    let mut news_pos_y = None::<i32>;
    let mut news_widget_width = None::<u32>;
    let mut news_widget_height = None::<u32>;
    let mut weather_refresh_seconds = None::<u64>;
    let mut weather_use_system_location = None::<bool>;
    let mut weather_location_override = None::<String>;
    let mut news_source = None::<String>;
    let mut news_custom_url = None::<String>;
    let mut news_render_mode = None::<String>;
    let mut news_fps = None::<f32>;
    let mut news_refresh_seconds = None::<u64>;
    let mut news_audio_enabled = None::<bool>;
    let mut show_news_ticker2 = None::<bool>;
    let mut news_ticker2_pos_x = None::<i32>;
    let mut news_ticker2_pos_y = None::<i32>;
    let mut news_ticker2_width = None::<u32>;
    let mut news_ticker2_source = None::<String>;
    let mut news_ticker2_custom_url = None::<String>;
    let mut news_ticker2_fps = None::<f32>;
    let mut news_ticker2_refresh_seconds = None::<u64>;
    let mut cams_pos_x = None::<i32>;
    let mut cams_pos_y = None::<i32>;
    let mut cams_widget_width = None::<u32>;
    let mut cams_widget_height = None::<u32>;
    let mut cams_source = None::<String>;
    let mut cams_render_mode = None::<String>;
    let mut cams_custom_urls = None::<String>;
    let mut cams_refresh_seconds = None::<u64>;
    let mut cams_fps = None::<f32>;
    let mut cams_count = None::<u32>;
    let mut cams_columns = None::<u32>;
    let mut overlay_script_ticker_enabled = None::<bool>;
    let mut overlay_script_ticker_command = None::<String>;
    let mut overlay_script_ticker_refresh_seconds = None::<u64>;
    let mut overlay_script_ticker_pos_x = None::<i32>;
    let mut overlay_script_ticker_pos_y = None::<i32>;
    let mut overlay_script_ticker_width = None::<u32>;
    let mut overlay_script_ticker_height = None::<u32>;
    let mut overlay_script_ticker_font_size = None::<u32>;
    let mut login_screen_integration = None::<bool>;
    let mut boot_screen_integration = None::<bool>;

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
            "show_background_layer" => show_background_layer = parse_bool(value),
            "show_quote_layer" => show_quote_layer = parse_bool(value),
            "show_clock_layer" => show_clock_layer = parse_bool(value),
            "show_weather_layer" => show_weather_layer = parse_bool(value),
            "show_news_layer" => show_news_layer = parse_bool(value),
            "show_cams_layer" => show_cams_layer = parse_bool(value),
            "layer_z_quote" => layer_z_quote = parse_i32(value),
            "layer_z_clock" => layer_z_clock = parse_i32(value),
            "layer_z_weather" => layer_z_weather = parse_i32(value),
            "layer_z_news" => layer_z_news = parse_i32(value),
            "layer_z_cams" => layer_z_cams = parse_i32(value),
            "weather_pos_x" => weather_pos_x = parse_i32(value),
            "weather_pos_y" => weather_pos_y = parse_i32(value),
            "weather_widget_width" => weather_widget_width = parse_u32(value),
            "weather_widget_height" => weather_widget_height = parse_u32(value),
            "weather_font_size" => weather_font_size = parse_u32(value),
            "weather_font_family" => weather_font_family = parse_string(value),
            "weather_color" => weather_color = parse_string(value),
            "weather_undercolor" => weather_undercolor = parse_string(value),
            "weather_stroke_color" => weather_stroke_color = parse_string(value),
            "weather_stroke_width" => weather_stroke_width = parse_u32(value),
            "news_pos_x" => news_pos_x = parse_i32(value),
            "news_pos_y" => news_pos_y = parse_i32(value),
            "news_widget_width" => news_widget_width = parse_u32(value),
            "news_widget_height" => news_widget_height = parse_u32(value),
            "weather_refresh_seconds" => weather_refresh_seconds = value.parse::<u64>().ok(),
            "weather_use_system_location" => weather_use_system_location = parse_bool(value),
            "weather_location_override" => weather_location_override = parse_string(value),
            "news_source" => news_source = parse_string(value),
            "news_custom_url" => news_custom_url = parse_string(value),
            "news_render_mode" => news_render_mode = parse_string(value),
            "news_fps" => news_fps = value.parse::<f32>().ok(),
            "news_refresh_seconds" => news_refresh_seconds = value.parse::<u64>().ok(),
            "news_audio_enabled" => news_audio_enabled = parse_bool(value),
            "show_news_ticker2" => show_news_ticker2 = parse_bool(value),
            "news_ticker2_pos_x" => news_ticker2_pos_x = parse_i32(value),
            "news_ticker2_pos_y" => news_ticker2_pos_y = parse_i32(value),
            "news_ticker2_width" => news_ticker2_width = parse_u32(value),
            "news_ticker2_source" => news_ticker2_source = parse_string(value),
            "news_ticker2_custom_url" => news_ticker2_custom_url = parse_string(value),
            "news_ticker2_fps" => news_ticker2_fps = value.parse::<f32>().ok(),
            "news_ticker2_refresh_seconds" => {
                news_ticker2_refresh_seconds = value.parse::<u64>().ok()
            }
            "cams_pos_x" => cams_pos_x = parse_i32(value),
            "cams_pos_y" => cams_pos_y = parse_i32(value),
            "cams_widget_width" => cams_widget_width = parse_u32(value),
            "cams_widget_height" => cams_widget_height = parse_u32(value),
            "cams_source" => cams_source = parse_string(value),
            "cams_render_mode" => cams_render_mode = parse_string(value),
            "cams_custom_urls" => cams_custom_urls = parse_string(value),
            "cams_refresh_seconds" => cams_refresh_seconds = value.parse::<u64>().ok(),
            "cams_fps" => cams_fps = value.parse::<f32>().ok(),
            "cams_count" => cams_count = parse_u32(value),
            "cams_columns" => cams_columns = parse_u32(value),
            "overlay_script_ticker_enabled" => overlay_script_ticker_enabled = parse_bool(value),
            "overlay_script_ticker_command" => overlay_script_ticker_command = parse_string(value),
            "overlay_script_ticker_refresh_seconds" => {
                overlay_script_ticker_refresh_seconds = value.parse::<u64>().ok()
            }
            "overlay_script_ticker_pos_x" => overlay_script_ticker_pos_x = parse_i32(value),
            "overlay_script_ticker_pos_y" => overlay_script_ticker_pos_y = parse_i32(value),
            "overlay_script_ticker_width" => overlay_script_ticker_width = parse_u32(value),
            "overlay_script_ticker_height" => overlay_script_ticker_height = parse_u32(value),
            "overlay_script_ticker_font_size" => overlay_script_ticker_font_size = parse_u32(value),
            "login_screen_integration" => login_screen_integration = parse_bool(value),
            "boot_screen_integration" => boot_screen_integration = parse_bool(value),
            _ => {}
        }
    }

    Ok(AppConfig {
        config_version: config_version.unwrap_or(1),
        image_dir: image_dir.ok_or_else(|| anyhow::anyhow!("missing key: image_dir"))?,
        quotes_path: quotes_path.ok_or_else(|| anyhow::anyhow!("missing key: quotes_path"))?,
        image_source: image_source.unwrap_or_else(|| "preset".to_string()),
        image_source_url: sanitize_optional_string(image_source_url),
        image_source_preset: sanitize_optional_string(image_source_preset)
            .or_else(|| Some("placecats_1920_1080".to_string())),
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
        output_image: normalize_output_image_path(output_image),
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
        show_background_layer: show_background_layer.unwrap_or(true),
        show_quote_layer: show_quote_layer.unwrap_or(true),
        show_clock_layer: show_clock_layer.unwrap_or(true),
        show_weather_layer: show_weather_layer.unwrap_or(false),
        show_news_layer: show_news_layer.unwrap_or(false),
        show_cams_layer: show_cams_layer.unwrap_or(false),
        layer_z_quote: layer_z_quote.unwrap_or(40).clamp(0, 100),
        layer_z_clock: layer_z_clock.unwrap_or(50).clamp(0, 100),
        layer_z_weather: layer_z_weather.unwrap_or(60).clamp(0, 100),
        layer_z_news: layer_z_news.unwrap_or(70).clamp(0, 100),
        layer_z_cams: layer_z_cams.unwrap_or(80).clamp(0, 100),
        weather_pos_x: weather_pos_x.unwrap_or(120),
        weather_pos_y: weather_pos_y.unwrap_or(120),
        weather_widget_width: weather_widget_width.unwrap_or(640).clamp(120, 1920),
        weather_widget_height: weather_widget_height.unwrap_or(180).clamp(80, 1080),
        weather_font_size: weather_font_size.unwrap_or(30).clamp(10, 200),
        weather_font_family: weather_font_family.unwrap_or_else(|| "DejaVu-Sans".to_string()),
        weather_color: weather_color.unwrap_or_else(|| "#00F5FF".to_string()),
        weather_undercolor: weather_undercolor.unwrap_or_else(|| "#0B0014B3".to_string()),
        weather_stroke_color: weather_stroke_color.unwrap_or_else(|| "#001A22".to_string()),
        weather_stroke_width: weather_stroke_width.unwrap_or(1).min(20),
        news_pos_x: news_pos_x.unwrap_or(980),
        news_pos_y: news_pos_y.unwrap_or(180),
        news_widget_width: news_widget_width.unwrap_or(760).clamp(180, 1920),
        news_widget_height: news_widget_height.unwrap_or(240).clamp(120, 1080),
        weather_refresh_seconds: weather_refresh_seconds.unwrap_or(600).max(60),
        weather_use_system_location: weather_use_system_location.unwrap_or(true),
        weather_location_override: weather_location_override.unwrap_or_default(),
        news_source: news_source.unwrap_or_else(|| "euronews".to_string()),
        news_custom_url: news_custom_url.unwrap_or_default(),
        news_render_mode: normalize_live_media_render_mode(news_render_mode),
        news_fps: news_fps.unwrap_or(1.0).clamp(0.05, 30.0),
        news_refresh_seconds: news_refresh_seconds.unwrap_or(90).max(10),
        news_audio_enabled: news_audio_enabled.unwrap_or(false),
        show_news_ticker2: show_news_ticker2.unwrap_or(false),
        news_ticker2_pos_x: news_ticker2_pos_x.unwrap_or(120),
        news_ticker2_pos_y: news_ticker2_pos_y.unwrap_or(980),
        news_ticker2_width: news_ticker2_width.unwrap_or(1280).clamp(220, 1920),
        news_ticker2_source: news_ticker2_source.unwrap_or_else(|| "google_world_en".to_string()),
        news_ticker2_custom_url: news_ticker2_custom_url.unwrap_or_default(),
        news_ticker2_fps: news_ticker2_fps.unwrap_or(1.2).clamp(0.05, 30.0),
        news_ticker2_refresh_seconds: news_ticker2_refresh_seconds.unwrap_or(120).max(10),
        cams_pos_x: cams_pos_x.unwrap_or(980),
        cams_pos_y: cams_pos_y.unwrap_or(640),
        cams_widget_width: cams_widget_width.unwrap_or(760).clamp(180, 1920),
        cams_widget_height: cams_widget_height.unwrap_or(428).clamp(120, 1080),
        cams_source: cams_source.unwrap_or_else(|| "auto_local".to_string()),
        cams_render_mode: normalize_live_media_render_mode(cams_render_mode),
        cams_custom_urls: cams_custom_urls.unwrap_or_default(),
        cams_refresh_seconds: cams_refresh_seconds.unwrap_or(75).max(10),
        cams_fps: cams_fps.unwrap_or(1.0).clamp(0.05, 30.0),
        cams_count: cams_count.unwrap_or(2).clamp(1, 9),
        cams_columns: cams_columns.unwrap_or(2).clamp(1, 4),
        overlay_script_ticker_enabled: overlay_script_ticker_enabled.unwrap_or(false),
        overlay_script_ticker_command: overlay_script_ticker_command.unwrap_or_default(),
        overlay_script_ticker_refresh_seconds: overlay_script_ticker_refresh_seconds
            .unwrap_or(30)
            .max(1),
        overlay_script_ticker_pos_x: overlay_script_ticker_pos_x.unwrap_or(120),
        overlay_script_ticker_pos_y: overlay_script_ticker_pos_y.unwrap_or(920),
        overlay_script_ticker_width: overlay_script_ticker_width.unwrap_or(1280).clamp(220, 1920),
        overlay_script_ticker_height: overlay_script_ticker_height.unwrap_or(56).clamp(32, 240),
        overlay_script_ticker_font_size: overlay_script_ticker_font_size
            .unwrap_or(30)
            .clamp(10, 120),
        login_screen_integration: login_screen_integration.unwrap_or(false),
        boot_screen_integration: boot_screen_integration.unwrap_or(false),
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

fn normalize_output_image_path(value: Option<String>) -> String {
    let default_path = "~/.local/state/wallpaper-composer/current.png";
    let legacy_tmp = "/tmp/wallpaper-composer-current.png";
    match value {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed == legacy_tmp {
                default_path.to_string()
            } else {
                trimmed.to_string()
            }
        }
        None => default_path.to_string(),
    }
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

fn normalize_live_media_render_mode(input: Option<String>) -> String {
    let _ = input;
    "overlay".to_string()
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

pub fn ensure_local_quotes_file(cfg: &mut AppConfig) -> Result<Option<PathBuf>> {
    if !cfg.quote_source.trim().eq_ignore_ascii_case("local") {
        return Ok(None);
    }

    let configured_path = expand_tilde(&cfg.quotes_path)?;
    if configured_path.is_file() {
        return Ok(None);
    }

    let seed = load_seed_local_quotes();
    if !configured_path.exists() && write_local_quotes_file(&configured_path, &seed).is_ok() {
        return Ok(Some(configured_path));
    }

    for candidate in packaged_local_quotes_paths() {
        if !candidate.is_file() {
            continue;
        }
        let candidate_raw = candidate.display().to_string();
        if cfg.quotes_path != candidate_raw {
            cfg.quotes_path = candidate_raw;
            return Ok(Some(candidate));
        }
        return Ok(None);
    }

    let fallback_path = expand_tilde(DEFAULT_LOCAL_QUOTES_PATH)?;
    write_local_quotes_file(&fallback_path, &seed)?;
    let fallback_raw = fallback_path.display().to_string();
    if cfg.quotes_path != fallback_raw {
        cfg.quotes_path = fallback_raw;
    }
    Ok(Some(fallback_path))
}

fn packaged_local_quotes_paths() -> Vec<PathBuf> {
    PACKAGED_LOCAL_QUOTES_PATHS
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>()
}

fn load_seed_local_quotes() -> String {
    for path in packaged_local_quotes_paths() {
        if let Ok(raw) = fs::read_to_string(&path)
            && !raw.trim().is_empty()
        {
            return raw;
        }
    }
    BUNDLED_LOCAL_QUOTES.to_string()
}

fn write_local_quotes_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
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
    recent_indices: &[usize],
) -> Result<(PathBuf, usize)> {
    let candidates = list_background_images(image_dir)?;
    if candidates.is_empty() {
        return Err(anyhow::anyhow!(
            "no images available in {}",
            image_dir.display()
        ));
    }

    let idx = pick_index_with_mode(candidates.len(), index, mode, avoid_repeat, recent_indices);

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
    recent_indices: &[usize],
) -> Result<(String, usize)> {
    let quotes = load_quotes(quotes_path)?;
    if quotes.is_empty() {
        return Err(anyhow::anyhow!(
            "no quotes available in {}",
            quotes_path.display()
        ));
    }
    let idx = pick_index_with_mode(quotes.len(), index, mode, avoid_repeat, recent_indices);
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
    recent_indices: &[usize],
) -> usize {
    let mut idx = match mode.trim().to_ascii_lowercase().as_str() {
        "random" => pseudo_random_index(index, len),
        _ => (index as usize) % len,
    };

    if avoid_repeat && len > 1 {
        let history_window = if mode.trim().eq_ignore_ascii_case("random") {
            // Use collection size to reduce quick repeats in random mode.
            (len / 3).clamp(3, 8).min(len.saturating_sub(1))
        } else {
            1
        };
        let blocked = recent_indices
            .iter()
            .copied()
            .take(history_window)
            .filter(|i| *i < len)
            .collect::<Vec<_>>();
        if blocked.len() < len && blocked.contains(&idx) {
            for step in 1..=len {
                let candidate = (idx + step) % len;
                if !blocked.contains(&candidate) {
                    idx = candidate;
                    break;
                }
            }
        }
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
            id: "placecats_1920_1080",
            display_label: "PlaceCats 1920x1080",
            name: "PlaceCats",
            endpoint: "https://placecats.com/1920/1080",
            category: "fun",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Direct 1920x1080 cat image endpoint requested as the default background preset.",
        },
        SourcePreset {
            id: "picsum_random_hd",
            display_label: "Picsum Random HD",
            name: "Picsum Random",
            endpoint: "https://picsum.photos/3840/2160.jpg",
            category: "photos",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Direct random photo endpoint that returns an image file via redirect.",
        },
        SourcePreset {
            id: "wallhaven_random_4k",
            display_label: "Wallhaven Random 4K",
            name: "Wallhaven Random",
            endpoint: "https://wallhaven.cc/api/v1/search?sorting=random&atleast=3840x2160&purity=100",
            category: "wallpapers",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Wallhaven random search API filtered to SFW and minimum 4K resolution.",
        },
        SourcePreset {
            id: "loremflickr_landscape_4k",
            display_label: "LoremFlickr Landscape 4K",
            name: "LoremFlickr Landscape",
            endpoint: "https://loremflickr.com/3840/2160/landscape,nature/all",
            category: "photos",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Random large landscape photo endpoint.",
        },
        SourcePreset {
            id: "pexels_curated_4k",
            display_label: "LoremFlickr City 4K",
            name: "LoremFlickr City",
            endpoint: "https://loremflickr.com/3840/2160/city,architecture/all",
            category: "photos",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Random city/architecture 4K-like endpoint.",
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
        SourcePreset {
            id: "dummyjson_quote",
            display_label: "DummyJSON Random Quote",
            name: "DummyJSON Quote",
            endpoint: "https://dummyjson.com/quotes/random",
            category: "quotes",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Simple random quote payload for integration testing and demos.",
        },
        SourcePreset {
            id: "advice_slip",
            display_label: "Advice Slip",
            name: "Advice Slip",
            endpoint: "https://api.adviceslip.com/advice",
            category: "advice",
            auth: "none",
            rate_limit: "provider-defined",
            notes: "Short advice text endpoint; useful as non-book quote source.",
        },
    ]
}

pub fn image_preset_endpoint(id: &str) -> Option<&'static str> {
    if let Some(endpoint) = preset_endpoint(&builtin_image_presets(), id) {
        return Some(endpoint);
    }
    // Backward-compatibility for older configs.
    match id {
        "nasa_apod" => {
            Some("https://wallhaven.cc/api/v1/search?sorting=random&atleast=3840x2160&purity=100")
        }
        "wikimedia_featured" => Some("https://loremflickr.com/3840/2160/landscape,nature/all"),
        "unsplash_nature_hd" => Some(
            "https://images.pexels.com/photos/1103970/pexels-photo-1103970.jpeg?auto=compress&cs=tinysrgb&h=2160&w=3840",
        ),
        _ => None,
    }
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
    let news = builtin_news_sources()
        .iter()
        .copied()
        .map(news_preset_to_json)
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "{{\n  \"catalog_version\": 1,\n  \"image_presets\": [\n{}\n  ],\n  \"quote_presets\": [\n{}\n  ],\n  \"news_presets\": [\n{}\n  ]\n}}",
        indent_block(&images, 4),
        indent_block(&quotes, 4),
        indent_block(&news, 4)
    )
}

fn preset_to_json(p: SourcePreset) -> String {
    format!(
        "{{\"id\":{:?},\"display_label\":{:?},\"name\":{:?},\"endpoint\":{:?},\"category\":{:?},\"auth\":{:?},\"rate_limit\":{:?},\"notes\":{:?}}}",
        p.id, p.display_label, p.name, p.endpoint, p.category, p.auth, p.rate_limit, p.notes
    )
}

fn news_preset_to_json(p: NewsSourcePreset) -> String {
    format!(
        "{{\"id\":{:?},\"display_label\":{:?},\"name\":{:?},\"stream_url\":{:?},\"ticker_url\":{:?},\"is_live_video\":{},\"notes\":{:?}}}",
        p.id, p.display_label, p.name, p.stream_url, p.ticker_url, p.is_live_video, p.notes
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
        build_doctor_report, builtin_image_presets, builtin_news_sources, builtin_quote_presets,
        default_config_toml, ensure_local_quotes_file, expand_tilde, load_config, load_quotes,
        parse_bool, parse_i32, parse_u32, presets_catalog_json, sanitize_optional_string,
        settings_schema_json, settings_ui_blueprint_json, to_config_toml,
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
        assert_eq!(cfg.image_source, "preset");
        assert_eq!(
            cfg.image_source_preset.as_deref(),
            Some("placecats_1920_1080")
        );
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
        assert_eq!(cfg.news_render_mode, "overlay");
        assert_eq!(cfg.cams_render_mode, "overlay");
        assert!(!cfg.overlay_script_ticker_enabled);
        assert_eq!(cfg.overlay_script_ticker_refresh_seconds, 30);
        assert_eq!(cfg.overlay_script_ticker_width, 1280);
        assert_eq!(cfg.overlay_script_ticker_height, 56);
        assert_eq!(cfg.overlay_script_ticker_font_size, 30);
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
    fn ensure_local_quotes_file_creates_missing_path() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!("wc-core-quotes-recover-{nonce}"));
        let cfg_path = temp_root.join("config.toml");
        fs::create_dir_all(&temp_root).expect("temp root should be creatable");
        fs::write(&cfg_path, default_config_toml()).expect("config should be writable");

        let mut cfg = load_config(&cfg_path).expect("config should parse");
        let broken_quotes_path = temp_root.join("missing/sub/local-quotes.md");
        cfg.quotes_path = broken_quotes_path.display().to_string();
        cfg.quote_source = "local".to_string();

        let recovered = ensure_local_quotes_file(&mut cfg).expect("recovery should work");
        assert!(recovered.is_some());
        assert!(broken_quotes_path.is_file());
        assert!(
            !load_quotes(&broken_quotes_path)
                .expect("recovered quotes should parse")
                .is_empty()
        );

        let _ = fs::remove_dir_all(temp_root);
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
        assert!(!builtin_news_sources().is_empty());
        assert!(
            builtin_image_presets()
                .iter()
                .any(|p| p.id == "picsum_random_hd")
        );
        assert!(
            builtin_image_presets()
                .iter()
                .any(|p| p.id == "placecats_1920_1080")
        );
        assert!(
            builtin_news_sources()
                .iter()
                .any(|p| p.id == "google_world_en")
        );
    }

    #[test]
    fn preset_endpoint_lookup_works() {
        assert!(super::image_preset_endpoint("placecats_1920_1080").is_some());
        assert!(super::image_preset_endpoint("picsum_random_hd").is_some());
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
        assert!(schema.contains("\"news_render_mode\""));
        assert!(schema.contains("\"cams_render_mode\""));
        assert!(schema.contains("\"overlay_script_ticker_command\""));
    }

    #[test]
    fn ui_blueprint_contains_sections_and_conditions() {
        let blueprint = settings_ui_blueprint_json();
        assert!(blueprint.contains("\"sections\""));
        assert!(blueprint.contains("\"conditions\""));
        assert!(blueprint.contains("\"wallpaper_backend\""));
        assert!(blueprint.contains("\"wallpaper_fit_mode\""));
        assert!(blueprint.contains("\"news_render_mode\""));
        assert!(blueprint.contains("\"cams_render_mode\""));
        assert!(blueprint.contains("\"overlay_script_ticker_enabled\""));
    }

    #[test]
    fn presets_catalog_contains_expected_keys() {
        let json = presets_catalog_json();
        assert!(json.contains("\"image_presets\""));
        assert!(json.contains("\"quote_presets\""));
        assert!(json.contains("\"news_presets\""));
        assert!(json.contains("\"display_label\""));
        assert!(json.contains("\"rate_limit\""));
        assert!(json.contains("\"stream_url\""));
    }

    #[test]
    fn pick_index_avoids_last_three_when_enabled() {
        let idx = super::pick_index_with_mode(12, 0, "random", true, &[0, 1, 2, 3]);
        assert!(![0, 1, 2, 3].contains(&idx));
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
