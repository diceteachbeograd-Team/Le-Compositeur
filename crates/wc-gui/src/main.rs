use eframe::egui;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use wc_core::{
    AppConfig, default_config_path, expand_tilde, list_background_images, load_config, load_quotes,
    to_config_toml,
};

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Wallpaper Composer Settings",
        native_options,
        Box::new(|_cc| Ok(Box::new(WcGuiApp::new()))),
    )
}

struct ThumbnailItem {
    label: String,
    texture: egui::TextureHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuiTab {
    Images,
    Quotes,
    Style,
    System,
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
            active_tab: GuiTab::Images,
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

    fn stop_runner(&mut self) {
        let Some(mut child) = self.runner.take() else {
            self.status = "Runner is not active".to_string();
            return;
        };
        let _ = child.kill();
        let _ = child.wait();
        self.status = "Runner stopped".to_string();
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
            return pick_linux_path_dialog(&start, true);
        }
        #[cfg(not(target_os = "linux"))]
        {
            rfd::FileDialog::new().set_directory(start).pick_folder()
        }
    }

    fn pick_quotes_dialog(&self, base: PathBuf) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            return pick_linux_path_dialog(&base, false);
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
            return;
        };

        let images = match list_background_images(&dir) {
            Ok(list) => list,
            Err(e) => {
                self.status = format!("Thumbnail scan failed: {e}");
                self.thumbnails.clear();
                self.thumbnails_for_dir.clear();
                return;
            }
        };

        self.thumbnails.clear();
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

        self.thumbnails_for_dir = self.cfg.image_dir.clone();
        if self.thumbnails.is_empty() {
            self.status = "No previewable images found in folder".to_string();
        }
    }

    fn render_images_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Image Source");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.cfg.image_source, "local".to_string(), "Local");
            ui.selectable_value(
                &mut self.cfg.image_source,
                "preset".to_string(),
                "Open Preset",
            );
            ui.selectable_value(&mut self.cfg.image_source, "url".to_string(), "Custom URL");
        });

        match self.cfg.image_source.as_str() {
            "preset" => {
                ui.horizontal(|ui| {
                    ui.label("Preset");
                    let mut selected = self
                        .cfg
                        .image_source_preset
                        .clone()
                        .unwrap_or_else(|| "nasa_apod".to_string());
                    egui::ComboBox::from_id_salt("image_source_preset")
                        .selected_text(&selected)
                        .show_ui(ui, |ui| {
                            for id in ["nasa_apod", "wikimedia_featured"] {
                                ui.selectable_value(&mut selected, id.to_string(), id);
                            }
                        });
                    self.cfg.image_source_preset = Some(selected);
                });
            }
            "url" => {
                ui.horizontal(|ui| {
                    ui.label("Image URL");
                    let mut url = self.cfg.image_source_url.clone().unwrap_or_default();
                    ui.text_edit_singleline(&mut url);
                    self.cfg.image_source_url = Some(url);
                });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Image dir");
                    ui.text_edit_singleline(&mut self.cfg.image_dir)
                        .on_hover_text("Local folder for background images (jpg/png/webp/bmp).");
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
            ui.add(egui::DragValue::new(&mut self.cfg.image_refresh_seconds).speed(1));
        });
    }

    fn render_quotes_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Quote Source");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.cfg.quote_source, "local".to_string(), "Local");
            ui.selectable_value(
                &mut self.cfg.quote_source,
                "preset".to_string(),
                "Open Preset",
            );
            ui.selectable_value(&mut self.cfg.quote_source, "url".to_string(), "Custom URL");
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
                            for id in ["zenquotes_daily", "quotable_random"] {
                                ui.selectable_value(&mut selected, id.to_string(), id);
                            }
                        });
                    self.cfg.quote_source_preset = Some(selected);
                });
            }
            "url" => {
                ui.horizontal(|ui| {
                    ui.label("Quote URL");
                    let mut url = self.cfg.quote_source_url.clone().unwrap_or_default();
                    ui.text_edit_singleline(&mut url);
                    self.cfg.quote_source_url = Some(url);
                });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.label("Quotes path");
                    ui.text_edit_singleline(&mut self.cfg.quotes_path)
                        .on_hover_text(
                            "Quotes file. Supports ***...*** blocks and optional author syntax.",
                        );
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
        ui.horizontal(|ui| {
            ui.label("Quote sec");
            ui.add(egui::DragValue::new(&mut self.cfg.quote_refresh_seconds).speed(1));
        });
    }

    fn render_style_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Text Style");
        ui.horizontal(|ui| {
            ui.label("Quote size");
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
            egui::ComboBox::from_id_salt("font_family")
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
        });
        ui.horizontal(|ui| {
            ui.label("Clock size");
            ui.add(egui::DragValue::new(&mut self.cfg.clock_font_size).speed(1));
            ui.label("X");
            ui.add(egui::DragValue::new(&mut self.cfg.clock_pos_x).speed(1));
            ui.label("Y");
            ui.add(egui::DragValue::new(&mut self.cfg.clock_pos_y).speed(1));
        });
        ui.horizontal(|ui| {
            ui.label("Text box");
            egui::ComboBox::from_id_salt("text_box_size")
                .selected_text(&self.cfg.text_box_size)
                .show_ui(ui, |ui| {
                    for mode in ["quarter", "third", "half", "full", "custom"] {
                        ui.selectable_value(&mut self.cfg.text_box_size, mode.to_string(), mode);
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
            edit_color_field(ui, "Quote color", &mut self.cfg.quote_color, false);
            edit_color_field(ui, "Clock color", &mut self.cfg.clock_color, false);
        });
        ui.horizontal(|ui| {
            edit_color_field(ui, "Stroke color", &mut self.cfg.text_stroke_color, false);
            ui.label("Stroke width");
            ui.add(egui::DragValue::new(&mut self.cfg.text_stroke_width).speed(1));
        });
        ui.horizontal(|ui| {
            edit_color_field(ui, "Undercolor", &mut self.cfg.text_undercolor, true);
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.cfg.text_shadow_enabled, "Shadow");
            edit_color_field(ui, "Shadow color", &mut self.cfg.text_shadow_color, true);
            ui.label("dx");
            ui.add(egui::DragValue::new(&mut self.cfg.text_shadow_offset_x).speed(1));
            ui.label("dy");
            ui.add(egui::DragValue::new(&mut self.cfg.text_shadow_offset_y).speed(1));
        });
    }

    fn render_system_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Runtime");
        ui.horizontal(|ui| {
            ui.label("Runner tick");
            ui.add(egui::DragValue::new(&mut self.cfg.refresh_seconds).speed(1));
        });
        ui.separator();
        ui.heading("Wallpaper");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.cfg.apply_wallpaper, "Apply wallpaper");
            ui.label("Backend");
            egui::ComboBox::from_id_salt("backend")
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
            egui::ComboBox::from_id_salt("fit")
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
                    .on_hover_text("Path to config.toml used by all actions.");
                if ui.button("Load").clicked() {
                    self.load_from_path(ctx);
                }
                if ui.button("Save").clicked() {
                    self.save_to_path();
                }
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
                if ui.button("Stop Loop").clicked() {
                    self.stop_runner();
                }
            });
            ui.horizontal(|ui| {
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

fn default_cfg() -> AppConfig {
    AppConfig {
        config_version: 1,
        image_dir: "~/Pictures/Wallpapers".to_string(),
        quotes_path: "~/Documents/wallpaper-composer/quotes.md".to_string(),
        image_source: "local".to_string(),
        image_source_url: None,
        image_source_preset: Some("nasa_apod".to_string()),
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
    }
}

fn edit_color_field(ui: &mut egui::Ui, label: &str, value: &mut String, allow_alpha: bool) {
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
    ui.text_edit_singleline(value)
        .on_hover_text("Hex (#RRGGBB / #RRGGBBAA) oder RGB (r,g,b)");
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
