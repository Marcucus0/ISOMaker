//! Système d'entrées clavier et souris.

use std::collections::HashSet;

/// Touches du clavier (sous-ensemble courant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    // Lettres
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Chiffres
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    // Contrôle
    Escape, Enter, Space, Backspace, Delete, Tab,
    Left, Right, Up, Down,
    Shift, Control, Alt,
    // Fonction
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Autre
    Unknown,
}

/// Boutons de la souris.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// État complet des entrées pour le frame courant.
#[derive(Debug, Default, Clone)]
pub struct InputState {
    /// Touches actuellement enfoncées.
    pressed_keys: HashSet<Key>,
    /// Touches enfoncées CE frame uniquement (just pressed).
    just_pressed_keys: HashSet<Key>,
    /// Touches relâchées CE frame.
    just_released_keys: HashSet<Key>,

    /// Boutons souris actuellement enfoncés.
    pressed_buttons: HashSet<MouseButton>,
    just_pressed_buttons: HashSet<MouseButton>,
    just_released_buttons: HashSet<MouseButton>,

    /// Position de la souris en pixels écran.
    pub mouse_pos: (f32, f32),
    /// Déplacement de la souris ce frame.
    pub mouse_delta: (f32, f32),
    /// Défilement de la molette.
    pub scroll_delta: f32,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Mise à jour interne (appelée par la boucle de jeu) ──────────────────

    /// Prépare un nouveau frame : vide les "just" pressed/released.
    pub fn begin_frame(&mut self) {
        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
        self.just_pressed_buttons.clear();
        self.just_released_buttons.clear();
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = 0.0;
    }

    pub fn key_down(&mut self, key: Key) {
        if self.pressed_keys.insert(key) {
            self.just_pressed_keys.insert(key);
        }
    }

    pub fn key_up(&mut self, key: Key) {
        self.pressed_keys.remove(&key);
        self.just_released_keys.insert(key);
    }

    pub fn button_down(&mut self, btn: MouseButton) {
        if self.pressed_buttons.insert(btn) {
            self.just_pressed_buttons.insert(btn);
        }
    }

    pub fn button_up(&mut self, btn: MouseButton) {
        self.pressed_buttons.remove(&btn);
        self.just_released_buttons.insert(btn);
    }

    pub fn set_mouse_pos(&mut self, x: f32, y: f32) {
        self.mouse_delta = (x - self.mouse_pos.0, y - self.mouse_pos.1);
        self.mouse_pos = (x, y);
    }

    pub fn add_scroll(&mut self, delta: f32) {
        self.scroll_delta += delta;
    }

    // ── Requêtes (utilisées par les scripts et systèmes) ────────────────────

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: Key) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    pub fn is_key_just_released(&self, key: Key) -> bool {
        self.just_released_keys.contains(&key)
    }

    pub fn is_button_pressed(&self, btn: MouseButton) -> bool {
        self.pressed_buttons.contains(&btn)
    }

    pub fn is_button_just_pressed(&self, btn: MouseButton) -> bool {
        self.just_pressed_buttons.contains(&btn)
    }

    pub fn is_button_just_released(&self, btn: MouseButton) -> bool {
        self.just_released_buttons.contains(&btn)
    }
}
