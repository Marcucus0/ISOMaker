//! Hiérarchie persistante Projet → Monde → Scènes.

use serde::{Deserialize, Serialize};

use crate::{
    grid::{default_tile_definitions, TileDefinition},
    object::{CustomObjectTypeDefinition, CustomPropertyKind, PropValue},
    scene::Scene,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct World {
    pub name: String,
    /// Script Python global associé au monde, relatif au dossier du projet.
    #[serde(default)]
    pub script: Option<String>,
    pub scenes: Vec<Scene>,
    pub active_scene_index: usize,
    /// Scène chargée au démarrage de la Lecture et du lecteur autonome.
    #[serde(default)]
    pub main_scene_index: usize,
}

impl World {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            script: None,
            scenes: vec![Scene::new("Scène principale")],
            active_scene_index: 0,
            main_scene_index: 0,
        }
    }

    pub fn active_scene(&self) -> &Scene {
        self.scenes.get(self.active_scene_index)
            .expect("World must contain a valid active scene")
    }

    pub fn active_scene_mut(&mut self) -> &mut Scene {
        self.scenes.get_mut(self.active_scene_index)
            .expect("World must contain a valid active scene")
    }

    pub fn add_scene(&mut self, name: impl Into<String>) -> usize {
        self.scenes.push(Scene::new(name));
        let index = self.scenes.len() - 1;
        self.active_scene_index = index;
        index
    }

    pub fn select_scene(&mut self, index: usize) -> bool {
        if index >= self.scenes.len() {
            return false;
        }
        self.active_scene_index = index;
        true
    }

    /// Supprime une scène par index, en conservant toujours au moins une scène.
    /// Retourne `None` si l'index est invalide ou s'il s'agit de la dernière scène.
    pub fn remove_scene(&mut self, index: usize) -> Option<Scene> {
        if self.scenes.len() <= 1 || index >= self.scenes.len() {
            return None;
        }
        let removed = self.scenes.remove(index);
        if self.active_scene_index > index {
            self.active_scene_index -= 1;
        } else if self.active_scene_index == index {
            self.active_scene_index = index.min(self.scenes.len() - 1);
        }
        if self.main_scene_index > index {
            self.main_scene_index -= 1;
        } else if self.main_scene_index == index {
            self.main_scene_index = index.min(self.scenes.len() - 1);
        }
        Some(removed)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub name: String,
    /// A project owns exactly one world; scenes are collected by that world.
    pub world: World,
    /// Définitions de tuiles isolées dans ce projet.
    #[serde(default = "default_tile_definitions")]
    pub tile_definitions: Vec<TileDefinition>,
    /// Schémas de propriétés des types d'objets créés par l'utilisateur.
    #[serde(default)]
    pub object_types: Vec<CustomObjectTypeDefinition>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format_version: 1,
            name: name.into(),
            world: World::new("Monde principal"),
            tile_definitions: default_tile_definitions(),
            object_types: Vec::new(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let project: Self = serde_json::from_str(json)?;
        anyhow::ensure!(project.format_version == 1, "Version de projet non prise en charge");
        anyhow::ensure!(!project.world.scenes.is_empty(), "Le monde doit contenir au moins une scène");
        anyhow::ensure!(
            project.world.active_scene_index < project.world.scenes.len(),
            "L'indice de la scène active est invalide"
        );
        anyhow::ensure!(
            project.world.main_scene_index < project.world.scenes.len(),
            "L'indice de la scène principale est invalide"
        );
        let mut tile_ids = std::collections::HashSet::new();
        for tile in &project.tile_definitions {
            anyhow::ensure!(tile.id != 0, "L'identifiant 0 est réservé à la tuile vide");
            anyhow::ensure!(tile_ids.insert(tile.id), "Identifiant de tuile dupliqué : {}", tile.id);
        }
        let mut type_ids = std::collections::HashSet::new();
        let mut type_names = std::collections::HashSet::new();
        for object_type in &project.object_types {
            anyhow::ensure!(type_ids.insert(object_type.id), "Identifiant de type d'objet dupliqué");
            anyhow::ensure!(!object_type.name.trim().is_empty(), "Un type d'objet ne peut pas avoir un nom vide");
            anyhow::ensure!(type_names.insert(object_type.name.to_lowercase()), "Nom de type d'objet dupliqué : {}", object_type.name);
            let mut property_keys = std::collections::HashSet::new();
            for property in &object_type.properties {
                anyhow::ensure!(
                    is_valid_custom_property_key(&property.key),
                    "Clé de propriété personnalisée invalide : {}",
                    property.key
                );
                anyhow::ensure!(property.key != "texture", "La clé `texture` est réservée");
                anyhow::ensure!(property_keys.insert(property.key.as_str()), "Clé de propriété dupliquée : {}", property.key);
                let value_matches_kind = matches!(
                    (&property.kind, &property.default_value),
                    (CustomPropertyKind::Text | CustomPropertyKind::Scene, PropValue::Str(_))
                        | (CustomPropertyKind::Integer, PropValue::Int(_))
                        | (CustomPropertyKind::Float, PropValue::Float(_))
                        | (CustomPropertyKind::Boolean, PropValue::Bool(_))
                );
                anyhow::ensure!(value_matches_kind, "Valeur initiale incompatible avec le type de propriété `{}`", property.key);
            }
        }
        Ok(project)
    }
}

fn is_valid_custom_property_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{coords::TilePos, object::ObjectType};

    #[test]
    fn project_roundtrip_preserves_world_scenes_and_camera_follow_settings() {
        let mut project = Project::new("Release fixture");
        project.world.script = Some("project/scripts/world.py".to_owned());
        let active_index = project.world.add_scene("Gameplay");
        let mut camera = crate::object::SceneObject::new(
            "Main camera",
            ObjectType::Camera,
            TilePos::new(4, 6),
        );
        camera.camera_zoom = 1.75;
        camera.camera_interpolation = false;
        camera.camera_interpolation_speed = 13.5;
        project.world.active_scene_mut().add_object(camera);

        let saved = project.to_json().unwrap();
        let loaded = Project::from_json(&saved).unwrap();
        assert_eq!(loaded.world.name, "Monde principal");
        assert_eq!(loaded.world.script.as_deref(), Some("project/scripts/world.py"));
        assert_eq!(loaded.world.scenes.len(), 2);
        assert_eq!(loaded.world.active_scene_index, active_index);
        let camera = loaded.world.active_scene().objects.values().next().unwrap();
        assert_eq!(camera.camera_zoom, 1.75);
        assert!(!camera.camera_interpolation);
        assert_eq!(camera.camera_interpolation_speed, 13.5);
    }

    #[test]
    fn old_camera_data_receives_interpolation_defaults() {
        let camera = crate::object::SceneObject::new(
            "Legacy camera",
            ObjectType::Camera,
            TilePos::new(0, 0),
        );
        let mut value = serde_json::to_value(camera).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("camera_interpolation");
        object.remove("camera_interpolation_speed");

        let restored: crate::object::SceneObject = serde_json::from_value(value).unwrap();
        assert!(restored.camera_interpolation);
        assert_eq!(restored.camera_interpolation_speed, 8.0);
    }

    #[test]
    fn custom_tile_properties_roundtrip_with_project_and_old_projects_get_defaults() {
        let mut project = Project::new("Custom tiles");
        let mut tile = crate::grid::TileDefinition::new(7, "Animated grass", [20, 180, 70, 255]);
        tile.texture = Some("custom_tiles/assets/grass.gif".to_owned());
        tile.has_collisions = true;
        tile.randomize_texture_rotation = true;
        project.tile_definitions.push(tile.clone());

        let saved = project.to_json().unwrap();
        let loaded = Project::from_json(&saved).unwrap();
        assert_eq!(loaded.tile_definitions.last(), Some(&tile));

        let mut legacy = serde_json::to_value(Project::new("Legacy project")).unwrap();
        legacy.as_object_mut().unwrap().remove("tile_definitions");
        let loaded_legacy = Project::from_json(&legacy.to_string()).unwrap();
        assert_eq!(loaded_legacy.tile_definitions, default_tile_definitions());
    }
}
