//! Commandes générées par les scripts Python et appliquées par le moteur.
//!
//! Ce module définit le protocole d'échange Python → Rust.
//! Python empile des commandes ; Rust les dépile et les exécute.

use serde::{Deserialize, Serialize};

/// Une commande émise par un script Python.
///
/// Conçue pour être bon marché à cloner et simple à sérialiser si besoin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameCommand {
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
    /// Modifie une valeur de l'état global du jeu.
    SetGlobal {
        key: String,
        value: serde_json::Value,
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
