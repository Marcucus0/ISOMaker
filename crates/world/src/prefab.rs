//! Système de Prefabs et Construction Modulaire de bâtiments.
//!
//! Distingue clairement :
//! - `BuildingModuleKind` : catégorie d'un module de construction (mur, toit, porte...)
//! - `Prefab` : modèle réutilisable d'un bâtiment ou ensemble d'objets avec positions relatives
//! - `PrefabId` : identifiant stable d'un prefab
//! - `SceneObject` : instance concrète placée dans le monde

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::{
    coords::TilePos,
    layer::Layer,
    object::{ObjectType, PropValue, Rotation4, SceneObject, TileSize},
};

/// Identifiant unique d'un Prefab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrefabId(pub Uuid);

impl PrefabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PrefabId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PrefabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Types de modules de construction de bâtiments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingModuleKind {
    Wall,
    Corner,
    Door,
    Window,
    Roof,
    Stairs,
    Pillar,
    Decoration,
}

impl BuildingModuleKind {
    pub fn label(&self) -> &'static str {
        match self {
            BuildingModuleKind::Wall       => "Mur",
            BuildingModuleKind::Corner     => "Angle",
            BuildingModuleKind::Door       => "Porte",
            BuildingModuleKind::Window     => "Fenêtre",
            BuildingModuleKind::Roof       => "Toit",
            BuildingModuleKind::Stairs     => "Escalier",
            BuildingModuleKind::Pillar     => "Pilier",
            BuildingModuleKind::Decoration => "Décoration",
        }
    }

}

/// Description d'un élément composant un prefab (coordonnées relatives à l'ancre).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabElement {
    /// Nom lisible du module.
    pub name: String,
    /// Type de module.
    pub module_kind: BuildingModuleKind,
    /// Décalage relatif en tuiles par rapport à la racine du prefab.
    pub relative_pos: TilePos,
    /// Empreinte sur la grille.
    pub footprint: TileSize,
    /// Couche par défaut.
    pub layer: Layer,
    /// Chemin de ressource optionnel.
    pub asset_path: Option<String>,
    /// Orientation par défaut.
    pub rotation: Rotation4,
    /// Propriétés par défaut.
    pub properties: HashMap<String, PropValue>,
}

impl PrefabElement {
    pub fn new(name: impl Into<String>, module_kind: BuildingModuleKind, relative_pos: TilePos) -> Self {
        let mut properties = HashMap::new();
        properties.insert("texture".to_owned(), PropValue::Str(String::new()));
        Self {
            name: name.into(),
            module_kind,
            relative_pos,
            footprint: TileSize::default(),
            layer: Layer::Buildings,
            asset_path: None,
            rotation: Rotation4::default(),
            properties,
        }
    }
}

/// Un modèle réutilisable (Prefab) composé d'un ou plusieurs modules assemblés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prefab {
    pub id: PrefabId,
    pub name: String,
    /// Description optionnelle.
    pub description: String,
    /// Éléments composants du prefab avec leurs positions relatives.
    pub elements: Vec<PrefabElement>,
}

impl Prefab {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PrefabId::new(),
            name: name.into(),
            description: String::new(),
            elements: Vec::new(),
        }
    }

    /// Ajoute un module au prefab.
    pub fn add_element(&mut self, element: PrefabElement) {
        self.elements.push(element);
    }

    /// Crée des instances concrètes (`SceneObject`) dans la scène à la position donnée.
    pub fn instantiate_at(&self, base_pos: TilePos) -> Vec<SceneObject> {
        self.elements
            .iter()
            .map(|elem| {
                let mut obj = SceneObject::new(
                    format!("{}: {}", self.name, elem.name),
                    ObjectType::Building,
                    TilePos::with_z(
                        base_pos.x + elem.relative_pos.x,
                        base_pos.y + elem.relative_pos.y,
                        base_pos.z + elem.relative_pos.z,
                    ),
                );
                obj.footprint = elem.footprint;
                obj.layer = elem.layer;
                obj.asset_path = elem.asset_path.clone();
                obj.rotation = elem.rotation;
                obj.properties = elem.properties.clone();
                obj.prefab_id = Some(self.id.0);
                obj
            })
            .collect()
    }

    /// Crée un prefab prédéfini d'une maisonnette modulaire complète (4 murs + porte + toit).
    pub fn preset_house() -> Self {
        let mut prefab = Self::new("Maisonnette Type");
        prefab.description = "Bâtiment modulaire 2x2 avec murs, porte et toit".to_string();

        prefab.add_element(PrefabElement::new("Porte d'entrée", BuildingModuleKind::Door, TilePos::new(0, 0)));
        prefab.add_element(PrefabElement::new("Mur Ouest", BuildingModuleKind::Wall, TilePos::new(0, 1)));
        prefab.add_element(PrefabElement::new("Mur Nord", BuildingModuleKind::Wall, TilePos::new(1, 1)));
        prefab.add_element(PrefabElement::new("Fenêtre Est", BuildingModuleKind::Window, TilePos::new(1, 0)));
        // Toit à l'étage Z = 1
        prefab.add_element(PrefabElement::new("Toiture", BuildingModuleKind::Roof, TilePos::with_z(0, 0, 1)));

        prefab
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefab_instantiate() {
        let prefab = Prefab::preset_house();
        assert_eq!(prefab.elements.len(), 5);

        let instances = prefab.instantiate_at(TilePos::new(5, 5));
        assert_eq!(instances.len(), 5);

        // La porte est à (5, 5, 0)
        assert_eq!(instances[0].tile_pos, TilePos::with_z(5, 5, 0));
        assert_eq!(instances[0].prefab_id, Some(prefab.id.0));

        // La toiture est à (5, 5, 1)
        assert_eq!(instances[4].tile_pos, TilePos::with_z(5, 5, 1));
    }
}
