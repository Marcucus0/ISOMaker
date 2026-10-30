//! Inspecteur — panneau de propriétés de l'objet sélectionné.

use egui::{CollapsingHeader, Grid, Ui};
use world::{
    layer::Layer,
    object::{CustomObjectTypeDefinition, CustomPropertyDefinition, CustomPropertyKind, ObjectId, ObjectType, PropValue, Rotation4},
    scene::Scene,
};
use crate::panels::{custom_property_kind_label, default_custom_property_value, is_valid_property_key, show_custom_property_default};

#[derive(Debug, Default)]
pub struct TypeDefinitionEdit {
    pub changed: bool,
    pub renamed_property: Option<(uuid::Uuid, String, String)>,
}

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
        custom_object_types: &[CustomObjectTypeDefinition],
        scene_names: &[String],
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
        let mut footprint_width = obj.footprint.width.max(1);
        let mut footprint_height = obj.footprint.height.max(1);
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
                            ui.label("Empreinte (tuiles)");
                            ui.horizontal(|ui| {
                                ui.label("Largeur");
                                if ui.add(egui::DragValue::new(&mut footprint_width).range(1..=256)).changed() {
                                    modified = true;
                                }
                                ui.label("Hauteur");
                                if ui.add(egui::DragValue::new(&mut footprint_height).range(1..=256)).changed() {
                                    modified = true;
                                }
                            });
                            ui.end_row();
                        }

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

        let custom_definition = obj.custom_type_id
            .and_then(|type_id| custom_object_types.iter().find(|definition| definition.id == type_id));
        let declared_keys: std::collections::HashSet<&str> = custom_definition
            .into_iter().flat_map(|definition| definition.properties.iter())
            .map(|property| property.key.as_str()).collect();
        let mut custom_keys: Vec<_> = props.keys()
            .filter(|key| key.as_str() != "texture" && !declared_keys.contains(key.as_str()))
            .cloned().collect();
        custom_keys.sort();
        let custom_count = custom_keys.len() + custom_definition.map_or(0, |definition| definition.properties.len());
        CollapsingHeader::new(format!("Propriétés personnalisées ({custom_count})"))
            .id_salt("inspector_custom_properties")
            .default_open(true)
            .show(ui, |ui| {
                if custom_count == 0 {
                    ui.label(egui::RichText::new("Aucune propriété personnalisée")
                        .color(egui::Color32::GRAY).italics());
                } else {
                    Grid::new("inspector_custom_properties_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            if let Some(definition) = custom_definition {
                                for property in &definition.properties {
                                    let value = props.entry(property.key.clone())
                                        .or_insert_with(|| property.default_value.clone());
                                    ui.label(&property.key);
                                    if show_custom_property_editor(ui, definition.id, property, value, scene_names) {
                                        modified = true;
                                    }
                                    ui.end_row();
                                }
                            }
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
                obj.footprint.width = footprint_width.max(1);
                obj.footprint.height = footprint_height.max(1);
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

    pub fn show_type_definition(
        ui: &mut Ui,
        type_id: uuid::Uuid,
        object_types: &mut [CustomObjectTypeDefinition],
        scene_names: &[String],
        request_texture_picker: &mut bool,
        request_script_picker: &mut bool,
    ) -> TypeDefinitionEdit {
        let mut edit = TypeDefinitionEdit::default();
        let Some(index) = object_types.iter().position(|definition| definition.id == type_id) else {
            ui.label("Sélectionne un type d’objet dans l’onglet Types.");
            return edit;
        };
        let other_names: Vec<_> = object_types.iter().enumerate()
            .filter(|(other_index, _)| *other_index != index)
            .map(|(_, definition)| definition.name.to_lowercase()).collect();
        let definition = &mut object_types[index];

        ui.heading(format!("Type : {}", definition.name));
        ui.separator();
        ui.label(egui::RichText::new("Définition du type").strong());
        ui.horizontal(|ui| {
            ui.label("Nom");
            let mut proposed = definition.name.clone();
            if ui.text_edit_singleline(&mut proposed).changed() {
                let valid = !proposed.trim().is_empty()
                    && !other_names.iter().any(|name| name == &proposed.trim().to_lowercase());
                if valid {
                    definition.name = proposed.trim().to_owned();
                    edit.changed = true;
                }
            }
        });

        egui::CollapsingHeader::new("Propriétés génériques par défaut")
            .default_open(true)
            .show(ui, |ui| {
                let changed = ui.checkbox(&mut definition.defaults.has_collisions, "Collisions activées").changed()
                    | ui.checkbox(&mut definition.defaults.hidden, "Masqué").changed()
                    | ui.checkbox(&mut definition.defaults.fit_texture_to_diamond, "Ajuster la texture au losange").changed();
                edit.changed |= changed;
                ui.horizontal(|ui| {
                    ui.label("Empreinte");
                    let width = ui.add(egui::DragValue::new(&mut definition.defaults.footprint.width).range(1..=256).prefix("L ")).changed();
                    let height = ui.add(egui::DragValue::new(&mut definition.defaults.footprint.height).range(1..=256).prefix("H ")).changed();
                    if width || height {
                        definition.defaults.footprint.width = definition.defaults.footprint.width.max(1);
                        definition.defaults.footprint.height = definition.defaults.footprint.height.max(1);
                        edit.changed = true;
                    }
                });
                egui::ComboBox::from_id_salt(("custom_type_rotation", type_id))
                    .selected_text(format!("Rotation : {:?}", definition.defaults.rotation))
                    .show_ui(ui, |ui| {
                        for rotation in [Rotation4::North, Rotation4::East, Rotation4::South, Rotation4::West] {
                            edit.changed |= ui.selectable_value(&mut definition.defaults.rotation, rotation, format!("{rotation:?}")).changed();
                        }
                    });
                let layer_names = ["Terrain", "Décoration", "Ground", "Objects", "Buildings", "Foreground", "Overlay"];
                let layers = [Layer::Terrain, Layer::Decoration, Layer::Ground, Layer::Objects, Layer::Buildings, Layer::Foreground, Layer::Overlay];
                let layer_index = definition.defaults.layer.sort_key() as usize;
                egui::ComboBox::from_id_salt(("custom_type_layer", type_id))
                    .selected_text(format!("Couche : {}", layer_names.get(layer_index).copied().unwrap_or("Objects")))
                    .show_ui(ui, |ui| {
                        for (index, layer) in layers.into_iter().enumerate() {
                            edit.changed |= ui.selectable_value(&mut definition.defaults.layer, layer, layer_names[index]).changed();
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Texture");
                    let mut texture = definition.defaults.texture.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut texture).changed() {
                        definition.defaults.texture = (!texture.trim().is_empty()).then(|| texture.trim().to_owned());
                        edit.changed = true;
                    }
                    if ui.small_button("…").on_hover_text("Choisir une texture").clicked() {
                        *request_texture_picker = true;
                    }
                    if definition.defaults.texture.is_some() && ui.small_button("×").clicked() {
                        definition.defaults.texture = None;
                        edit.changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Script Python");
                    let mut script = definition.defaults.script.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut script).changed() {
                        definition.defaults.script = (!script.trim().is_empty()).then(|| script.trim().to_owned());
                        edit.changed = true;
                    }
                    if ui.small_button("…").on_hover_text("Choisir un script Python").clicked() {
                        *request_script_picker = true;
                    }
                    if definition.defaults.script.is_some() && ui.small_button("×").on_hover_text("Retirer le script").clicked() {
                        definition.defaults.script = None;
                        edit.changed = true;
                    }
                });
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Propriétés personnalisées").strong());
            if ui.small_button("+ Ajouter").clicked() {
                let mut number = 1;
                let key = loop {
                    let candidate = format!("propriete_{number}");
                    if !definition.properties.iter().any(|property| property.key == candidate) { break candidate; }
                    number += 1;
                };
                definition.properties.push(CustomPropertyDefinition {
                    key, kind: CustomPropertyKind::Text, default_value: PropValue::Str(String::new()),
                });
                edit.changed = true;
            }
        });
        let mut remove = None;
        let mut keys: Vec<_> = definition.properties.iter().map(|property| property.key.clone()).collect();
        for (property_index, property) in definition.properties.iter_mut().enumerate() {
            ui.separator();
            ui.horizontal(|ui| {
                let old_key = property.key.clone();
                let response = ui.add(egui::TextEdit::singleline(&mut property.key).desired_width(110.0));
                if response.changed() {
                    let valid = is_valid_property_key(&property.key) && property.key != "texture";
                    let unique = keys.iter().enumerate().all(|(i, key)| i == property_index || key != &property.key);
                    if valid && unique {
                        keys[property_index] = property.key.clone();
                        edit.renamed_property = Some((type_id, old_key, property.key.clone()));
                        edit.changed = true;
                    } else {
                        property.key = old_key;
                    }
                }
                egui::ComboBox::from_id_salt(("custom_property_kind", type_id, property_index))
                    .selected_text(custom_property_kind_label(property.kind))
                    .show_ui(ui, |ui| {
                        for kind in [CustomPropertyKind::Text, CustomPropertyKind::Integer, CustomPropertyKind::Float, CustomPropertyKind::Boolean, CustomPropertyKind::Scene] {
                            if ui.selectable_value(&mut property.kind, kind, custom_property_kind_label(kind)).changed() {
                                property.default_value = default_custom_property_value(kind, scene_names);
                                edit.changed = true;
                            }
                        }
                    });
                if ui.small_button("×").on_hover_text("Retirer cette propriété").clicked() {
                    remove = Some(property_index);
                }
            });
            edit.changed |= show_custom_property_default(ui, type_id, property_index, property, scene_names);
        }
        if let Some(index) = remove {
            definition.properties.remove(index);
            edit.changed = true;
        }
        edit
    }
}

fn show_custom_property_editor(
    ui: &mut Ui,
    type_id: uuid::Uuid,
    definition: &CustomPropertyDefinition,
    value: &mut PropValue,
    scene_names: &[String],
) -> bool {
    let mut changed = false;
    match (definition.kind, &mut *value) {
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
            let label = if value.is_empty() { "Choisir une scène…" } else { value.as_str() };
            egui::ComboBox::from_id_salt(("inspector_scene_property", type_id, &definition.key))
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for scene_name in scene_names {
                        changed |= ui.selectable_value(value, scene_name.clone(), scene_name).changed();
                    }
                });
        }
        _ => {
            *value = definition.default_value.clone();
            changed = true;
        }
    }
    changed
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
