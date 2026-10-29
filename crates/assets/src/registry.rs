//! Registre de ressources : stockage central.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::handle::{AssetId, Handle};

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
    pub fn new() -> Self { Self::default() }

    /// Enregistre un chemin et retourne un handle (pas encore chargé).
    pub fn register(&mut self, path: impl AsRef<Path>) -> Handle<T> {
        let path = path.as_ref().to_owned();
        if let Some(&id) = self.path_index.get(&path) {
            return Handle::new(id);
        }
        let id = AssetId::new();
        self.entries.insert(id, AssetEntry {
            id,
            source_path: path.clone(),
            data: None,
        });
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
        self.entries.get(&handle.id)
            .map(|e| e.data.is_some())
            .unwrap_or(false)
    }

    pub fn source_path(&self, handle: Handle<T>) -> Option<&Path> {
        self.entries.get(&handle.id).map(|e| e.source_path.as_path())
    }
}
