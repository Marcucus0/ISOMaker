//! Panneaux latéraux de l'éditeur.
//!
//! - Gauche : hiérarchie de la scène, palette de tuiles, construction modulaire & prefabs, assets.

use egui::Ui;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use world::{
    grid::TileDefinition,
    object::ObjectId,
    prefab::{BuildingModuleKind, Prefab},
    scene::Scene,
};

#[derive(Debug, Clone)]
pub enum BuildDragPayload {
    Module(BuildingModuleKind),
    Prefab(Prefab),
}

/// Onglets du panneau gauche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftPanelTab {
    #[default]
    Hierarchy,
    Tiles,
    Prefabs,
    Assets,
}

/// État du panneau gauche.
pub struct LeftPanel {
    pub tab: LeftPanelTab,
    /// Type de tuile sélectionné pour la palette.
    pub selected_tile_type: u16,
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
}

impl Default for LeftPanel {
    fn default() -> Self {
        Self {
            tab: LeftPanelTab::Hierarchy,
            selected_tile_type: 1,
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
    /// Retourne l'action demandée par l'utilisateur (sélection, ajout, instanciation prefab).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        scene: &mut Scene,
        current_selection: &mut Option<ObjectId>,
        project_root: Option<&Path>,
        active_project_file: Option<&Path>,
        tile_definitions: &mut Vec<TileDefinition>,
    ) -> PanelAction {
        // ── Onglets ──────────────────────────────────────────────────────────
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tab, LeftPanelTab::Hierarchy, "📋 Scène");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Tiles,     "🎨 Tuiles");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Prefabs,   "🏗 Bâtiments");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Assets,    "📁 Assets");
        });
        ui.separator();

        let action = match self.tab {
            LeftPanelTab::Hierarchy => {
                self.show_hierarchy(ui, scene, current_selection)
            }
            LeftPanelTab::Tiles => {
                self.show_tile_palette(ui, tile_definitions)
            }
            LeftPanelTab::Prefabs => {
                self.show_prefabs(ui)
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
    ) -> PanelAction {
        let mut action = PanelAction::None;

        ui.label(format!("{} objet(s) dans la scène", scene.objects.len()));
        ui.separator();

        // Bouton d'ajout rapide.
        if ui.button("+ Ajouter objet générique").clicked() {
            action = PanelAction::AddObject;
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
    fn show_tile_palette(&mut self, ui: &mut Ui, tiles: &mut Vec<TileDefinition>) -> PanelAction {
        let mut action = PanelAction::None;
        ui.label("Tuiles du projet");
        ui.separator();
        egui::Grid::new("tile_palette")
            .num_columns(2)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for tile in tiles.iter() {
                    let selected = self.selected_tile_type == tile.id;
                    let color = egui::Color32::from_rgba_unmultiplied(
                        tile.color[0], tile.color[1], tile.color[2], tile.color[3],
                    );
                    let label = format!("{} {}", if tile.texture.is_some() { "▧" } else { "◆" }, tile.name);
                    if ui.selectable_label(selected, egui::RichText::new(label)
                        .color(if selected { color } else { egui::Color32::GRAY })).clicked() {
                        self.selected_tile_type = tile.id;
                    }
                    ui.end_row();
                }
            });

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
        }
        action
    }

    /// Panneau de construction modulaire et préfabriqués.
    fn show_prefabs(&mut self, ui: &mut Ui) -> PanelAction {
        let mut action = PanelAction::None;

        ui.label(egui::RichText::new("Modèles préfabriqués (Prefabs)").strong());
        ui.label(egui::RichText::new("Assemblages complets de modules réutilisables").small().color(egui::Color32::GRAY));
        ui.separator();

        let house = Prefab::preset_house();
        let house_drag = ui.dnd_drag_source(
            egui::Id::new("drag_prefab_house"),
            BuildDragPayload::Prefab(house.clone()),
            |ui| ui.button("🏠 Maisonnette Type (glisser ou cliquer)"),
        );
        if house_drag.inner.clicked() {
            action = PanelAction::InstantiatePrefab(house);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.label(egui::RichText::new("Modules individuels").strong());

        let modules = [
            BuildingModuleKind::Wall,
            BuildingModuleKind::Door,
            BuildingModuleKind::Window,
            BuildingModuleKind::Roof,
            BuildingModuleKind::Corner,
            BuildingModuleKind::Stairs,
            BuildingModuleKind::Pillar,
            BuildingModuleKind::Decoration,
        ];

        for kind in modules {
            let drag = ui.dnd_drag_source(
                egui::Id::new(("drag_building_module", kind)),
                BuildDragPayload::Module(kind),
                |ui| ui.button(format!("{} (glisser ou cliquer)", kind.label())),
            );
            if drag.inner.clicked() {
                action = PanelAction::AddModule(kind);
            }
        }

        action
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
                });
            }
        });

        if let Some(dialog) = self.browser_dialog.clone() {
            let title = match dialog {
                BrowserDialog::CreateFile(_) => "Créer un fichier",
                BrowserDialog::CreateDirectory(_) => "Créer un dossier",
                BrowserDialog::Rename(_) => "Renommer",
            };
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new(title)
                .id(egui::Id::new("project_file_browser_dialog"))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
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
            if confirm && !self.browser_name.trim().is_empty() {
                let name = self.browser_name.trim();
                action = match dialog {
                    BrowserDialog::CreateFile(parent) => PanelAction::CreateFile(parent.join(name)),
                    BrowserDialog::CreateDirectory(parent) => PanelAction::CreateDirectory(parent.join(name)),
                    BrowserDialog::Rename(old) => PanelAction::RenamePath(old.clone(), old.with_file_name(name)),
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
    AddObject,
    AddCamera,
    AddModule(BuildingModuleKind),
    InstantiatePrefab(Prefab),
    SelectObject(ObjectId),
    DeleteObject(ObjectId),
    ChooseTileTexture(u16),
}
