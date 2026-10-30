//! Texture GPU — wrapping wgpu::Texture + view + sampler.

use image::DynamicImage;
use wgpu::{Device, Extent3d, Queue, Sampler, Texture, TextureView};

/// Une texture chargée sur le GPU avec sa vue et son sampler.
pub struct GpuTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub width: u32,
    pub height: u32,
}

impl GpuTexture {
    /// Crée une GpuTexture à partir d'une image CPU.
    pub fn from_image(device: &Device, queue: &Queue, image: &DynamicImage, label: &str) -> Self {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // pixel-art : pas d'interpolation
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            width,
            height,
        }
    }
}
