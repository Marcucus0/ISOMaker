//! Registre de ressources : stockage central.

use crate::handle::{AssetId, Handle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Entrée dans le registre.
#[derive(Debug)]
pub struct AssetEntry<T> {
    pub id: AssetId,
    /// Chemin source relatif au projet.
    pub source_path: PathBuf,
    /// Données de la ressource (None si pas encore chargée).
    pub data: Option<T>,
}

/// Registre typé d'assets.
pub struct AssetRegistry<T> {
    entries: HashMap<AssetId, AssetEntry<T>>,
    /// Index chemin → id pour la déduplication.
    path_index: HashMap<PathBuf, AssetId>,
}

impl<T> Default for AssetRegistry<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            path_index: HashMap::new(),
        }
    }
}

impl<T> AssetRegistry<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre un chemin et retourne un handle (pas encore chargé).
    pub fn register(&mut self, path: impl AsRef<Path>) -> Handle<T> {
        let path = path.as_ref().to_owned();
        if let Some(&id) = self.path_index.get(&path) {
            return Handle::new(id);
        }
        let id = AssetId::new();
        self.entries.insert(
            id,
            AssetEntry {
                id,
                source_path: path.clone(),
                data: None,
            },
        );
        self.path_index.insert(path, id);
        Handle::new(id)
    }

    /// Stocke les données d'une ressource dans le registre.
    pub fn store(&mut self, handle: Handle<T>, data: T) {
        if let Some(entry) = self.entries.get_mut(&handle.id) {
            entry.data = Some(data);
        }
    }

    /// Récupère les données d'une ressource.
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.entries.get(&handle.id)?.data.as_ref()
    }

    /// Libère les données d'une ressource (garde le handle).
    pub fn evict(&mut self, handle: Handle<T>) {
        if let Some(entry) = self.entries.get_mut(&handle.id) {
            entry.data = None;
        }
    }

    pub fn is_loaded(&self, handle: Handle<T>) -> bool {
        self.entries
            .get(&handle.id)
            .map(|e| e.data.is_some())
            .unwrap_or(false)
    }

    pub fn source_path(&self, handle: Handle<T>) -> Option<&Path> {
        self.entries
            .get(&handle.id)
            .map(|e| e.source_path.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_reuses_handle_and_evict_keeps_registration() {
        let mut registry = AssetRegistry::<String>::new();
        let first_id = registry.register("assets/terrain.png").id;
        let duplicate_id = registry.register("assets/terrain.png").id;
        assert_eq!(first_id, duplicate_id);
        assert_eq!(
            registry.source_path(Handle::new(first_id)),
            Some(Path::new("assets/terrain.png"))
        );
        assert!(!registry.is_loaded(Handle::new(first_id)));

        registry.store(Handle::new(first_id), "decoded image".to_owned());
        assert_eq!(
            registry.get(Handle::new(duplicate_id)).map(String::as_str),
            Some("decoded image")
        );
        registry.evict(Handle::new(first_id));
        assert!(!registry.is_loaded(Handle::new(first_id)));
        assert_eq!(
            registry.source_path(Handle::new(first_id)),
            Some(Path::new("assets/terrain.png"))
        );
    }
}
