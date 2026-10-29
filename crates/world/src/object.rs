//! Objets de scène : instances, types, prefabs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::{coords::TilePos, layer::Layer};

/// Identifiant unique et stable d'un objet dans la scène.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type général d'un objet (catégorie sémantique).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Tile,
    Decoration,
    Building,
    Character,
    Item,
    Camera,
    /// Type personnalisé défini par un script.
    Custom(String),
}

impl Default for ObjectType {
    fn default() -> Self {
        ObjectType::Custom("unknown".to_string())
    }
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::Tile => write!(f, "Tile"),
            ObjectType::Decoration => write!(f, "Decoration"),
            ObjectType::Building => write!(f, "Building"),
            ObjectType::Character => write!(f, "Character"),
            ObjectType::Item => write!(f, "Item"),
            ObjectType::Camera => write!(f, "Caméra"),
            ObjectType::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

/// Orientation isométrique (4 directions cardinales mappées à l'iso).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Rotation4 {
    #[default]
    North,
    East,
    South,
    West,
}

impl Rotation4 {
    pub fn next(self) -> Self {
        match self {
            Rotation4::North => Rotation4::East,
            Rotation4::East  => Rotation4::South,
            Rotation4::South => Rotation4::West,
            Rotation4::West  => Rotation4::North,
        }
    }
}

/// Empreinte d'un objet sur la grille (en tuiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileSize {
    pub width: u32,
    pub height: u32,
}

impl Default for TileSize {
    fn default() -> Self {
        Self { width: 1, height: 1 }
    }
}

impl TileSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Valeur générique utilisable dans les propriétés d'un objet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<PropValue>),
}

impl PropValue {
    pub fn as_str(&self) -> Option<&str> {
        if let PropValue::Str(s) = self { Some(s) } else { None }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let PropValue::Int(i) = self { Some(*i) } else { None }
    }
    pub fn as_float(&self) -> Option<f64> {
        if let PropValue::Float(f) = self { Some(*f) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let PropValue::Bool(b) = self { Some(*b) } else { None }
    }
}

/// Un objet placé dans la scène.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneObject {
    /// Identifiant unique (stable entre les sauvegardes).
    pub id: ObjectId,
    /// Nom lisible.
    pub name: String,
    /// Catégorie sémantique.
    pub object_type: ObjectType,
    /// Indique si l'objet participe aux collisions.
    #[serde(default)]
    pub has_collisions: bool,
    /// Masque l'objet dans la vue de scène et les rendus.
    #[serde(default)]
    pub hidden: bool,
    /// Position logique sur la grille.
    pub tile_pos: TilePos,
    /// Empreinte sur la grille.
    pub footprint: TileSize,
    /// Couche de rendu.
    pub layer: Layer,
    /// Chemin relatif vers la ressource graphique (PNG, etc.).
    pub asset_path: Option<String>,
    /// Déforme l'image dans le losange isométrique de la tuile.
    #[serde(default = "default_true")]
    pub fit_texture_to_diamond: bool,
    /// Orientation isométrique.
    pub rotation: Rotation4,
    /// Propriétés personnalisées (accessibles depuis Python).
    pub properties: HashMap<String, PropValue>,
    /// Script Python associé (chemin relatif au dossier du projet enregistré).
    pub script: Option<String>,
    /// Zoom appliqué quand cet objet sert de caméra de scène.
    #[serde(default = "default_camera_zoom")]
    pub camera_zoom: f32,
    /// Active le lissage du suivi de cette caméra en mode Lecture.
    #[serde(default = "default_true")]
    pub camera_interpolation: bool,
    /// Vitesse de convergence du suivi, en unités d'interpolation par seconde.
    #[serde(default = "default_camera_interpolation_speed")]
    pub camera_interpolation_speed: f32,
    /// Référence à un prefab si l'objet est une instance.
    pub prefab_id: Option<Uuid>,
}

impl SceneObject {
    pub fn new(name: impl Into<String>, object_type: ObjectType, tile_pos: TilePos) -> Self {
        Self {
            id: ObjectId::new(),
            name: name.into(),
            object_type,
            has_collisions: false,
            hidden: false,
            tile_pos,
            footprint: TileSize::default(),
            layer: Layer::default(),
            asset_path: None,
            fit_texture_to_diamond: true,
            rotation: Rotation4::default(),
            properties: HashMap::from([(
                "texture".to_owned(),
                PropValue::Str(String::new()),
            )]),
            script: None,
            camera_zoom: 1.0,
            camera_interpolation: true,
            camera_interpolation_speed: 8.0,
            prefab_id: None,
        }
    }

    pub fn get_prop(&self, key: &str) -> Option<&PropValue> {
        self.properties.get(key)
    }

    pub fn set_prop(&mut self, key: impl Into<String>, value: PropValue) {
        self.properties.insert(key.into(), value);
    }

    /// Dépth-sort key : pour le tri isométrique d'affichage.
    /// Les objets avec un `tile_pos.x + tile_pos.y` plus grand sont affichés après.
    pub fn iso_sort_key(&self) -> i32 {
        self.tile_pos.x + self.tile_pos.y
    }
}

fn default_true() -> bool {
    true
}

fn default_camera_zoom() -> f32 {
    1.0
}

fn default_camera_interpolation_speed() -> f32 {
    8.0
}
