//! Crate `assets` — Chargement, cache et gestion des ressources.

pub mod handle;
pub mod loader;
pub mod registry;

pub use handle::{AssetId, Handle};
pub use registry::AssetRegistry;
