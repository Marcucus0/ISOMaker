//! Crate `editor` — Application principale : éditeur + rendu egui/wgpu.

pub mod app;
pub mod camera;
pub mod toolbar;
pub mod scene_view;
pub mod inspector;
pub mod panels;

pub use app::{EditorApp, EditorState};
