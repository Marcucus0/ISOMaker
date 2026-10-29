//! Crate `engine` — Boucle de jeu, temps, entrées.

pub mod time;
pub mod input;
pub mod game_loop;

pub use time::Time;
pub use input::{InputState, Key, MouseButton};
pub use game_loop::{GameLoop, EngineMode};
