//! Standalone pixel-art tile sheet editor.

use egui_wgpu::ScreenDescriptor;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, path::PathBuf, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const MAX_PIXELS: usize = 1_048_576;

#[derive(Clone, Serialize, Deserialize)]
struct SheetFile {
    tile_width: usize,
    tile_height: usize,
    columns: usize,
    rows: usize,
    pixels: Vec<[u8; 4]>,
}

impl SheetFile {
    fn new(tile_width: usize, tile_height: usize, columns: usize, rows: usize) -> Self {
        let (width, height) = (tile_width * columns, tile_height * rows);
        Self {
            tile_width,
            tile_height,
            columns,
            rows,
            pixels: vec![[0, 0, 0, 0]; width * height],
        }
    }

    fn width(&self) -> usize {
        self.tile_width * self.columns
    }

    fn height(&self) -> usize {
        self.tile_height * self.rows
    }

    fn pixel_index(&self, x: usize, y: usize) -> usize {
        y * self.width() + x
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
}

struct TileStudio {
    project_dir: PathBuf,
    sheet_name: String,
    sheet: SheetFile,
    selected_tile: (usize, usize),
    tool: Tool,
    color: egui::Color32,
    zoom: f32,
    show_pixel_grid: bool,
    canvas_texture: Option<egui::TextureHandle>,
    canvas_dirty: bool,
    stroke_snapshot: Option<Vec<[u8; 4]>>,
    last_painted_pixel: Option<(usize, usize)>,
    undo: Vec<Vec<[u8; 4]>>,
    redo: Vec<Vec<[u8; 4]>>,
    available_sheets: Vec<PathBuf>,
    selected_sheet_path: Option<PathBuf>,
    last_export_folder: Option<PathBuf>,
    rename_sheet_target: Option<PathBuf>,
    rename_sheet_buffer: String,
    delete_sheet_target: Option<PathBuf>,
    status: String,
}

impl TileStudio {
    fn new(project_file: Option<PathBuf>) -> Self {
        let project_dir = project_file
            .as_deref()
            .and_then(project_assets_directory)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("assets"));
        let mut studio = Self {
            project_dir,
            sheet_name: "tileset".to_owned(),
            sheet: SheetFile::new(16, 16, 1, 1),
            selected_tile: (0, 0),
            tool: Tool::Pencil,
            color: egui::Color32::from_rgb(80, 180, 100),
            zoom: 16.0,
            show_pixel_grid: true,
            canvas_texture: None,
            canvas_dirty: true,
            stroke_snapshot: None,
            last_painted_pixel: None,
            undo: Vec::new(),
            redo: Vec::new(),
            available_sheets: Vec::new(),
            selected_sheet_path: None,
            last_export_folder: None,
            rename_sheet_target: None,
            rename_sheet_buffer: String::new(),
            delete_sheet_target: None,
            status: "Nouveau tileset — dessine, puis exporte les tuiles dans le projet.".to_owned(),
        };
        studio.refresh_sheet_list();
        studio
    }

    fn ui(&mut self, ctx: &egui::Context) {
        self.handle_shortcuts(ctx);
        egui::TopBottomPanel::top("tile_editor_toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Nouveau").clicked() {
                    self.sheet = SheetFile::new(16, 16, 1, 1);
                    self.sheet_name = "tileset".to_owned();
                    self.selected_tile = (0, 0);
                    self.selected_sheet_path = None;
                    self.clear_history();
                    self.canvas_dirty = true;
                }
                if ui.button("Enregistrer le projet de tuiles").clicked() {
                    self.save_sheet();
                }
                if ui.button("Exporter chaque tuile en PNG").clicked() {
                    self.export_tiles();
                }
                if let Some(folder) = self.last_export_folder.clone() {
                    if ui.button("Ouvrir le dossier exporté").clicked() {
                        match open_folder_in_file_manager(&folder) {
                            Ok(()) => self.status = format!("Dossier ouvert : {}", folder.display()),
                            Err(error) => self.status = format!("Impossible d'ouvrir le dossier : {error}"),
                        }
                    }
                }
                ui.separator();
                for (tool, label) in [
                    (Tool::Pencil, "Crayon"),
                    (Tool::Eraser, "Gomme"),
                    (Tool::Fill, "Pot de peinture"),
                ] {
                    if ui.selectable_label(self.tool == tool, label).clicked() {
                        self.tool = tool;
                    }
                }
                if ui.button("↩ Annuler").clicked() {
                    self.undo();
                }
                if ui.button("↪ Rétablir").clicked() {
                    self.redo();
                }
                ui.color_edit_button_srgba(&mut self.color);
                ui.checkbox(&mut self.show_pixel_grid, "Grille de pixels");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Tileset :");
                ui.text_edit_singleline(&mut self.sheet_name);
                ui.separator();
                ui.label("Taille tuile");
                let mut tile_width = self.sheet.tile_width;
                let mut tile_height = self.sheet.tile_height;
                let mut columns = self.sheet.columns;
                let mut rows = self.sheet.rows;
                let mut changed = ui
                    .add(egui::DragValue::new(&mut tile_width).range(1..=128))
                    .changed();
                ui.label("×");
                changed |= ui
                    .add(egui::DragValue::new(&mut tile_height).range(1..=128))
                    .changed();
                ui.separator();
                ui.label("Colonnes");
                changed |= ui
                    .add(egui::DragValue::new(&mut columns).range(1..=32))
                    .changed();
                ui.label("Lignes");
                changed |= ui
                    .add(egui::DragValue::new(&mut rows).range(1..=32))
                    .changed();
                if changed {
                    self.resize_sheet(tile_width, tile_height, columns, rows);
                }
                ui.separator();
                ui.label("Zoom");
                ui.add(egui::Slider::new(&mut self.zoom, 4.0..=32.0).suffix(" px/pixel"));
            });
        });

        egui::SidePanel::left("tile_sheet_panel")
            .default_width(190.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Tuiles du tileset");
                ui.label(format!(
                    "{} tuiles · {}×{} pixels",
                    self.sheet.columns * self.sheet.rows,
                    self.sheet.tile_width,
                    self.sheet.tile_height
                ));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for y in 0..self.sheet.rows {
                        for x in 0..self.sheet.columns {
                            let selected = self.selected_tile == (x, y);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!(
                                        "Tuile {} — ({}, {})",
                                        y * self.sheet.columns + x + 1,
                                        x + 1,
                                        y + 1
                                    ),
                                )
                                .clicked()
                            {
                                self.selected_tile = (x, y);
                            }
                        }
                    }
                });
                ui.separator();
                ui.label(format!(
                    "Sélection : colonne {}, ligne {}",
                    self.selected_tile.0 + 1,
                    self.selected_tile.1 + 1
                ));
                ui.separator();
                ui.heading("Ouvrir un tileset");
                if self.available_sheets.is_empty() {
                    ui.label("Aucun fichier .isotilesheet enregistré.");
                } else {
                    for path in self.available_sheets.clone() {
                        let label = path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Tileset");
                        let response = ui.selectable_label(
                            self.selected_sheet_path.as_ref() == Some(&path),
                            label,
                        );
                        if response.clicked() {
                            self.load_sheet(&path);
                        }
                        response.context_menu(|ui| {
                            if ui.button("Renommer…").clicked() {
                                self.rename_sheet_buffer = label.to_owned();
                                self.rename_sheet_target = Some(path.clone());
                                ui.close_menu();
                            }
                            if ui.button("Supprimer…").clicked() {
                                self.delete_sheet_target = Some(path.clone());
                                ui.close_menu();
                            }
                        });
                    }
                }
            });

        let mut confirm_rename = false;
        let mut cancel_rename = false;
        if self.rename_sheet_target.is_some() {
            egui::Window::new("Renommer le projet de tuiles")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.rename_sheet_buffer);
                    ui.horizontal(|ui| {
                        if ui.button("Renommer").clicked() {
                            confirm_rename = true;
                        }
                        if ui.button("Annuler").clicked() {
                            cancel_rename = true;
                        }
                    });
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm_rename = true;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        cancel_rename = true;
                    }
                });
        }
        if cancel_rename {
            self.rename_sheet_target = None;
        } else if confirm_rename {
            if let Some(old_path) = self.rename_sheet_target.take() {
                self.rename_sheet_file(&old_path, &self.rename_sheet_buffer.clone());
            }
        }

        let mut confirm_delete = false;
        let mut cancel_delete = false;
        if let Some(path) = &self.delete_sheet_target {
            let label = path.file_name().and_then(|name| name.to_str()).unwrap_or("tileset");
            egui::Window::new("Supprimer le projet de tuiles ?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Supprimer « {label} » du disque ?"));
                    ui.horizontal(|ui| {
                        if ui.button("Supprimer").clicked() {
                            confirm_delete = true;
                        }
                        if ui.button("Annuler").clicked() {
                            cancel_delete = true;
                        }
                    });
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm_delete = true;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        cancel_delete = true;
                    }
                });
        }
        if cancel_delete {
            self.delete_sheet_target = None;
        } else if confirm_delete {
            if let Some(path) = self.delete_sheet_target.take() {
                self.delete_sheet_file(&path);
            }
        }

        egui::TopBottomPanel::bottom("tile_editor_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Ctrl+Z annuler · Ctrl+Y rétablir");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.draw_canvas(ui, ctx);
                });
        });
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let undo = ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z));
        let redo = ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Y));
        let save = ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::S));
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if save {
            self.save_sheet();
        }
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.refresh_texture(ctx);
        let size = egui::vec2(
            self.sheet.width() as f32 * self.zoom,
            self.sheet.height() as f32 * self.zoom,
        );
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        if let Some(texture) = &self.canvas_texture {
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        if self.show_pixel_grid && self.zoom >= 7.0 {
            let stroke = egui::Stroke::new(0.5_f32, egui::Color32::from_black_alpha(70));
            for x in 0..=self.sheet.width() {
                let px = rect.left() + x as f32 * self.zoom;
                ui.painter().line_segment(
                    [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                    stroke,
                );
            }
            for y in 0..=self.sheet.height() {
                let py = rect.top() + y as f32 * self.zoom;
                ui.painter().line_segment(
                    [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                    stroke,
                );
            }
        }
        let tile_stroke = egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(245, 195, 70));
        for x in 0..=self.sheet.columns {
            let px = rect.left() + (x * self.sheet.tile_width) as f32 * self.zoom;
            ui.painter().line_segment(
                [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                tile_stroke,
            );
        }
        for y in 0..=self.sheet.rows {
            let py = rect.top() + (y * self.sheet.tile_height) as f32 * self.zoom;
            ui.painter().line_segment(
                [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                tile_stroke,
            );
        }
        let selected_rect = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + (self.selected_tile.0 * self.sheet.tile_width) as f32 * self.zoom,
                rect.top() + (self.selected_tile.1 * self.sheet.tile_height) as f32 * self.zoom,
            ),
            egui::vec2(
                self.sheet.tile_width as f32 * self.zoom,
                self.sheet.tile_height as f32 * self.zoom,
            ),
        );
        ui.painter().rect_stroke(
            selected_rect,
            0.0,
            egui::Stroke::new(2.5_f32, egui::Color32::WHITE),
        );
        if response.drag_started() || response.clicked() {
            self.stroke_snapshot = Some(self.sheet.pixels.clone());
            self.last_painted_pixel = None;
        }
        if response.dragged() || response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                if rect.contains(pointer) {
                    let x = ((pointer.x - rect.left()) / self.zoom) as usize;
                    let y = ((pointer.y - rect.top()) / self.zoom) as usize;
                    self.selected_tile = (x / self.sheet.tile_width, y / self.sheet.tile_height);
                    let pixel = (x, y);
                    if self.last_painted_pixel != Some(pixel) {
                        let pixel_index = self.sheet.pixel_index(x, y);
                        match self.tool {
                            Tool::Pencil => self.sheet.pixels[pixel_index] = self.color.to_array(),
                            Tool::Eraser => self.sheet.pixels[pixel_index] = [0, 0, 0, 0],
                            Tool::Fill if response.clicked() => self.fill_tile(x, y),
                            Tool::Fill => {}
                        }
                        self.last_painted_pixel = Some(pixel);
                        self.canvas_dirty = true;
                    }
                }
            }
        }
        if response.drag_stopped() || response.clicked() {
            if let Some(snapshot) = self.stroke_snapshot.take() {
                if snapshot != self.sheet.pixels {
                    self.undo.push(snapshot);
                    self.redo.clear();
                }
            }
            self.last_painted_pixel = None;
        }
    }

    fn refresh_texture(&mut self, ctx: &egui::Context) {
        if !self.canvas_dirty {
            return;
        }
        let bytes: Vec<u8> = self
            .sheet
            .pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect();
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [self.sheet.width(), self.sheet.height()],
            &bytes,
        );
        if let Some(texture) = &mut self.canvas_texture {
            texture.set(image, egui::TextureOptions::NEAREST);
        } else {
            self.canvas_texture =
                Some(ctx.load_texture("tile_studio_sheet", image, egui::TextureOptions::NEAREST));
        }
        self.canvas_dirty = false;
    }

    fn fill_tile(&mut self, x: usize, y: usize) {
        let tile_x = x / self.sheet.tile_width * self.sheet.tile_width;
        let tile_y = y / self.sheet.tile_height * self.sheet.tile_height;
        let start = self.sheet.pixel_index(x, y);
        let target = self.sheet.pixels[start];
        let replacement = self.color.to_array();
        if target == replacement {
            return;
        }
        let mut queue = VecDeque::from([(x, y)]);
        while let Some((px, py)) = queue.pop_front() {
            if px < tile_x
                || py < tile_y
                || px >= tile_x + self.sheet.tile_width
                || py >= tile_y + self.sheet.tile_height
            {
                continue;
            }
            let index = self.sheet.pixel_index(px, py);
            if self.sheet.pixels[index] != target {
                continue;
            }
            self.sheet.pixels[index] = replacement;
            if px > tile_x {
                queue.push_back((px - 1, py));
            }
            if px + 1 < tile_x + self.sheet.tile_width {
                queue.push_back((px + 1, py));
            }
            if py > tile_y {
                queue.push_back((px, py - 1));
            }
            if py + 1 < tile_y + self.sheet.tile_height {
                queue.push_back((px, py + 1));
            }
        }
    }

    fn resize_sheet(&mut self, tile_width: usize, tile_height: usize, columns: usize, rows: usize) {
        let (old_w, old_h) = (self.sheet.width(), self.sheet.height());
        let tile_width = tile_width.clamp(1, 128);
        let tile_height = tile_height.clamp(1, 128);
        let columns = columns.clamp(1, 32);
        let rows = rows.clamp(1, 32);
        let new_w = tile_width * columns;
        let new_h = tile_height * rows;
        if new_w.saturating_mul(new_h) > MAX_PIXELS {
            self.status = format!("Tileset trop grand (maximum : {MAX_PIXELS} pixels). Les dimensions n'ont pas été modifiées.");
            return;
        }
        let mut pixels = vec![[0, 0, 0, 0]; new_w * new_h];
        for y in 0..old_h.min(new_h) {
            let old_start = y * old_w;
            let new_start = y * new_w;
            let count = old_w.min(new_w);
            pixels[new_start..new_start + count]
                .copy_from_slice(&self.sheet.pixels[old_start..old_start + count]);
        }
        self.sheet.tile_width = tile_width;
        self.sheet.tile_height = tile_height;
        self.sheet.columns = columns;
        self.sheet.rows = rows;
        self.sheet.pixels = pixels;
        self.selected_tile.0 = self.selected_tile.0.min(self.sheet.columns - 1);
        self.selected_tile.1 = self.selected_tile.1.min(self.sheet.rows - 1);
        self.clear_history();
        self.canvas_dirty = true;
    }

    fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.sheet.pixels, previous));
            self.canvas_dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo
                .push(std::mem::replace(&mut self.sheet.pixels, next));
            self.canvas_dirty = true;
        }
    }

    fn sheet_directory(&self) -> PathBuf {
        self.project_dir.join("tile_sheets")
    }

    fn safe_name(&self) -> String {
        safe_sheet_name(&self.sheet_name)
    }

    fn rename_sheet_file(&mut self, old_path: &std::path::Path, new_name: &str) {
        let new_name = safe_sheet_name(new_name);
        let Some(parent) = old_path.parent() else {
            self.status = "Dossier du projet de tuiles introuvable.".to_owned();
            return;
        };
        let new_path = parent.join(format!("{new_name}.isotilesheet"));
        if new_path == old_path {
            self.rename_sheet_target = None;
            return;
        }
        if new_path != old_path && new_path.exists() {
            self.status = format!("Un projet de tuiles nommé « {new_name} » existe déjà.");
            return;
        }
        match std::fs::rename(old_path, &new_path) {
            Ok(()) => {
                if self.selected_sheet_path.as_deref() == Some(old_path) {
                    self.selected_sheet_path = Some(new_path.clone());
                    self.sheet_name = new_name;
                }
                self.refresh_sheet_list();
                self.status = format!("Projet de tuiles renommé : {}", new_path.display());
            }
            Err(error) => self.status = format!("Erreur de renommage : {error}"),
        }
    }

    fn delete_sheet_file(&mut self, path: &std::path::Path) {
        match std::fs::remove_file(path) {
            Ok(()) => {
                if self.selected_sheet_path.as_deref() == Some(path) {
                    self.selected_sheet_path = None;
                }
                self.refresh_sheet_list();
                self.status = format!("Projet de tuiles supprimé : {}", path.display());
            }
            Err(error) => self.status = format!("Erreur de suppression : {error}"),
        }
    }

    fn save_sheet(&mut self) {
        let directory = self.sheet_directory();
        let result = (|| -> anyhow::Result<PathBuf> {
            std::fs::create_dir_all(&directory)?;
            let path = directory.join(format!("{}.isotilesheet", self.safe_name()));
            let json = serde_json::to_vec_pretty(&self.sheet)?;
            std::fs::write(&path, json)?;
            Ok(path)
        })();
        match result {
            Ok(path) => {
                self.selected_sheet_path = Some(path.clone());
                self.status = format!("Tileset enregistré : {}", path.display());
                self.refresh_sheet_list();
            }
            Err(error) => self.status = format!("Erreur d'enregistrement : {error}"),
        }
    }

    fn refresh_sheet_list(&mut self) {
        self.available_sheets = std::fs::read_dir(self.sheet_directory())
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "isotilesheet")
            })
            .collect();
        self.available_sheets.sort();
    }

    fn load_sheet(&mut self, path: &std::path::Path) {
        let result = (|| -> anyhow::Result<SheetFile> {
            let sheet: SheetFile = serde_json::from_slice(&std::fs::read(path)?)?;
            anyhow::ensure!(
                (1..=128).contains(&sheet.tile_width)
                    && (1..=128).contains(&sheet.tile_height)
                    && (1..=32).contains(&sheet.columns)
                    && (1..=32).contains(&sheet.rows),
                "Dimensions invalides"
            );
            anyhow::ensure!(
                sheet.width() * sheet.height() <= MAX_PIXELS
                    && sheet.pixels.len() == sheet.width() * sheet.height(),
                "Tileset invalide ou trop grand"
            );
            Ok(sheet)
        })();
        match result {
            Ok(sheet) => {
                self.sheet_name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("tileset")
                    .to_owned();
                self.sheet = sheet;
                self.selected_tile = (0, 0);
                self.selected_sheet_path = Some(path.to_owned());
                self.clear_history();
                self.canvas_dirty = true;
                self.status = format!("Tileset chargé : {}", path.display());
            }
            Err(error) => self.status = format!("Erreur de chargement : {error}"),
        }
    }

    fn export_tiles(&mut self) {
        // Keep generated PNGs alongside other images for the asset browser
        // and the texture picker.
        let folder = self.project_dir.join("images").join("tiles").join(self.safe_name());
        let result = (|| -> anyhow::Result<()> {
            std::fs::create_dir_all(&folder)?;
            for tile_y in 0..self.sheet.rows {
                for tile_x in 0..self.sheet.columns {
                    let mut rgba =
                        Vec::with_capacity(self.sheet.tile_width * self.sheet.tile_height * 4);
                    for y in 0..self.sheet.tile_height {
                        for x in 0..self.sheet.tile_width {
                            let pixel = self.sheet.pixels[self.sheet.pixel_index(
                                tile_x * self.sheet.tile_width + x,
                                tile_y * self.sheet.tile_height + y,
                            )];
                            rgba.extend_from_slice(&pixel);
                        }
                    }
                    let image = image::RgbaImage::from_raw(
                        self.sheet.tile_width as u32,
                        self.sheet.tile_height as u32,
                        rgba,
                    )
                    .ok_or_else(|| anyhow::anyhow!("Impossible de construire l'image"))?;
                    image.save(folder.join(format!(
                        "{}_{}_{}.png",
                        self.safe_name(),
                        tile_x + 1,
                        tile_y + 1
                    )))?;
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.last_export_folder = Some(folder.clone());
                self.status = format!(
                    "{} tuiles PNG exportées dans {}",
                    self.sheet.columns * self.sheet.rows,
                    folder.display()
                )
            }
            Err(error) => self.status = format!("Erreur d'export : {error}"),
        }
    }
}

fn safe_sheet_name(name: &str) -> String {
    let normalized: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if normalized.is_empty() {
        "tileset".to_owned()
    } else {
        normalized
    }
}

fn open_folder_in_file_manager(path: &std::path::Path) -> anyhow::Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        std::process::Command::new("explorer.exe")
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else {
        std::process::Command::new("xdg-open")
    };
    command.arg(path).spawn()?;
    Ok(())
}

fn project_assets_directory(project_file: &std::path::Path) -> Option<PathBuf> {
    let parent = project_file.parent()?;
    let stem = project_file.file_stem()?;
    Some(parent.join(stem).join("assets"))
}

enum App {
    Uninitialized(TileStudio),
    Running(StudioWindow),
}

struct StudioWindow {
    studio: TileStudio,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    last_frame: Instant,
}

fn main() {
    let project_file = std::env::args_os().nth(1).map(PathBuf::from);
    let mut app = App::Uninitialized(TileStudio::new(project_file));
    let event_loop = EventLoop::new().expect("Unable to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut app)
        .expect("Tile editor window failed");
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if matches!(self, App::Running(_)) {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("ISOMaker — Atelier de tuiles")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280_u32, 850_u32)),
                )
                .expect("Unable to create tile editor window"),
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("Unable to create surface");
        let (device, queue, config, format) =
            pollster::block_on(init_wgpu(&instance, &surface, window.inner_size()));
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(egui::Visuals::dark());
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
        let App::Uninitialized(studio) =
            std::mem::replace(self, App::Uninitialized(TileStudio::new(None)))
        else {
            unreachable!()
        };
        *self = App::Running(StudioWindow {
            studio,
            window,
            surface,
            config,
            device,
            queue,
            egui_ctx,
            egui_state,
            renderer,
            last_frame: Instant::now(),
        });
        if let App::Running(app) = self {
            app.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let App::Running(app) = self else {
            return;
        };
        let response = app.egui_state.on_window_event(&app.window, &event);
        if response.consumed
            && !matches!(
                event,
                WindowEvent::RedrawRequested | WindowEvent::CloseRequested
            )
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                app.config.width = size.width;
                app.config.height = size.height;
                app.surface.configure(&app.device, &app.config);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let _delta = now.duration_since(app.last_frame).as_secs_f32();
                app.last_frame = now;
                let output = match app.surface.get_current_texture() {
                    Ok(output) => output,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        app.surface.configure(&app.device, &app.config);
                        return;
                    }
                    Err(wgpu::SurfaceError::Timeout) => return,
                    Err(error) => {
                        log::error!("Tile editor render error: {error}");
                        event_loop.exit();
                        return;
                    }
                };
                let view = output.texture.create_view(&Default::default());
                let input = app.egui_state.take_egui_input(&app.window);
                let full_output = app.egui_ctx.run(input, |ctx| app.studio.ui(ctx));
                app.egui_state
                    .handle_platform_output(&app.window, full_output.platform_output);
                let primitives = app
                    .egui_ctx
                    .tessellate(full_output.shapes, app.egui_ctx.pixels_per_point());
                for (id, delta) in full_output.textures_delta.set {
                    app.renderer
                        .update_texture(&app.device, &app.queue, id, &delta);
                }
                for id in full_output.textures_delta.free {
                    app.renderer.free_texture(&id);
                }
                let screen = ScreenDescriptor {
                    size_in_pixels: [app.config.width, app.config.height],
                    pixels_per_point: app.egui_ctx.pixels_per_point(),
                };
                let mut encoder =
                    app.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("tile_editor_encoder"),
                        });
                app.renderer.update_buffers(
                    &app.device,
                    &app.queue,
                    &mut encoder,
                    &primitives,
                    &screen,
                );
                {
                    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("tile_editor_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.08,
                                    g: 0.09,
                                    b: 0.11,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    let mut pass = pass.forget_lifetime();
                    app.renderer.render(&mut pass, &primitives, &screen);
                }
                app.queue.submit(std::iter::once(encoder.finish()));
                output.present();
                app.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let App::Running(app) = self {
            app.window.request_redraw();
        }
    }
}

async fn init_wgpu(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    size: winit::dpi::PhysicalSize<u32>,
) -> (
    wgpu::Device,
    wgpu::Queue,
    wgpu::SurfaceConfiguration,
    wgpu::TextureFormat,
) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("No compatible GPU found");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("ISOMakerTileStudio"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .expect("Unable to create GPU device");
    let capabilities = surface.get_capabilities(&adapter);
    let format = capabilities
        .formats
        .iter()
        .find(|format| format.is_srgb())
        .copied()
        .unwrap_or(capabilities.formats[0]);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);
    (device, queue, config, format)
}
