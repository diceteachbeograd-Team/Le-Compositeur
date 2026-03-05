use eframe::egui;
use std::path::{Path, PathBuf};
use std::process::Command;
use wc_core::{
    AppConfig, default_config_path, expand_tilde, list_background_images, load_config,
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

struct WcGuiApp {
    config_path: String,
    cfg: AppConfig,
    status: String,
    thumbnails: Vec<ThumbnailItem>,
    thumbnails_for_dir: String,
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
        }
    }

    fn load_from_path(&mut self, ctx: &egui::Context) {
        match load_config(PathBuf::from(&self.config_path).as_path()) {
            Ok(cfg) => {
                self.cfg = cfg;
                self.status = "Config loaded".to_string();
                self.refresh_thumbnails(ctx);
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
        if let Err(e) = self.save_to_path_inner() {
            self.status = format!("Cannot run command before save: {e}");
            return;
        }

        let path = self.config_path.clone();
        let direct = Command::new("wc-cli")
            .args(args)
            .args(["--config", &path])
            .output();

        let output = match direct {
            Ok(out) => out,
            Err(_) => match Command::new("cargo")
                .args(["run", "-p", "wc-cli", "--"])
                .args(args)
                .args(["--config", &path])
                .output()
            {
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

    fn pick_image_dir(&mut self, ctx: &egui::Context) {
        let start = expand_tilde(&self.cfg.image_dir).unwrap_or_else(|_| PathBuf::from("."));
        if let Some(path) = rfd::FileDialog::new().set_directory(start).pick_folder() {
            self.cfg.image_dir = path.display().to_string();
            self.refresh_thumbnails(ctx);
            self.status = "Image folder selected".to_string();
        }
    }

    fn pick_quotes_file(&mut self) {
        let start = expand_tilde(&self.cfg.quotes_path).unwrap_or_else(|_| PathBuf::from("."));
        let base = if start.is_file() {
            start.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            start
        };

        if let Some(path) = rfd::FileDialog::new()
            .set_directory(base)
            .add_filter("Quotes", &["md", "txt"])
            .pick_file()
        {
            self.cfg.quotes_path = path.display().to_string();
            self.status = "Quotes file selected".to_string();
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
}

impl eframe::App for WcGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.thumbnails.is_empty() || self.thumbnails_for_dir != self.cfg.image_dir {
            self.refresh_thumbnails(ctx);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Config");
                ui.text_edit_singleline(&mut self.config_path);
                if ui.button("Load").clicked() {
                    self.load_from_path(ctx);
                }
                if ui.button("Save").clicked() {
                    self.save_to_path();
                }
            });

            ui.horizontal(|ui| {
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
                    self.run_wc_cli(&["run", "--once"]);
                }
                if ui.button("Migrate").clicked() {
                    self.run_wc_cli(&["migrate"]);
                }
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
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Sources");
                ui.horizontal(|ui| {
                    ui.label("Image dir");
                    ui.text_edit_singleline(&mut self.cfg.image_dir);
                    if ui.button("Browse...").clicked() {
                        self.pick_image_dir(ctx);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Quotes path");
                    ui.text_edit_singleline(&mut self.cfg.quotes_path);
                    if ui.button("Browse...").clicked() {
                        self.pick_quotes_file();
                    }
                });
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

                ui.separator();
                ui.heading("Timing");
                ui.horizontal(|ui| {
                    ui.label("Runner tick");
                    ui.add(egui::DragValue::new(&mut self.cfg.refresh_seconds).speed(1));
                    ui.label("Image sec");
                    ui.add(egui::DragValue::new(&mut self.cfg.image_refresh_seconds).speed(1));
                    ui.label("Quote sec");
                    ui.add(egui::DragValue::new(&mut self.cfg.quote_refresh_seconds).speed(1));
                });

                ui.separator();
                ui.heading("Layout");
                ui.horizontal(|ui| {
                    ui.label("Quote size");
                    ui.add(egui::DragValue::new(&mut self.cfg.quote_font_size).speed(1));
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.cfg.quote_pos_x).speed(1));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.cfg.quote_pos_y).speed(1));
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
                    ui.label("Quote color");
                    ui.text_edit_singleline(&mut self.cfg.quote_color);
                    ui.label("Clock color");
                    ui.text_edit_singleline(&mut self.cfg.clock_color);
                });
                ui.horizontal(|ui| {
                    ui.label("Stroke color");
                    ui.text_edit_singleline(&mut self.cfg.text_stroke_color);
                    ui.label("Stroke width");
                    ui.add(egui::DragValue::new(&mut self.cfg.text_stroke_width).speed(1));
                });
                ui.horizontal(|ui| {
                    ui.label("Undercolor");
                    ui.text_edit_singleline(&mut self.cfg.text_undercolor);
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
        quote_font_size: 36,
        quote_pos_x: 80,
        quote_pos_y: 860,
        quote_color: "#FFFFFF".to_string(),
        clock_font_size: 44,
        clock_pos_x: 1600,
        clock_pos_y: 960,
        clock_color: "#FFD700".to_string(),
        text_stroke_color: "#000000".to_string(),
        text_stroke_width: 2,
        text_undercolor: "#00000066".to_string(),
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
