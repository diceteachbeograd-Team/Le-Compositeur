use eframe::egui;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};
use wc_core::{
    AppConfig, builtin_image_presets, builtin_quote_presets, default_config_path,
    ensure_local_quotes_file, expand_tilde, list_background_images, load_config, load_quotes,
    to_config_toml,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const ORDERING_WORLD_WIDTH: i32 = 1920;
const ORDERING_WORLD_HEIGHT: i32 = 1080;
const ORDERING_GRID_STEP: i32 = 24;
const ORDERING_COLLISION_ITERS: usize = 32;

fn app_version_label() -> &'static str {
    static LABEL: OnceLock<String> = OnceLock::new();
    LABEL
        .get_or_init(|| {
            if let Some(pkg) = installed_package_version() {
                format!("Version: v{APP_VERSION} (pkg {pkg})")
            } else {
                format!("Version: v{APP_VERSION}")
            }
        })
        .as_str()
}

#[cfg(target_os = "linux")]
fn installed_package_version() -> Option<String> {
    if let Ok(output) = Command::new("rpm")
        .args(["-q", "--qf", "%{VERSION}-%{RELEASE}", "le-compositeur"])
        .output()
        && output.status.success()
        && let Ok(text) = String::from_utf8(output.stdout)
    {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(output) = Command::new("dpkg-query")
        .args(["-W", "-f=${Version}", "le-compositeur"])
        .output()
        && output.status.success()
        && let Ok(text) = String::from_utf8(output.stdout)
    {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn installed_package_version() -> Option<String> {
    None
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default().with_app_id("le-compositeur");
    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Le Compositeur Settings (diceteach)",
        native_options,
        Box::new(|_cc| Ok(Box::new(WcGuiApp::new()))),
    )
}

fn load_app_icon() -> Option<egui::IconData> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(p) = std::env::var("WC_GUI_ICON") {
        candidates.push(PathBuf::from(p));
    }
    candidates.push(PathBuf::from("assets/icons/le-compositeur.png"));
    candidates.push(PathBuf::from(
        "/usr/share/icons/hicolor/512x512/apps/le-compositeur.png",
    ));

    for path in candidates {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(img) = image::load_from_memory(&bytes) else {
            continue;
        };
        let rgba = img.into_rgba8();
        let width = rgba.width();
        let height = rgba.height();
        return Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        });
    }
    None
}

struct ThumbnailItem {
    label: String,
    texture: egui::TextureHandle,
}

#[derive(Debug, Clone)]
struct ReleaseInfo {
    tag: String,
    html_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiTab {
    Ordering,
    Images,
    Quotes,
    Weather,
    News,
    Cams,
    System,
}

#[derive(Debug, Clone, Copy)]
enum UiLang {
    En,
    De,
    Sr,
    Zh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutElement {
    Quote,
    Clock,
    Weather,
    News,
    Cams,
}

#[derive(Clone, Copy)]
struct WorldRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl WorldRect {
    fn overlaps(self, other: Self) -> bool {
        let self_r = self.x.saturating_add(self.w);
        let self_b = self.y.saturating_add(self.h);
        let other_r = other.x.saturating_add(other.w);
        let other_b = other.y.saturating_add(other.h);
        self.x < other_r && self_r > other.x && self.y < other_b && self_b > other.y
    }
}

struct WcGuiApp {
    config_path: String,
    cfg: AppConfig,
    status: String,
    thumbnails: Vec<ThumbnailItem>,
    thumbnails_for_dir: String,
    quote_preview: Vec<String>,
    runner: Option<Child>,
    active_tab: GuiTab,
    ui_lang: UiLang,
    selected_element: LayoutElement,
    weather_status: String,
    weather_details: Vec<String>,
    weather_last_refresh: Option<Instant>,
    autostart_toggle: bool,
    ordering_bg_texture: Option<egui::TextureHandle>,
    cams_public_choice: String,
    update_status: String,
    update_release: Option<ReleaseInfo>,
    update_check_rx: Option<Receiver<Result<Option<ReleaseInfo>, String>>>,
    self_update_rx: Option<Receiver<Result<String, String>>>,
    ui_compact_mode: bool,
    show_preview_panel: bool,
    ui_style_compact_applied: Option<bool>,
}

impl WcGuiApp {
    fn new() -> Self {
        let config_path = default_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.config/wallpaper-composer/config.toml".to_string());

        let (cfg, loaded_from_disk) = match load_config(PathBuf::from(&config_path).as_path()) {
            Ok(cfg) => (cfg, true),
            Err(_) => (default_cfg(), false),
        };

        let mut app = Self {
            config_path,
            cfg,
            status: "Ready".to_string(),
            thumbnails: Vec::new(),
            thumbnails_for_dir: String::new(),
            quote_preview: Vec::new(),
            runner: None,
            active_tab: GuiTab::Ordering,
            ui_lang: detect_ui_lang(),
            selected_element: LayoutElement::Quote,
            weather_status: "No weather data yet".to_string(),
            weather_details: Vec::new(),
            weather_last_refresh: None,
            autostart_toggle: Self::autostart_enabled(),
            ordering_bg_texture: None,
            cams_public_choice: "belgrade_center".to_string(),
            update_status: "Update check: pending".to_string(),
            update_release: None,
            update_check_rx: None,
            self_update_rx: None,
            ui_compact_mode: true,
            show_preview_panel: false,
            ui_style_compact_applied: None,
        };
        if let Some(msg) = app.recover_local_quotes(loaded_from_disk) {
            app.status = msg;
        }
        app.start_update_check();
        app
    }

    fn t<'a>(&self, en: &'a str, de: &'a str, sr: &'a str, zh: &'a str) -> &'a str {
        match self.ui_lang {
            UiLang::En => en,
            UiLang::De => de,
            UiLang::Sr => sr,
            UiLang::Zh => zh,
        }
    }

    fn active_tab_title(&self) -> &'static str {
        match self.active_tab {
            GuiTab::Ordering => "Layout Ordering",
            GuiTab::Images => "Background Images",
            GuiTab::Quotes => "Quote Source & Style",
            GuiTab::Weather => "Weather Widget",
            GuiTab::News => "News & Ticker Widgets",
            GuiTab::Cams => "Cams Widget",
            GuiTab::System => "Runtime & Integrations",
        }
    }

    fn active_tab_hint(&self) -> &'static str {
        match self.active_tab {
            GuiTab::Ordering => {
                "Use drag, snap, and layer Z to place widgets without overlap in the 16:9 workspace."
            }
            GuiTab::Images => {
                "Configure source mode, ordering policy, and wallpaper backend behavior."
            }
            GuiTab::Quotes => {
                "Select quote provider, ordering mode, and typography/styling controls."
            }
            GuiTab::Weather => {
                "Configure weather source behavior, refresh budget, placement and visual style."
            }
            GuiTab::News => {
                "Configure primary stream plus secondary ticker with independent source and caps."
            }
            GuiTab::Cams => {
                "Configure camera source mode, grid layout, and network/CPU refresh limits."
            }
            GuiTab::System => {
                "Manage runtime controls, updates, autostart, and desktop integration toggles."
            }
        }
    }

    fn tab_button_label(tab: GuiTab) -> &'static str {
        match tab {
            GuiTab::Ordering => "LAY Ordering",
            GuiTab::Images => "IMG Images",
            GuiTab::Quotes => "QTE Quotes",
            GuiTab::Weather => "WTH Weather",
            GuiTab::News => "NWS News",
            GuiTab::Cams => "CAM Cams",
            GuiTab::System => "SYS System",
        }
    }

    fn enforce_news_widget_size_preset(&mut self) {
        let (w, h) =
            nearest_news_size_preset(self.cfg.news_widget_width, self.cfg.news_widget_height);
        self.cfg.news_widget_width = w;
        self.cfg.news_widget_height = h;
    }

    fn layout_element_label(element: LayoutElement) -> &'static str {
        match element {
            LayoutElement::Quote => "Quote Box",
            LayoutElement::Clock => "Clock",
            LayoutElement::Weather => "Weather",
            LayoutElement::News => "News",
            LayoutElement::Cams => "Cams",
        }
    }

    fn layout_element_z(&self, element: LayoutElement) -> i32 {
        match element {
            LayoutElement::Quote => self.cfg.layer_z_quote,
            LayoutElement::Clock => self.cfg.layer_z_clock,
            LayoutElement::Weather => self.cfg.layer_z_weather,
            LayoutElement::News => self.cfg.layer_z_news,
            LayoutElement::Cams => self.cfg.layer_z_cams,
        }
    }

    fn layout_element_z_mut(&mut self, element: LayoutElement) -> &mut i32 {
        match element {
            LayoutElement::Quote => &mut self.cfg.layer_z_quote,
            LayoutElement::Clock => &mut self.cfg.layer_z_clock,
            LayoutElement::Weather => &mut self.cfg.layer_z_weather,
            LayoutElement::News => &mut self.cfg.layer_z_news,
            LayoutElement::Cams => &mut self.cfg.layer_z_cams,
        }
    }

    fn normalize_layout_z(&mut self) {
        let mut layers = [
            (LayoutElement::Quote, self.cfg.layer_z_quote),
            (LayoutElement::Clock, self.cfg.layer_z_clock),
            (LayoutElement::Weather, self.cfg.layer_z_weather),
            (LayoutElement::News, self.cfg.layer_z_news),
            (LayoutElement::Cams, self.cfg.layer_z_cams),
        ];
        layers.sort_by_key(|(_, z)| *z);
        for (idx, (element, _)) in layers.iter().enumerate() {
            *self.layout_element_z_mut(*element) = 10 * (idx as i32 + 1);
        }
    }

    fn draw_ordering_element(
        &self,
        painter: &egui::Painter,
        element: LayoutElement,
        rect: egui::Rect,
    ) {
        let selected = self.selected_element == element;
        let (active, idle) = match element {
            LayoutElement::Quote => (
                egui::Color32::from_rgb(255, 0, 190),
                egui::Color32::from_rgb(158, 46, 133),
            ),
            LayoutElement::Clock => (
                egui::Color32::from_rgb(0, 255, 255),
                egui::Color32::from_rgb(54, 150, 150),
            ),
            LayoutElement::Weather => (
                egui::Color32::from_rgb(128, 255, 0),
                egui::Color32::from_rgb(74, 140, 48),
            ),
            LayoutElement::News => (
                egui::Color32::from_rgb(255, 120, 0),
                egui::Color32::from_rgb(166, 91, 28),
            ),
            LayoutElement::Cams => (
                egui::Color32::from_rgb(0, 255, 140),
                egui::Color32::from_rgb(35, 140, 92),
            ),
        };
        let neon = if selected { active } else { idle };
        painter.rect_filled(rect, 4.0, neon.linear_multiply(0.16));
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(2.0, neon),
            egui::StrokeKind::Middle,
        );
        painter.text(
            rect.left_top() + egui::vec2(6.0, 6.0),
            egui::Align2::LEFT_TOP,
            Self::layout_element_label(element),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }

    fn layout_elements_in_z_order(&self) -> Vec<LayoutElement> {
        let mut ordered = vec![
            LayoutElement::Quote,
            LayoutElement::Clock,
            LayoutElement::Weather,
            LayoutElement::News,
            LayoutElement::Cams,
        ];
        ordered.sort_by_key(|element| self.layout_element_z(*element));
        ordered
    }

    fn layout_element_enabled(&self, element: LayoutElement) -> bool {
        match element {
            LayoutElement::Quote => self.cfg.show_quote_layer,
            LayoutElement::Clock => self.cfg.show_clock_layer,
            LayoutElement::Weather => self.cfg.show_weather_layer,
            LayoutElement::News => self.cfg.show_news_layer,
            LayoutElement::Cams => self.cfg.show_cams_layer,
        }
    }

    fn layout_element_world_rect(&self, element: LayoutElement) -> WorldRect {
        match element {
            LayoutElement::Quote => {
                let (w, h) = quote_box_world_size(
                    self.cfg.text_box_size.as_str(),
                    self.cfg.text_box_width_pct,
                    self.cfg.text_box_height_pct,
                );
                let (x, y) = clamp_world_pos(self.cfg.quote_pos_x, self.cfg.quote_pos_y, w, h);
                WorldRect { x, y, w, h }
            }
            LayoutElement::Clock => {
                let (w, h) = (180, 64);
                let (x, y) = clamp_world_pos(self.cfg.clock_pos_x, self.cfg.clock_pos_y, w, h);
                WorldRect { x, y, w, h }
            }
            LayoutElement::Weather => {
                let w = self.cfg.weather_widget_width.clamp(120, 1920) as i32;
                let h = self.cfg.weather_widget_height.clamp(80, 1080) as i32;
                let (x, y) = clamp_world_pos(self.cfg.weather_pos_x, self.cfg.weather_pos_y, w, h);
                WorldRect { x, y, w, h }
            }
            LayoutElement::News => {
                let w = self.cfg.news_widget_width.clamp(180, 1920) as i32;
                let h = self.cfg.news_widget_height.clamp(120, 1080) as i32;
                let (x, y) = clamp_world_pos(self.cfg.news_pos_x, self.cfg.news_pos_y, w, h);
                WorldRect { x, y, w, h }
            }
            LayoutElement::Cams => {
                let w = self.cfg.cams_widget_width.clamp(180, 1920) as i32;
                let h = self.cfg.cams_widget_height.clamp(120, 1080) as i32;
                let (x, y) = clamp_world_pos(self.cfg.cams_pos_x, self.cfg.cams_pos_y, w, h);
                WorldRect { x, y, w, h }
            }
        }
    }

    fn set_layout_world_pos(&mut self, element: LayoutElement, x: i32, y: i32) {
        match element {
            LayoutElement::Quote => {
                self.cfg.quote_pos_x = x;
                self.cfg.quote_pos_y = y;
            }
            LayoutElement::Clock => {
                self.cfg.clock_pos_x = x;
                self.cfg.clock_pos_y = y;
            }
            LayoutElement::Weather => {
                self.cfg.weather_pos_x = x;
                self.cfg.weather_pos_y = y;
            }
            LayoutElement::News => {
                self.cfg.news_pos_x = x;
                self.cfg.news_pos_y = y;
            }
            LayoutElement::Cams => {
                self.cfg.cams_pos_x = x;
                self.cfg.cams_pos_y = y;
            }
        }
    }

    fn resolve_selected_collision(&mut self, selected: LayoutElement) {
        if !self.layout_element_enabled(selected) {
            return;
        }
        for _ in 0..ORDERING_COLLISION_ITERS {
            let selected_rect = self.layout_element_world_rect(selected);
            let mut overlapping = None;
            for element in self.layout_elements_in_z_order() {
                if element == selected || !self.layout_element_enabled(element) {
                    continue;
                }
                let other = self.layout_element_world_rect(element);
                if selected_rect.overlaps(other) {
                    overlapping = Some(other);
                    break;
                }
            }
            let Some(other) = overlapping else {
                break;
            };

            let candidates = [
                (other.x.saturating_sub(selected_rect.w), selected_rect.y),
                (other.x.saturating_add(other.w), selected_rect.y),
                (selected_rect.x, other.y.saturating_sub(selected_rect.h)),
                (selected_rect.x, other.y.saturating_add(other.h)),
            ];
            let mut best = None::<(i32, i32, i32)>;
            for (cx, cy) in candidates {
                let snapped_x = snap_to_grid(cx);
                let snapped_y = snap_to_grid(cy);
                let (nx, ny) =
                    clamp_world_pos(snapped_x, snapped_y, selected_rect.w, selected_rect.h);
                let candidate = WorldRect {
                    x: nx,
                    y: ny,
                    w: selected_rect.w,
                    h: selected_rect.h,
                };
                if candidate.overlaps(other) {
                    continue;
                }
                let dist = (nx - selected_rect.x).abs() + (ny - selected_rect.y).abs();
                match best {
                    None => best = Some((dist, nx, ny)),
                    Some((best_dist, _, _)) if dist < best_dist => best = Some((dist, nx, ny)),
                    _ => {}
                }
            }

            let Some((_, nx, ny)) = best else {
                break;
            };
            self.set_layout_world_pos(selected, nx, ny);
        }
    }

    fn hover_text(&self, key: &str) -> &str {
        match key {
            "config_path" => self.t(
                "What: config.toml path used by all actions. How: load/save here before running actions. Recommended: keep the default user config path.",
                "Was: Pfad zur config.toml für alle Aktionen. Wie: hier laden/speichern, bevor Aktionen laufen. Empfehlung: Standardpfad im Benutzerprofil beibehalten.",
                "Sta: putanja config.toml koju koriste sve akcije. Kako: ovde učitaj/snimi pre pokretanja akcija. Preporuka: zadrži podrazumevanu korisničku putanju.",
                "作用: 所有操作使用的 config.toml 路径。方法: 先在这里加载/保存再执行操作。建议: 保持默认用户配置路径。"
            ),
            "image_source_mode" => self.t(
                "What: choose where images come from. How: local folder, built-in online preset, or custom URL. Recommended: start with local for stability.",
                "Was: Quelle der Bilder festlegen. Wie: lokaler Ordner, Online-Preset oder eigene URL. Empfehlung: für Stabilität zuerst lokal nutzen.",
                "Sta: bira odakle dolaze slike. Kako: lokalni folder, online preset ili prilagođeni URL. Preporuka: za stabilnost kreni sa lokalnim izvorom.",
                "作用: 选择图片来源。方法: 本地文件夹、内置在线预设或自定义 URL。建议: 先用本地来源更稳定。"
            ),
            "image_preset" => self.t(
                "What: online image source preset. How: pick one and test with Run Once. Recommended: start with Picsum; then try Wallhaven/LoremFlickr for variety.",
                "Was: Online-Bildquellen-Preset. Wie: auswählen und mit Run Once testen. Empfehlung: mit Picsum starten, danach Wallhaven/LoremFlickr für Vielfalt.",
                "Sta: preset za online slike. Kako: izaberi i testiraj sa Run Once. Preporuka: prvo Picsum, pa Wallhaven/LoremFlickr za raznolikost.",
                "作用: 在线图片预设来源。方法: 选中后用 Run Once 测试。建议: 先用 Picsum，再试 Wallhaven/LoremFlickr。"
            ),
            "image_url" => self.t(
                "What: custom image URL list. How: add multiple URLs (one per line, or separated by ';'). Requirements: endpoint must return a real image (jpg/png/webp/bmp), preferably >=1920x1080, reachable within ~8s. Recommended: keep 3-10 stable sources.",
                "Was: eigene Bild-URL-Liste. Wie: mehrere URLs eintragen (eine pro Zeile oder mit ';' trennen). Anforderungen: Endpoint muss ein echtes Bild liefern (jpg/png/webp/bmp), ideal >=1920x1080, erreichbar in ~8s. Empfehlung: 3-10 stabile Quellen verwenden.",
                "Sta: lista prilagođenih URL-ova za slike. Kako: unesi više URL-ova (jedan po redu ili odvojeno sa ';'). Uslovi: endpoint mora da vrati pravu sliku (jpg/png/webp/bmp), poželjno >=1920x1080, dostupan za ~8s. Preporuka: koristi 3-10 stabilnih izvora.",
                "作用: 自定义图片 URL 列表。方法: 可填多个 URL（每行一个，或用 ';' 分隔）。要求: 必须直接返回图片（jpg/png/webp/bmp），建议 >=1920x1080，约 8 秒内可访问。建议: 保持 3-10 个稳定来源。"
            ),
            "image_dir" => self.t(
                "What: local wallpaper folder. How: choose a folder with jpg/png/webp/bmp files. Recommended: dedicated folder with curated images.",
                "Was: lokaler Wallpaper-Ordner. Wie: Ordner mit jpg/png/webp/bmp auswählen. Empfehlung: separaten, kuratierten Ordner verwenden.",
                "Sta: lokalni folder sa pozadinama. Kako: izaberi folder sa jpg/png/webp/bmp fajlovima. Preporuka: poseban folder sa proverenim slikama.",
                "作用: 本地壁纸文件夹。方法: 选择含 jpg/png/webp/bmp 的目录。建议: 使用专门整理好的文件夹。"
            ),
            "quotes_source_mode" => self.t(
                "What: choose where quote text comes from. How: local file, built-in API preset, or custom URL. Recommended: local file for predictable quality.",
                "Was: Quelle der Zitate festlegen. Wie: lokale Datei, API-Preset oder eigene URL. Empfehlung: lokale Datei für konstante Qualität.",
                "Sta: bira izvor citata. Kako: lokalni fajl, API preset ili URL. Preporuka: lokalni fajl za stabilan kvalitet.",
                "作用: 选择引文文本来源。方法: 本地文件、内置 API 预设或自定义 URL。建议: 本地文件更可控。"
            ),
            "quote_preset" => self.t(
                "What: online quote provider. How: pick a provider and run preview/once. Recommended: ZenQuotes or DummyJSON for testing.",
                "Was: Online-Zitatquelle. Wie: Provider wählen und Vorschau/Einmallauf starten. Empfehlung: ZenQuotes oder DummyJSON zum Testen.",
                "Sta: online provider citata. Kako: izaberi provider i pokreni pregled/jednom. Preporuka: ZenQuotes ili DummyJSON za test.",
                "作用: 在线引文提供方。方法: 选择后运行预览或单次执行。建议: 测试优先 ZenQuotes 或 DummyJSON。"
            ),
            "quote_url" => self.t(
                "What: custom quote URL list. How: add multiple endpoints (one per line, or separated by ';'). Requirements: each endpoint should return usable text/JSON quickly and consistently. Recommended: keep a small set of reliable APIs.",
                "Was: eigene Zitat-URL-Liste. Wie: mehrere Endpoints eintragen (eine pro Zeile oder mit ';' trennen). Anforderungen: jeder Endpoint sollte schnell und zuverlässig nutzbaren Text/JSON liefern. Empfehlung: kleine, stabile API-Liste verwenden.",
                "Sta: lista prilagođenih URL-ova za citate. Kako: unesi više endpointa (jedan po redu ili odvojeno sa ';'). Uslovi: svaki endpoint treba brzo i pouzdano da vrati upotrebljiv tekst/JSON. Preporuka: drži malu listu stabilnih API-ja.",
                "作用: 自定义语录 URL 列表。方法: 可填多个接口（每行一个，或用 ';' 分隔）。要求: 每个接口都应稳定、快速返回可用文本/JSON。建议: 保持少量可靠 API。"
            ),
            "quotes_path" => self.t(
                "What: local quote file path. How: choose .txt or .md file, then reload quotes preview. Recommended: keep short clean entries.",
                "Was: Pfad zur lokalen Zitatdatei. Wie: .txt/.md wählen und Vorschau neu laden. Empfehlung: kurze, saubere Einträge pflegen.",
                "Sta: putanja lokalnog fajla sa citatima. Kako: izaberi .txt/.md pa osveži prikaz citata. Preporuka: drži unose kratkim i čistim.",
                "作用: 本地引文文件路径。方法: 选择 .txt/.md 后刷新预览。建议: 保持条目简短清晰。"
            ),
            "image_refresh" => self.t(
                "What: image rotation interval in seconds. How: set the value used for image cycle index. Recommended: 300-900 seconds.",
                "Was: Intervall für Bildwechsel in Sekunden. Wie: steuert den Bild-Zyklusindex. Empfehlung: 300-900 Sekunden.",
                "Sta: interval promene slike u sekundama. Kako: određuje ciklus slika. Preporuka: 300-900 sekundi.",
                "作用: 图片轮换间隔（秒）。方法: 控制图片周期索引。建议: 300-900 秒。"
            ),
            "quote_refresh" => self.t(
                "What: quote rotation interval in seconds. How: set independent cycle interval for text. Recommended: 60-300 seconds.",
                "Was: Intervall für Zitatwechsel in Sekunden. Wie: unabhängiger Text-Zyklus. Empfehlung: 60-300 Sekunden.",
                "Sta: interval promene citata u sekundama. Kako: nezavisan ciklus za tekst. Preporuka: 60-300 sekundi.",
                "作用: 引文轮换间隔（秒）。方法: 设置文本独立周期。建议: 60-300 秒。"
            ),
            "runner_tick" => self.t(
                "What: loop wake-up interval. How: app wakes every min(refresh_seconds, 60). Recommended: keep >=60 to reduce overhead; clock still updates once per minute.",
                "Was: Intervall des Hauptloops. Wie: Aufwachen alle min(refresh_seconds, 60). Empfehlung: >=60 für weniger Last; Uhr wird trotzdem minütlich aktualisiert.",
                "Sta: interval glavne petlje. Kako: budi se na min(refresh_seconds, 60). Preporuka: >=60 zbog manjeg opterećenja; sat se i dalje osvežava na minut.",
                "作用: 主循环唤醒间隔。方法: 每 min(refresh_seconds, 60) 触发一次。建议: 设为 >=60 降低开销；时钟仍每分钟更新。"
            ),
            "apply_wallpaper" => self.t(
                "What: apply rendered output as current wallpaper. How: enable and choose backend/fit mode. Recommended: test with Run Once before Start Loop.",
                "Was: gerendertes Bild als Wallpaper anwenden. Wie: aktivieren und Backend/Fit wählen. Empfehlung: zuerst mit Run Once testen.",
                "Sta: postavlja renderovanu sliku kao pozadinu. Kako: uključi i izaberi backend/fit. Preporuka: prvo testiraj sa Run Once.",
                "作用: 将渲染结果设置为当前壁纸。方法: 启用后选择后端和适配模式。建议: 先用 Run Once 测试。"
            ),
            "autostart_enable" => self.t(
                "What: starts wallpaper loop automatically after login. How: writes/removes a desktop autostart entry with a startup delay. Recommended: enable for daily usage.",
                "Was: startet die Wallpaper-Schleife automatisch nach dem Login. Wie: erstellt/entfernt einen Desktop-Autostart mit Startverzögerung. Empfehlung: für täglichen Einsatz aktivieren.",
                "Sta: automatski pokreće wallpaper petlju posle prijave. Kako: upisuje/uklanja autostart unos sa kašnjenjem pri pokretanju. Preporuka: uključi za svakodnevnu upotrebu.",
                "作用: 登录后自动启动壁纸循环。方法: 写入/移除带延迟的自动启动项。建议: 日常使用可开启。"
            ),
            "weather_refresh_seconds" => self.t(
                "What: weather update interval. How: app refreshes weather in this period when Weather layer is enabled. Recommended: 600 seconds.",
                "Was: Aktualisierungsintervall für Wetter. Wie: bei aktivem Wetter-Layer werden Daten in diesem Abstand neu geholt. Empfehlung: 600 Sekunden.",
                "Sta: interval osvežavanja vremena. Kako: kada je Weather sloj uključen, podaci se obnavljaju u ovom periodu. Preporuka: 600 sekundi.",
                "作用: 天气刷新间隔。方法: 启用天气图层后按该周期更新。建议: 600 秒。"
            ),
            "weather_use_system_location" => self.t(
                "What: use auto-detected location for weather. How: resolves location via network geolocation. Recommended: keep enabled unless detection is wrong.",
                "Was: automatisch erkannten Standort für Wetter nutzen. Wie: Standort wird per Netzwerk-Geolokalisierung bestimmt. Empfehlung: aktiviert lassen, außer die Erkennung ist falsch.",
                "Sta: koristi automatski otkrivenu lokaciju za vreme. Kako: lokacija se određuje mrežnom geolokacijom. Preporuka: ostavi uključeno osim ako detekcija greši.",
                "作用: 使用自动定位获取天气。方法: 通过网络地理定位解析位置。建议: 除非定位错误，否则保持开启。"
            ),
            "weather_location_override" => self.t(
                "What: manual weather location override. How: set city or city,country (example: Belgrade,RS). Recommended: use only if auto-location is inaccurate.",
                "Was: manuelle Wetter-Location. Wie: Stadt oder Stadt,Ländercode eintragen (z. B. Belgrade,RS). Empfehlung: nur nutzen, wenn Auto-Lokalisierung ungenau ist.",
                "Sta: ručno zadavanje lokacije za vreme. Kako: unesi grad ili grad,država (npr. Belgrade,RS). Preporuka: koristi samo ako auto-lokacija nije tačna.",
                "作用: 手动天气位置覆盖。方法: 填写城市或城市,国家代码（如 Belgrade,RS）。建议: 仅在自动定位不准时使用。"
            ),
            "news_source" => self.t(
                "What: news/documentary stream source used by the News widget. How: choose one from the list or custom URL. Recommended: start with Euronews.",
                "Was: Quelle für News/Doku-Stream im News-Widget. Wie: aus Liste wählen oder eigene URL nutzen. Empfehlung: mit Euronews starten.",
                "Sta: izvor vesti/dokumentarnog strima za News widget. Kako: izaberi iz liste ili koristi sopstveni URL. Preporuka: kreni sa Euronews.",
                "作用: News 小组件的视频来源。方法: 从列表选择或使用自定义 URL。建议: 先用 Euronews。"
            ),
            "news_custom_url" => self.t(
                "What: custom stream/snapshot URL for News widget. How: use direct image URL, YouTube link, or camera stream URL. Recommended: camera/video streams are auto-kept smooth (minimum 15 FPS).",
                "Was: eigene Stream-/Snapshot-URL für das News-Widget. Wie: direkte Bild-URL, YouTube-Link oder Kamera-Stream-URL nutzen. Empfehlung: Kamera-/Videostreams werden automatisch flüssig gehalten (mindestens 15 FPS).",
                "Sta: prilagođeni stream/snapshot URL za News widget. Kako: koristi direktan URL slike, YouTube link ili URL kamere. Preporuka: video/kamera strim se automatski drži glatkim (minimum 15 FPS).",
                "作用: News 小组件的自定义流/快照 URL。方法: 可用图片直链、YouTube 链接或摄像头流 URL。建议: 摄像头/视频流会自动保持流畅（至少 15 FPS）。"
            ),
            "news_fps" => self.t(
                "What: playback frame rate target for stream widget. How: choose 0.05-30 FPS. Recommended: 15-24 FPS for smooth motion; runtime enforces a minimum 15 FPS for video/camera streams.",
                "Was: Ziel-Bildrate für das Stream-Widget. Wie: zwischen 0,05 und 30 FPS wählen. Empfehlung: 15-24 FPS für flüssige Bewegung; Laufzeit erzwingt mindestens 15 FPS für Video-/Kamerastreams.",
                "Sta: ciljna brzina kadrova za stream widget. Kako: izaberi 0,05-30 FPS. Preporuka: 15-24 FPS za glatko kretanje; runtime nameće minimum 15 FPS za video/kamera stream.",
                "作用: 流媒体组件目标帧率。方法: 设置 0.05-30 FPS。建议: 15-24 FPS 更流畅；运行时会对视频/摄像头流强制至少 15 FPS。"
            ),
            "widget_size" => self.t(
                "What: widget width/height in pixels. How: change W/H to resize weather/news boxes on wallpaper and in Ordering preview. Recommended: start with defaults, then fit your resolution.",
                "Was: Widget-Breite/Höhe in Pixeln. Wie: W/H anpassen, um Wetter-/News-Boxen im Wallpaper und in der Ordering-Vorschau zu skalieren. Empfehlung: mit Standardwerten starten und dann an Auflösung anpassen.",
                "Sta: širina/visina widgeta u pikselima. Kako: promeni W/H da bi menjao veličinu weather/news polja na pozadini i u Ordering pregledu. Preporuka: kreni od podrazumevanih vrednosti.",
                "作用: 组件宽/高（像素）。方法: 调整 W/H 可改变天气/新闻框在壁纸与 Ordering 预览中的尺寸。建议: 先用默认值，再按分辨率微调。"
            ),
            "news_audio_enabled" => self.t(
                "What: audio flag for future embedded stream playback. How: toggle on/off. Recommended: off by default to avoid disruptive playback.",
                "Was: Audio-Schalter für zukünftige eingebettete Stream-Wiedergabe. Wie: ein/aus. Empfehlung: standardmäßig aus, um störende Wiedergabe zu vermeiden.",
                "Sta: audio prekidač za buduću ugrađenu reprodukciju streama. Kako: uključi/isključi. Preporuka: podrazumevano isključeno zbog ometanja.",
                "作用: 未来内嵌流播放的音频开关。方法: 开/关。建议: 默认关闭，避免打扰。"
            ),
            "login_screen_integration" => self.t(
                "What: enable login-screen background integration. How: keeps this feature toggle in config for the login integration workflow. Recommended: keep off until login integration is validated on your distro.",
                "Was: Login-Screen-Hintergrund-Integration aktivieren. Wie: speichert den Schalter in der Config für den Login-Workflow. Empfehlung: ausgeschaltet lassen, bis die Login-Integration auf deiner Distribution validiert ist.",
                "Sta: ukljucuje integraciju pozadine za login ekran. Kako: cuva ovaj prekidac u config fajlu za login workflow. Preporuka: ostavi iskljuceno dok se ne potvrdi na tvojoj distribuciji.",
                "作用: 启用登录界面背景集成。方法: 在配置中保存该开关，供登录集成流程使用。建议: 在你的发行版完成验证前保持关闭。"
            ),
            "boot_screen_integration" => self.t(
                "What: enable boot-screen (splash) integration. How: keeps this feature toggle in config for boot theme workflow. Recommended: enable only if you know how to recover theme changes.",
                "Was: Boot-Screen/Splash-Integration aktivieren. Wie: speichert den Schalter in der Config für den Boot-Theme-Workflow. Empfehlung: nur aktivieren, wenn du Theme-Änderungen sicher zurückrollen kannst.",
                "Sta: ukljucuje integraciju boot/splash ekrana. Kako: cuva ovaj prekidac u config fajlu za boot theme workflow. Preporuka: ukljuci samo ako znas kako da vratis promene teme.",
                "作用: 启用启动画面（splash）集成。方法: 在配置中保存该开关，供启动主题流程使用。建议: 仅在你清楚如何恢复主题修改时启用。"
            ),
            "color_format" => self.t(
                "What: text color format. How: use #RRGGBB or #RRGGBBAA, or numeric RGB like 255,255,255. Recommended: keep strong contrast with background.",
                "Was: Text-Farbformat. Wie: #RRGGBB oder #RRGGBBAA, alternativ RGB wie 255,255,255. Empfehlung: starken Kontrast zum Hintergrund wählen.",
                "Sta: format boje teksta. Kako: koristi #RRGGBB ili #RRGGBBAA, ili RGB kao 255,255,255. Preporuka: drži jak kontrast sa pozadinom.",
                "作用: 文本颜色格式。方法: 使用 #RRGGBB / #RRGGBBAA，或 RGB 如 255,255,255。建议: 与背景保持高对比度。"
            ),
            _ => self.t(
                "Field help: hover explains what this setting does, how to use it, and a practical default recommendation.",
                "Feldhilfe: Hover erklärt Zweck, Nutzung und praxisnahe Standardempfehlung.",
                "Pomoć za polje: hover objašnjava svrhu, korišćenje i praktičnu preporuku.",
                "字段帮助: 悬停会说明用途、使用方式和推荐默认值。"
            ),
        }
    }

    fn recover_local_quotes(&mut self, persist_config: bool) -> Option<String> {
        match ensure_local_quotes_file(&mut self.cfg) {
            Ok(Some(path)) => {
                if persist_config && let Err(err) = self.save_to_path_inner() {
                    return Some(format!(
                        "Recovered quotes at {} but failed to save config: {err}",
                        path.display()
                    ));
                }
                Some(format!(
                    "Recovered missing local quotes file: {}",
                    path.display()
                ))
            }
            Ok(None) => None,
            Err(err) => Some(format!("Local quotes recovery failed: {err}")),
        }
    }

    fn start_update_check(&mut self) {
        if self.update_check_rx.is_some() {
            return;
        }
        let current_version =
            installed_package_version().unwrap_or_else(|| APP_VERSION.to_string());
        self.update_status = format!("Update check running (current {current_version})...");
        self.update_release = None;
        let (tx, rx) = mpsc::channel::<Result<Option<ReleaseInfo>, String>>();
        self.update_check_rx = Some(rx);
        std::thread::spawn(move || {
            let result = fetch_latest_release_info().map(|release| {
                if is_newer_release(&release.tag, &current_version) {
                    Some(release)
                } else {
                    None
                }
            });
            let _ = tx.send(result);
        });
    }

    fn poll_update_check(&mut self) {
        let Some(rx) = self.update_check_rx.take() else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(Some(release))) => {
                self.update_status = format!("Update available: {}", release.tag);
                self.update_release = Some(release);
            }
            Ok(Ok(None)) => {
                let current_version =
                    installed_package_version().unwrap_or_else(|| APP_VERSION.to_string());
                self.update_status = format!("Up to date ({current_version})");
                self.update_release = None;
            }
            Ok(Err(err)) => {
                self.update_status = format!("Update check failed: {err}");
                self.update_release = None;
            }
            Err(TryRecvError::Empty) => {
                self.update_check_rx = Some(rx);
            }
            Err(TryRecvError::Disconnected) => {
                self.update_status = "Update check failed: worker disconnected".to_string();
                self.update_release = None;
            }
        }
    }

    fn poll_self_update(&mut self) {
        let Some(rx) = self.self_update_rx.take() else {
            return;
        };

        match rx.try_recv() {
            Ok(Ok(msg)) => {
                self.status = msg;
                self.update_status =
                    "Self-update finished. Click Check Updates to verify package version."
                        .to_string();
                self.update_release = None;
            }
            Ok(Err(err)) => {
                self.status = err.clone();
                self.update_status = format!("Self-update failed: {err}");
            }
            Err(TryRecvError::Empty) => {
                self.self_update_rx = Some(rx);
            }
            Err(TryRecvError::Disconnected) => {
                self.status = "Self-update failed: worker disconnected".to_string();
                self.update_status = "Self-update failed: worker disconnected".to_string();
            }
        }
    }

    fn start_self_update(&mut self) {
        if self.self_update_rx.is_some() {
            self.status = "Self-update already running. Wait for completion.".to_string();
            return;
        }

        #[cfg(target_os = "linux")]
        {
            let mut launch_errors = Vec::<String>::new();
            if command_exists("pkexec") {
                for (backend, manager, args) in [
                    ("dnf", "dnf", vec!["dnf", "-y", "upgrade", "le-compositeur"]),
                    (
                        "apt-get",
                        "apt-get",
                        vec![
                            "apt-get",
                            "-y",
                            "install",
                            "--only-upgrade",
                            "le-compositeur",
                        ],
                    ),
                    (
                        "zypper",
                        "zypper",
                        vec!["zypper", "--non-interactive", "update", "le-compositeur"],
                    ),
                ] {
                    if !command_exists(manager) {
                        continue;
                    }
                    match Command::new("pkexec")
                        .args(args)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                    {
                        Ok(child) => {
                            let (tx, rx) = mpsc::channel::<Result<String, String>>();
                            self.self_update_rx = Some(rx);
                            self.status = format!(
                                "Self-update started via {backend} (authorization dialog may appear)."
                            );
                            self.update_status = format!(
                                "Self-update running via {backend} (waiting for completion)..."
                            );
                            std::thread::spawn(move || {
                                let _ = tx.send(wait_self_update_result(child, backend));
                            });
                            return;
                        }
                        Err(err) => launch_errors.push(format!("{backend}: {err}")),
                    }
                }
            } else {
                launch_errors.push("pkexec not found in PATH".to_string());
            }

            let release_url = self
                .update_release
                .as_ref()
                .map(|r| r.html_url.clone())
                .unwrap_or_else(|| {
                    "https://github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest"
                        .to_string()
                });
            let detail = if launch_errors.is_empty() {
                "No compatible package manager command found (dnf/apt-get/zypper).".to_string()
            } else {
                format!(
                    "Failed to start automatic update command: {}",
                    launch_errors.join(" | ")
                )
            };
            if open_url(&release_url) {
                self.status =
                    format!("{detail} Opened release page for manual update: {release_url}");
            } else {
                self.status = format!("{detail} Open manually: {release_url}");
            }
            self.update_status =
                "Self-update fallback active: download and install package manually.".to_string();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let release_url = self
                .update_release
                .as_ref()
                .map(|r| r.html_url.clone())
                .unwrap_or_else(|| {
                    "https://github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest"
                        .to_string()
                });
            if open_url(&release_url) {
                self.status = format!("Opened release page: {release_url}");
            } else {
                self.status = format!("Open manually: {release_url}");
            }
            self.update_status = "Automatic package update is only available on Linux.".to_string();
        }
    }

    fn load_from_path(&mut self, ctx: &egui::Context) {
        let path =
            expand_tilde(&self.config_path).unwrap_or_else(|_| PathBuf::from(&self.config_path));
        match load_config(path.as_path()) {
            Ok(cfg) => {
                self.cfg = cfg;
                self.status = "Config loaded".to_string();
                if let Some(msg) = self.recover_local_quotes(true) {
                    self.status = msg;
                }
                self.refresh_thumbnails(ctx);
                self.refresh_quotes_preview();
                self.autostart_toggle = Self::autostart_enabled();
            }
            Err(e) => self.status = format!("Load failed: {e}"),
        }
    }

    fn save_to_path_inner(&mut self) -> Result<(), String> {
        // Keep one effective rotation timer: image interval drives quote and loop cadence.
        self.cfg.refresh_seconds = self.cfg.image_refresh_seconds.max(1);
        self.cfg.quote_refresh_seconds = self.cfg.image_refresh_seconds.max(1);
        ensure_local_quotes_file(&mut self.cfg)
            .map_err(|e| format!("Quote recovery failed: {e}"))?;
        let path =
            expand_tilde(&self.config_path).unwrap_or_else(|_| PathBuf::from(&self.config_path));
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            return Err(format!("Create dir failed: {e}"));
        }
        let raw = to_config_toml(&self.cfg);
        std::fs::write(&path, raw).map_err(|e| format!("Save failed: {e}"))
    }

    fn save_to_path(&mut self) {
        match self.save_to_path_inner() {
            Ok(()) => self.status = "Config saved".to_string(),
            Err(e) => self.status = e,
        }
    }

    fn run_wc_cli(&mut self, args: &[&str]) {
        self.status = format!("Running: wc-cli {}", args.join(" "));
        if let Err(e) = self.save_to_path_inner() {
            self.status = format!("Cannot run command before save: {e}");
            return;
        }

        let path = self.config_path.clone();
        let mut launch_errors = Vec::<String>::new();
        let mut output = None;
        for bin in self.wc_cli_command_candidates() {
            match self
                .build_wc_cli_direct_with_bin(&bin, args, &path)
                .output()
            {
                Ok(out) => {
                    output = Some((out, format!("bin:{bin}")));
                    break;
                }
                Err(e) => launch_errors.push(format!("{bin}: {e}")),
            }
        }
        if output.is_none() && self.allow_cargo_fallback() {
            match self.build_wc_cli_cargo(args, &path).output() {
                Ok(out) => output = Some((out, "cargo".to_string())),
                Err(e) => launch_errors.push(format!("cargo: {e}")),
            }
        }

        let Some((output, runner_name)) = output else {
            self.status = format!(
                "Command start failed.\n{}\nHint: install `le-compositeur-cli`/`wc-cli`, or set WC_CLI_BIN to full path.",
                launch_errors.join("\n")
            );
            return;
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            self.status = format!("{} OK ({runner_name})\n{stdout}", args.join(" "));
        } else {
            self.status = format!(
                "{} failed ({runner_name})\n{stderr}\n{stdout}",
                args.join(" ")
            );
        }
    }

    fn apply_now(&mut self) {
        let was_enabled = self.cfg.apply_wallpaper;
        if !self.cfg.apply_wallpaper {
            self.cfg.apply_wallpaper = true;
        }
        self.run_wc_cli(&["run", "--once"]);
        if !was_enabled {
            self.cfg.apply_wallpaper = false;
            let _ = self.save_to_path_inner();
            self.status
                .push_str("\nApply Now used temporary apply_wallpaper=true.");
        }
    }

    fn start_runner(&mut self) {
        if self.runner.is_some() {
            self.stop_runner();
        }
        if let Err(e) = self.save_to_path_inner() {
            self.status = format!("Cannot start runner before save: {e}");
            return;
        }
        let replaced_external = self.kill_external_runner_processes();

        let path = self.config_path.clone();
        let child = match self.spawn_runner_command(&["run", "--replace-existing"], &path, true) {
            Ok(child) => child,
            Err(e) => {
                self.status = format!("Runner start failed: {e}");
                return;
            }
        };

        self.runner = Some(child);
        self.status = if replaced_external {
            "Runner started (replaced previous background runner)".to_string()
        } else {
            "Runner started (continuous updates active)".to_string()
        };
    }

    fn start_detached_runner(&mut self) {
        if let Err(e) = self.save_to_path_inner() {
            self.status = format!("Cannot start detached runner before save: {e}");
            return;
        }
        let replaced_external = self.kill_external_runner_processes();

        let path = self.config_path.clone();
        let result = self
            .spawn_runner_command(&["run", "--replace-existing"], &path, false)
            .map(|_child| ());

        match result {
            Ok(()) => {
                self.status = if replaced_external {
                    "Detached runner started (replaced previous background runner).".to_string()
                } else {
                    "Detached runner started (GUI can be closed; reopen GUI manually).".to_string()
                }
            }
            Err(e) => self.status = format!("Detached runner start failed: {e}"),
        }
    }

    fn stop_runner(&mut self) {
        let mut stopped_any = false;
        if let Some(mut child) = self.runner.take() {
            let _ = child.kill();
            let _ = child.wait();
            stopped_any = true;
        }
        if self.kill_external_runner_processes() {
            stopped_any = true;
        }
        self.status = if stopped_any {
            "Runner stopped".to_string()
        } else {
            "Runner is not active".to_string()
        };
    }

    fn autostart_paths() -> Vec<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let Some(home) = std::env::var_os("HOME") else {
                return Vec::new();
            };
            let base = PathBuf::from(home).join(".config").join("autostart");
            return vec![
                base.join("le-compositeur.desktop"),
                base.join("wallpaper-composer.desktop"),
            ];
        }
        #[cfg(not(target_os = "linux"))]
        {
            Vec::new()
        }
    }

    fn autostart_primary_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            Self::autostart_paths().into_iter().next()
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    fn autostart_enabled() -> bool {
        Self::autostart_paths().iter().any(|p| p.exists())
    }

    fn install_autostart(&mut self) {
        let Some(path) = Self::autostart_primary_path() else {
            self.status = "Autostart install is currently supported on Linux only.".to_string();
            return;
        };

        let config = shell_quote_single(&self.config_path);
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.status = format!("Autostart install failed (mkdir): {e}");
            return;
        }

        for old in Self::autostart_paths() {
            if old != path {
                let _ = std::fs::remove_file(old);
            }
        }
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Le Compositeur Runner\nComment=Start Le Compositeur background runner on login\nTryExec=/usr/bin/wc-cli\nExec=/usr/bin/bash -lc \"sleep 12; /usr/bin/pkill -f 'wc-cli run --config' >/dev/null 2>&1 || true; /usr/bin/wc-cli run --once --config {0}; /usr/bin/wc-cli run --replace-existing --config {0}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\nX-GNOME-Autostart-Delay=12\n",
            config
        );
        match std::fs::write(&path, content) {
            Ok(()) => {
                self.autostart_toggle = true;
                self.status = format!("Autostart installed: {}", path.display());
            }
            Err(e) => self.status = format!("Autostart install failed: {e}"),
        }
    }

    fn remove_autostart(&mut self) {
        if Self::autostart_paths().is_empty() {
            self.status = "Autostart remove is currently supported on Linux only.".to_string();
            return;
        }
        let mut removed = 0usize;
        for path in Self::autostart_paths() {
            if path.exists() && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        if removed == 0 {
            self.status = "Autostart file not present.".to_string();
            return;
        }
        self.autostart_toggle = false;
        self.status = format!("Autostart removed ({} file(s)).", removed);
    }

    fn sync_autostart_toggle(&mut self) {
        if self.autostart_toggle && !Self::autostart_enabled() {
            self.install_autostart();
        } else if !self.autostart_toggle && Self::autostart_enabled() {
            self.remove_autostart();
        }
    }

    fn poll_runner_state(&mut self) {
        let Some(child) = self.runner.as_mut() else {
            return;
        };
        let polled = child.try_wait();
        match polled {
            Ok(Some(exit)) => {
                let mut stderr_tail = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    let mut buf = Vec::<u8>::new();
                    let _ = stderr.read_to_end(&mut buf);
                    let text = String::from_utf8_lossy(&buf).trim().to_string();
                    if !text.is_empty() {
                        stderr_tail = text;
                    }
                }
                self.status = if stderr_tail.is_empty() {
                    format!("Runner exited: {exit}")
                } else {
                    format!("Runner exited: {exit}\n{stderr_tail}")
                };
                self.runner = None;
            }
            Ok(None) => {}
            Err(e) => {
                self.status = format!("Runner state check failed: {e}");
                self.runner = None;
            }
        }
    }

    fn kill_external_runner_processes(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            let mut killed = false;
            for pattern in ["wc-cli run --config", "le-compositeur-cli run --config"] {
                if Command::new("pkill")
                    .args(["-f", pattern])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    killed = true;
                }
            }
            killed
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn refresh_weather_if_needed(&mut self) {
        if !self.cfg.show_weather_layer {
            return;
        }
        let interval = Duration::from_secs(self.cfg.weather_refresh_seconds.max(60));
        if let Some(last) = self.weather_last_refresh
            && last.elapsed() < interval
        {
            return;
        }
        self.refresh_weather_now();
    }

    fn refresh_weather_now(&mut self) {
        match fetch_weather_snapshot(&self.cfg) {
            Ok((headline, details)) => {
                self.weather_status = headline;
                self.weather_details = details;
                self.weather_last_refresh = Some(Instant::now());
            }
            Err(err) => {
                self.weather_status = format!("Weather refresh failed: {err}");
                self.weather_details.clear();
                self.weather_last_refresh = Some(Instant::now());
            }
        }
    }

    fn refresh_quotes_preview(&mut self) {
        if let Some(msg) = self.recover_local_quotes(false) {
            self.status = msg;
        }
        let Ok(path) = expand_tilde(&self.cfg.quotes_path) else {
            self.status = "Cannot expand quotes_path".to_string();
            self.quote_preview.clear();
            return;
        };
        match load_quotes(&path) {
            Ok(quotes) => {
                self.quote_preview = quotes.into_iter().take(3).collect();
                if self.quote_preview.is_empty() {
                    self.status = "Quotes loaded but empty".to_string();
                }
            }
            Err(e) => {
                self.quote_preview.clear();
                self.status = format!("Quote parse failed: {e}");
            }
        }
    }

    fn wc_cli_command_candidates(&self) -> Vec<String> {
        let mut bins = Vec::<String>::new();
        if let Ok(custom) = std::env::var("WC_CLI_BIN") {
            let custom = custom.trim();
            if !custom.is_empty() {
                bins.push(custom.to_string());
            }
        }
        for bin in [
            "wc-cli",
            "le-compositeur-cli",
            "/usr/bin/wc-cli",
            "/usr/bin/le-compositeur-cli",
            "/usr/libexec/le-compositeur/le-compositeur-cli",
        ] {
            bins.push(bin.to_string());
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                for bin in ["wc-cli", "le-compositeur-cli"] {
                    bins.push(dir.join(bin).display().to_string());
                    bins.push(dir.join("..").join(bin).display().to_string());
                }
            }
            if let Some(dir) = exe.parent().and_then(|d| d.parent()) {
                for bin in ["wc-cli", "le-compositeur-cli"] {
                    bins.push(dir.join(bin).display().to_string());
                }
            }
        }

        let mut deduped = Vec::<String>::new();
        for bin in bins {
            if !deduped.iter().any(|existing| existing == &bin) {
                deduped.push(bin);
            }
        }
        deduped
    }

    fn allow_cargo_fallback(&self) -> bool {
        if std::env::var("WC_GUI_ALLOW_CARGO_FALLBACK").is_ok_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "on" | "yes"
            )
        }) {
            return true;
        }
        if !cfg!(debug_assertions) {
            return false;
        }
        Path::new("Cargo.toml").exists() || Path::new("crates/wc-cli/Cargo.toml").exists()
    }

    fn build_wc_cli_direct_with_bin(&self, bin: &str, args: &[&str], path: &str) -> Command {
        let mut cmd = Command::new(bin);
        cmd.args(args).args(["--config", path]);
        cmd
    }

    fn build_wc_cli_cargo(&self, args: &[&str], path: &str) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-p", "wc-cli", "--"])
            .args(args)
            .args(["--config", path]);
        cmd
    }

    fn spawn_runner_command(
        &self,
        args: &[&str],
        path: &str,
        capture_stderr: bool,
    ) -> Result<Child, String> {
        let mut launch_errors = Vec::<String>::new();
        for bin in self.wc_cli_command_candidates() {
            let mut cmd = self.build_wc_cli_direct_with_bin(&bin, args, path);
            cmd.stdout(Stdio::null());
            if capture_stderr {
                cmd.stderr(Stdio::piped());
            } else {
                cmd.stderr(Stdio::null());
            }
            match cmd.spawn() {
                Ok(child) => return Ok(child),
                Err(e) => launch_errors.push(format!("{bin}: {e}")),
            }
        }

        if self.allow_cargo_fallback() {
            let mut cmd = self.build_wc_cli_cargo(args, path);
            cmd.stdout(Stdio::null());
            if capture_stderr {
                cmd.stderr(Stdio::piped());
            } else {
                cmd.stderr(Stdio::null());
            }
            match cmd.spawn() {
                Ok(child) => return Ok(child),
                Err(e) => launch_errors.push(format!("cargo: {e}")),
            }
        }

        Err(format!(
            "could not start CLI command.\n{}\nHint: install `le-compositeur-cli`/`wc-cli`, or set WC_CLI_BIN to full path.",
            launch_errors.join("\n")
        ))
    }

    fn pick_image_dir(&mut self, ctx: &egui::Context) {
        let start = expand_tilde(&self.cfg.image_dir).unwrap_or_else(|_| PathBuf::from("."));
        let picked = self.pick_folder_dialog(start);
        if let Some(path) = picked {
            self.cfg.image_dir = path.display().to_string();
            self.refresh_thumbnails(ctx);
            self.status = "Image folder selected".to_string();
        } else {
            self.status = "Folder selection canceled (or no folder selected)".to_string();
        }
    }

    fn pick_quotes_file(&mut self) {
        let start = expand_tilde(&self.cfg.quotes_path).unwrap_or_else(|_| PathBuf::from("."));
        let base = if start.is_file() {
            start.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            start
        };

        let picked = self.pick_quotes_dialog(base);
        if let Some(path) = picked {
            self.cfg.quotes_path = path.display().to_string();
            self.refresh_quotes_preview();
            self.status = "Quotes file selected".to_string();
        } else {
            self.status = "Quotes file selection canceled".to_string();
        }
    }

    fn pick_folder_dialog(&self, start: PathBuf) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            pick_linux_path_dialog(&start, true)
        }
        #[cfg(not(target_os = "linux"))]
        {
            rfd::FileDialog::new().set_directory(start).pick_folder()
        }
    }

    fn pick_quotes_dialog(&self, base: PathBuf) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            pick_linux_path_dialog(&base, false)
        }
        #[cfg(not(target_os = "linux"))]
        {
            rfd::FileDialog::new()
                .set_directory(base)
                .add_filter("Quotes", &["md", "txt"])
                .pick_file()
        }
    }

    fn refresh_thumbnails(&mut self, ctx: &egui::Context) {
        let Ok(dir) = expand_tilde(&self.cfg.image_dir) else {
            self.status = "Cannot expand image_dir path".to_string();
            self.thumbnails.clear();
            self.thumbnails_for_dir.clear();
            self.ordering_bg_texture = None;
            return;
        };

        let images = match list_background_images(&dir) {
            Ok(list) => list,
            Err(e) => {
                self.status = format!("Thumbnail scan failed: {e}");
                self.thumbnails.clear();
                self.thumbnails_for_dir.clear();
                self.ordering_bg_texture = None;
                return;
            }
        };

        self.thumbnails.clear();
        self.ordering_bg_texture = None;
        for (idx, path) in images.iter().take(3).enumerate() {
            match load_thumbnail(ctx, path, idx) {
                Ok(texture) => {
                    let label = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("image")
                        .to_string();
                    self.thumbnails.push(ThumbnailItem { label, texture });
                }
                Err(err) => {
                    self.status = format!("Thumbnail decode failed for {}: {err}", path.display());
                }
            }
        }
        if let Some(first_path) = images.first() {
            self.ordering_bg_texture = load_ordering_background_texture(ctx, first_path).ok();
        }

        self.thumbnails_for_dir = self.cfg.image_dir.clone();
        if self.thumbnails.is_empty() {
            self.status = "No previewable images found in folder".to_string();
        }
    }

    fn render_images_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Image Source");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.cfg.image_source, "local".to_string(), "Local")
                .on_hover_text(self.hover_text("image_source_mode"));
            ui.selectable_value(
                &mut self.cfg.image_source,
                "preset".to_string(),
                "Open Preset",
            )
            .on_hover_text(self.hover_text("image_source_mode"));
            ui.selectable_value(&mut self.cfg.image_source, "url".to_string(), "Custom URL")
                .on_hover_text(self.hover_text("image_source_mode"));
        });

        match self.cfg.image_source.as_str() {
            "preset" => {
                ui.horizontal(|ui| {
                    ui.label("Preset");
                    let mut selected = self
                        .cfg
                        .image_source_preset
                        .clone()
                        .unwrap_or_else(|| "picsum_random_hd".to_string());
                    egui::ComboBox::from_id_salt("image_source_preset")
                        .selected_text(&selected)
                        .show_ui(ui, |ui| {
                            for p in builtin_image_presets() {
                                ui.selectable_value(
                                    &mut selected,
                                    p.id.to_string(),
                                    format!("{} ({})", p.display_label, p.id),
                                );
                            }
                        })
                        .response
                        .on_hover_text(self.hover_text("image_preset"));
                    self.cfg.image_source_preset = Some(selected);
                });
            }
            "url" => {
                ui.horizontal(|ui| {
                    ui.label("Image URL(s)");
                    let mut url = self.cfg.image_source_url.clone().unwrap_or_default();
                    ui.add(
                        egui::TextEdit::multiline(&mut url)
                            .desired_rows(3)
                            .desired_width(360.0),
                    )
                    .on_hover_text(self.hover_text("image_url"));
                    self.cfg.image_source_url = Some(url);
                });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Image dir");
                    ui.text_edit_singleline(&mut self.cfg.image_dir)
                        .on_hover_text(self.hover_text("image_dir"));
                    if ui.button("Browse...").clicked() {
                        self.pick_image_dir(ctx);
                    }
                });
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Image order");
            egui::ComboBox::from_id_salt("image_order_mode")
                .selected_text(&self.cfg.image_order_mode)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.cfg.image_order_mode,
                        "sequential".to_string(),
                        "sequential",
                    );
                    ui.selectable_value(
                        &mut self.cfg.image_order_mode,
                        "random".to_string(),
                        "random",
                    );
                });
            ui.checkbox(&mut self.cfg.image_avoid_repeat, "Avoid repeat");
        });
        ui.horizontal(|ui| {
            ui.label("Image sec");
            ui.add(egui::DragValue::new(&mut self.cfg.image_refresh_seconds).speed(1))
                .on_hover_text(self.hover_text("image_refresh"));
        });

        ui.separator();
        ui.heading("Wallpaper");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.cfg.apply_wallpaper, "Apply wallpaper")
                .on_hover_text(self.hover_text("apply_wallpaper"));
            ui.label("Backend");
            egui::ComboBox::from_id_salt("backend_images_tab")
                .selected_text(&self.cfg.wallpaper_backend)
                .show_ui(ui, |ui| {
                    for mode in ["auto", "gnome", "sway", "feh", "noop"] {
                        ui.selectable_value(
                            &mut self.cfg.wallpaper_backend,
                            mode.to_string(),
                            mode,
                        );
                    }
                });
            ui.label("Fit");
            egui::ComboBox::from_id_salt("fit_images_tab")
                .selected_text(&self.cfg.wallpaper_fit_mode)
                .show_ui(ui, |ui| {
                    for mode in [
                        "zoom",
                        "scaled",
                        "stretched",
                        "spanned",
                        "centered",
                        "wallpaper",
                    ] {
                        ui.selectable_value(
                            &mut self.cfg.wallpaper_fit_mode,
                            mode.to_string(),
                            mode,
                        );
                    }
                });
        });
    }

    fn render_quotes_tab(&mut self, ui: &mut egui::Ui) {
        let color_help = self.hover_text("color_format").to_string();
        ui.heading("Quote Source");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.cfg.quote_source, "local".to_string(), "Local")
                .on_hover_text(self.hover_text("quotes_source_mode"));
            ui.selectable_value(
                &mut self.cfg.quote_source,
                "preset".to_string(),
                "Open Preset",
            )
            .on_hover_text(self.hover_text("quotes_source_mode"));
            ui.selectable_value(&mut self.cfg.quote_source, "url".to_string(), "Custom URL")
                .on_hover_text(self.hover_text("quotes_source_mode"));
        });

        match self.cfg.quote_source.as_str() {
            "preset" => {
                ui.horizontal(|ui| {
                    ui.label("Preset");
                    let mut selected = self
                        .cfg
                        .quote_source_preset
                        .clone()
                        .unwrap_or_else(|| "zenquotes_daily".to_string());
                    egui::ComboBox::from_id_salt("quote_source_preset")
                        .selected_text(&selected)
                        .show_ui(ui, |ui| {
                            for p in builtin_quote_presets() {
                                ui.selectable_value(
                                    &mut selected,
                                    p.id.to_string(),
                                    format!("{} ({})", p.display_label, p.id),
                                );
                            }
                        })
                        .response
                        .on_hover_text(self.hover_text("quote_preset"));
                    self.cfg.quote_source_preset = Some(selected);
                });
            }
            "url" => {
                ui.horizontal(|ui| {
                    ui.label("Quote URL(s)");
                    let mut url = self.cfg.quote_source_url.clone().unwrap_or_default();
                    ui.add(
                        egui::TextEdit::multiline(&mut url)
                            .desired_rows(3)
                            .desired_width(360.0),
                    )
                    .on_hover_text(self.hover_text("quote_url"));
                    self.cfg.quote_source_url = Some(url);
                });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Quotes path");
                    ui.text_edit_singleline(&mut self.cfg.quotes_path)
                        .on_hover_text(self.hover_text("quotes_path"));
                    if ui.button("Browse...").clicked() {
                        self.pick_quotes_file();
                    }
                });
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Quote order");
            egui::ComboBox::from_id_salt("quote_order_mode")
                .selected_text(&self.cfg.quote_order_mode)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.cfg.quote_order_mode,
                        "sequential".to_string(),
                        "sequential",
                    );
                    ui.selectable_value(
                        &mut self.cfg.quote_order_mode,
                        "random".to_string(),
                        "random",
                    );
                });
            ui.checkbox(&mut self.cfg.quote_avoid_repeat, "Avoid repeat");
        });

        ui.separator();
        ui.heading("Quote Style");
        ui.label("Style options previously in Style tab are now here.");
        ui.horizontal(|ui| {
            ui.label("Font family");
            egui::ComboBox::from_id_salt("font_family_quotes_tab")
                .selected_text(&self.cfg.font_family)
                .show_ui(ui, |ui| {
                    for family in [
                        "DejaVu-Sans",
                        "Noto-Sans",
                        "Liberation-Sans",
                        "Serif",
                        "Monospace",
                    ] {
                        ui.selectable_value(&mut self.cfg.font_family, family.to_string(), family);
                    }
                });
            ui.label("Quote size");
            ui.add(
                egui::DragValue::new(&mut self.cfg.quote_font_size)
                    .speed(1)
                    .range(8..=220),
            );
            ui.checkbox(&mut self.cfg.quote_auto_fit, "Auto fit");
            ui.label("Min");
            ui.add(
                egui::DragValue::new(&mut self.cfg.quote_min_font_size)
                    .speed(1)
                    .range(8..=220),
            );
        });
        ui.horizontal(|ui| {
            edit_color_field(
                ui,
                "Quote color",
                &mut self.cfg.quote_color,
                false,
                &color_help,
            );
            edit_color_field(
                ui,
                "Stroke color",
                &mut self.cfg.text_stroke_color,
                false,
                &color_help,
            );
            ui.label("Stroke width");
            ui.add(egui::DragValue::new(&mut self.cfg.text_stroke_width).speed(1));
        });
        ui.horizontal(|ui| {
            edit_color_field(
                ui,
                "Undercolor",
                &mut self.cfg.text_undercolor,
                true,
                &color_help,
            );
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.cfg.text_shadow_enabled, "Shadow");
            edit_color_field(
                ui,
                "Shadow color",
                &mut self.cfg.text_shadow_color,
                true,
                &color_help,
            );
            ui.label("dx");
            ui.add(egui::DragValue::new(&mut self.cfg.text_shadow_offset_x).speed(1));
            ui.label("dy");
            ui.add(egui::DragValue::new(&mut self.cfg.text_shadow_offset_y).speed(1));
        });
    }

    fn render_ordering_tab(&mut self, ui: &mut egui::Ui) {
        let color_help = self.hover_text("color_format").to_string();
        ui.heading("Ordering");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.cfg.show_background_layer, "Background")
                .on_hover_text("Enable/disable rendered background layer.");
            ui.checkbox(&mut self.cfg.show_quote_layer, "Quote");
            ui.checkbox(&mut self.cfg.show_clock_layer, "Clock");
            ui.checkbox(&mut self.cfg.show_weather_layer, "Weather");
            ui.checkbox(&mut self.cfg.show_news_layer, "News");
            ui.checkbox(&mut self.cfg.show_cams_layer, "Cams");
        });

        ui.horizontal(|ui| {
            ui.label("Element");
            egui::ComboBox::from_id_salt("layout_selected_element")
                .selected_text(Self::layout_element_label(self.selected_element))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.selected_element,
                        LayoutElement::Quote,
                        "Quote Box",
                    );
                    ui.selectable_value(&mut self.selected_element, LayoutElement::Clock, "Clock");
                    ui.selectable_value(
                        &mut self.selected_element,
                        LayoutElement::Weather,
                        "Weather",
                    );
                    ui.selectable_value(&mut self.selected_element, LayoutElement::News, "News");
                    ui.selectable_value(&mut self.selected_element, LayoutElement::Cams, "Cams");
                });
        });
        ui.horizontal(|ui| {
            ui.label("Layer Z");
            let z = self.layout_element_z_mut(self.selected_element);
            *z = (*z).clamp(0, 100);
            ui.add(egui::DragValue::new(z).speed(1).range(0..=100));
            ui.label("(higher = in front)");
            if ui.button("Normalize Z").clicked() {
                self.normalize_layout_z();
            }
        });
        ui.label(format!(
            "Grid snap: {}px ({}x{} canvas)",
            ORDERING_GRID_STEP, ORDERING_WORLD_WIDTH, ORDERING_WORLD_HEIGHT
        ));

        let canvas_w = ui.available_width().clamp(560.0, 900.0);
        let canvas_size = egui::vec2(canvas_w, canvas_w * 9.0 / 16.0);
        let (rect, response) = ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, egui::Color32::from_rgb(14, 15, 18));
        if self.cfg.show_background_layer
            && let Some(tex) = &self.ordering_bg_texture
        {
            painter.image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painter.rect_filled(rect, 8.0, egui::Color32::from_black_alpha(80));
        }
        painter.rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
            egui::StrokeKind::Middle,
        );

        let sx = rect.width() / ORDERING_WORLD_WIDTH as f32;
        let sy = rect.height() / ORDERING_WORLD_HEIGHT as f32;
        let step_x = (ORDERING_GRID_STEP as f32 * sx.max(0.001)).max(1.0);
        let step_y = (ORDERING_GRID_STEP as f32 * sy.max(0.001)).max(1.0);
        let grid_color = egui::Color32::from_gray(70).linear_multiply(0.45);
        let mut gx = rect.left();
        while gx <= rect.right() {
            painter.line_segment(
                [egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())],
                egui::Stroke::new(1.0, grid_color),
            );
            gx += step_x;
        }
        let mut gy = rect.top();
        while gy <= rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), gy), egui::pos2(rect.right(), gy)],
                egui::Stroke::new(1.0, grid_color),
            );
            gy += step_y;
        }

        let quote_size = quote_box_px(
            self.cfg.text_box_size.as_str(),
            self.cfg.text_box_width_pct,
            self.cfg.text_box_height_pct,
            rect.size(),
        );
        let mut quote_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.cfg.quote_pos_x as f32 * sx,
                rect.top() + self.cfg.quote_pos_y as f32 * sy,
            ),
            quote_size,
        );
        let clock_size = egui::vec2(180.0 * sx.max(0.2), 64.0 * sy.max(0.2));
        let mut clock_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.cfg.clock_pos_x as f32 * sx,
                rect.top() + self.cfg.clock_pos_y as f32 * sy,
            ),
            clock_size,
        );
        let weather_size = egui::vec2(
            self.cfg.weather_widget_width as f32 * sx.max(0.2),
            self.cfg.weather_widget_height as f32 * sy.max(0.2),
        );
        let news_size = egui::vec2(
            self.cfg.news_widget_width as f32 * sx.max(0.2),
            self.cfg.news_widget_height as f32 * sy.max(0.2),
        );
        let mut weather_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.cfg.weather_pos_x as f32 * sx,
                rect.top() + self.cfg.weather_pos_y as f32 * sy,
            ),
            weather_size,
        );
        let mut news_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.cfg.news_pos_x as f32 * sx,
                rect.top() + self.cfg.news_pos_y as f32 * sy,
            ),
            news_size,
        );
        let cams_size = egui::vec2(
            self.cfg.cams_widget_width as f32 * sx.max(0.2),
            self.cfg.cams_widget_height as f32 * sy.max(0.2),
        );
        let mut cams_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.cfg.cams_pos_x as f32 * sx,
                rect.top() + self.cfg.cams_pos_y as f32 * sy,
            ),
            cams_size,
        );

        let mut ordered_layers = Vec::<(LayoutElement, egui::Rect, i32)>::with_capacity(5);
        if self.cfg.show_quote_layer {
            ordered_layers.push((
                LayoutElement::Quote,
                quote_rect,
                self.layout_element_z(LayoutElement::Quote),
            ));
        }
        if self.cfg.show_clock_layer {
            ordered_layers.push((
                LayoutElement::Clock,
                clock_rect,
                self.layout_element_z(LayoutElement::Clock),
            ));
        }
        if self.cfg.show_weather_layer {
            ordered_layers.push((
                LayoutElement::Weather,
                weather_rect,
                self.layout_element_z(LayoutElement::Weather),
            ));
        }
        if self.cfg.show_news_layer {
            ordered_layers.push((
                LayoutElement::News,
                news_rect,
                self.layout_element_z(LayoutElement::News),
            ));
        }
        if self.cfg.show_cams_layer {
            ordered_layers.push((
                LayoutElement::Cams,
                cams_rect,
                self.layout_element_z(LayoutElement::Cams),
            ));
        }

        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            ordered_layers.sort_by(|a, b| b.2.cmp(&a.2));
            if let Some((element, _, _)) = ordered_layers.iter().find(|(_, r, _)| r.contains(pos)) {
                self.selected_element = *element;
            }
        }
        if response.dragged()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let x = (pos.x - rect.left()).clamp(0.0, rect.width());
            let y = (pos.y - rect.top()).clamp(0.0, rect.height());
            let mut world_x = snap_to_grid((x / sx).round() as i32);
            let mut world_y = snap_to_grid((y / sy).round() as i32);
            let selected_world = self.layout_element_world_rect(self.selected_element);
            let (clamped_x, clamped_y) =
                clamp_world_pos(world_x, world_y, selected_world.w, selected_world.h);
            world_x = clamped_x;
            world_y = clamped_y;
            match self.selected_element {
                LayoutElement::Quote => {
                    self.cfg.quote_pos_x = world_x;
                    self.cfg.quote_pos_y = world_y;
                }
                LayoutElement::Clock => {
                    self.cfg.clock_pos_x = world_x;
                    self.cfg.clock_pos_y = world_y;
                }
                LayoutElement::Weather => {
                    self.cfg.weather_pos_x = world_x;
                    self.cfg.weather_pos_y = world_y;
                }
                LayoutElement::News => {
                    self.cfg.news_pos_x = world_x;
                    self.cfg.news_pos_y = world_y;
                }
                LayoutElement::Cams => {
                    self.cfg.cams_pos_x = world_x;
                    self.cfg.cams_pos_y = world_y;
                }
            }
            self.resolve_selected_collision(self.selected_element);
            quote_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.cfg.quote_pos_x as f32 * sx,
                    rect.top() + self.cfg.quote_pos_y as f32 * sy,
                ),
                quote_size,
            );
            clock_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.cfg.clock_pos_x as f32 * sx,
                    rect.top() + self.cfg.clock_pos_y as f32 * sy,
                ),
                clock_size,
            );
            weather_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.cfg.weather_pos_x as f32 * sx,
                    rect.top() + self.cfg.weather_pos_y as f32 * sy,
                ),
                weather_size,
            );
            news_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.cfg.news_pos_x as f32 * sx,
                    rect.top() + self.cfg.news_pos_y as f32 * sy,
                ),
                news_size,
            );
            cams_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.cfg.cams_pos_x as f32 * sx,
                    rect.top() + self.cfg.cams_pos_y as f32 * sy,
                ),
                cams_size,
            );
            ordered_layers.clear();
            if self.cfg.show_quote_layer {
                ordered_layers.push((
                    LayoutElement::Quote,
                    quote_rect,
                    self.layout_element_z(LayoutElement::Quote),
                ));
            }
            if self.cfg.show_clock_layer {
                ordered_layers.push((
                    LayoutElement::Clock,
                    clock_rect,
                    self.layout_element_z(LayoutElement::Clock),
                ));
            }
            if self.cfg.show_weather_layer {
                ordered_layers.push((
                    LayoutElement::Weather,
                    weather_rect,
                    self.layout_element_z(LayoutElement::Weather),
                ));
            }
            if self.cfg.show_news_layer {
                ordered_layers.push((
                    LayoutElement::News,
                    news_rect,
                    self.layout_element_z(LayoutElement::News),
                ));
            }
            if self.cfg.show_cams_layer {
                ordered_layers.push((
                    LayoutElement::Cams,
                    cams_rect,
                    self.layout_element_z(LayoutElement::Cams),
                ));
            }
        }

        ordered_layers.sort_by(|a, b| a.2.cmp(&b.2));
        for (element, layer_rect, _) in &ordered_layers {
            self.draw_ordering_element(&painter, *element, *layer_rect);
        }

        ui.separator();
        match self.selected_element {
            LayoutElement::Quote => {
                ui.heading("Quote Settings");
                ui.horizontal(|ui| {
                    ui.label("Size");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.quote_font_size)
                            .speed(1)
                            .range(8..=160),
                    );
                    ui.checkbox(&mut self.cfg.quote_auto_fit, "Auto fit");
                    ui.label("Min");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.quote_min_font_size)
                            .speed(1)
                            .range(8..=160),
                    );
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.quote_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.quote_pos_y).speed(1));
                });
                ui.horizontal(|ui| {
                    ui.label("Font family");
                    egui::ComboBox::from_id_salt("font_family_elements")
                        .selected_text(&self.cfg.font_family)
                        .show_ui(ui, |ui| {
                            for family in [
                                "DejaVu-Sans",
                                "Noto-Sans",
                                "Liberation-Sans",
                                "Serif",
                                "Monospace",
                            ] {
                                ui.selectable_value(
                                    &mut self.cfg.font_family,
                                    family.to_string(),
                                    family,
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Text box");
                    egui::ComboBox::from_id_salt("text_box_size_elements")
                        .selected_text(&self.cfg.text_box_size)
                        .show_ui(ui, |ui| {
                            for mode in ["quarter", "third", "half", "full", "custom"] {
                                ui.selectable_value(
                                    &mut self.cfg.text_box_size,
                                    mode.to_string(),
                                    mode,
                                );
                            }
                        });
                    ui.label("W%");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.text_box_width_pct)
                            .speed(1)
                            .range(10..=100),
                    );
                    ui.label("H%");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.text_box_height_pct)
                            .speed(1)
                            .range(10..=100),
                    );
                });
                ui.horizontal(|ui| {
                    edit_color_field(
                        ui,
                        "Quote color",
                        &mut self.cfg.quote_color,
                        false,
                        &color_help,
                    );
                });
            }
            LayoutElement::Clock => {
                ui.heading("Clock Settings");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.clock_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.clock_pos_y).speed(1));
                    ui.label("Text size");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.clock_font_size)
                            .speed(1)
                            .range(8..=220),
                    );
                });
                ui.horizontal(|ui| {
                    edit_color_field(
                        ui,
                        "Clock color",
                        &mut self.cfg.clock_color,
                        false,
                        &color_help,
                    );
                });
            }
            LayoutElement::Weather => {
                ui.heading("Weather Widget Settings");
                ui.checkbox(&mut self.cfg.show_weather_layer, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.weather_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.weather_pos_y).speed(1));
                    ui.label("W");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_widget_width)
                            .speed(2)
                            .range(120..=1920),
                    )
                    .on_hover_text(self.hover_text("widget_size"));
                    ui.label("H");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_widget_height)
                            .speed(2)
                            .range(80..=1080),
                    )
                    .on_hover_text(self.hover_text("widget_size"));
                });
                ui.horizontal(|ui| {
                    ui.label("Font");
                    egui::ComboBox::from_id_salt("weather_font_family_ordering")
                        .selected_text(&self.cfg.weather_font_family)
                        .show_ui(ui, |ui| {
                            for family in [
                                "DejaVu-Sans",
                                "Noto-Sans",
                                "Liberation-Sans",
                                "Serif",
                                "Monospace",
                            ] {
                                ui.selectable_value(
                                    &mut self.cfg.weather_font_family,
                                    family.to_string(),
                                    family,
                                );
                            }
                        });
                    ui.label("Size");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_font_size)
                            .speed(1)
                            .range(10..=220),
                    );
                    ui.label("Stroke");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_stroke_width)
                            .speed(1)
                            .range(0..=20),
                    );
                });
                ui.horizontal(|ui| {
                    edit_color_field(ui, "Text", &mut self.cfg.weather_color, false, &color_help);
                    edit_color_field(
                        ui,
                        "Undercolor",
                        &mut self.cfg.weather_undercolor,
                        true,
                        &color_help,
                    );
                    edit_color_field(
                        ui,
                        "Stroke color",
                        &mut self.cfg.weather_stroke_color,
                        false,
                        &color_help,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_refresh_seconds)
                            .speed(10)
                            .range(60..=3600),
                    )
                    .on_hover_text(self.hover_text("weather_refresh_seconds"));
                    if ui.button("Refresh now").clicked() {
                        self.refresh_weather_now();
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.cfg.weather_use_system_location,
                        "Use system location",
                    )
                    .on_hover_text(self.hover_text("weather_use_system_location"));
                });
                if !self.cfg.weather_use_system_location {
                    ui.horizontal(|ui| {
                        ui.label("Location");
                        ui.text_edit_singleline(&mut self.cfg.weather_location_override)
                            .on_hover_text(self.hover_text("weather_location_override"));
                    });
                }
            }
            LayoutElement::News => {
                ui.heading("News Widget Settings");
                ui.checkbox(&mut self.cfg.show_news_layer, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_pos_y).speed(1));
                    ui.label("Size (16:9)");
                    let mut selected = current_news_size_id(
                        self.cfg.news_widget_width,
                        self.cfg.news_widget_height,
                    )
                    .to_string();
                    egui::ComboBox::from_id_salt("news_size_ordering")
                        .selected_text(news_size_label(selected.as_str()))
                        .show_ui(ui, |ui| {
                            for (id, label, _, _) in news_size_presets() {
                                ui.selectable_value(&mut selected, (*id).to_string(), *label);
                            }
                        })
                        .response
                        .on_hover_text(self.hover_text("widget_size"));
                    if let Some((_, _, w, h)) = news_size_presets()
                        .iter()
                        .copied()
                        .find(|(id, _, _, _)| *id == selected)
                    {
                        self.cfg.news_widget_width = w;
                        self.cfg.news_widget_height = h;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Source");
                    egui::ComboBox::from_id_salt("news_source_ordering")
                        .selected_text(news_source_label(&self.cfg.news_source))
                        .show_ui(ui, |ui| {
                            for &(id, label) in news_sources() {
                                ui.selectable_value(
                                    &mut self.cfg.news_source,
                                    id.to_string(),
                                    label,
                                );
                            }
                        })
                        .response
                        .on_hover_text(self.hover_text("news_source"));
                });
                if self.cfg.news_source == "custom" {
                    ui.horizontal(|ui| {
                        ui.label("Custom URL");
                        ui.text_edit_singleline(&mut self.cfg.news_custom_url)
                            .on_hover_text(self.hover_text("news_custom_url"));
                    });
                    if is_camera_like_url(&self.cfg.news_custom_url) && self.cfg.news_fps < 15.0 {
                        self.cfg.news_fps = 15.0;
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 100),
                            "Camera source detected: minimum FPS raised to 15.0",
                        );
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("FPS");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_fps)
                            .speed(0.1)
                            .range(0.05..=30.0),
                    )
                    .on_hover_text(self.hover_text("news_fps"));
                    ui.label("Refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_refresh_seconds)
                            .speed(5)
                            .range(10..=3600),
                    );
                    ui.checkbox(&mut self.cfg.news_audio_enabled, "Audio")
                        .on_hover_text(self.hover_text("news_audio_enabled"));
                });
                ui.separator();
                ui.heading("Secondary Ticker");
                ui.checkbox(&mut self.cfg.show_news_ticker2, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_ticker2_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_ticker2_pos_y).speed(1));
                    ui.label("Width");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_ticker2_width)
                            .speed(4)
                            .range(220..=1920),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Source");
                    egui::ComboBox::from_id_salt("news_ticker2_source_ordering")
                        .selected_text(news_source_label(&self.cfg.news_ticker2_source))
                        .show_ui(ui, |ui| {
                            for &(id, label) in news_sources() {
                                ui.selectable_value(
                                    &mut self.cfg.news_ticker2_source,
                                    id.to_string(),
                                    label,
                                );
                            }
                        });
                });
                if self.cfg.news_ticker2_source == "custom" {
                    ui.horizontal(|ui| {
                        ui.label("Custom URL");
                        ui.text_edit_singleline(&mut self.cfg.news_ticker2_custom_url)
                            .on_hover_text(self.hover_text("news_custom_url"));
                    });
                    if is_camera_like_url(&self.cfg.news_ticker2_custom_url)
                        && self.cfg.news_ticker2_fps < 15.0
                    {
                        self.cfg.news_ticker2_fps = 15.0;
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Ticker speed");
                    ui.monospace("Auto (reading-speed)");
                    ui.label("Ticker refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_ticker2_refresh_seconds)
                            .speed(5)
                            .range(10..=3600),
                    );
                });
            }
            LayoutElement::Cams => {
                ui.heading("Cams Widget Settings");
                ui.checkbox(&mut self.cfg.show_cams_layer, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.cams_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.cams_pos_y).speed(1));
                    ui.label("W");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_widget_width)
                            .speed(2)
                            .range(240..=1920),
                    )
                    .on_hover_text(self.hover_text("widget_size"));
                    ui.label("H");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_widget_height)
                            .speed(2)
                            .range(140..=1080),
                    )
                    .on_hover_text(self.hover_text("widget_size"));
                });
                ui.horizontal(|ui| {
                    ui.label("Source");
                    egui::ComboBox::from_id_salt("cams_source_ordering")
                        .selected_text(match self.cfg.cams_source.as_str() {
                            "city_public" => "City public",
                            "custom" => "Custom URLs",
                            _ => "Auto local/public",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.cfg.cams_source,
                                "auto_local".to_string(),
                                "Auto local/public",
                            );
                            ui.selectable_value(
                                &mut self.cfg.cams_source,
                                "city_public".to_string(),
                                "City public",
                            );
                            ui.selectable_value(
                                &mut self.cfg.cams_source,
                                "custom".to_string(),
                                "Custom URLs",
                            );
                        });
                    ui.label("Count");
                    if ui.button("-").clicked() {
                        self.cfg.cams_count = self.cfg.cams_count.saturating_sub(1).max(1);
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_count)
                            .speed(1)
                            .range(1..=5),
                    );
                    if ui.button("+").clicked() {
                        self.cfg.cams_count = self.cfg.cams_count.saturating_add(1).clamp(1, 5);
                    }
                    ui.label("Columns");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_columns)
                            .speed(1)
                            .range(1..=4),
                    );
                    ui.label("FPS");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_fps)
                            .speed(0.1)
                            .range(0.05..=30.0),
                    );
                    ui.label("Refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_refresh_seconds)
                            .speed(5)
                            .range(10..=3600),
                    );
                });
            }
        }

        ui.separator();
        ui.label("Click a neon box to edit it. Drag inside the frame to place selected element.");
    }

    fn render_weather_tab(&mut self, ui: &mut egui::Ui) {
        let color_help = self.hover_text("color_format").to_string();
        settings_section(
            ui,
            "Weather Widget",
            "Live weather data with controllable refresh budget and source location mode.",
            |ui| {
                ui.checkbox(&mut self.cfg.show_weather_layer, "Enable weather widget");
                ui.horizontal(|ui| {
                    ui.label("Refresh seconds");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_refresh_seconds)
                            .speed(10)
                            .range(60..=3600),
                    )
                    .on_hover_text(self.hover_text("weather_refresh_seconds"));
                    if ui.button("Refresh now").clicked() {
                        self.refresh_weather_now();
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.cfg.weather_use_system_location,
                        "Use system location",
                    )
                    .on_hover_text(self.hover_text("weather_use_system_location"));
                });
                if !self.cfg.weather_use_system_location {
                    ui.horizontal(|ui| {
                        ui.label("Location override");
                        ui.text_edit_singleline(&mut self.cfg.weather_location_override)
                            .on_hover_text(self.hover_text("weather_location_override"));
                    });
                }
            },
        );

        ui.add_space(8.0);
        settings_section(
            ui,
            "Layout & Style",
            "Placement, dimensions and visual style for the weather overlay.",
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Position X");
                    ui.add(egui::DragValue::new(&mut self.cfg.weather_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.weather_pos_y).speed(1));
                    ui.label("W");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_widget_width)
                            .speed(2)
                            .range(120..=1920),
                    );
                    ui.label("H");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_widget_height)
                            .speed(2)
                            .range(80..=1080),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Font family");
                    egui::ComboBox::from_id_salt("weather_font_family_tab")
                        .selected_text(&self.cfg.weather_font_family)
                        .show_ui(ui, |ui| {
                            for family in [
                                "DejaVu-Sans",
                                "Noto-Sans",
                                "Liberation-Sans",
                                "Serif",
                                "Monospace",
                            ] {
                                ui.selectable_value(
                                    &mut self.cfg.weather_font_family,
                                    family.to_string(),
                                    family,
                                );
                            }
                        });
                    ui.label("Text size");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_font_size)
                            .speed(1)
                            .range(10..=220),
                    );
                    ui.label("Stroke");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.weather_stroke_width)
                            .speed(1)
                            .range(0..=20),
                    );
                });
                ui.horizontal(|ui| {
                    edit_color_field(
                        ui,
                        "Text color",
                        &mut self.cfg.weather_color,
                        false,
                        &color_help,
                    );
                    edit_color_field(
                        ui,
                        "Undercolor",
                        &mut self.cfg.weather_undercolor,
                        true,
                        &color_help,
                    );
                    edit_color_field(
                        ui,
                        "Stroke color",
                        &mut self.cfg.weather_stroke_color,
                        false,
                        &color_help,
                    );
                });
            },
        );

        ui.add_space(8.0);
        settings_section(
            ui,
            "Live Snapshot",
            "Current snapshot state and provider diagnostics.",
            |ui| {
                ui.label(&self.weather_status);
                for line in &self.weather_details {
                    ui.label(line);
                }
            },
        );
    }

    fn render_news_tab(&mut self, ui: &mut egui::Ui) {
        settings_section(
            ui,
            "Primary Stream",
            "Configurable live/news stream source in fixed 16:9 widget frame.",
            |ui| {
                ui.checkbox(&mut self.cfg.show_news_layer, "Enable news widget");
                ui.horizontal(|ui| {
                    ui.label("Channel");
                    egui::ComboBox::from_id_salt("news_source_tab")
                        .selected_text(news_source_label(&self.cfg.news_source))
                        .show_ui(ui, |ui| {
                            for &(id, label) in news_sources() {
                                ui.selectable_value(
                                    &mut self.cfg.news_source,
                                    id.to_string(),
                                    label,
                                );
                            }
                        })
                        .response
                        .on_hover_text(self.hover_text("news_source"));
                });
                if self.cfg.news_source == "custom" {
                    ui.horizontal(|ui| {
                        ui.label("Custom URL");
                        ui.text_edit_singleline(&mut self.cfg.news_custom_url)
                            .on_hover_text(self.hover_text("news_custom_url"));
                    });
                    if is_camera_like_url(&self.cfg.news_custom_url) && self.cfg.news_fps < 15.0 {
                        self.cfg.news_fps = 15.0;
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 100),
                            "Camera source detected: minimum FPS raised to 15.0",
                        );
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Playback FPS");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_fps)
                            .speed(0.1)
                            .range(0.05..=30.0),
                    )
                    .on_hover_text(self.hover_text("news_fps"));
                    ui.label("Refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_refresh_seconds)
                            .speed(5)
                            .range(10..=3600),
                    );
                    ui.checkbox(&mut self.cfg.news_audio_enabled, "Audio")
                        .on_hover_text(self.hover_text("news_audio_enabled"));
                });
                ui.horizontal(|ui| {
                    ui.label("Position X");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_pos_y).speed(1));
                    ui.label("Size (16:9)");
                    let mut selected = current_news_size_id(
                        self.cfg.news_widget_width,
                        self.cfg.news_widget_height,
                    )
                    .to_string();
                    egui::ComboBox::from_id_salt("news_size_tab")
                        .selected_text(news_size_label(selected.as_str()))
                        .show_ui(ui, |ui| {
                            for (id, label, _, _) in news_size_presets() {
                                ui.selectable_value(&mut selected, (*id).to_string(), *label);
                            }
                        });
                    if let Some((_, _, w, h)) = news_size_presets()
                        .iter()
                        .copied()
                        .find(|(id, _, _, _)| *id == selected)
                    {
                        self.cfg.news_widget_width = w;
                        self.cfg.news_widget_height = h;
                    }
                });
            },
        );

        ui.add_space(8.0);
        settings_section(ui, "Resolved Stream URL", "", |ui| {
            ui.monospace(news_source_url(
                &self.cfg.news_source,
                &self.cfg.news_custom_url,
            ));
        });

        ui.add_space(8.0);
        settings_section(
            ui,
            "Secondary Ticker",
            "Independent ticker stream with own source, limits, and placement.",
            |ui| {
                ui.checkbox(&mut self.cfg.show_news_ticker2, "Enable secondary ticker");
                ui.horizontal(|ui| {
                    ui.label("Ticker source");
                    egui::ComboBox::from_id_salt("news_ticker2_source_tab")
                        .selected_text(news_source_label(&self.cfg.news_ticker2_source))
                        .show_ui(ui, |ui| {
                            for &(id, label) in news_sources() {
                                ui.selectable_value(
                                    &mut self.cfg.news_ticker2_source,
                                    id.to_string(),
                                    label,
                                );
                            }
                        });
                });
                if self.cfg.news_ticker2_source == "custom" {
                    ui.horizontal(|ui| {
                        ui.label("Ticker custom URL");
                        ui.text_edit_singleline(&mut self.cfg.news_ticker2_custom_url)
                            .on_hover_text(self.hover_text("news_custom_url"));
                    });
                    if is_camera_like_url(&self.cfg.news_ticker2_custom_url)
                        && self.cfg.news_ticker2_fps < 15.0
                    {
                        self.cfg.news_ticker2_fps = 15.0;
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 180, 100),
                            "Camera source detected: ticker minimum FPS raised to 15.0",
                        );
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Ticker speed");
                    ui.monospace("Auto (reading-speed)");
                    ui.label("Ticker refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_ticker2_refresh_seconds)
                            .speed(5)
                            .range(10..=3600),
                    );
                    ui.label("Ticker width");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_ticker2_width)
                            .speed(4)
                            .range(220..=1920),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Ticker X");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_ticker2_pos_x).speed(1));
                    ui.label("Ticker Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.news_ticker2_pos_y).speed(1));
                });
                ui.label("Ticker source URL:");
                ui.monospace(news_source_url(
                    &self.cfg.news_ticker2_source,
                    &self.cfg.news_ticker2_custom_url,
                ));
            },
        );
    }

    fn render_cams_tab(&mut self, ui: &mut egui::Ui) {
        settings_section(
            ui,
            "Source & Presets",
            "Public/private camera feeds with optional custom URL lists.",
            |ui| {
                ui.checkbox(&mut self.cfg.show_cams_layer, "Enable cams widget");
                ui.horizontal(|ui| {
                    ui.label("Source mode");
                    egui::ComboBox::from_id_salt("cams_source_tab")
                        .selected_text(match self.cfg.cams_source.as_str() {
                            "city_public" => "City public",
                            "custom" => "Custom URLs",
                            _ => "Auto local/public",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.cfg.cams_source,
                                "auto_local".to_string(),
                                "Auto local/public",
                            );
                            ui.selectable_value(
                                &mut self.cfg.cams_source,
                                "city_public".to_string(),
                                "City public",
                            );
                            ui.selectable_value(
                                &mut self.cfg.cams_source,
                                "custom".to_string(),
                                "Custom URLs",
                            );
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Public preset");
                    egui::ComboBox::from_id_salt("cams_public_choice_tab")
                        .selected_text(match self.cams_public_choice.as_str() {
                            "belgrade_center" => "Capital starter mix",
                            "europe_mix" => "Europe capitals mix",
                            "world_mix" => "Global capitals mix",
                            _ => "Capital starter mix",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.cams_public_choice,
                                "belgrade_center".to_string(),
                                "Capital starter mix",
                            );
                            ui.selectable_value(
                                &mut self.cams_public_choice,
                                "europe_mix".to_string(),
                                "Europe capitals mix",
                            );
                            ui.selectable_value(
                                &mut self.cams_public_choice,
                                "world_mix".to_string(),
                                "Global capitals mix",
                            );
                        });
                    if ui.button("Apply preset URLs").clicked() {
                        self.cfg.cams_source = "custom".to_string();
                        self.cfg.cams_custom_urls =
                            cams_public_preset_urls(&self.cams_public_choice);
                    }
                });
                if self.cfg.cams_source == "custom" {
                    ui.label("Custom cam URLs (one per line). Optional format: Label => URL");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.cfg.cams_custom_urls)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY),
                    );
                }
            },
        );

        ui.add_space(8.0);
        settings_section(
            ui,
            "Grid & Performance",
            "Visible camera count, grid shape, FPS, and refresh budget controls.",
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Cams shown");
                    if ui.button("-").clicked() {
                        self.cfg.cams_count = self.cfg.cams_count.saturating_sub(1).max(1);
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_count)
                            .speed(1)
                            .range(1..=5),
                    );
                    if ui.button("+").clicked() {
                        self.cfg.cams_count = self.cfg.cams_count.saturating_add(1).clamp(1, 5);
                    }
                    ui.label("Columns");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_columns)
                            .speed(1)
                            .range(1..=4),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Cams FPS");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_fps)
                            .speed(0.1)
                            .range(0.05..=30.0),
                    );
                    ui.label("Refresh sec");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_refresh_seconds)
                            .speed(5)
                            .range(10..=3600),
                    );
                });
            },
        );

        ui.add_space(8.0);
        settings_section(
            ui,
            "Placement",
            "Absolute overlay placement and dimensions on the render canvas.",
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Position X");
                    ui.add(egui::DragValue::new(&mut self.cfg.cams_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.cams_pos_y).speed(1));
                    ui.label("W");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_widget_width)
                            .speed(2)
                            .range(240..=1920),
                    );
                    ui.label("H");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.cams_widget_height)
                            .speed(2)
                            .range(140..=1080),
                    );
                });
            },
        );
    }

    fn render_system_tab(&mut self, ui: &mut egui::Ui) {
        settings_section(
            ui,
            "Runtime & Updates",
            "Loop timing status and package update controls.",
            |ui| {
                ui.horizontal(|ui| {
                    self.cfg.refresh_seconds = self.cfg.image_refresh_seconds.max(1);
                    self.cfg.quote_refresh_seconds = self.cfg.image_refresh_seconds.max(1);
                    ui.label("Master timer (from Images tab)");
                    ui.monospace(format!("{}s", self.cfg.image_refresh_seconds));
                });
                ui.horizontal(|ui| {
                    let checking = self.update_check_rx.is_some();
                    let self_updating = self.self_update_rx.is_some();
                    if ui
                        .add_enabled(!checking, egui::Button::new("Check Updates"))
                        .clicked()
                    {
                        self.start_update_check();
                    }
                    if self.update_release.is_some()
                        && ui
                            .add_enabled(!self_updating, egui::Button::new("Update Now"))
                            .clicked()
                    {
                        self.start_self_update();
                    }
                    if let Some(release) = self.update_release.clone() {
                        ui.hyperlink_to("Release Notes", release.html_url);
                    }
                });
                ui.monospace(&self.update_status);
            },
        );

        ui.add_space(8.0);
        settings_section(
            ui,
            "Autostart",
            "Control automatic startup behavior and install/remove desktop autostart entry.",
            |ui| {
                ui.horizontal(|ui| {
                    let response = ui
                        .checkbox(
                            &mut self.autostart_toggle,
                            "Start automatically after login",
                        )
                        .on_hover_text(self.hover_text("autostart_enable"));
                    if response.changed() {
                        self.sync_autostart_toggle();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(if Self::autostart_enabled() {
                        "Status: enabled"
                    } else {
                        "Status: disabled"
                    });
                    if ui.button("Install Autostart").clicked() {
                        self.install_autostart();
                    }
                    if ui.button("Remove Autostart").clicked() {
                        self.remove_autostart();
                    }
                });
            },
        );

        ui.add_space(8.0);
        settings_section(
            ui,
            "Integrations",
            "Login/boot integration switches and shared widget channel controls.",
            |ui| {
                ui.checkbox(
                    &mut self.cfg.login_screen_integration,
                    "Enable login-screen integration",
                )
                .on_hover_text(self.hover_text("login_screen_integration"));
                ui.checkbox(
                    &mut self.cfg.boot_screen_integration,
                    "Enable boot-screen integration",
                )
                .on_hover_text(self.hover_text("boot_screen_integration"));

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("News channel");
                    egui::ComboBox::from_id_salt("news_source_system")
                        .selected_text(news_source_label(&self.cfg.news_source))
                        .show_ui(ui, |ui| {
                            for &(id, label) in news_sources() {
                                ui.selectable_value(
                                    &mut self.cfg.news_source,
                                    id.to_string(),
                                    label,
                                );
                            }
                        })
                        .response
                        .on_hover_text(self.hover_text("news_source"));
                });
                if self.cfg.news_source == "custom" {
                    ui.horizontal(|ui| {
                        ui.label("Custom URL");
                        ui.text_edit_singleline(&mut self.cfg.news_custom_url);
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("News FPS");
                    ui.add(
                        egui::DragValue::new(&mut self.cfg.news_fps)
                            .speed(0.1)
                            .range(0.05..=30.0),
                    )
                    .on_hover_text(self.hover_text("news_fps"));
                    ui.checkbox(&mut self.cfg.news_audio_enabled, "Audio")
                        .on_hover_text(self.hover_text("news_audio_enabled"));
                });
            },
        );

        ui.add_space(8.0);
        settings_section(ui, "Support the Team", "", |ui| {
            ui.label("If Le Compositeur helps you, you can support diceteachbeograd-Team.");
            ui.label("XRP/Monero-style address:");
            ui.monospace("raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5");
            ui.label("Litecoin address:");
            ui.monospace("LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt");
            ui.horizontal(|ui| {
                ui.label("QR (XRP):");
                ui.hyperlink_to(
                    "open",
                    "https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=raRPBVcyRzfs4QsVMUK4UczYM4SaepuMr5",
                );
            });
            ui.horizontal(|ui| {
                ui.label("QR (LTC):");
                ui.hyperlink_to(
                    "open",
                    "https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=LLBCyZ3PwdprKYkuegouxkSbGfQxa7z9Rt",
                );
            });
        });
    }
}

#[cfg(target_os = "linux")]
fn pick_linux_path_dialog(start: &Path, directory: bool) -> Option<PathBuf> {
    run_zenity_picker(start, directory).or_else(|| run_kdialog_picker(start, directory))
}

#[cfg(target_os = "linux")]
fn run_zenity_picker(start: &Path, directory: bool) -> Option<PathBuf> {
    let mut cmd = Command::new("zenity");
    cmd.arg("--file-selection");
    if directory {
        cmd.arg("--directory");
    }
    cmd.arg("--filename").arg(format!("{}/", start.display()));
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

#[cfg(target_os = "linux")]
fn run_kdialog_picker(start: &Path, directory: bool) -> Option<PathBuf> {
    let mut cmd = Command::new("kdialog");
    if directory {
        cmd.arg("--getexistingdirectory");
    } else {
        cmd.arg("--getopenfilename");
    }
    cmd.arg(start.display().to_string());
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn shell_quote_single(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

fn fetch_latest_release_info() -> Result<ReleaseInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| format!("client init failed: {e}"))?;
    let payload = client
        .get("https://api.github.com/repos/diceteachbeograd-Team/Le-Compositeur/releases/latest")
        .header(reqwest::header::USER_AGENT, "le-compositeur-gui")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(|e| format!("request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("release endpoint error: {e}"))?
        .json::<serde_json::Value>()
        .map_err(|e| format!("response decode failed: {e}"))?;

    let tag = payload
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if tag.is_empty() {
        return Err("missing tag_name in release payload".to_string());
    }
    let html_url = payload
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/diceteachbeograd-Team/Le-Compositeur/releases/latest")
        .to_string();
    Ok(ReleaseInfo { tag, html_url })
}

fn is_newer_release(latest: &str, current: &str) -> bool {
    let latest_parts = numeric_version_parts(latest);
    let current_parts = numeric_version_parts(current);
    if latest_parts.is_empty() || current_parts.is_empty() {
        return latest.trim() != current.trim();
    }
    let len = latest_parts.len().max(current_parts.len());
    for idx in 0..len {
        let a = *latest_parts.get(idx).unwrap_or(&0);
        let b = *current_parts.get(idx).unwrap_or(&0);
        if a > b {
            return true;
        }
        if a < b {
            return false;
        }
    }
    false
}

fn numeric_version_parts(raw: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in raw.trim().trim_start_matches(['v', 'V']).chars() {
        if ch.is_ascii_digit() {
            buf.push(ch);
            continue;
        }
        if !buf.is_empty() {
            out.push(buf.parse::<u32>().unwrap_or(0));
            buf.clear();
        }
    }
    if !buf.is_empty() {
        out.push(buf.parse::<u32>().unwrap_or(0));
    }
    out
}

#[cfg(target_os = "linux")]
fn command_exists(name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(name).is_file())
}

#[cfg(target_os = "linux")]
fn summarize_process_output(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw)
        .replace('\r', "\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join(" | ");
    if text.len() <= 240 {
        return text;
    }
    format!("{}...", &text[..240])
}

#[cfg(target_os = "linux")]
fn wait_self_update_result(child: Child, backend: &str) -> Result<String, String> {
    let output = child
        .wait_with_output()
        .map_err(|err| format!("Self-update process failed to wait ({backend}): {err}"))?;
    if output.status.success() {
        return Ok(format!(
            "Self-update finished via {backend}. Restart the app to load the new package."
        ));
    }
    let details = summarize_process_output(&output.stderr);
    if details.is_empty() {
        return Err(format!(
            "Self-update failed via {backend} (exit {}).",
            output.status
        ));
    }
    Err(format!(
        "Self-update failed via {backend} (exit {}): {details}",
        output.status
    ))
}

fn open_url(url: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
        false
    }
}

fn news_sources() -> &'static [(&'static str, &'static str)] {
    &[
        ("euronews", "News: Euronews Live"),
        ("aljazeera", "News: Al Jazeera English"),
        ("france24", "News: France24 English"),
        ("dw", "News: DW News"),
        ("yahoo_finance", "Boerse: Yahoo Finance Live"),
        ("bloomberg_tv", "Boerse: Bloomberg TV"),
        ("techcrunch", "Tech: TechCrunch Live"),
        ("theverge", "Tech: The Verge"),
        ("nasa_tv", "Docs: NASA TV"),
        ("documentary_heaven", "Docs: DocumentaryHeaven"),
        ("custom", "Custom URL"),
    ]
}

fn news_size_presets() -> &'static [(&'static str, &'static str, u32, u32)] {
    &[
        ("xs", "XS (320x180)", 320, 180),
        ("s", "S (480x270)", 480, 270),
        ("m", "M (640x360)", 640, 360),
        ("l", "L (800x450)", 800, 450),
        ("xl", "XL (960x540)", 960, 540),
        ("hd", "HD (1280x720)", 1280, 720),
    ]
}

fn news_size_label(id: &str) -> &'static str {
    news_size_presets()
        .iter()
        .find_map(|(k, label, _, _)| if *k == id { Some(*label) } else { None })
        .unwrap_or("M (640x360)")
}

fn current_news_size_id(width: u32, height: u32) -> &'static str {
    if let Some((id, _, _, _)) = news_size_presets()
        .iter()
        .find(|(_, _, w, h)| *w == width && *h == height)
    {
        return id;
    }
    let (w, h) = nearest_news_size_preset(width, height);
    news_size_presets()
        .iter()
        .find_map(|(id, _, ww, hh)| {
            if *ww == w && *hh == h {
                Some(*id)
            } else {
                None
            }
        })
        .unwrap_or("m")
}

fn nearest_news_size_preset(width: u32, height: u32) -> (u32, u32) {
    let mut best = (640, 360);
    let mut best_score = u64::MAX;
    for (_, _, w, h) in news_size_presets() {
        let dw = width.abs_diff(*w) as u64;
        let dh = height.abs_diff(*h) as u64;
        let score = dw.saturating_mul(dw).saturating_add(dh.saturating_mul(dh));
        if score < best_score {
            best_score = score;
            best = (*w, *h);
        }
    }
    best
}

fn news_source_label(id: &str) -> &'static str {
    news_sources()
        .iter()
        .find_map(|(k, v)| if *k == id { Some(*v) } else { None })
        .unwrap_or("Custom URL")
}

fn news_source_url(id: &str, custom: &str) -> String {
    match id {
        "euronews" => "https://www.youtube.com/watch?v=pykpO5kQJ98".to_string(),
        "aljazeera" => "https://www.youtube.com/watch?v=gCNeDWCI0vo".to_string(),
        "france24" => "https://www.youtube.com/watch?v=l8PMl7tUDIE".to_string(),
        "dw" => "https://www.youtube.com/watch?v=GE_SfNVNyqk".to_string(),
        "yahoo_finance" => "https://www.youtube.com/watch?v=9Auq9mYxFEE".to_string(),
        "bloomberg_tv" => "https://www.youtube.com/watch?v=dp8PhLsUcFE".to_string(),
        "techcrunch" => "https://techcrunch.com/".to_string(),
        "theverge" => "https://www.theverge.com/tech".to_string(),
        "nasa_tv" => "https://www.youtube.com/watch?v=21X5lGlDOfg".to_string(),
        "documentary_heaven" => "https://documentaryheaven.com/".to_string(),
        _ => custom.to_string(),
    }
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

fn weather_code_label(code: i64) -> &'static str {
    match code {
        0 => "Clear sky",
        1..=3 => "Partly cloudy",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 | 77 => "Snow",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

fn parse_lat_lon_from_geocode(
    client: &reqwest::blocking::Client,
    query: &str,
) -> Option<(f64, f64)> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let safe_query = query.replace(' ', "+");
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={safe_query}&count=1&language=en&format=json"
    );
    let geo = client
        .get(url)
        .send()
        .ok()?
        .json::<serde_json::Value>()
        .ok()?;
    let first = geo.get("results")?.as_array()?.first()?;
    let lat = first.get("latitude")?.as_f64()?;
    let lon = first.get("longitude")?.as_f64()?;
    Some((lat, lon))
}

fn parse_lat_lon_pair(raw: &str) -> Option<(f64, f64)> {
    let mut parts = raw.split(',');
    let lat = parts.next()?.trim().parse::<f64>().ok()?;
    let lon = parts.next()?.trim().parse::<f64>().ok()?;
    Some((lat, lon))
}

fn lookup_system_location_snapshot(
    client: &reqwest::blocking::Client,
) -> Result<(f64, f64, String, String), String> {
    let mut errors = Vec::<String>::new();

    let ipapi = client
        .get("https://ipapi.co/json/")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())
        .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()));
    match ipapi {
        Ok(geo) => {
            if let (Some(lat), Some(lon)) = (
                geo.get("latitude").and_then(|v| v.as_f64()),
                geo.get("longitude").and_then(|v| v.as_f64()),
            ) {
                let city = geo
                    .get("city")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown city");
                let country = geo
                    .get("country_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown country");
                return Ok((
                    lat,
                    lon,
                    format!("{city}, {country}"),
                    "ipapi.co".to_string(),
                ));
            }
            errors.push("ipapi.co: missing coordinates".to_string());
        }
        Err(e) => errors.push(format!("ipapi.co: {e}")),
    }

    let ipwho = client
        .get("https://ipwho.is/")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())
        .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()));
    match ipwho {
        Ok(geo) => {
            let success = geo.get("success").and_then(|v| v.as_bool()).unwrap_or(true);
            if success {
                if let (Some(lat), Some(lon)) = (
                    geo.get("latitude").and_then(|v| v.as_f64()),
                    geo.get("longitude").and_then(|v| v.as_f64()),
                ) {
                    let city = geo
                        .get("city")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown city");
                    let country = geo
                        .get("country")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown country");
                    return Ok((
                        lat,
                        lon,
                        format!("{city}, {country}"),
                        "ipwho.is".to_string(),
                    ));
                }
                errors.push("ipwho.is: missing coordinates".to_string());
            } else {
                errors.push("ipwho.is: success=false".to_string());
            }
        }
        Err(e) => errors.push(format!("ipwho.is: {e}")),
    }

    let ipinfo = client
        .get("https://ipinfo.io/json")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())
        .and_then(|r| r.json::<serde_json::Value>().map_err(|e| e.to_string()));
    match ipinfo {
        Ok(geo) => {
            if let Some((lat, lon)) = geo
                .get("loc")
                .and_then(|v| v.as_str())
                .and_then(parse_lat_lon_pair)
            {
                let city = geo
                    .get("city")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown city");
                let country = geo
                    .get("country")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown country");
                return Ok((
                    lat,
                    lon,
                    format!("{city}, {country}"),
                    "ipinfo.io".to_string(),
                ));
            }
            errors.push("ipinfo.io: missing loc".to_string());
        }
        Err(e) => errors.push(format!("ipinfo.io: {e}")),
    }

    Err(format!(
        "location lookup failed across providers: {}",
        errors.join(" | ")
    ))
}

fn fetch_weather_snapshot(cfg: &AppConfig) -> Result<(String, Vec<String>), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("http init failed: {e}"))?;

    let (lat, lon, location_name, geo_provider) = if cfg.weather_use_system_location {
        match lookup_system_location_snapshot(&client) {
            Ok(v) => v,
            Err(primary_err) => {
                let manual = cfg.weather_location_override.trim();
                if let Some((lat, lon)) = parse_lat_lon_from_geocode(&client, manual) {
                    (
                        lat,
                        lon,
                        manual.to_string(),
                        "manual geocode fallback".to_string(),
                    )
                } else {
                    return fetch_weather_snapshot_wttr(&client)
                        .map_err(|wttr_err| format!("{primary_err}; {wttr_err}"));
                }
            }
        }
    } else if let Some((lat, lon)) =
        parse_lat_lon_from_geocode(&client, &cfg.weather_location_override)
    {
        (
            lat,
            lon,
            cfg.weather_location_override.clone(),
            "manual geocode".to_string(),
        )
    } else {
        return Err("manual location could not be resolved (use e.g. Belgrade,RS)".to_string());
    };

    let weather_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m,wind_direction_10m,precipitation&timezone=auto"
    );
    let payload = client
        .get(weather_url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("weather request failed: {e}"))?
        .json::<serde_json::Value>()
        .map_err(|e| format!("weather payload invalid: {e}"))?;

    let current = payload
        .get("current")
        .ok_or("weather payload has no 'current' field")?;
    let temp = current
        .get("temperature_2m")
        .and_then(|v| v.as_f64())
        .ok_or("weather payload misses temperature_2m")?;
    let apparent = current
        .get("apparent_temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(temp);
    let humidity = current
        .get("relative_humidity_2m")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let wind_speed = current
        .get("wind_speed_10m")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let wind_dir = current
        .get("wind_direction_10m")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let precipitation = current
        .get("precipitation")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let code = current
        .get("weather_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    let condition = weather_code_label(code);

    let headline = format!("{location_name}: {temp:.1}°C ({condition}) | feels {apparent:.1}°C");
    let details = vec![
        format!("Humidity: {humidity:.0}%"),
        format!("Wind: {wind_speed:.1} km/h @ {wind_dir:.0}°"),
        format!("Precipitation: {precipitation:.1} mm"),
        format!("Source: Open-Meteo + {geo_provider}"),
    ];
    Ok((headline, details))
}

fn fetch_weather_snapshot_wttr(
    client: &reqwest::blocking::Client,
) -> Result<(String, Vec<String>), String> {
    let payload = client
        .get("https://wttr.in/?format=j1")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("wttr request failed: {e}"))?
        .json::<serde_json::Value>()
        .map_err(|e| format!("wttr payload invalid: {e}"))?;

    let current = payload
        .get("current_condition")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .ok_or("wttr payload has no current_condition")?;

    let area = payload
        .get("nearest_area")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first());
    let city = area
        .and_then(|a| a.get("areaName"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("Auto location");
    let country = area
        .and_then(|a| a.get("country"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let desc = current
        .get("weatherDesc")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let temp = current
        .get("temp_C")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let apparent = current
        .get("FeelsLikeC")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(temp);
    let humidity = current
        .get("humidity")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let wind_speed = current
        .get("windspeedKmph")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let headline = format!("{city}, {country}: {temp:.1}°C ({desc}) | feels {apparent:.1}°C");
    let details = vec![
        format!("Humidity: {humidity:.0}%"),
        format!("Wind: {wind_speed:.1} km/h"),
        "Source: wttr.in fallback".to_string(),
    ];
    Ok((headline, details))
}

fn apply_visual_system(ctx: &egui::Context, compact: bool) {
    let mut style = (*ctx.style()).clone();
    if compact {
        style.spacing.item_spacing = egui::vec2(7.0, 5.0);
        style.spacing.button_padding = egui::vec2(9.0, 4.0);
        style.spacing.menu_margin = egui::Margin::symmetric(6, 4);
        style.spacing.window_margin = egui::Margin::symmetric(8, 8);
        style.spacing.indent = 16.0;
    } else {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.menu_margin = egui::Margin::symmetric(8, 6);
        style.spacing.window_margin = egui::Margin::symmetric(10, 10);
        style.spacing.indent = 20.0;
    }

    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(if compact { 17.0 } else { 20.0 }),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(if compact { 13.5 } else { 15.0 }),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(if compact { 12.5 } else { 14.0 }),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(if compact { 12.0 } else { 13.5 }),
    );

    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(230, 237, 243));
    visuals.window_fill = egui::Color32::from_rgb(8, 13, 18);
    visuals.panel_fill = egui::Color32::from_rgb(10, 16, 22);
    visuals.faint_bg_color = egui::Color32::from_rgb(16, 24, 32);
    visuals.extreme_bg_color = egui::Color32::from_rgb(5, 10, 15);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(12, 19, 27);
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_gray(170));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(18, 29, 41);
    visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(195, 208, 219));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(24, 41, 58);
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(225, 237, 246));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(24, 81, 105);
    visuals.widgets.active.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(240, 250, 255));
    visuals.selection.bg_fill = egui::Color32::from_rgb(18, 96, 127);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(111, 237, 250));
    visuals.hyperlink_color = egui::Color32::from_rgb(124, 225, 255);
    visuals.warn_fg_color = egui::Color32::from_rgb(255, 193, 90);
    visuals.error_fg_color = egui::Color32::from_rgb(255, 110, 110);
    style.visuals = visuals;

    ctx.set_style(style);
}

fn detect_ui_lang() -> UiLang {
    let locale = std::env::var("LC_ALL")
        .ok()
        .or_else(|| std::env::var("LC_MESSAGES").ok())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if locale.starts_with("de") {
        return UiLang::De;
    }
    if locale.starts_with("sr") {
        return UiLang::Sr;
    }
    if locale.starts_with("zh") {
        return UiLang::Zh;
    }
    UiLang::En
}

fn ui_lang_label(lang: UiLang) -> &'static str {
    match lang {
        UiLang::En => "EN",
        UiLang::De => "DE",
        UiLang::Sr => "SR",
        UiLang::Zh => "ZH",
    }
}

impl eframe::App for WcGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_runner_state();
        self.poll_update_check();
        self.poll_self_update();
        if self.cfg.quote_min_font_size > self.cfg.quote_font_size {
            self.cfg.quote_min_font_size = self.cfg.quote_font_size;
        }
        self.enforce_news_widget_size_preset();
        if self.ui_style_compact_applied != Some(self.ui_compact_mode) {
            apply_visual_system(ctx, self.ui_compact_mode);
            self.ui_style_compact_applied = Some(self.ui_compact_mode);
        }

        if self.thumbnails.is_empty() || self.thumbnails_for_dir != self.cfg.image_dir {
            self.refresh_thumbnails(ctx);
        }
        if self.quote_preview.is_empty() {
            self.refresh_quotes_preview();
        }
        self.refresh_weather_if_needed();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("Session");
                    ui.separator();
                    ui.label("Config");
                    ui.text_edit_singleline(&mut self.config_path)
                        .on_hover_text(self.hover_text("config_path"));
                    if ui.button("Load").clicked() {
                        self.load_from_path(ctx);
                    }
                    if ui.button("Save").clicked() {
                        self.save_to_path();
                    }
                    ui.separator();
                    ui.label(format!("Language: {}", ui_lang_label(self.ui_lang)));
                    ui.checkbox(&mut self.ui_compact_mode, "Compact UI");
                    ui.checkbox(&mut self.show_preview_panel, "Preview Panel");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(app_version_label());
                    });
                });
            });

            ui.add_space(6.0);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Actions");
                    ui.separator();
                    ui.label(if self.runner.is_some() {
                        "Runner: ACTIVE"
                    } else {
                        "Runner: STOPPED"
                    });
                    if ui.button("Validate").clicked() {
                        self.run_wc_cli(&["validate"]);
                    }
                    if ui.button("Render Preview").clicked() {
                        self.run_wc_cli(&["render-preview"]);
                    }
                    if ui.button("Run Once").clicked() {
                        self.run_wc_cli(&["run", "--once"]);
                    }
                    if ui.button("Apply Now").clicked() {
                        self.apply_now();
                    }
                    if ui.button("Migrate").clicked() {
                        self.run_wc_cli(&["migrate"]);
                    }
                    if ui.button("Start Loop").clicked() {
                        self.start_runner();
                    }
                    if ui.button("Start Loop + Hide").clicked() {
                        self.start_runner();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    if ui.button("Run Detached").clicked() {
                        self.start_detached_runner();
                    }
                    if ui.button("Stop Loop").clicked() {
                        self.stop_runner();
                    }
                    if ui.button("Hide Window").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                });
            });

            ui.add_space(6.0);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Workspace");
                    ui.separator();
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::Ordering,
                        Self::tab_button_label(GuiTab::Ordering),
                    );
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::Images,
                        Self::tab_button_label(GuiTab::Images),
                    );
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::Quotes,
                        Self::tab_button_label(GuiTab::Quotes),
                    );
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::Weather,
                        Self::tab_button_label(GuiTab::Weather),
                    );
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::News,
                        Self::tab_button_label(GuiTab::News),
                    );
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::Cams,
                        Self::tab_button_label(GuiTab::Cams),
                    );
                    ui.selectable_value(
                        &mut self.active_tab,
                        GuiTab::System,
                        Self::tab_button_label(GuiTab::System),
                    );
                });
            });

            ui.add_space(6.0);
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Updates");
                    ui.separator();
                    let checking = self.update_check_rx.is_some();
                    let self_updating = self.self_update_rx.is_some();
                    if ui
                        .add_enabled(!checking, egui::Button::new("Check Updates"))
                        .clicked()
                    {
                        self.start_update_check();
                    }
                    if self.update_release.is_some()
                        && ui
                            .add_enabled(!self_updating, egui::Button::new("Update Now"))
                            .clicked()
                    {
                        self.start_self_update();
                    }
                    if let Some(release) = self.update_release.clone() {
                        ui.hyperlink_to("Release Notes", release.html_url);
                    }
                    ui.monospace(&self.update_status);
                });
            });
            ui.add_space(2.0);
        });

        if self.show_preview_panel {
            egui::SidePanel::right("thumbs")
                .default_width(320.0)
                .resizable(true)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.group(|ui| {
                            ui.heading("Image Preview");
                            ui.label("First 2-3 images from selected folder");
                            if ui.button("Refresh Preview Images").clicked() {
                                self.refresh_thumbnails(ctx);
                            }
                            ui.separator();

                            if self.thumbnails.is_empty() {
                                ui.label("No thumbnails available");
                            } else {
                                for item in &self.thumbnails {
                                    ui.label(&item.label);
                                    ui.image((item.texture.id(), egui::vec2(280.0, 158.0)));
                                    ui.separator();
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.group(|ui| {
                            ui.heading("Quote Preview");
                            if ui.button("Reload Quotes").clicked() {
                                self.refresh_quotes_preview();
                            }
                            for (i, q) in self.quote_preview.iter().enumerate() {
                                ui.label(format!("#{}", i + 1));
                                ui.label(q);
                                ui.separator();
                            }
                        });
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(self.active_tab_title());
            ui.label(self.active_tab_hint());
            ui.separator();
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.active_tab {
                    GuiTab::Ordering => self.render_ordering_tab(ui),
                    GuiTab::Images => self.render_images_tab(ui, ctx),
                    GuiTab::Quotes => self.render_quotes_tab(ui),
                    GuiTab::Weather => self.render_weather_tab(ui),
                    GuiTab::News => self.render_news_tab(ui),
                    GuiTab::Cams => self.render_cams_tab(ui),
                    GuiTab::System => self.render_system_tab(ui),
                });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong("Status");
                    ui.separator();
                    ui.label(if self.runner.is_some() {
                        "Loop process is running"
                    } else {
                        "Loop process is stopped"
                    });
                });
                ui.add(
                    egui::TextEdit::multiline(&mut self.status)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );
            });
        });
    }
}

impl Drop for WcGuiApp {
    fn drop(&mut self) {
        if let Some(mut child) = self.runner.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn load_thumbnail(
    ctx: &egui::Context,
    path: &Path,
    idx: usize,
) -> Result<egui::TextureHandle, String> {
    let img = image::open(path).map_err(|e| format!("decode failed: {e}"))?;
    let thumb = img.thumbnail(480, 270).to_rgba8();
    let size = [thumb.width() as usize, thumb.height() as usize];
    let pixels = thumb.into_raw();
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Ok(ctx.load_texture(
        format!("thumb-{idx}-{}", path.display()),
        color,
        egui::TextureOptions::LINEAR,
    ))
}

fn load_ordering_background_texture(
    ctx: &egui::Context,
    path: &Path,
) -> Result<egui::TextureHandle, String> {
    let img = image::open(path).map_err(|e| format!("decode failed: {e}"))?;
    let gray = img.thumbnail(1280, 720).grayscale().to_rgba8();
    let size = [gray.width() as usize, gray.height() as usize];
    let pixels = gray.into_raw();
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Ok(ctx.load_texture(
        format!("ordering-bg-{}", path.display()),
        color,
        egui::TextureOptions::LINEAR,
    ))
}

fn default_cfg() -> AppConfig {
    AppConfig {
        config_version: 1,
        image_dir: "~/Pictures/Wallpapers".to_string(),
        quotes_path: "~/Documents/wallpaper-composer/quotes.md".to_string(),
        image_source: "local".to_string(),
        image_source_url: None,
        image_source_preset: Some("picsum_random_hd".to_string()),
        quote_source: "local".to_string(),
        quote_source_url: None,
        quote_source_preset: Some("zenquotes_daily".to_string()),
        quote_format: "lines".to_string(),
        image_order_mode: "sequential".to_string(),
        image_avoid_repeat: true,
        quote_order_mode: "sequential".to_string(),
        quote_avoid_repeat: true,
        quote_font_size: 36,
        quote_pos_x: 80,
        quote_pos_y: 860,
        quote_auto_fit: true,
        quote_min_font_size: 18,
        font_family: "DejaVu-Sans".to_string(),
        quote_color: "#FFFFFF".to_string(),
        clock_font_size: 44,
        clock_pos_x: 1600,
        clock_pos_y: 960,
        clock_color: "#FFD700".to_string(),
        text_stroke_color: "#000000".to_string(),
        text_stroke_width: 2,
        text_undercolor: "#00000066".to_string(),
        text_shadow_enabled: true,
        text_shadow_color: "#00000099".to_string(),
        text_shadow_offset_x: 3,
        text_shadow_offset_y: 3,
        text_box_size: "quarter".to_string(),
        text_box_width_pct: 50,
        text_box_height_pct: 50,
        rotation_use_persistent_state: true,
        rotation_state_file: "~/.local/state/wallpaper-composer/rotation.state".to_string(),
        output_image: "~/.local/state/wallpaper-composer/current.png".to_string(),
        refresh_seconds: 300,
        image_refresh_seconds: 300,
        quote_refresh_seconds: 300,
        time_format: "%H:%M".to_string(),
        apply_wallpaper: false,
        wallpaper_backend: "auto".to_string(),
        wallpaper_fit_mode: "zoom".to_string(),
        show_background_layer: true,
        show_quote_layer: true,
        show_clock_layer: true,
        show_weather_layer: false,
        show_news_layer: false,
        show_cams_layer: false,
        layer_z_quote: 40,
        layer_z_clock: 50,
        layer_z_weather: 60,
        layer_z_news: 70,
        layer_z_cams: 80,
        weather_pos_x: 120,
        weather_pos_y: 120,
        weather_widget_width: 640,
        weather_widget_height: 180,
        weather_font_size: 30,
        weather_font_family: "DejaVu-Sans".to_string(),
        weather_color: "#00F5FF".to_string(),
        weather_undercolor: "#0B0014B3".to_string(),
        weather_stroke_color: "#001A22".to_string(),
        weather_stroke_width: 1,
        news_pos_x: 980,
        news_pos_y: 180,
        news_widget_width: 760,
        news_widget_height: 240,
        weather_refresh_seconds: 600,
        weather_use_system_location: true,
        weather_location_override: String::new(),
        news_source: "euronews".to_string(),
        news_custom_url: String::new(),
        news_fps: 1.0,
        news_refresh_seconds: 90,
        news_audio_enabled: false,
        show_news_ticker2: false,
        news_ticker2_pos_x: 120,
        news_ticker2_pos_y: 980,
        news_ticker2_width: 1280,
        news_ticker2_source: "techcrunch".to_string(),
        news_ticker2_custom_url: String::new(),
        news_ticker2_fps: 1.2,
        news_ticker2_refresh_seconds: 120,
        cams_pos_x: 980,
        cams_pos_y: 640,
        cams_widget_width: 760,
        cams_widget_height: 428,
        cams_source: "auto_local".to_string(),
        cams_custom_urls: String::new(),
        cams_refresh_seconds: 75,
        cams_fps: 1.0,
        cams_count: 2,
        cams_columns: 2,
        login_screen_integration: false,
        boot_screen_integration: false,
    }
}

fn cams_public_preset_urls(id: &str) -> String {
    let urls: &[&str] = match id {
        "europe_mix" => &[
            "Berlin => https://www.youtube.com/watch?v=GE_SfNVNyqk",
            "Paris => https://www.youtube.com/watch?v=l8PMl7tUDIE",
            "Brussels => https://www.youtube.com/watch?v=pykpO5kQJ98",
        ],
        "world_mix" => &[
            "New York => https://www.youtube.com/watch?v=1-iS7LArMPA",
            "Doha => https://www.youtube.com/watch?v=gCNeDWCI0vo",
            "Brussels => https://www.youtube.com/watch?v=pykpO5kQJ98",
            "Earth Orbit => https://www.youtube.com/watch?v=21X5lGlDOfg",
        ],
        _ => &[
            "New York => https://www.youtube.com/watch?v=1-iS7LArMPA",
            "Berlin => https://www.youtube.com/watch?v=GE_SfNVNyqk",
            "Paris => https://www.youtube.com/watch?v=l8PMl7tUDIE",
            "Doha => https://www.youtube.com/watch?v=gCNeDWCI0vo",
        ],
    };
    urls.join("\n")
}

fn quote_box_px(
    mode: &str,
    custom_w_pct: u32,
    custom_h_pct: u32,
    canvas: egui::Vec2,
) -> egui::Vec2 {
    let (w_pct, h_pct) = quote_box_pct(mode, custom_w_pct, custom_h_pct);
    egui::vec2(
        (canvas.x * w_pct as f32 / 100.0).max(80.0),
        (canvas.y * h_pct as f32 / 100.0).max(60.0),
    )
}

fn quote_box_pct(mode: &str, custom_w_pct: u32, custom_h_pct: u32) -> (u32, u32) {
    match mode {
        "quarter" => (50_u32, 50_u32),
        "third" => (66_u32, 50_u32),
        "half" => (75_u32, 60_u32),
        "full" => (100_u32, 100_u32),
        "custom" => (custom_w_pct.clamp(10, 100), custom_h_pct.clamp(10, 100)),
        _ => (50_u32, 50_u32),
    }
}

fn quote_box_world_size(mode: &str, custom_w_pct: u32, custom_h_pct: u32) -> (i32, i32) {
    let (w_pct, h_pct) = quote_box_pct(mode, custom_w_pct, custom_h_pct);
    (
        ((ORDERING_WORLD_WIDTH as i64 * w_pct as i64) / 100).max(80) as i32,
        ((ORDERING_WORLD_HEIGHT as i64 * h_pct as i64) / 100).max(60) as i32,
    )
}

fn snap_to_grid(value: i32) -> i32 {
    let step = ORDERING_GRID_STEP.max(1);
    ((value as f32 / step as f32).round() as i32) * step
}

fn clamp_world_pos(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let max_x = (ORDERING_WORLD_WIDTH - w).max(0);
    let max_y = (ORDERING_WORLD_HEIGHT - h).max(0);
    (x.clamp(0, max_x), y.clamp(0, max_y))
}

fn settings_section(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.group(|ui| {
        egui::CollapsingHeader::new(title)
            .default_open(true)
            .show(ui, |ui| {
                if !subtitle.trim().is_empty() {
                    ui.label(subtitle);
                }
                ui.separator();
                add_contents(ui);
            });
    });
}

fn edit_color_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    allow_alpha: bool,
    help_text: &str,
) {
    ui.label(label);
    let mut color = parse_color_value(value).unwrap_or(egui::Color32::WHITE);
    let picker = ui.color_edit_button_srgba(&mut color);
    if picker.changed() {
        if allow_alpha {
            *value = format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                color.r(),
                color.g(),
                color.b(),
                color.a()
            );
        } else {
            *value = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        }
    }
    ui.text_edit_singleline(value).on_hover_text(help_text);
}

fn parse_color_value(input: &str) -> Option<egui::Color32> {
    let raw = input.trim();
    if let Some(hex) = raw.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    parse_rgb_triplet(raw)
}

fn parse_hex_color(hex: &str) -> Option<egui::Color32> {
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(egui::Color32::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(egui::Color32::from_rgba_premultiplied(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_rgb_triplet(raw: &str) -> Option<egui::Color32> {
    let parts = raw
        .split(',')
        .map(|p| p.trim().parse::<u8>().ok())
        .collect::<Vec<_>>();
    if parts.len() < 3 || parts.iter().take(3).any(Option::is_none) {
        return None;
    }
    let r = parts[0]?;
    let g = parts[1]?;
    let b = parts[2]?;
    let a = if parts.len() >= 4 {
        parts[3].unwrap_or(255)
    } else {
        255
    };
    Some(egui::Color32::from_rgba_premultiplied(r, g, b, a))
}

#[cfg(test)]
mod tests {
    use super::{is_newer_release, numeric_version_parts};

    #[test]
    fn numeric_version_parts_parses_common_formats() {
        assert_eq!(numeric_version_parts("2026.03.10-8"), vec![2026, 3, 10, 8]);
        assert_eq!(numeric_version_parts("v2026.3.10-8"), vec![2026, 3, 10, 8]);
        assert_eq!(numeric_version_parts("2026.3.10"), vec![2026, 3, 10]);
    }

    #[test]
    fn is_newer_release_detects_upgrade() {
        assert!(is_newer_release("2026.03.10-8", "2026.03.10-7"));
        assert!(is_newer_release("v2026.3.10-8", "2026.3.10-7"));
    }

    #[test]
    fn is_newer_release_detects_no_upgrade() {
        assert!(!is_newer_release("2026.03.10-7", "2026.03.10-7"));
        assert!(!is_newer_release("2026.03.10-6", "2026.03.10-7"));
    }
}
