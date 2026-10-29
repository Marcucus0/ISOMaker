//! Scène : conteneur principal d'un monde éditable.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{
    coords::{IsoProjection, TilePos},
    grid::{TileData, TileGrid},
    object::{ObjectId, ObjectType, SceneObject},
};

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
        let id = obj.id;
        self.objects.insert(id, obj);
        id
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<SceneObject> {
        self.objects.remove(&id)
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

    /// Instancie un Prefab complet dans la scène à la position logique donnée.
    pub fn instantiate_prefab(&mut self, prefab: &crate::prefab::Prefab, pos: TilePos) -> Vec<ObjectId> {
        let objects = prefab.instantiate_at(pos);
        objects.into_iter().map(|obj| self.add_object(obj)).collect()
    }

    /// Retourne les objets triés pour l'affichage isométrique (profondeur croissante).
    pub fn objects_sorted_for_render(&self) -> Vec<&SceneObject> {
        let mut objects: Vec<&SceneObject> = self.objects.values().collect();
        objects.sort_by(|a, b| {
            // Tri par couche d'abord, puis par profondeur isométrique.
            a.layer.sort_key()
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
        let far  = scene.spawn("far",  ObjectType::Item, TilePos::new(5, 5));
        let near = scene.spawn("near", ObjectType::Item, TilePos::new(0, 0));
        let sorted = scene.objects_sorted_for_render();
        let near_idx = sorted.iter().position(|o| o.id == near).unwrap();
        let far_idx  = sorted.iter().position(|o| o.id == far).unwrap();
        assert!(near_idx < far_idx, "L'objet le plus proche doit être affiché en premier");
    }
}
