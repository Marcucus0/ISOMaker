//! Crate `editor` — Application principale : éditeur + rendu egui/wgpu.

pub mod app;
pub mod camera;
pub mod inspector;
pub mod panels;
pub mod scene_view;
pub mod toolbar;

pub use app::{EditorApp, EditorState};
