//! Mesure du temps et calcul du delta.

use std::time::{Duration, Instant};

/// Gestion du temps de la boucle de jeu.
#[derive(Debug)]
pub struct Time {
    /// Instant du dernier frame.
    last_frame: Instant,
    /// Durée du dernier frame (delta time).
    pub delta: Duration,
    /// Temps total écoulé depuis le démarrage.
    pub elapsed: Duration,
    /// Numéro de frame.
    pub frame_count: u64,
    /// FPS cible (0 = illimité).
    pub target_fps: u32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            last_frame: Instant::now(),
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
            frame_count: 0,
            target_fps: 60,
        }
    }
}

impl Time {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appelé au début de chaque frame — met à jour delta et elapsed.
    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = now.duration_since(self.last_frame);
        self.elapsed += self.delta;
        self.last_frame = now;
        self.frame_count += 1;
    }

    /// Delta time en secondes (f32, pratique pour la physique).
    pub fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// FPS mesuré (1 / delta).
    pub fn fps(&self) -> f32 {
        if self.delta.is_zero() {
            0.0
        } else {
            1.0 / self.delta.as_secs_f32()
        }
    }

    /// Retourne `true` si on est en avance sur le FPS cible et peut dormir.
    pub fn sleep_if_needed(&self) {
        if self.target_fps == 0 {
            return;
        }
        let target_frame = Duration::from_secs_f64(1.0 / self.target_fps as f64);
        if self.delta < target_frame {
            std::thread::sleep(target_frame - self.delta);
        }
    }
}
