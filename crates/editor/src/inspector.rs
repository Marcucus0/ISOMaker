//! Inspecteur — panneau de propriétés de l'objet sélectionné.

use egui::{CollapsingHeader, Grid, Ui};
use world::{
    object::{ObjectId, ObjectType, PropValue, Rotation4},
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
            ui.label(egui::RichText::new(message).color(egui::Color32::GRAY).italics());
            return false;
        };

        let Some(obj) = scene.get_object(id) else {
            ui.colored_label(egui::Color32::RED, "Objet introuvable");
            return false;
        };

        let mut name = obj.name.clone();
        let mut tile_x = obj.tile_pos.x;
        let mut tile_y = obj.tile_pos.y;
        let mut tile_z = obj.tile_pos.z;
        let mut rotation = obj.rotation;
        let mut has_collisions = obj.has_collisions;
        let mut hidden = obj.hidden;
        let mut fit_texture_to_diamond = obj.fit_texture_to_diamond;
        let mut script = obj.script.clone().unwrap_or_default();
        let mut layer_key = obj.layer.sort_key();
        let obj_type = obj.object_type.clone();
        let is_camera = obj_type == ObjectType::Camera;
        let mut camera_zoom = obj.camera_zoom;
        let mut camera_interpolation = obj.camera_interpolation;
        let mut camera_interpolation_speed = obj.camera_interpolation_speed;
        let mut camera_is_main = scene.main_camera == Some(id);
        let mut camera_main_changed = false;
        let obj_id_str = id.to_string();
        let mut props = obj.properties.clone();
        let mut texture_path = props
            .get("texture")
            .and_then(PropValue::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .or_else(|| obj.asset_path.clone())
            .unwrap_or_default();

        CollapsingHeader::new("Identité")
            .id_salt("inspector_identity")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("inspector_identity_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("ID");
                        ui.label(
                            egui::RichText::new(&obj_id_str[..8])
                                .monospace()
                                .color(egui::Color32::GRAY),
                        );
                        ui.end_row();

                        ui.label("Nom");
                        if ui.text_edit_singleline(&mut name).changed() {
                            modified = true;
                        }
                        ui.end_row();

                        ui.label("Type");
                        ui.label(obj_type.to_string());
                        ui.end_row();
                    });
            });

        CollapsingHeader::new("Transformation")
            .id_salt("inspector_transform")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("inspector_transform_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Position X");
                        if ui.add(egui::DragValue::new(&mut tile_x)).changed() {
                            modified = true;
                        }
                        ui.end_row();

                        ui.label("Position Y");
                        if ui.add(egui::DragValue::new(&mut tile_y)).changed() {
                            modified = true;
                        }
                        ui.end_row();

                        ui.label("Élévation Z");
                        if ui.add(egui::DragValue::new(&mut tile_z)).changed() {
                            modified = true;
                        }
                        ui.end_row();

                        if !is_camera {
                            ui.label("Rotation");
                            egui::ComboBox::from_id_salt("rot_combo")
                                .selected_text(format!("{:?}", rotation))
                                .show_ui(ui, |ui| {
                                    for value in [
                                        Rotation4::North,
                                        Rotation4::East,
                                        Rotation4::South,
                                        Rotation4::West,
                                    ] {
                                        if ui
                                            .selectable_value(
                                                &mut rotation,
                                                value,
                                                format!("{:?}", value),
                                            )
                                            .clicked()
                                        {
                                            modified = true;
                                        }
                                    }
                                });
                            ui.end_row();
                        }
                    });
            });

        if is_camera {
            CollapsingHeader::new("Caméra")
                .id_salt("inspector_camera")
                .default_open(true)
                .show(ui, |ui| {
                    Grid::new("inspector_camera_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Zoom");
                            if ui.add(egui::DragValue::new(&mut camera_zoom)
                                .speed(0.1)
                                .range(0.1..=10.0))
                                .changed()
                            {
                                modified = true;
                            }
                            ui.end_row();

                            ui.label("Caméra principale");
                            if ui.checkbox(&mut camera_is_main, "Au lancement").changed() {
                                modified = true;
                                camera_main_changed = true;
                            }
                            ui.end_row();

                            ui.label("Interpolation");
                            if ui.checkbox(&mut camera_interpolation, "Activée").changed() {
                                modified = true;
                            }
                            ui.end_row();

                            ui.label("Vitesse");
                            ui.add_enabled_ui(camera_interpolation, |ui| {
                                if ui.add(egui::Slider::new(&mut camera_interpolation_speed, 0.1..=20.0)
                                    .suffix("×"))
                                    .changed()
                                {
                                    modified = true;
                                }
                            });
                            ui.end_row();
                        });
                });
        }

        CollapsingHeader::new("Apparence et rendu")
            .id_salt("inspector_appearance")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("inspector_appearance_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Masqué");
                        if ui.checkbox(&mut hidden, "Oui").changed() {
                            modified = true;
                        }
                        ui.end_row();

                        if !is_camera {
                            ui.label("Texture dans le losange");
                            if ui.checkbox(&mut fit_texture_to_diamond, "Ajuster").changed() {
                                modified = true;
                            }
                            ui.end_row();
                        }

                        if !is_camera {
                            ui.label("Texture");
                            ui.horizontal(|ui| {
                                if ui.text_edit_singleline(&mut texture_path).changed() {
                                    modified = true;
                                }
                                if ui
                                    .small_button("…")
                                    .on_hover_text("Choisir un fichier image")
                                    .clicked()
                                {
                                    *request_texture_picker = true;
                                }
                            });
                            ui.end_row();
                        }

                        ui.label("Couche");
                        let layer_names = [
                            "Terrain",
                            "Decoration",
                            "Ground",
                            "Objects",
                            "Buildings",
                            "Foreground",
                            "Overlay",
                        ];
                        egui::ComboBox::from_id_salt("layer_combo")
                            .selected_text(
                                layer_names.get(layer_key as usize).copied().unwrap_or("?"),
                            )
                            .show_ui(ui, |ui| {
                                for (index, label) in layer_names.iter().enumerate() {
                                    if ui
                                        .selectable_value(&mut layer_key, index as u8, *label)
                                        .clicked()
                                    {
                                        modified = true;
                                    }
                                }
                            });
                        ui.end_row();
                    });
            });

        if !is_camera {
        CollapsingHeader::new("Physique")
            .id_salt("inspector_physics")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("inspector_physics_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Collisions");
                        if ui.checkbox(&mut has_collisions, "Activées").changed() {
                            modified = true;
                        }
                        ui.end_row();
                    });
            });
        }

        CollapsingHeader::new("Script")
            .id_salt("inspector_script")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Script Python");
                    if ui.text_edit_singleline(&mut script).changed() {
                        modified = true;
                    }
                    if ui.small_button("Choisir…").clicked() {
                        *request_script_picker = true;
                    }
                    if ui.small_button("Créer…").clicked() {
                        *request_script_create = true;
                    }
                });
            });

        let mut custom_keys: Vec<_> = props
            .keys()
            .filter(|key| key.as_str() != "texture")
            .cloned()
            .collect();
        custom_keys.sort();
        let custom_count = custom_keys.len();
        CollapsingHeader::new(format!("Propriétés personnalisées ({custom_count})"))
            .id_salt("inspector_custom_properties")
            .default_open(true)
            .show(ui, |ui| {
                if custom_keys.is_empty() {
                    ui.label(egui::RichText::new("Aucune propriété personnalisée")
                        .color(egui::Color32::GRAY).italics());
                } else {
                    Grid::new("inspector_custom_properties_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for key in &custom_keys {
                                if let Some(value) = props.get(key) {
                                    ui.label(key);
                                    ui.label(prop_value_str(value));
                                    ui.end_row();
                                }
                            }
                        });
                }
            });

        if modified {
            if let Some(obj) = scene.get_object_mut(id) {
                obj.name = name;
                obj.tile_pos.x = tile_x;
                obj.tile_pos.y = tile_y;
                obj.tile_pos.z = tile_z;
                obj.rotation = rotation;
                obj.camera_zoom = camera_zoom.clamp(0.1, 10.0);
                obj.camera_interpolation = camera_interpolation;
                obj.camera_interpolation_speed = camera_interpolation_speed.clamp(0.1, 20.0);
                obj.has_collisions = has_collisions;
                obj.hidden = hidden;
                obj.fit_texture_to_diamond = fit_texture_to_diamond;
                obj.script = if script.trim().is_empty() {
                    None
                } else {
                    Some(script)
                };
                obj.asset_path = if texture_path.is_empty() {
                    None
                } else {
                    Some(texture_path.clone())
                };
                props.insert("texture".to_owned(), PropValue::Str(texture_path));
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

        if is_camera && camera_main_changed {
            if camera_is_main {
                scene.main_camera = Some(id);
            } else if scene.main_camera == Some(id) {
                scene.main_camera = None;
            }
        }

        modified
    }
}

fn prop_value_str(value: &PropValue) -> String {
    match value {
        PropValue::Bool(value) => value.to_string(),
        PropValue::Int(value) => value.to_string(),
        PropValue::Float(value) => format!("{value:.3}"),
        PropValue::Str(value) => value.clone(),
        PropValue::List(values) => format!("[{} éléments]", values.len()),
    }
}
