//! Crate `world` — Logique du monde isométrique.
//!
//! Responsabilités :
//! - Coordonnées logiques (tile) et projection isométrique vers l'écran.
//! - Grille, chunks de tuiles, couches.
//! - Objets et scènes.
//! - Sérialisation/désérialisation de la scène.

pub mod coords;
pub mod grid;
pub mod layer;
pub mod object;
pub mod project;
pub mod scene;

pub use coords::{IsoProjection, ScreenPos, TilePos};
pub use grid::TileDefinition;
pub use grid::TileGrid;
pub use layer::Layer;
pub use object::{
    CustomObjectTypeDefinition, CustomPropertyDefinition, CustomPropertyKind, ObjectId, ObjectType,
    ObjectTypeDefaults, PropValue, Rotation4, SceneObject,
};
pub use project::{Project, World};
pub use scene::Scene;
