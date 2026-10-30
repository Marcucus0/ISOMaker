//! Contexte de rendu wgpu — Device, Queue, Surface.

use anyhow::{Context, Result};
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};

/// Contexte wgpu partagé par l'éditeur et le futur renderer de sprites.
pub struct RenderContext {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface_format: wgpu::TextureFormat,
}

impl RenderContext {
    /// Initialise le contexte wgpu sur la surface donnée.
    ///
    /// Appelé une seule fois au démarrage, après la création de la fenêtre.
    pub async fn new_async(
        instance: Instance,
        surface: &Surface<'_>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Aucun adaptateur GPU compatible trouvé")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("MoteurDevice"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .context("Impossible de créer le device GPU")?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        log::info!(
            "GPU : {} | Format surface : {:?}",
            adapter.get_info().name,
            surface_format
        );

        Ok(Self {
            instance,
            device,
            queue,
            surface_format,
        })
    }
}
