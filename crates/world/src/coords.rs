//! Coordonnées isométriques et projection.

use serde::{Deserialize, Serialize};

/// Position logique sur la grille (en tuiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
    /// Hauteur logique (élévation, ex: étage d'un bâtiment).
    pub z: i32,
}

impl TilePos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y, z: 0 }
    }

    pub fn with_z(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Position en pixels à l'écran.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPos {
    pub x: f32,
    pub y: f32,
}

impl ScreenPos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Paramètres de la projection isométrique.
///
/// La projection classique "dimetric" 2:1 utilise :
/// - `tile_width`  : largeur totale d'une tuile en pixels (ex: 64)
/// - `tile_height` : hauteur d'une tuile en pixels (ex: 32)  → ratio 2:1
///
/// Formules :
/// ```text
/// screen_x = origin_x + (tx - ty) * (tile_width / 2)
/// screen_y = origin_y + (tx + ty) * (tile_height / 2) - tz * tile_height
/// ```
#[derive(Debug, Clone, Copy)]
pub struct IsoProjection {
    /// Largeur d'une tuile (diamètre horizontal du losange).
    pub tile_width: f32,
    /// Hauteur d'une tuile (diamètre vertical du losange).
    pub tile_height: f32,
}

impl Default for IsoProjection {
    fn default() -> Self {
        Self {
            tile_width: 64.0,
            tile_height: 32.0,
        }
    }
}

impl IsoProjection {
    pub fn new(tile_width: f32, tile_height: f32) -> Self {
        Self {
            tile_width,
            tile_height,
        }
    }

    /// Projette une position logique en position écran relative à l'origine (0, 0).
    /// La caméra et le zoom sont appliqués par l'appelant.
    pub fn tile_to_screen(&self, pos: TilePos) -> ScreenPos {
        let tx = pos.x as f32;
        let ty = pos.y as f32;
        let tz = pos.z as f32;
        ScreenPos {
            x: (tx - ty) * (self.tile_width * 0.5),
            y: (tx + ty) * (self.tile_height * 0.5) - tz * self.tile_height,
        }
    }

    /// Inverse la projection : pixel écran (relatif à l'origine de la grille) → tile logique.
    ///
    /// Formule inverse :
    /// ```text
    /// tx = (screen_x / (tile_width/2) + screen_y / (tile_height/2)) / 2
    /// ty = (screen_y / (tile_height/2) - screen_x / (tile_width/2)) / 2
    /// ```
    pub fn screen_to_tile(&self, screen: ScreenPos) -> TilePos {
        let sx = screen.x / (self.tile_width * 0.5);
        let sy = screen.y / (self.tile_height * 0.5);
        let tile_x = (sx + sy) * 0.5;
        let tile_y = (sy - sx) * 0.5;

        // Une tuile dessinée en losange correspond à |dx| + |dy| <= 1
        // dans l'espace logique. Floor() séparément sur x/y crée des cases
        // rectangulaires décalées par rapport aux losanges affichés.
        let base_x = tile_x.floor() as i32;
        let base_y = tile_y.floor() as i32;
        let mut closest = None;
        let mut closest_distance = f32::INFINITY;
        for y in (base_y - 1)..=(base_y + 2) {
            for x in (base_x - 1)..=(base_x + 2) {
                let distance = (tile_x - x as f32).abs() + (tile_y - y as f32).abs();
                if distance <= 1.0 + 1e-5 && distance < closest_distance {
                    closest = Some(TilePos::new(x, y));
                    closest_distance = distance;
                }
            }
        }
        closest.unwrap_or_else(|| TilePos::new(tile_x.round() as i32, tile_y.round() as i32))
    }

    /// Demi-largeur de tuile (raccourci utile pour le rendu).
    pub fn half_width(&self) -> f32 {
        self.tile_width * 0.5
    }

    /// Demi-hauteur de tuile.
    pub fn half_height(&self) -> f32 {
        self.tile_height * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_projection() {
        let proj = IsoProjection::default();
        let pos = TilePos::new(3, 5);
        let screen = proj.tile_to_screen(pos);
        let back = proj.screen_to_tile(screen);
        assert_eq!(pos, back, "La projection aller-retour doit être stable");
    }

    #[test]
    fn origin_at_zero() {
        let proj = IsoProjection::default();
        let screen = proj.tile_to_screen(TilePos::new(0, 0));
        assert_eq!(screen.x, 0.0);
        assert_eq!(screen.y, 0.0);
    }

    #[test]
    fn elevation_offsets_y() {
        let proj = IsoProjection::default();
        let base = proj.tile_to_screen(TilePos::with_z(2, 2, 0));
        let raised = proj.tile_to_screen(TilePos::with_z(2, 2, 1));
        assert!(
            raised.y < base.y,
            "L'élévation doit déplacer vers le haut (y négatif)"
        );
    }
}
