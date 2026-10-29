//! Panneaux latéraux de l'éditeur.
//!
//! - Gauche : hiérarchie de la scène, palette de tuiles, construction modulaire & prefabs, assets.

use egui::Ui;
use world::{
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
}

impl Default for LeftPanel {
    fn default() -> Self {
        Self {
            tab: LeftPanelTab::Hierarchy,
            selected_tile_type: 1,
        }
    }
}

impl LeftPanel {
    pub fn new() -> Self { Self::default() }

    /// Affiche le panneau gauche.
    ///
    /// Retourne l'action demandée par l'utilisateur (sélection, ajout, instanciation prefab).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        scene: &mut Scene,
        current_selection: &mut Option<ObjectId>,
    ) -> PanelAction {
        let mut action = PanelAction::None;

        // ── Onglets ──────────────────────────────────────────────────────────
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.tab, LeftPanelTab::Hierarchy, "📋 Scène");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Tiles,     "🎨 Tuiles");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Prefabs,   "🏗 Bâtiments");
            ui.selectable_value(&mut self.tab, LeftPanelTab::Assets,    "📁 Assets");
        });
        ui.separator();

        match self.tab {
            LeftPanelTab::Hierarchy => {
                action = self.show_hierarchy(ui, scene, current_selection);
            }
            LeftPanelTab::Tiles => {
                self.show_tile_palette(ui);
            }
            LeftPanelTab::Prefabs => {
                action = self.show_prefabs(ui);
            }
            LeftPanelTab::Assets => {
                self.show_assets(ui);
            }
        }

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
                scene.remove_object(id);
                if *current_selection == Some(id) {
                    *current_selection = None;
                }
            }
        });

        action
    }

    /// Palette de tuiles : sélecteur du type à peindre.
    fn show_tile_palette(&mut self, ui: &mut Ui) {
        ui.label("Palette de terrain :");
        ui.separator();

        let tiles = [
            (1u16, "🟢 Herbe",    egui::Color32::from_rgb(80, 140, 70)),
            (2,    "🔵 Eau",      egui::Color32::from_rgb(60, 100, 160)),
            (3,    "🟡 Sable",    egui::Color32::from_rgb(160, 140, 100)),
            (4,    "🟤 Terre",    egui::Color32::from_rgb(100, 90, 80)),
            (5,    "⬜ Pierre",   egui::Color32::from_rgb(130, 130, 130)),
            (6,    "🟠 Désert",   egui::Color32::from_rgb(200, 180, 140)),
        ];

        egui::Grid::new("tile_palette")
            .num_columns(2)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for (tile_type, label, color) in &tiles {
                    let selected = self.selected_tile_type == *tile_type;
                    let text = egui::RichText::new(*label)
                        .color(if selected { *color } else { egui::Color32::GRAY });
                    if ui.selectable_label(selected, text).clicked() {
                        self.selected_tile_type = *tile_type;
                    }
                    ui.end_row();
                }
            });
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

    /// Panneau assets.
    fn show_assets(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Bibliothèque d'assets")
            .italics()
            .color(egui::Color32::GRAY));
        ui.separator();
        ui.label("Textures & tilesets chargés dans le projet.");
    }
}

/// Actions déclenchées depuis le panneau gauche.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelAction {
    None,
    AddObject,
    AddModule(BuildingModuleKind),
    InstantiatePrefab(Prefab),
    SelectObject(ObjectId),
    DeleteObject(ObjectId),
}
