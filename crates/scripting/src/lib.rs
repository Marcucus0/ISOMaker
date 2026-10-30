//! Crate `scripting` — Intégration Python via PyO3.
//!
//! Architecture retenue : **file de commandes** (Command Queue).
//!
//! Python reçoit un `GameContext` (objet PyO3) qui expose une API limitée.
//! Quand Python appelle `game.spawn(...)`, cela empile un `GameCommand` dans un Vec.
//! Le moteur lit ce Vec après le retour de Python et applique les commandes.
//!
//! Avantages :
//! - Python ne touche jamais directement aux structures Rust pendant le rendu.
//! - Les erreurs Python sont capturées proprement.
//! - Le thread de rendu n'est jamais bloqué par Python.

pub mod api;
pub mod command;
pub mod runtime;

pub use command::GameCommand;
pub use runtime::{ScriptError, ScriptRuntime};
