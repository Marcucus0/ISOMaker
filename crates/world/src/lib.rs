//! Crate `world` — Logique du monde isométrique.
//!
//! Responsabilités :
//! - Coordonnées logiques (tile) et projection isométrique vers l'écran.
//! - Grille, chunks de tuiles, couches.
//! - Objets de scène, prefabs, instances.
//! - Sérialisation/désérialisation de la scène.

pub mod coords;
pub mod grid;
pub mod layer;
pub mod object;
pub mod scene;
pub mod prefab;
pub mod project;

pub use coords::{TilePos, ScreenPos, IsoProjection};
pub use grid::TileGrid;
pub use grid::TileDefinition;
pub use layer::Layer;
pub use object::{ObjectId, SceneObject, ObjectType, Rotation4};
pub use scene::Scene;
pub use prefab::{Prefab, PrefabId, PrefabElement, BuildingModuleKind};
pub use project::{Project, World};
