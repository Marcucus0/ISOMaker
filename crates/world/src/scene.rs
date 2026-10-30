//! Scène : conteneur principal d'un monde éditable.

use crate::{
    coords::{IsoProjection, TilePos},
    grid::{TileData, TileGrid},
    object::{ObjectId, ObjectType, SceneObject},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Métadonnées d'une scène.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMeta {
    pub format_version: u32,
    pub name: String,
    pub tile_width: f32,
    pub tile_height: f32,
}

impl Default for SceneMeta {
    fn default() -> Self {
        Self {
            format_version: 1,
            name: "Nouvelle scène".to_string(),
            tile_width: 64.0,
            tile_height: 32.0,
        }
    }
}

/// Scène complète : grille de tuiles + objets.
#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {
    pub meta: SceneMeta,
    pub grid: TileGrid,
    /// Objets de la scène indexés par leur identifiant.
    pub objects: HashMap<ObjectId, SceneObject>,
    /// Caméra utilisée par défaut au lancement de la lecture.
    #[serde(default)]
    pub main_camera: Option<ObjectId>,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            meta: SceneMeta {
                name: name.into(),
                ..SceneMeta::default()
            },
            grid: TileGrid::new(),
            objects: HashMap::new(),
            main_camera: None,
        }
    }

    /// Projection isométrique calculée depuis les métadonnées.
    pub fn projection(&self) -> IsoProjection {
        IsoProjection::new(self.meta.tile_width, self.meta.tile_height)
    }

    // ── Grille ────────────────────────────────────────────────────────────────

    pub fn set_tile(&mut self, pos: TilePos, data: TileData) {
        self.grid.set(pos, data);
    }

    pub fn get_tile(&self, pos: TilePos) -> TileData {
        self.grid.get(pos)
    }

    pub fn clear_tile(&mut self, pos: TilePos) {
        self.grid.clear(pos);
    }

    // ── Objets ────────────────────────────────────────────────────────────────

    pub fn add_object(&mut self, obj: SceneObject) -> ObjectId {
        let mut obj = obj;
        obj.name = self.unique_object_name(&obj.name, None);
        let id = obj.id;
        if obj.object_type == ObjectType::Camera && self.main_camera.is_none() {
            self.main_camera = Some(id);
        }
        self.objects.insert(id, obj);
        id
    }

    /// Retourne un nom libre dans la scène, en ignorant éventuellement un objet.
    /// En cas de doublon, ajoute le premier suffixe numérique disponible.
    pub fn unique_object_name(&self, requested: &str, except: Option<ObjectId>) -> String {
        let existing: std::collections::HashSet<&str> = self
            .objects
            .values()
            .filter(|object| Some(object.id) != except)
            .map(|object| object.name.as_str())
            .collect();

        if !existing.contains(requested) {
            return requested.to_owned();
        }

        let base = requested
            .rsplit_once(" (")
            .and_then(|(base, suffix)| {
                suffix.strip_suffix(')')?.parse::<u32>().ok().map(|_| base)
            })
            .unwrap_or(requested);

        let mut number = 1_u32;
        loop {
            let candidate = format!("{base} ({number})");
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            number = number.saturating_add(1);
        }
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<SceneObject> {
        let removed = self.objects.remove(&id);
        if self.main_camera == Some(id) {
            self.main_camera = self
                .objects
                .values()
                .find(|object| object.object_type == ObjectType::Camera)
                .map(|object| object.id);
        }
        removed
    }

    pub fn get_object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.get(&id)
    }

    pub fn get_object_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.objects.get_mut(&id)
    }

    /// Spawn simple : crée et ajoute un objet, retourne son identifiant.
    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        object_type: ObjectType,
        pos: TilePos,
    ) -> ObjectId {
        let obj = SceneObject::new(name, object_type, pos);
        self.add_object(obj)
    }

    /// Retourne les objets triés pour l'affichage isométrique (profondeur croissante).
    pub fn objects_sorted_for_render(&self) -> Vec<&SceneObject> {
        let mut objects: Vec<&SceneObject> = self.objects.values().collect();
        objects.sort_by(|a, b| {
            // Tri par couche d'abord, puis par profondeur isométrique.
            a.layer
                .sort_key()
                .cmp(&b.layer.sort_key())
                .then(a.iso_sort_key().cmp(&b.iso_sort_key()))
        });
        objects
    }

    // ── Sérialisation ─────────────────────────────────────────────────────────

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectType;

    #[test]
    fn add_and_remove_object() {
        let mut scene = Scene::new("Test");
        let id = scene.spawn("house", ObjectType::Building, TilePos::new(3, 4));
        assert!(scene.get_object(id).is_some());
        scene.remove_object(id);
        assert!(scene.get_object(id).is_none());
    }

    #[test]
    fn serialize_roundtrip() {
        let mut scene = Scene::new("RoundtripTest");
        scene.spawn("tree", ObjectType::Decoration, TilePos::new(1, 2));
        scene.set_tile(TilePos::new(0, 0), TileData::terrain(5));

        let json = scene.to_json().unwrap();
        let loaded = Scene::from_json(&json).unwrap();
        assert_eq!(loaded.meta.name, "RoundtripTest");
        assert_eq!(loaded.objects.len(), 1);
        assert_eq!(loaded.get_tile(TilePos::new(0, 0)).tile_type, 5);
    }

    #[test]
    fn iso_sort_order() {
        let mut scene = Scene::new("Sort");
        let far = scene.spawn("far", ObjectType::Item, TilePos::new(5, 5));
        let near = scene.spawn("near", ObjectType::Item, TilePos::new(0, 0));
        let sorted = scene.objects_sorted_for_render();
        let near_idx = sorted.iter().position(|o| o.id == near).unwrap();
        let far_idx = sorted.iter().position(|o| o.id == far).unwrap();
        assert!(
            near_idx < far_idx,
            "L'objet le plus proche doit être affiché en premier"
        );
    }
}
