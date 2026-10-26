//! PixKit — минимальный GUI на egui (eframe), 1:1 с оригинальным TUI:
//! входной файл, куда сохранить, порядок кадров down/up — и всё.
//! Никаких ANSI/консоли: просто окно, кнопки и превью.

use std::path::Path;

use eframe::egui;
use egui::{ColorImage, TextureHandle, TextureOptions, Vec2};

use crate::ops;

pub struct PixApp {
    src: String,
    dst: String,
    dst_auto: bool,
    order_down: bool,
    preview: Option<TextureHandle>,
    preview_size: Option<Vec2>,
    loaded_src: String,
    meta: String,
    logs: Vec<(bool, String)>,
}

impl PixApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            src: String::new(),
            dst: String::new(),
            dst_auto: true,
            order_down: true,
            preview: None,
            preview_size: None,
            loaded_src: String::new(),
            meta: String::new(),
            logs: Vec::new(),
        }
    }

    fn auto_out(&self) -> String {
        let p = Path::new(&self.src);
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "out".into());
        let dir = p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
        let name = format!("{stem}-stack.png");
        if dir.is_empty() {
            name
        } else {
            format!("{}/{}", dir.trim_end_matches(['/', '\\']), name)
        }
    }

    fn set_src(&mut self, path: String) {
        self.src = path;
        self.dst = self.auto_out();
        self.dst_auto = true;
    }

    /// Грузит превью, только если файл сменился (не каждый кадр).
    fn refresh_preview(&mut self, ctx: &egui::Context) {
        if self.src == self.loaded_src {
            return;
        }
        self.loaded_src = self.src.clone();
        self.preview = None;
        self.meta.clear();

        if self.src.trim().is_empty() {
            return;
        }
        match ops::preview_frame(&self.src.trim()) {
            Ok(rgba) => {
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let color = ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
                let tex = ctx.load_texture("preview", color, TextureOptions::LINEAR);
                self.preview_size = Some(Vec2::new(w as f32, h as f32));
                self.preview = Some(tex);
            }
            Err(_) => {
                self.meta = "не удалось открыть файл".into();
                return;
            }
        }
        match ops::file_info(&self.src.trim()) {
            Ok(i) => {
                self.meta = if i.animated {
                    format!(
                        "{} · {}×{} · кадров {}, ~{:.1} fps",
                        i.format, i.width, i.height, i.frames, i.fps
                    )
                } else {
                    format!("{} · {}×{}", i.format, i.width, i.height)
                };
            }
            Err(e) => self.meta = e,
        }
    }

    fn pick_src(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("GIF/APNG/изображения", &["gif", "apng", "png", "webp", "jpg", "jpeg", "bmp"])
            .add_filter("Любой файл", &["*"])
            .pick_file()
        {
            self.set_src(p.to_string_lossy().into_owned());
        }
    }

    fn pick_dst(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .set_file_name(
                Path::new(&self.dst)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "out-stack.png".into()),
            )
            .save_file()
        {
            self.dst = p.to_string_lossy().into_owned();
            self.dst_auto = false;
        }
    }

    fn run(&mut self) {
        if self.src.trim().is_empty() {
            self.logs.push((false, "выберите входной файл".into()));
            return;
        }
        let src = self.src.trim().to_string();
        let dst = if self.dst_auto {
            self.auto_out()
        } else {
            let d = self.dst.trim().to_string();
            if d.is_empty() {
                self.auto_out()
            } else {
                d
            }
        };

        let order = if self.order_down {
            ops::StackOrder::Down
        } else {
            ops::StackOrder::Up
        };
        match ops::stack(&src, &dst, order) {
            Ok((n, w, h)) => {
                self.logs.push((
                    true,
                    format!("→ {dst} ({n} кадров, кадр {w}×{h}, стек {w}×{})", h * n as u32),
                ));
                self.logs.push((true, "готово ✔".into()));
            }
            Err(e) => {
                self.logs.push((false, e));
            }
        }
        if self.logs.len() > 100 {
            self.logs.drain(0..self.logs.len() - 100);
        }
    }
}

impl eframe::App for PixApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Перетащили файл в окно.
        let dropped = ui.ctx().input(|i| {
            i.raw.dropped_files.iter().find_map(|f| f.path().to_str().map(|s| s.to_string()))
        });
        if let Some(p) = dropped {
            self.set_src(p);
        }

        self.refresh_preview(ui.ctx());

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("🖼 PixKit");
            ui.label(egui::RichText::new("GIF/APNG → стек").weak());
        });
        ui.separator();

        egui::Grid::new("files").num_columns(3).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label("Откуда:");
            ui.add(
                egui::TextEdit::singleline(&mut self.src)
                    .desired_width(240.0)
                    .hint_text("перетащите файл или введите путь"),
            );
            if ui.button("📂").on_hover_text("выбрать файл").clicked() {
                self.pick_src();
            }
            ui.end_row();

            ui.label("Куда:");
            ui.add(egui::TextEdit::singleline(&mut self.dst).desired_width(240.0));
            if ui.button("💾").on_hover_text("куда сохранить").clicked() {
                self.pick_dst();
            }
            ui.end_row();
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Порядок кадров:");
            ui.selectable_value(&mut self.order_down, true, "первый сверху (down)");
            ui.selectable_value(&mut self.order_down, false, "первый снизу (up)");
        });

        ui.add_space(8.0);
        if ui.button(egui::RichText::new("▶ Склеить").size(15.0)).clicked() {
            self.run();
        }

        // Превью и инфо
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if let Some(tex) = &self.preview {
                let (w, h) = self
                    .preview_size
                    .map(|s| (s.x, s.y))
                    .unwrap_or_else(|| (tex.size_vec2().x, tex.size_vec2().y));
                let max = Vec2::new(300.0, 180.0);
                let scale = (max.x / w).min(max.y / h).min(1.0);
                let shown = Vec2::new((w * scale).max(1.0), (h * scale).max(1.0));
                ui.add(egui::Image::new(tex).fit_to_exact_size(shown));
            } else {
                ui.allocate_space(Vec2::new(300.0, 80.0));
            }
        });
        if !self.meta.is_empty() {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(&self.meta).monospace());
        }

        // Лог
        if !self.logs.is_empty() {
            ui.add_space(6.0);
            ui.separator();
            for (ok, msg) in &self.logs {
                let color = if *ok {
                    egui::Color32::from_rgb(90, 200, 120)
                } else {
                    egui::Color32::from_rgb(240, 100, 100)
                };
                ui.label(egui::RichText::new(msg).color(color));
            }
        }
    }
}