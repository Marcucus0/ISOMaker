//! Caméra isométrique : position, zoom, déplacement.

/// Caméra 2D pour la vue isométrique.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Position du centre de la caméra dans l'espace monde (pixels iso).
    pub position: egui::Vec2,
    /// Facteur de zoom (1.0 = taille réelle).
    pub zoom: f32,
    /// Limites du zoom.
    pub zoom_min: f32,
    pub zoom_max: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: egui::Vec2::ZERO,
            zoom: 1.0,
            zoom_min: 0.1,
            zoom_max: 10.0,
        }
    }
}

impl Camera {
    pub fn new() -> Self { Self::default() }

    /// Applique le zoom en gardant le point d'écran fixe (zoom centré sur la souris).
    pub fn zoom_at(&mut self, screen_point: egui::Vec2, delta: f32) {
        let anchor_world = self.position + screen_point / self.zoom;
        // `smooth_scroll_delta` is measured in points and can exceed 100 for
        // one wheel notch. Exponential scaling keeps the response consistent
        // for both wheel mice and high-resolution trackpads.
        self.zoom = (self.zoom * (delta * 0.001).exp())
            .clamp(self.zoom_min, self.zoom_max);
        // Keep the world point under the cursor fixed while changing zoom.
        self.position = anchor_world - screen_point / self.zoom;
    }

    /// Déplace la caméra (en pixels écran, avant zoom).
    pub fn pan(&mut self, delta: egui::Vec2) {
        self.position -= delta;
    }

    /// Transforme une position monde (pixels iso) en position écran.
    pub fn world_to_screen(&self, world: egui::Vec2, viewport_center: egui::Vec2) -> egui::Vec2 {
        (world - self.position) * self.zoom + viewport_center
    }

    /// Transforme une position écran en position monde.
    pub fn screen_to_world(&self, screen: egui::Vec2, viewport_center: egui::Vec2) -> egui::Vec2 {
        (screen - viewport_center) / self.zoom + self.position
    }
}
