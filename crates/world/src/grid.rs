//! Grille de tuiles — stockage compact par chunks.

use serde::{Deserialize, Serialize};
use crate::coords::TilePos;

/// Définition d'un type de tuile propre à un projet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileDefinition {
    pub id: u16,
    pub name: String,
    /// Couleur RGBA utilisée quand aucune texture n'est assignée.
    pub color: [u8; 4],
    /// Chemin de texture relatif au dossier racine du projet.
    #[serde(default)]
    pub texture: Option<String>,
    /// Métadonnée destinée aux futurs outils de collision.
    #[serde(default)]
    pub has_collisions: bool,
    /// Choisit une orientation stable pseudo-aléatoire par tuile pour la texture.
    #[serde(default)]
    pub randomize_texture_rotation: bool,
}

impl TileDefinition {
    pub fn new(id: u16, name: impl Into<String>, color: [u8; 4]) -> Self {
        Self {
            id,
            name: name.into(),
            color,
            texture: None,
            has_collisions: false,
            randomize_texture_rotation: false,
        }
    }
}

/// Les six types historiques restent le jeu de tuiles initial de chaque projet.
pub fn default_tile_definitions() -> Vec<TileDefinition> {
    [
        (1, "Herbe", [80, 140, 70, 255]),
        (2, "Eau", [60, 100, 160, 255]),
        (3, "Sable", [160, 140, 100, 255]),
        (4, "Terre", [100, 90, 80, 255]),
        (5, "Pierre", [130, 130, 130, 255]),
        (6, "Désert", [200, 180, 140, 255]),
    ]
    .into_iter()
    .map(|(id, name, color)| TileDefinition::new(id, name, color))
    .collect()
}

/// Données d'une tuile individuelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TileData {
    /// Index du type de tuile dans la tileset (0 = vide).
    pub tile_type: u16,
    /// Index de décoration superposée (0 = aucune).
    pub decoration: u16,
}

impl TileData {
    pub fn empty() -> Self {
        Self { tile_type: 0, decoration: 0 }
    }

    pub fn is_empty(self) -> bool {
        self.tile_type == 0 && self.decoration == 0
    }

    pub fn terrain(tile_type: u16) -> Self {
        Self { tile_type, decoration: 0 }
    }
}

/// Taille d'un chunk (en tuiles par côté).
/// 16×16 = 256 tuiles, bon compromis entre granularité et overhead.
pub const CHUNK_SIZE: i32 = 16;

/// Un chunk de tuiles (région fixe de la grille).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileChunk {
    /// Coordonnée du coin supérieur-gauche du chunk (en coordonnées chunk, pas tuile).
    pub chunk_x: i32,
    pub chunk_y: i32,
    /// Données plates, ligne par ligne (row-major) : [y * CHUNK_SIZE + x].
    pub tiles: Vec<TileData>,
}

impl TileChunk {
    pub fn new(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            tiles: vec![TileData::empty(); (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    fn local_index(local_x: i32, local_y: i32) -> usize {
        (local_y * CHUNK_SIZE + local_x) as usize
    }

    pub fn get(&self, local_x: i32, local_y: i32) -> TileData {
        self.tiles[Self::local_index(local_x, local_y)]
    }

    pub fn set(&mut self, local_x: i32, local_y: i32, data: TileData) {
        let idx = Self::local_index(local_x, local_y);
        self.tiles[idx] = data;
    }

    /// Retourne `true` si le chunk ne contient que des tuiles vides.
    pub fn is_empty(&self) -> bool {
        self.tiles.iter().all(|t| t.is_empty())
    }
}

/// Grille isométrique complète, partitionnée en chunks.
///
/// Seuls les chunks non vides sont stockés.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TileGrid {
    /// Chunks indexés par (chunk_x, chunk_y).
    #[serde(
        serialize_with = "serialize_chunks",
        deserialize_with = "deserialize_chunks"
    )]
    pub chunks: std::collections::HashMap<(i32, i32), TileChunk>,
}

fn serialize_chunks<S>(
    chunks: &std::collections::HashMap<(i32, i32), TileChunk>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let list: Vec<&TileChunk> = chunks.values().collect();
    list.serialize(serializer)
}

fn deserialize_chunks<'de, D>(
    deserializer: D,
) -> Result<std::collections::HashMap<(i32, i32), TileChunk>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let list = Vec::<TileChunk>::deserialize(deserializer)?;
    let mut map = std::collections::HashMap::with_capacity(list.len());
    for chunk in list {
        map.insert((chunk.chunk_x, chunk.chunk_y), chunk);
    }
    Ok(map)
}

impl TileGrid {
    pub fn new() -> Self {
        Self::default()
    }

    fn chunk_coords(tile: TilePos) -> (i32, i32, i32, i32) {
        let cx = tile.x.div_euclid(CHUNK_SIZE);
        let cy = tile.y.div_euclid(CHUNK_SIZE);
        let lx = tile.x.rem_euclid(CHUNK_SIZE);
        let ly = tile.y.rem_euclid(CHUNK_SIZE);
        (cx, cy, lx, ly)
    }

    pub fn get(&self, tile: TilePos) -> TileData {
        let (cx, cy, lx, ly) = Self::chunk_coords(tile);
        self.chunks
            .get(&(cx, cy))
            .map(|c| c.get(lx, ly))
            .unwrap_or_default()
    }

    pub fn set(&mut self, tile: TilePos, data: TileData) {
        let (cx, cy, lx, ly) = Self::chunk_coords(tile);
        let chunk = self.chunks
            .entry((cx, cy))
            .or_insert_with(|| TileChunk::new(cx, cy));
        chunk.set(lx, ly, data);
    }

    /// Supprime le chunk si on le vide.
    pub fn clear(&mut self, tile: TilePos) {
        let (cx, cy, lx, ly) = Self::chunk_coords(tile);
        if let Some(chunk) = self.chunks.get_mut(&(cx, cy)) {
            chunk.set(lx, ly, TileData::empty());
            if chunk.is_empty() {
                self.chunks.remove(&(cx, cy));
            }
        }
    }

    /// Itère sur tous les chunks actifs.
    pub fn iter_chunks(&self) -> impl Iterator<Item = &TileChunk> {
        self.chunks.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_tile() {
        let mut grid = TileGrid::new();
        let pos = TilePos::new(5, 7);
        grid.set(pos, TileData::terrain(3));
        assert_eq!(grid.get(pos).tile_type, 3);
    }

    #[test]
    fn negative_coords() {
        let mut grid = TileGrid::new();
        let pos = TilePos::new(-3, -10);
        grid.set(pos, TileData::terrain(7));
        assert_eq!(grid.get(pos).tile_type, 7);
    }

    #[test]
    fn clear_removes_empty_chunk() {
        let mut grid = TileGrid::new();
        let pos = TilePos::new(0, 0);
        grid.set(pos, TileData::terrain(1));
        assert_eq!(grid.chunks.len(), 1);
        grid.clear(pos);
        assert_eq!(grid.chunks.len(), 0, "Le chunk vide doit être supprimé");
    }
}
