//! Couches de rendu isométrique.

use serde::{Deserialize, Serialize};

/// Les couches logiques de la scène, du bas vers le haut.
///
/// L'ordre d'affichage respecte cet ordre + le tri isométrique intra-couche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Layer {
    /// Terrain de base (dalles, herbe, eau…).
    Terrain = 0,
    /// Décorations posées sur le terrain (fleurs, cailloux…).
    Decoration = 1,
    /// Objets traversables (chemins, tapis…).
    Ground = 2,
    /// Objets solides, personnages, meubles.
    Objects = 3,
    /// Constructions et bâtiments.
    Buildings = 4,
    /// Éléments de premier plan (toitures, feuillages hauts…).
    Foreground = 5,
    /// Interface ou effets toujours au-dessus.
    Overlay = 6,
}

impl Default for Layer {
    fn default() -> Self {
        Layer::Objects
    }
}

impl Layer {
    /// Retourne un entier pour le tri (peut servir de clé dans un BTreeMap).
    pub fn sort_key(self) -> u8 {
        self as u8
    }
}
