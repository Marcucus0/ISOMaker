//! Commandes générées par les scripts Python et appliquées par le moteur.
//!
//! Ce module définit le protocole d'échange Python → Rust.
//! Python empile des commandes ; Rust les dépile et les exécute.

use serde::{Deserialize, Serialize};

/// Point d'ancrage du texte HUD dans le viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HudAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Texte affiché par-dessus le viewport pendant la lecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudText {
    pub id: String,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub anchor: HudAnchor,
    pub font_size: f32,
    pub color: [u8; 4],
    pub visible: bool,
    pub background_color: Option<[u8; 4]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudButton {
    pub id: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub anchor: HudAnchor,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudSlider {
    pub id: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub anchor: HudAnchor,
}

/// Une commande émise par un script Python.
///
/// Conçue pour être bon marché à cloner et simple à sérialiser si besoin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameCommand {
    /// Crée ou remplace un texte du HUD.
    SetHudText(HudText),
    /// Met à jour le contenu d'un texte HUD existant.
    UpdateHudText {
        id: String,
        text: String,
    },
    /// Met à jour la position d'un texte HUD existant.
    UpdateHudPosition {
        id: String,
        x: f32,
        y: f32,
    },
    /// Met à jour l'ancrage d'un texte HUD existant.
    UpdateHudAnchor {
        id: String,
        anchor: HudAnchor,
    },
    /// Met à jour la taille d'un texte HUD existant.
    UpdateHudFontSize {
        id: String,
        font_size: f32,
    },
    /// Met à jour la couleur d'un texte HUD existant.
    UpdateHudColor {
        id: String,
        color: [u8; 4],
    },
    UpdateHudVisibility {
        id: String,
        visible: bool,
    },
    /// Fait disparaître progressivement un texte HUD, puis le supprime.
    FadeOutHudText {
        id: String,
        duration: f32,
    },
    /// Fait apparaître progressivement un texte HUD.
    FadeInHudText {
        id: String,
        duration: f32,
    },
    /// Supprime un texte du HUD par son identifiant.
    RemoveHudText {
        id: String,
    },
    SetHudButton(HudButton),
    RemoveHudButton {
        id: String,
    },
    SetHudSlider(HudSlider),
    RemoveHudSlider {
        id: String,
    },
    Pause,
    Resume,
    PauseAudio,
    ResumeAudio,
    /// Enregistre une source audio nommée pour une lecture ultérieure.
    CreateSound {
        id: String,
        path: String,
        source: String,
    },
    /// Joue une source précédemment créée.
    PlaySound {
        id: String,
        volume: f32,
        looping: bool,
        source: String,
    },
    /// Arrête un son précédemment démarré par un script.
    StopSound {
        id: String,
    },
    /// Modifie le volume d'un son précédemment démarré par un script.
    SetSoundVolume {
        id: String,
        volume: f32,
    },
    /// Crée un objet à la position donnée.
    Spawn {
        name: String,
        object_type: String,
        x: i32,
        y: i32,
    },
    /// Supprime un objet par son identifiant (UUID string).
    Destroy {
        object_id: String,
    },
    /// Déplace un objet existant.
    Move {
        object_id: String,
        x: i32,
        y: i32,
    },
    /// Modifie une propriété d'un objet.
    SetProperty {
        object_id: String,
        key: String,
        value: serde_json::Value,
    },
    /// Charge une autre scène du monde et empile la scène courante.
    LoadScene {
        name: String,
    },
    /// Quitte la scène active et revient à la scène précédente de la pile.
    UnloadScene,
    /// Transfers an existing object into another world scene.
    TransferObject {
        object_id: String,
        scene_name: String,
        x: i32,
        y: i32,
    },
    /// Message de log émis depuis Python.
    Log {
        /// Script Python à l'origine du message.
        source: String,
        level: LogLevel,
        message: String,
    },
    /// Demande d'arrêt de la simulation.
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}
