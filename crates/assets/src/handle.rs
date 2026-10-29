//! Handles typés pour les ressources.

use uuid::Uuid;
use std::marker::PhantomData;

/// Identifiant unique d'une ressource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub Uuid);

impl AssetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AssetId {
    fn default() -> Self { Self::new() }
}

/// Handle typé vers une ressource dans le registre.
/// N'implique pas de propriété — la ressource est dans le registre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub id: AssetId,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub fn new(id: AssetId) -> Self {
        Self { id, _phantom: PhantomData }
    }

    pub fn new_unique() -> Self {
        Self::new(AssetId::new())
    }
}
