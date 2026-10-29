//! Crate `renderer` — Contexte wgpu et rendu de sprites.
//!
//! Pour le prototype, le rendu de la grille iso est géré par egui (Painter),
//! ce module expose le contexte wgpu partagé entre egui-wgpu et le rendu futur.

pub mod context;
pub mod texture;

pub use context::RenderContext;
pub use texture::GpuTexture;
