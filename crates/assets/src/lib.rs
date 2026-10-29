//! Crate `assets` — Chargement, cache et gestion des ressources.

pub mod handle;
pub mod registry;
pub mod loader;

pub use handle::{Handle, AssetId};
pub use registry::AssetRegistry;
