use eframe::egui;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use wc_core::{
    AppConfig, builtin_image_presets, builtin_quote_presets, default_config_path, expand_tilde,
    list_background_images, load_config, load_quotes, to_config_toml,
};

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default().with_app_id("wallpaper-composer");
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
    candidates.push(PathBuf::from("assets/icons/wallpaper-composer.png"));
    candidates.push(PathBuf::from(
        "/usr/share/icons/hicolor/512x512/apps/wallpaper-composer.png",
    ));
    candidates.push(PathBuf::from(
        "/usr/share/icons/hicolor/256x256/apps/wallpaper-composer.png",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiTab {
    Ordering,
    Images,
    Quotes,
    Style,
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
    WidgetA,
    WidgetB,
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
    widget_url: String,
    widget_source: String,
    widget_a_enabled: bool,
    widget_b_enabled: bool,
    widget_a_pos_x: i32,
    widget_a_pos_y: i32,
    widget_b_pos_x: i32,
    widget_b_pos_y: i32,
    ordering_bg_texture: Option<egui::TextureHandle>,
}

impl WcGuiApp {
    fn new() -> Self {
        let config_path = default_config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.config/wallpaper-composer/config.toml".to_string());

        let cfg =
            load_config(PathBuf::from(&config_path).as_path()).unwrap_or_else(|_| default_cfg());

        Self {
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
            widget_url: String::new(),
            widget_source: "custom_url".to_string(),
            widget_a_enabled: true,
            widget_b_enabled: true,
            widget_a_pos_x: 120,
            widget_a_pos_y: 120,
            widget_b_pos_x: 980,
            widget_b_pos_y: 180,
            ordering_bg_texture: None,
        }
    }

    fn t<'a>(&self, en: &'a str, de: &'a str, sr: &'a str, zh: &'a str) -> &'a str {
        match self.ui_lang {
            UiLang::En => en,
            UiLang::De => de,
            UiLang::Sr => sr,
            UiLang::Zh => zh,
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

    fn load_from_path(&mut self, ctx: &egui::Context) {
        match load_config(PathBuf::from(&self.config_path).as_path()) {
            Ok(cfg) => {
                self.cfg = cfg;
                self.status = "Config loaded".to_string();
                self.refresh_thumbnails(ctx);
                self.refresh_quotes_preview();
            }
            Err(e) => self.status = format!("Load failed: {e}"),
        }
    }

    fn save_to_path_inner(&mut self) -> Result<(), String> {
        // Keep one effective rotation timer: image interval drives quote and loop cadence.
        self.cfg.refresh_seconds = self.cfg.image_refresh_seconds.max(1);
        self.cfg.quote_refresh_seconds = self.cfg.image_refresh_seconds.max(1);
        let path = PathBuf::from(&self.config_path);
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
        let direct = self.build_wc_cli_direct(args, &path).output();

        let output = match direct {
            Ok(out) => out,
            Err(_) => match self.build_wc_cli_cargo(args, &path).output() {
                Ok(out) => out,
                Err(e) => {
                    self.status = format!("Command start failed: {e}");
                    return;
                }
            },
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            self.status = format!("{} OK\n{stdout}", args.join(" "));
        } else {
            self.status = format!("{} failed\n{stderr}\n{stdout}", args.join(" "));
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
            self.status = "Runner already active".to_string();
            return;
        }
        if let Err(e) = self.save_to_path_inner() {
            self.status = format!("Cannot start runner before save: {e}");
            return;
        }

        let path = self.config_path.clone();
        let spawn_direct = self
            .build_wc_cli_direct(&["run"], &path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn();
        let child = match spawn_direct {
            Ok(child) => child,
            Err(_) => match self
                .build_wc_cli_cargo(&["run"], &path)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    self.status = format!("Runner start failed: {e}");
                    return;
                }
            },
        };

        self.runner = Some(child);
        self.status = "Runner started (continuous updates active)".to_string();
    }

    fn start_detached_runner(&mut self) {
        if let Err(e) = self.save_to_path_inner() {
            self.status = format!("Cannot start detached runner before save: {e}");
            return;
        }

        let path = self.config_path.clone();
        let spawn_direct = self
            .build_wc_cli_direct(&["run"], &path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let result = match spawn_direct {
            Ok(_child) => Ok(()),
            Err(_) => self
                .build_wc_cli_cargo(&["run"], &path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map(|_child| ()),
        };

        match result {
            Ok(()) => {
                self.status =
                    "Detached runner started (GUI can be closed; reopen GUI manually).".to_string()
            }
            Err(e) => self.status = format!("Detached runner start failed: {e}"),
        }
    }

    fn stop_runner(&mut self) {
        let Some(mut child) = self.runner.take() else {
            self.status = "Runner is not active".to_string();
            return;
        };
        let _ = child.kill();
        let _ = child.wait();
        self.status = "Runner stopped".to_string();
    }

    fn autostart_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var_os("HOME")?;
            Some(
                PathBuf::from(home)
                    .join(".config")
                    .join("autostart")
                    .join("wallpaper-composer.desktop"),
            )
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    fn autostart_enabled() -> bool {
        Self::autostart_path().is_some_and(|p| p.exists())
    }

    fn install_autostart(&mut self) {
        let Some(path) = Self::autostart_path() else {
            self.status = "Autostart install is currently supported on Linux only.".to_string();
            return;
        };

        let config = self.config_path.clone();
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            self.status = format!("Autostart install failed (mkdir): {e}");
            return;
        }

        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Wallpaper Composer Runner\nComment=Start Wallpaper Composer background runner on login\nExec=wc-cli run --config {}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            config
        );
        match std::fs::write(&path, content) {
            Ok(()) => self.status = format!("Autostart installed: {}", path.display()),
            Err(e) => self.status = format!("Autostart install failed: {e}"),
        }
    }

    fn remove_autostart(&mut self) {
        let Some(path) = Self::autostart_path() else {
            self.status = "Autostart remove is currently supported on Linux only.".to_string();
            return;
        };
        if !path.exists() {
            self.status = "Autostart file not present.".to_string();
            return;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => self.status = format!("Autostart removed: {}", path.display()),
            Err(e) => self.status = format!("Autostart remove failed: {e}"),
        }
    }

    fn poll_runner_state(&mut self) {
        let Some(child) = self.runner.as_mut() else {
            return;
        };
        let polled = child.try_wait();
        match polled {
            Ok(Some(exit)) => {
                self.status = format!("Runner exited: {exit}");
                self.runner = None;
            }
            Ok(None) => {}
            Err(e) => {
                self.status = format!("Runner state check failed: {e}");
                self.runner = None;
            }
        }
    }

    fn refresh_quotes_preview(&mut self) {
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

    fn build_wc_cli_direct(&self, args: &[&str], path: &str) -> Command {
        let mut cmd = Command::new("wc-cli");
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
    }

    fn render_ordering_tab(&mut self, ui: &mut egui::Ui) {
        let color_help = self.hover_text("color_format").to_string();
        ui.heading("Ordering");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.cfg.show_background_layer, "Background")
                .on_hover_text("Enable/disable rendered background layer.");
            ui.checkbox(&mut self.cfg.show_quote_layer, "Quote");
            ui.checkbox(&mut self.cfg.show_clock_layer, "Clock");
            ui.checkbox(&mut self.widget_a_enabled, "Widget A");
            ui.checkbox(&mut self.widget_b_enabled, "Widget B");
        });

        ui.horizontal(|ui| {
            ui.label("Element");
            egui::ComboBox::from_id_salt("layout_selected_element")
                .selected_text(match self.selected_element {
                    LayoutElement::Quote => "Quote Box",
                    LayoutElement::Clock => "Clock",
                    LayoutElement::WidgetA => "Widget A",
                    LayoutElement::WidgetB => "Widget B",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.selected_element,
                        LayoutElement::Quote,
                        "Quote Box",
                    );
                    ui.selectable_value(&mut self.selected_element, LayoutElement::Clock, "Clock");
                    ui.selectable_value(
                        &mut self.selected_element,
                        LayoutElement::WidgetA,
                        "Widget A",
                    );
                    ui.selectable_value(
                        &mut self.selected_element,
                        LayoutElement::WidgetB,
                        "Widget B",
                    );
                });
        });

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

        let sx = rect.width() / 1920.0;
        let sy = rect.height() / 1080.0;
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
        let widget_size = egui::vec2(260.0 * sx.max(0.2), 120.0 * sy.max(0.2));
        let mut widget_a_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.widget_a_pos_x as f32 * sx,
                rect.top() + self.widget_a_pos_y as f32 * sy,
            ),
            widget_size,
        );
        let mut widget_b_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + self.widget_b_pos_x as f32 * sx,
                rect.top() + self.widget_b_pos_y as f32 * sy,
            ),
            widget_size,
        );

        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            if quote_rect.contains(pos) {
                self.selected_element = LayoutElement::Quote;
            } else if clock_rect.contains(pos) {
                self.selected_element = LayoutElement::Clock;
            } else if widget_a_rect.contains(pos) {
                self.selected_element = LayoutElement::WidgetA;
            } else if widget_b_rect.contains(pos) {
                self.selected_element = LayoutElement::WidgetB;
            }
        }
        if response.dragged()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let x = (pos.x - rect.left()).clamp(0.0, rect.width());
            let y = (pos.y - rect.top()).clamp(0.0, rect.height());
            let world_x = (x / sx).round() as i32;
            let world_y = (y / sy).round() as i32;
            match self.selected_element {
                LayoutElement::Quote => {
                    self.cfg.quote_pos_x = world_x;
                    self.cfg.quote_pos_y = world_y;
                }
                LayoutElement::Clock => {
                    self.cfg.clock_pos_x = world_x;
                    self.cfg.clock_pos_y = world_y;
                }
                LayoutElement::WidgetA => {
                    self.widget_a_pos_x = world_x;
                    self.widget_a_pos_y = world_y;
                }
                LayoutElement::WidgetB => {
                    self.widget_b_pos_x = world_x;
                    self.widget_b_pos_y = world_y;
                }
            }
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
            widget_a_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.widget_a_pos_x as f32 * sx,
                    rect.top() + self.widget_a_pos_y as f32 * sy,
                ),
                widget_size,
            );
            widget_b_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + self.widget_b_pos_x as f32 * sx,
                    rect.top() + self.widget_b_pos_y as f32 * sy,
                ),
                widget_size,
            );
        }

        if self.cfg.show_quote_layer {
            let neon = if self.selected_element == LayoutElement::Quote {
                egui::Color32::from_rgb(255, 0, 190)
            } else {
                egui::Color32::from_rgb(158, 46, 133)
            };
            painter.rect_filled(quote_rect, 4.0, neon.linear_multiply(0.18));
            painter.rect_stroke(
                quote_rect,
                4.0,
                egui::Stroke::new(2.0, neon),
                egui::StrokeKind::Middle,
            );
            painter.text(
                quote_rect.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                "Quote",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        if self.cfg.show_clock_layer {
            let neon = if self.selected_element == LayoutElement::Clock {
                egui::Color32::from_rgb(0, 255, 255)
            } else {
                egui::Color32::from_rgb(54, 150, 150)
            };
            painter.rect_filled(clock_rect, 4.0, neon.linear_multiply(0.2));
            painter.rect_stroke(
                clock_rect,
                4.0,
                egui::Stroke::new(2.0, neon),
                egui::StrokeKind::Middle,
            );
            painter.text(
                clock_rect.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                "Clock",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        if self.widget_a_enabled {
            let neon = if self.selected_element == LayoutElement::WidgetA {
                egui::Color32::from_rgb(128, 255, 0)
            } else {
                egui::Color32::from_rgb(74, 140, 48)
            };
            painter.rect_filled(widget_a_rect, 4.0, neon.linear_multiply(0.15));
            painter.rect_stroke(
                widget_a_rect,
                4.0,
                egui::Stroke::new(2.0, neon),
                egui::StrokeKind::Middle,
            );
            painter.text(
                widget_a_rect.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                "Widget A",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
        }
        if self.widget_b_enabled {
            let neon = if self.selected_element == LayoutElement::WidgetB {
                egui::Color32::from_rgb(255, 120, 0)
            } else {
                egui::Color32::from_rgb(166, 91, 28)
            };
            painter.rect_filled(widget_b_rect, 4.0, neon.linear_multiply(0.15));
            painter.rect_stroke(
                widget_b_rect,
                4.0,
                egui::Stroke::new(2.0, neon),
                egui::StrokeKind::Middle,
            );
            painter.text(
                widget_b_rect.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                "Widget B",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
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
            LayoutElement::WidgetA | LayoutElement::WidgetB => {
                let (enabled, x, y) = if self.selected_element == LayoutElement::WidgetA {
                    (
                        &mut self.widget_a_enabled,
                        &mut self.widget_a_pos_x,
                        &mut self.widget_a_pos_y,
                    )
                } else {
                    (
                        &mut self.widget_b_enabled,
                        &mut self.widget_b_pos_x,
                        &mut self.widget_b_pos_y,
                    )
                };
                ui.heading("Widget Placeholder");
                ui.checkbox(enabled, "Enabled");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(y).speed(1));
                });
                ui.horizontal(|ui| {
                    ui.label("Widget source");
                    egui::ComboBox::from_id_salt("future_widget_source")
                        .selected_text(&self.widget_source)
                        .show_ui(ui, |ui| {
                            for s in ["custom_url", "euronews", "aljazeera", "cnn", "weather"] {
                                ui.selectable_value(&mut self.widget_source, s.to_string(), s);
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Widget URL / stream");
                    ui.text_edit_singleline(&mut self.widget_url);
                });
                ui.horizontal(|ui| {
                    ui.label("Playback fps");
                    let mut fps = 15_i32;
                    ui.add(egui::DragValue::new(&mut fps).speed(1).range(1..=60));
                });
            }
        }

        ui.separator();
        ui.label("Click a neon box to edit it. Drag inside the frame to place selected element.");
    }

    fn render_style_tab(&mut self, ui: &mut egui::Ui) {
        let color_help = self.hover_text("color_format").to_string();
        ui.heading("Style");
        ui.label("Primary element styling is edited in Ordering when Quote is selected.");
        ui.separator();
        ui.horizontal(|ui| {
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

    fn render_system_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Runtime");
        ui.horizontal(|ui| {
            self.cfg.refresh_seconds = self.cfg.image_refresh_seconds.max(1);
            self.cfg.quote_refresh_seconds = self.cfg.image_refresh_seconds.max(1);
            ui.label("Master timer (from Images tab)");
            ui.monospace(format!("{}s", self.cfg.image_refresh_seconds));
        });
        ui.separator();
        ui.heading("Autostart");
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
        ui.separator();
        ui.heading("System Integrations");
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
        if self.cfg.quote_min_font_size > self.cfg.quote_font_size {
            self.cfg.quote_min_font_size = self.cfg.quote_font_size;
        }

        if self.thumbnails.is_empty() || self.thumbnails_for_dir != self.cfg.image_dir {
            self.refresh_thumbnails(ctx);
        }
        if self.quote_preview.is_empty() {
            self.refresh_quotes_preview();
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
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
            });

            ui.horizontal(|ui| {
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
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, GuiTab::Ordering, "Ordering");
                ui.selectable_value(&mut self.active_tab, GuiTab::Images, "Images");
                ui.selectable_value(&mut self.active_tab, GuiTab::Quotes, "Quotes");
                ui.selectable_value(&mut self.active_tab, GuiTab::Style, "Style");
                ui.selectable_value(&mut self.active_tab, GuiTab::System, "System");
            });
        });

        egui::SidePanel::right("thumbs")
            .default_width(320.0)
            .resizable(true)
            .show(ctx, |ui| {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                GuiTab::Ordering => self.render_ordering_tab(ui),
                GuiTab::Images => self.render_images_tab(ui, ctx),
                GuiTab::Quotes => self.render_quotes_tab(ui),
                GuiTab::Style => self.render_style_tab(ui),
                GuiTab::System => self.render_system_tab(ui),
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label("Status");
            ui.add(
                egui::TextEdit::multiline(&mut self.status)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY),
            );
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
        output_image: "/tmp/wallpaper-composer-current.png".to_string(),
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
        login_screen_integration: false,
        boot_screen_integration: false,
    }
}

fn quote_box_px(
    mode: &str,
    custom_w_pct: u32,
    custom_h_pct: u32,
    canvas: egui::Vec2,
) -> egui::Vec2 {
    let (w_pct, h_pct) = match mode {
        "quarter" => (50_u32, 50_u32),
        "third" => (66_u32, 50_u32),
        "half" => (75_u32, 60_u32),
        "full" => (100_u32, 100_u32),
        "custom" => (custom_w_pct.clamp(10, 100), custom_h_pct.clamp(10, 100)),
        _ => (50_u32, 50_u32),
    };
    egui::vec2(
        (canvas.x * w_pct as f32 / 100.0).max(80.0),
        (canvas.y * h_pct as f32 / 100.0).max(60.0),
    )
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
