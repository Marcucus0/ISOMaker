//! Panneaux latéraux de l'éditeur.
//!
//! - Gauche : hiérarchie de la scène, palette de tuiles et gestionnaire de fichiers du projet.

use egui::Ui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use world::{
    grid::TileDefinition,
    object::{CustomObjectTypeDefinition, CustomPropertyDefinition, CustomPropertyKind, ObjectId, PropValue},
    scene::Scene,
};
use uuid::Uuid;

/// Onglets du panneau gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftPanelTab {
    #[default]
    Hierarchy,
    Tiles,
    ObjectTypes,
    Assets,
}

/// État du panneau gauche.
pub struct LeftPanel {
    pub tab: LeftPanelTab,
    /// Type de tuile sélectionné pour la palette.
    pub selected_tile_type: u16,
    selected_object_type: Option<Uuid>,
    pending_object_type_deletion: Option<Uuid>,
    tile_preview_cache: HashMap<String, Option<egui::TextureHandle>>,
    browser_root: Option<PathBuf>,
    browser_current: Option<PathBuf>,
    browser_scan_path: Option<PathBuf>,
    browser_entries: Vec<BrowserEntry>,
    browser_refreshed_at: Instant,
    browser_dialog: Option<BrowserDialog>,
    browser_name: String,
}

#[derive(Clone)]
struct BrowserEntry {
    path: PathBuf,
    is_directory: bool,
}

#[derive(Clone)]
enum BrowserDialog {
    CreateFile(PathBuf),
    CreateDirectory(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
}

impl Default for LeftPanel {
    fn default() -> Self {
        Self {
            tab: LeftPanelTab::Hierarchy,
            selected_tile_type: 1,
            selected_object_type: None,
            pending_object_type_deletion: None,
            tile_preview_cache: HashMap::new(),
            browser_root: None,
            browser_current: None,
            browser_scan_path: None,
            browser_entries: Vec::new(),
            browser_refreshed_at: Instant::now(),
            browser_dialog: None,
            browser_name: String::new(),
        }
    }
}

impl LeftPanel {
    pub fn new() -> Self { Self::default() }

    pub fn refresh_file_browser(&mut self) {
        self.browser_scan_path = None;
    }

    pub fn current_browser_directory(&self) -> Option<&Path> {
        self.browser_current.as_deref()
    }

    /// Affiche le panneau gauche.
    ///
    /// Retourne l'action demandée par l'utilisateur (sélection ou ajout).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        scene: &mut Scene,
        current_selection: &mut Option<ObjectId>,
        project_root: Option<&Path>,
        active_project_file: Option<&Path>,
        tile_definitions: &mut Vec<TileDefinition>,
        object_types: &mut Vec<CustomObjectTypeDefinition>,
        scene_names: &[String],
    ) -> PanelAction {
        // ── Onglets ──────────────────────────────────────────────────────────
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tab, LeftPanelTab::Hierarchy, "📋 Scène");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Tiles,     "🎨 Tuiles");
            ui.selectable_value(&mut self.tab, LeftPanelTab::ObjectTypes, "◈ Types");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Assets,    "📁 Assets");
        });
        ui.separator();

        let action = match self.tab {
            LeftPanelTab::Hierarchy => {
                self.show_hierarchy(ui, scene, current_selection, object_types)
            }
            LeftPanelTab::Tiles => {
                self.show_tile_palette(ui, tile_definitions, active_project_file.and_then(Path::parent))
            }
            LeftPanelTab::ObjectTypes => {
                self.show_object_types(ui, object_types, scene_names)
            }
            LeftPanelTab::Assets => {
                self.show_assets(ui, project_root, active_project_file)
            }
        };

        action
    }

    /// Hiérarchie des objets : liste cliquable.
    fn show_hierarchy(
        &mut self,
        ui: &mut Ui,
        scene: &mut Scene,
        current_selection: &mut Option<ObjectId>,
        object_types: &[CustomObjectTypeDefinition],
    ) -> PanelAction {
        let mut action = PanelAction::None;

        ui.label(format!("{} objet(s) dans la scène", scene.objects.len()));
        ui.separator();

        // Bouton d'ajout rapide.
        if ui.button("+ Ajouter objet générique").clicked() {
            action = PanelAction::AddObject;
        }
        for definition in object_types {
            if ui.button(format!("+ Ajouter {}", definition.name)).clicked() {
                action = PanelAction::AddCustomObject(definition.id);
            }
        }
        if ui.button("+ Ajouter caméra").clicked() {
            action = PanelAction::AddCamera;
        }

        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Collecte d'abord les ids pour éviter d'emprunter scene pendant l'UI.
            let mut ids: Vec<ObjectId> = scene.objects.keys().copied().collect();
            ids.sort_by_key(|id| id.0); // Ordre stable.

            let mut to_delete: Option<ObjectId> = None;

            for id in ids {
                let Some(obj) = scene.objects.get(&id) else { continue };
                let name  = obj.name.clone();
                let otype = obj.object_type.to_string();
                let selected = *current_selection == Some(id);

                ui.horizontal(|ui| {
                    let label = egui::RichText::new(format!("  {} [{}]", name, otype));
                    if ui.selectable_label(selected, label).clicked() {
                        *current_selection = Some(id);
                    }
                    if ui.small_button("🗑").on_hover_text("Supprimer").clicked() {
                        to_delete = Some(id);
                    }
                });
            }

            if let Some(id) = to_delete {
                action = PanelAction::DeleteObject(id);
            }
        });

        action
    }

    /// Palette de tuiles : sélecteur du type à peindre.
    fn show_tile_palette(
        &mut self,
        ui: &mut Ui,
        tiles: &mut Vec<TileDefinition>,
        asset_root: Option<&Path>,
    ) -> PanelAction {
        let mut action = PanelAction::None;
        ui.label("Tuiles du projet");
        ui.separator();
        for tile in tiles.iter() {
            ui.horizontal(|ui| {
                let size = egui::Vec2::splat(18.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let texture_id = tile.texture.as_deref().and_then(|path| {
                    asset_root.and_then(|root| self.tile_preview_texture(ui.ctx(), root, path))
                });
                if let Some(texture_id) = texture_id {
                    ui.painter().image(
                        texture_id,
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    let color = egui::Color32::from_rgba_unmultiplied(
                        tile.color[0], tile.color[1], tile.color[2], tile.color[3],
                    );
                    ui.painter().rect_filled(rect, 1.0, color);
                }
                ui.painter().rect_stroke(
                    rect,
                    1.0,
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                let selected = self.selected_tile_type == tile.id;
                if ui.selectable_label(selected, &tile.name).clicked() {
                    self.selected_tile_type = tile.id;
                }
            });
        }

        let next_id = (1..=u16::MAX).find(|id| !tiles.iter().any(|tile| tile.id == *id));
        if ui.add_enabled(next_id.is_some(), egui::Button::new("＋ Ajouter une tuile personnalisée")).clicked() {
            if let Some(id) = next_id {
                tiles.push(TileDefinition::new(id, format!("Tuile {id}"), [120, 120, 140, 255]));
                self.selected_tile_type = id;
            }
        }

        if let Some(tile) = tiles.iter_mut().find(|tile| tile.id == self.selected_tile_type) {
            ui.separator();
            ui.label(egui::RichText::new("Propriétés de la tuile").strong());
            ui.horizontal(|ui| {
                ui.label("Nom");
                ui.text_edit_singleline(&mut tile.name);
            });
            let mut color = egui::Color32::from_rgba_unmultiplied(
                tile.color[0], tile.color[1], tile.color[2], tile.color[3],
            );
            ui.horizontal(|ui| {
                ui.label("Couleur");
                if ui.color_edit_button_srgba(&mut color).changed() {
                    tile.color = [color.r(), color.g(), color.b(), color.a()];
                }
            });
            ui.checkbox(&mut tile.has_collisions, "Collisions activées");
            ui.checkbox(&mut tile.randomize_texture_rotation, "Orientation de texture aléatoire");
            if tile.id > 6 {
                ui.checkbox(&mut tile.editor_only, "Visible dans l'éditeur uniquement")
                    .on_hover_text("Cette tuile reste visible dans l'éditeur et disparaît en Lecture.");
            }
            ui.horizontal(|ui| {
                if ui.button("Choisir une texture…").clicked() {
                    action = PanelAction::ChooseTileTexture(tile.id);
                }
                if tile.texture.is_some() && ui.small_button("Retirer").clicked() {
                    tile.texture = None;
                }
            });
            if let Some(texture) = &tile.texture {
                ui.label(egui::RichText::new(texture).small().weak());
            } else {
                ui.label(egui::RichText::new("Aucune texture").small().weak());
            }
            ui.label(egui::RichText::new(format!("ID : {}", tile.id)).small().weak());
            if tile.id > 6 && ui.button("Supprimer cette tuile…").clicked() {
                action = PanelAction::DeleteTileDefinition(tile.id);
            }
        }
        action
    }

    fn tile_preview_texture(
        &mut self,
        ctx: &egui::Context,
        asset_root: &Path,
        stored_path: &str,
    ) -> Option<egui::TextureId> {
        let key = format!("{}|{}", asset_root.display(), stored_path);
        if !self.tile_preview_cache.contains_key(&key) {
            let stored = Path::new(stored_path);
            let path = if stored.is_absolute() { stored.to_owned() } else { asset_root.join(stored) };
            let texture = image::open(path).ok().map(|image| {
                let rgba = image.into_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                ctx.load_texture(key.clone(), pixels, egui::TextureOptions::NEAREST)
            });
            self.tile_preview_cache.insert(key.clone(), texture);
        }
        self.tile_preview_cache.get(&key)
            .and_then(Option::as_ref)
            .map(egui::TextureHandle::id)
    }

    fn show_object_types(
        &mut self,
        ui: &mut Ui,
        object_types: &mut Vec<CustomObjectTypeDefinition>,
        _scene_names: &[String],
    ) -> PanelAction {
        let mut action = PanelAction::None;
        ui.label(egui::RichText::new("Types d’objets du projet").strong());
        ui.label(egui::RichText::new("Sélectionne un type pour modifier sa définition dans l’inspecteur.")
            .small().color(egui::Color32::GRAY));

        if ui.button("+ Créer un type d’objet").clicked() {
            let mut number = 1;
            let name = loop {
                let candidate = format!("Nouveau type {number}");
                if !object_types.iter().any(|definition| definition.name.eq_ignore_ascii_case(&candidate)) {
                    break candidate;
                }
                number += 1;
            };
            let definition = CustomObjectTypeDefinition::new(name);
            self.selected_object_type = Some(definition.id);
            object_types.push(definition);
            action = PanelAction::ObjectTypesChanged;
        }
        ui.separator();

        if object_types.is_empty() {
            ui.label(egui::RichText::new("Aucun type défini.").italics().color(egui::Color32::GRAY));
        } else {
            if !self.selected_object_type.is_some_and(|id| object_types.iter().any(|definition| definition.id == id)) {
                self.selected_object_type = object_types.first().map(|definition| definition.id);
            }
            let mut delete_requested = None;
            for definition in object_types.iter() {
                ui.horizontal(|ui| {
                    let selected = self.selected_object_type == Some(definition.id);
                    if ui.selectable_label(selected, format!("◈ {}", definition.name)).clicked() {
                        self.selected_object_type = Some(definition.id);
                    }
                    if ui.small_button("×").on_hover_text("Supprimer ce type").clicked() {
                        delete_requested = Some(definition.id);
                    }
                });
            }
            if delete_requested.is_some() {
                self.pending_object_type_deletion = delete_requested;
            }
        }

        if let Some(type_id) = self.pending_object_type_deletion {
            let type_name = object_types.iter().find(|definition| definition.id == type_id)
                .map(|definition| definition.name.clone()).unwrap_or_else(|| "ce type".to_owned());
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Supprimer le type d’objet ?")
                .id(egui::Id::new("delete_custom_object_type"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Supprimer « {type_name} » ? Les instances seront conservées comme objets génériques."));
                    ui.horizontal(|ui| {
                        confirm = ui.button("Supprimer").clicked();
                        cancel = ui.button("Annuler").clicked();
                    });
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) { cancel = true; }
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) { confirm = true; }
                });
            if confirm {
                action = PanelAction::DeleteCustomObjectType(type_id);
                self.pending_object_type_deletion = None;
                self.selected_object_type = None;
            } else if cancel {
                self.pending_object_type_deletion = None;
            }
        }
        action
    }

    pub fn selected_object_type_id(&self) -> Option<Uuid> {
        (self.tab == LeftPanelTab::ObjectTypes).then_some(self.selected_object_type).flatten()
    }
    /// Mini gestionnaire de fichiers enraciné dans le dossier du projet.
    fn show_assets(
        &mut self,
        ui: &mut Ui,
        project_root: Option<&Path>,
        active_project_file: Option<&Path>,
    ) -> PanelAction {
        let Some(root) = project_root.map(Path::to_path_buf) else {
            ui.label("Enregistre ou ouvre un projet pour parcourir ses fichiers.");
            return PanelAction::None;
        };
        if self.browser_root.as_ref() != Some(&root) {
            self.browser_root = Some(root.clone());
            self.browser_current = Some(root.clone());
            self.browser_scan_path = None;
        }
        let current = self.browser_current.clone().unwrap_or_else(|| root.clone());
        if self.browser_scan_path.as_ref() != Some(&current)
            || self.browser_refreshed_at.elapsed() >= Duration::from_secs(1)
        {
            self.browser_entries = read_browser_entries(&current);
            self.browser_scan_path = Some(current.clone());
            self.browser_refreshed_at = Instant::now();
        }

        let mut action = PanelAction::None;
        ui.horizontal(|ui| {
            let back_response = ui.add_enabled(
                current != root,
                egui::Button::new("").min_size(egui::vec2(25.0, ui.spacing().interact_size.y)),
            )
                .on_hover_text("Retour au dossier parent");
            let back_rect = back_response.rect;
            let icon_color = ui.style().interact(&back_response).fg_stroke.color;
            ui.painter().line_segment(
                [egui::pos2(back_rect.right() - 5.0, back_rect.center().y),
                 egui::pos2(back_rect.left() + 5.0, back_rect.center().y)],
                egui::Stroke::new(1.8_f32, icon_color),
            );
            ui.painter().line_segment(
                [egui::pos2(back_rect.left() + 5.0, back_rect.center().y),
                 egui::pos2(back_rect.left() + 10.0, back_rect.center().y - 5.0)],
                egui::Stroke::new(1.8_f32, icon_color),
            );
            ui.painter().line_segment(
                [egui::pos2(back_rect.left() + 5.0, back_rect.center().y),
                 egui::pos2(back_rect.left() + 10.0, back_rect.center().y + 5.0)],
                egui::Stroke::new(1.8_f32, icon_color),
            );
            if back_response.clicked() {
                if let Some(parent) = current.parent() {
                    self.browser_current = Some(parent.to_path_buf());
                }
            }
            let displayed = current.strip_prefix(&root).unwrap_or(&current);
            let displayed = if displayed.as_os_str().is_empty() { ".".into() } else { displayed.display().to_string() };
            ui.label(egui::RichText::new(displayed).strong())
                .context_menu(|ui| {
                    if ui.button("Nouveau dossier…").clicked() {
                        self.browser_dialog = Some(BrowserDialog::CreateDirectory(current.clone()));
                        self.browser_name.clear();
                        ui.close_menu();
                    }
                    if ui.button("Nouveau fichier…").clicked() {
                        self.browser_dialog = Some(BrowserDialog::CreateFile(current.clone()));
                        self.browser_name.clear();
                        ui.close_menu();
                    }
                });
            if ui.button("Actualiser").clicked() {
                self.browser_scan_path = None;
            }
        });
        ui.separator();

        let background_rect = ui.available_rect_before_wrap();
        ui.interact(
            background_rect,
            egui::Id::new("project_browser_background_context"),
            egui::Sense::click(),
        ).context_menu(|ui| {
            if ui.button("Nouveau dossier…").clicked() {
                self.browser_dialog = Some(BrowserDialog::CreateDirectory(current.clone()));
                self.browser_name.clear();
                ui.close_menu();
            }
            if ui.button("Nouveau fichier…").clicked() {
                self.browser_dialog = Some(BrowserDialog::CreateFile(current.clone()));
                self.browser_name.clear();
                ui.close_menu();
            }
        });

        let entries = self.browser_entries.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in entries {
                let name = entry.path.file_name().and_then(|name| name.to_str()).unwrap_or("?");
                let response = ui.horizontal(|ui| {
                    draw_browser_icon(ui, entry.is_directory);
                    ui.selectable_label(false, name)
                }).inner;
                if entry.is_directory && response.clicked() {
                    self.browser_current = Some(entry.path.clone());
                } else if !entry.is_directory && response.double_clicked() {
                    action = PanelAction::OpenFile(entry.path.clone());
                }
                response.on_hover_text(entry.path.display().to_string()).context_menu(|ui| {
                    if !entry.is_directory && ui.button("Ouvrir").clicked() {
                        action = PanelAction::OpenFile(entry.path.clone());
                        ui.close_menu();
                    }
                    if ui.button("Nouveau dossier…").clicked() {
                        self.browser_dialog = Some(BrowserDialog::CreateDirectory(current.clone()));
                        self.browser_name.clear();
                        ui.close_menu();
                    }
                    if ui.button("Nouveau fichier…").clicked() {
                        self.browser_dialog = Some(BrowserDialog::CreateFile(current.clone()));
                        self.browser_name.clear();
                        ui.close_menu();
                    }
                    ui.separator();
                    if active_project_file != Some(entry.path.as_path()) {
                        if ui.button("Renommer…").clicked() {
                            self.browser_dialog = Some(BrowserDialog::Rename(entry.path.clone()));
                            self.browser_name = name.to_owned();
                            ui.close_menu();
                        }
                    }
                    if ui.button("Supprimer…").clicked() {
                        self.browser_dialog = Some(BrowserDialog::Delete(entry.path.clone()));
                        ui.close_menu();
                    }
                });
            }
        });

        if let Some(dialog) = self.browser_dialog.clone() {
            let title = match dialog {
                BrowserDialog::CreateFile(_) => "Créer un fichier",
                BrowserDialog::CreateDirectory(_) => "Créer un dossier",
                BrowserDialog::Rename(_) => "Renommer",
                BrowserDialog::Delete(_) => "Supprimer",
            };
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new(title)
                .id(egui::Id::new("project_file_browser_dialog"))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    if let BrowserDialog::Delete(path) = &dialog {
                        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("cet élément");
                        ui.label(format!("Supprimer « {name} » ?"));
                        if path.is_dir() {
                            ui.label("Le dossier et tout son contenu seront supprimés.");
                        }
                        ui.horizontal(|ui| {
                            confirm = ui.button("Supprimer").clicked();
                            cancel = ui.button("Annuler").clicked();
                        });
                        if ui.input(|input| input.key_pressed(egui::Key::Escape)) { cancel = true; }
                        if ui.input(|input| input.key_pressed(egui::Key::Enter)) { confirm = true; }
                        return;
                    }
                    ui.label("Nom :");
                    let response = ui.text_edit_singleline(&mut self.browser_name);
                    response.request_focus();
                    let valid_name = is_valid_browser_name(&self.browser_name);
                    if valid_name && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm = true;
                    }
                    if !valid_name {
                        ui.label(egui::RichText::new("Nom invalide ou séparateurs interdits.")
                            .color(egui::Color32::LIGHT_RED));
                    }
                    ui.horizontal(|ui| {
                        confirm |= ui.add_enabled(valid_name, egui::Button::new("Valider")).clicked();
                        cancel = ui.button("Annuler").clicked();
                    });
                });
            if confirm {
                action = match dialog {
                    BrowserDialog::CreateFile(parent) if !self.browser_name.trim().is_empty() => PanelAction::CreateFile(parent.join(self.browser_name.trim())),
                    BrowserDialog::CreateDirectory(parent) if !self.browser_name.trim().is_empty() => PanelAction::CreateDirectory(parent.join(self.browser_name.trim())),
                    BrowserDialog::Rename(old) if !self.browser_name.trim().is_empty() => PanelAction::RenamePath(old.clone(), old.with_file_name(self.browser_name.trim())),
                    BrowserDialog::Delete(path) => PanelAction::DeletePath(path.clone()),
                    _ => PanelAction::None,
                };
                self.browser_dialog = None;
            } else if cancel {
                self.browser_dialog = None;
            }
        }
        action
    }
}

fn read_browser_entries(directory: &Path) -> Vec<BrowserEntry> {
    let Ok(entries) = std::fs::read_dir(directory) else { return Vec::new() };
    let mut entries: Vec<_> = entries.flatten().filter_map(|entry| {
        let file_type = entry.file_type().ok()?;
        if file_type.is_dir() {
            Some(BrowserEntry { path: entry.path(), is_directory: true })
        } else if file_type.is_file() {
            Some(BrowserEntry { path: entry.path(), is_directory: false })
        } else {
            None
        }
    }).collect();
    entries.sort_by(|a, b| b.is_directory.cmp(&a.is_directory)
        .then_with(|| a.path.file_name().cmp(&b.path.file_name())));
    entries
}

fn draw_browser_icon(ui: &mut Ui, is_directory: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 17.0), egui::Sense::hover());
    let painter = ui.painter();
    let p = |x: f32, y: f32| egui::pos2(rect.left() + x, rect.top() + y);
    if is_directory {
        let fill = egui::Color32::from_rgb(205, 157, 68);
        let edge = egui::Color32::from_rgb(245, 197, 94);
        painter.rect_filled(egui::Rect::from_min_size(p(1.0, 5.0), egui::vec2(14.0, 10.0)), 2.0, fill);
        painter.rect_filled(egui::Rect::from_min_size(p(2.0, 3.0), egui::vec2(7.0, 4.0)), 1.0, edge);
        let points = [p(1.0, 6.0), p(1.0, 5.0), p(2.0, 4.0), p(8.0, 4.0), p(10.0, 6.0), p(14.0, 6.0), p(15.0, 7.0), p(14.0, 14.0), p(2.0, 14.0), p(1.0, 13.0), p(1.0, 6.0)];
        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.0_f32, edge));
        }
    } else {
        let fill = egui::Color32::from_rgb(190, 202, 216);
        let edge = egui::Color32::from_rgb(226, 234, 243);
        painter.rect_filled(egui::Rect::from_min_size(p(3.0, 1.0), egui::vec2(10.0, 14.0)), 1.0, fill);
        let points = [p(3.0, 1.0), p(9.0, 1.0), p(13.0, 5.0), p(13.0, 15.0), p(3.0, 15.0), p(3.0, 1.0)];
        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.0_f32, edge));
        }
        painter.line_segment([p(9.0, 1.0), p(9.0, 5.0)], egui::Stroke::new(1.0_f32, edge));
        painter.line_segment([p(9.0, 5.0), p(13.0, 5.0)], egui::Stroke::new(1.0_f32, edge));
        painter.line_segment([p(5.0, 8.0), p(11.0, 8.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(104, 119, 137)));
        painter.line_segment([p(5.0, 11.0), p(11.0, 11.0)], egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(104, 119, 137)));
    }
}

fn is_valid_browser_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name != "."
        && name != ".."
        && !name.chars().any(|character| matches!(character, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        && !name.ends_with(' ')
        && !name.ends_with('.')
}

/// Actions déclenchées depuis le panneau gauche.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelAction {
    None,
    OpenFile(PathBuf),
    CreateFile(PathBuf),
    CreateDirectory(PathBuf),
    RenamePath(PathBuf, PathBuf),
    DeletePath(PathBuf),
    AddObject,
    AddCamera,
    AddCustomObject(Uuid),
    ObjectTypesChanged,
    RenameCustomProperty(Uuid, String, String),
    DeleteCustomObjectType(Uuid),
    SelectObject(ObjectId),
    DeleteObject(ObjectId),
    DeleteTileDefinition(u16),
    ChooseTileTexture(u16),
}

pub(crate) fn custom_property_kind_label(kind: CustomPropertyKind) -> &'static str {
    match kind {
        CustomPropertyKind::Text => "Texte",
        CustomPropertyKind::Integer => "Entier",
        CustomPropertyKind::Float => "Décimal",
        CustomPropertyKind::Boolean => "Booléen",
        CustomPropertyKind::Scene => "Scène",
    }
}

pub(crate) fn default_custom_property_value(kind: CustomPropertyKind, scene_names: &[String]) -> PropValue {
    match kind {
        CustomPropertyKind::Text => PropValue::Str(String::new()),
        CustomPropertyKind::Integer => PropValue::Int(0),
        CustomPropertyKind::Float => PropValue::Float(0.0),
        CustomPropertyKind::Boolean => PropValue::Bool(false),
        CustomPropertyKind::Scene => PropValue::Str(scene_names.first().cloned().unwrap_or_default()),
    }
}

pub(crate) fn show_custom_property_default(
    ui: &mut Ui,
    type_id: Uuid,
    index: usize,
    property: &mut CustomPropertyDefinition,
    scene_names: &[String],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Valeur initiale");
        match (property.kind, &mut property.default_value) {
            (CustomPropertyKind::Text, PropValue::Str(value)) => {
                changed |= ui.text_edit_singleline(value).changed();
            }
            (CustomPropertyKind::Integer, PropValue::Int(value)) => {
                changed |= ui.add(egui::DragValue::new(value)).changed();
            }
            (CustomPropertyKind::Float, PropValue::Float(value)) => {
                changed |= ui.add(egui::DragValue::new(value).speed(0.1)).changed();
            }
            (CustomPropertyKind::Boolean, PropValue::Bool(value)) => {
                changed |= ui.checkbox(value, "Activée").changed();
            }
            (CustomPropertyKind::Scene, PropValue::Str(value)) => {
                let current_is_valid = scene_names.iter().any(|name| name == value);
                if !current_is_valid {
                    if let Some(first) = scene_names.first() {
                        *value = first.clone();
                        changed = true;
                    }
                }
                egui::ComboBox::from_id_salt(("custom_property_scene", type_id, index))
                    .selected_text(if value.is_empty() { "Choisir une scène…" } else { value.as_str() })
                    .show_ui(ui, |ui| {
                        for name in scene_names {
                            changed |= ui.selectable_value(value, name.clone(), name).changed();
                        }
                    });
            }
            _ => {
                property.default_value = default_custom_property_value(property.kind, scene_names);
                changed = true;
            }
        }
    });
    changed
}

pub(crate) fn is_valid_property_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
