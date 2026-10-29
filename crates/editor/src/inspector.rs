//! Inspecteur — panneau de propriétés de l'objet sélectionné.

use egui::Ui;
use world::{
    object::{ObjectId, PropValue, Rotation4},
    scene::Scene,
};

/// Panneau de droite : affiche et édite les propriétés de l'objet sélectionné.
pub struct Inspector;

impl Inspector {
    /// Affiche l'inspecteur.
    ///
    /// Retourne `true` si une propriété a été modifiée.
    pub fn show(
        ui: &mut Ui,
        scene: &mut Scene,
        selected: Option<ObjectId>,
        selected_count: usize,
        request_texture_picker: &mut bool,
        request_script_picker: &mut bool,
        request_script_create: &mut bool,
    ) -> bool {
        let mut modified = false;

        ui.heading("Inspecteur");
        ui.separator();

        let Some(id) = selected else {
            let message = if selected_count > 1 {
                format!("{selected_count} objets sélectionnés")
            } else {
                "Aucun objet sélectionné".to_owned()
            };
            ui.label(egui::RichText::new(message)
                .color(egui::Color32::GRAY).italics());
            return false;
        };

        // On clone les valeurs éditables pour éviter les emprunts simultanés.
        let Some(obj) = scene.get_object(id) else {
            ui.colored_label(egui::Color32::RED, "Objet introuvable");
            return false;
        };

        // Clone des champs à éditer.
        let mut name       = obj.name.clone();
        let mut tile_x     = obj.tile_pos.x;
        let mut tile_y     = obj.tile_pos.y;
        let mut tile_z     = obj.tile_pos.z;
        let mut rotation   = obj.rotation;
        let mut has_collisions = obj.has_collisions;
        let mut hidden = obj.hidden;
        let mut fit_texture_to_diamond = obj.fit_texture_to_diamond;
        let mut script = obj.script.clone().unwrap_or_default();
        let mut layer_key  = obj.layer.sort_key();
        let obj_type       = obj.object_type.clone();
        let obj_id_str     = id.to_string();
        let mut props      = obj.properties.clone();
        props.entry("texture".to_owned()).or_insert_with(|| {
            PropValue::Str(obj.asset_path.clone().unwrap_or_default())
        });

        egui::Grid::new("inspector_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                // ── Identifiant (lecture seule) ───────────────────────────────
                ui.label("ID");
                ui.label(egui::RichText::new(&obj_id_str[..8]).monospace().color(egui::Color32::GRAY));
                ui.end_row();

                // ── Nom ────────────────────────────────────────────────────────
                ui.label("Nom");
                if ui.text_edit_singleline(&mut name).changed() {
                    modified = true;
                }
                ui.end_row();

                // ── Type ───────────────────────────────────────────────────────
                ui.label("Type");
                ui.label(obj_type.to_string());
                ui.end_row();

                ui.label("Collisions");
                if ui.checkbox(&mut has_collisions, "Activées").changed() {
                    modified = true;
                }
                ui.end_row();

                ui.label("Masqué");
                if ui.checkbox(&mut hidden, "Oui").changed() {
                    modified = true;
                }
                ui.end_row();

                ui.label("Texture dans le losange");
                if ui.checkbox(&mut fit_texture_to_diamond, "Ajuster").changed() {
                    modified = true;
                }
                ui.end_row();

                ui.label("Script Python");
                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut script).changed() { modified = true; }
                    if ui.small_button("Choisir…").clicked() { *request_script_picker = true; }
                    if ui.small_button("Créer…").clicked() { *request_script_create = true; }
                });
                ui.end_row();

                // ── Position ──────────────────────────────────────────────────
                ui.label("Position X");
                if ui.add(egui::DragValue::new(&mut tile_x)).changed() { modified = true; }
                ui.end_row();

                ui.label("Position Y");
                if ui.add(egui::DragValue::new(&mut tile_y)).changed() { modified = true; }
                ui.end_row();

                ui.label("Élévation Z");
                if ui.add(egui::DragValue::new(&mut tile_z)).changed() { modified = true; }
                ui.end_row();

                // ── Rotation ──────────────────────────────────────────────────
                ui.label("Rotation");
                egui::ComboBox::from_id_salt("rot_combo")
                    .selected_text(format!("{:?}", rotation))
                    .show_ui(ui, |ui| {
                        for r in [Rotation4::North, Rotation4::East, Rotation4::South, Rotation4::West] {
                            if ui.selectable_value(&mut rotation, r, format!("{:?}", r)).clicked() {
                                modified = true;
                            }
                        }
                    });
                ui.end_row();

                // ── Couche de rendu ────────────────────────────────────────────
                ui.label("Couche");
                let layer_names = ["Terrain", "Decoration", "Ground", "Objects", "Buildings", "Foreground", "Overlay"];
                egui::ComboBox::from_id_salt("layer_combo")
                    .selected_text(layer_names.get(layer_key as usize).copied().unwrap_or("?"))
                    .show_ui(ui, |ui| {
                        for (i, name) in layer_names.iter().enumerate() {
                            if ui.selectable_value(&mut layer_key, i as u8, *name).clicked() {
                                modified = true;
                            }
                        }
                    });
                ui.end_row();

                // ── Asset ──────────────────────────────────────────────────────
            });

        // ── Propriétés personnalisées ─────────────────────────────────────────
        if !props.is_empty() {
            ui.separator();
            ui.label("Propriétés");
            egui::Grid::new("props_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for (key, val) in &mut props {
                        ui.label(if key == "texture" { "Texture" } else { key.as_str() });
                        if key == "texture" {
                            if let PropValue::Str(path) = val {
                                ui.horizontal(|ui| {
                                    if ui.text_edit_singleline(path).changed() {
                                        modified = true;
                                    }
                                    if ui.small_button("…").on_hover_text("Choisir un fichier image").clicked() {
                                        *request_texture_picker = true;
                                    }
                                });
                            } else {
                                ui.label(prop_value_str(val));
                            }
                        } else {
                            ui.label(prop_value_str(val));
                        }
                        ui.end_row();
                    }
                });
        }

        let has_texture_property = props.contains_key("texture");
        let texture_path = props.get("texture").and_then(PropValue::as_str).map(str::to_owned);

        // ── Applique les changements ──────────────────────────────────────────
        if modified {
            if let Some(obj) = scene.get_object_mut(id) {
                obj.name        = name;
                obj.tile_pos.x  = tile_x;
                obj.tile_pos.y  = tile_y;
                obj.tile_pos.z  = tile_z;
                obj.rotation    = rotation;
                obj.has_collisions = has_collisions;
                obj.hidden = hidden;
                obj.fit_texture_to_diamond = fit_texture_to_diamond;
                obj.script = if script.trim().is_empty() { None } else { Some(script) };
                if has_texture_property {
                    obj.asset_path = texture_path.filter(|path| !path.is_empty());
                }
                obj.properties = props;
                obj.layer = match layer_key {
                    0 => world::Layer::Terrain,
                    1 => world::Layer::Decoration,
                    2 => world::Layer::Ground,
                    3 => world::Layer::Objects,
                    4 => world::Layer::Buildings,
                    5 => world::Layer::Foreground,
                    _ => world::Layer::Overlay,
                };
            }
        }

        modified
    }
}

fn prop_value_str(v: &PropValue) -> String {
    match v {
        PropValue::Bool(b)   => b.to_string(),
        PropValue::Int(i)    => i.to_string(),
        PropValue::Float(f)  => format!("{:.3}", f),
        PropValue::Str(s)    => s.clone(),
        PropValue::List(l)   => format!("[{} éléments]", l.len()),
    }
}
