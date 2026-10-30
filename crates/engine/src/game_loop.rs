//! Boucle de jeu principale et modes de l'application.

/// Mode courant de l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    /// L'éditeur est actif, la simulation est arrêtée.
    Editor,
    /// La simulation tourne.
    Playing,
    /// La simulation est suspendue.
    Paused,
}

impl Default for EngineMode {
    fn default() -> Self {
        EngineMode::Editor
    }
}

impl std::fmt::Display for EngineMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineMode::Editor => write!(f, "Éditeur"),
            EngineMode::Playing => write!(f, "Lecture"),
            EngineMode::Paused => write!(f, "Pause"),
        }
    }
}

/// Contrôle de la boucle de jeu (transitions de mode).
#[derive(Debug, Default)]
pub struct GameLoop {
    pub mode: EngineMode,
}

impl GameLoop {
    pub fn new() -> Self {
        Self::default()
    }

    /// Démarre la simulation depuis l'éditeur.
    /// Retourne `true` si la transition est valide.
    pub fn play(&mut self) -> bool {
        if self.mode == EngineMode::Editor || self.mode == EngineMode::Paused {
            self.mode = EngineMode::Playing;
            true
        } else {
            false
        }
    }

    /// Met la simulation en pause.
    pub fn pause(&mut self) -> bool {
        if self.mode == EngineMode::Playing {
            self.mode = EngineMode::Paused;
            true
        } else {
            false
        }
    }

    /// Reprend depuis la pause.
    pub fn resume(&mut self) -> bool {
        if self.mode == EngineMode::Paused {
            self.mode = EngineMode::Playing;
            true
        } else {
            false
        }
    }

    /// Arrête la simulation, retour à l'éditeur.
    pub fn stop(&mut self) -> bool {
        if self.mode != EngineMode::Editor {
            self.mode = EngineMode::Editor;
            true
        } else {
            false
        }
    }

    pub fn is_simulating(&self) -> bool {
        self.mode == EngineMode::Playing
    }

    pub fn is_editor(&self) -> bool {
        self.mode == EngineMode::Editor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_pause_stop_cycle() {
        let mut gl = GameLoop::new();
        assert_eq!(gl.mode, EngineMode::Editor);

        assert!(gl.play());
        assert_eq!(gl.mode, EngineMode::Playing);

        assert!(gl.pause());
        assert_eq!(gl.mode, EngineMode::Paused);

        // Rejouer depuis pause est valide
        assert!(gl.play());
        assert_eq!(gl.mode, EngineMode::Playing);

        assert!(gl.stop());
        assert_eq!(gl.mode, EngineMode::Editor);
    }

    #[test]
    fn cannot_pause_from_editor() {
        let mut gl = GameLoop::new();
        assert!(!gl.pause());
        assert_eq!(gl.mode, EngineMode::Editor);
    }
}
