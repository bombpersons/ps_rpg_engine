use std::collections::HashMap;

use bevy_ecs::prelude::*;

enum TextureManagerError {
  FileNotFound,
  NameNotInManifest
}

#[derive(Resource, Debug)]
pub struct TextureManager {
  manifest: HashMap<String, String>,
  textures: HashMap<String, wgpu::Texture>
}

impl TextureManager {
  pub fn new(texture_paths: HashMap<String, String>) -> Self {
    
    Self {
      manifest: texture_paths,
      textures: HashMap::new()
    }
  }

  // Get a texture. If it isn't loaded, load it.
  pub fn get_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, name: &str) -> Result<&wgpu::Texture, TextureManagerError> {
    // Get the texture if it's already loaded.
    if self.textures.contains_key(name) {
      return Ok(self.textures.get(name).unwrap());
    }

    // Does the texture name appear in the manifest?
    let image_path = self.manifest.get(name).ok_or(Err(TextureManagerError::NameNotInManifest))?;

    // Otherwise, load it.
    let image_path = 
    let image = image::io::Reader::open()

      let image = image::io::Reader::open(image_path)
          .unwrap().decode().unwrap();
      let image = image.to_rgba8();

      // Create the texture in wgpu.
      let texture_desc = wgpu::TextureDescriptor {
          label: Some("Field Background Texture"),
          size: wgpu::Extent3d {
              width: image.width(),
              height: image.height(),
              depth_or_array_layers: 1
          },
          mip_level_count: 1, 
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: wgpu::TextureFormat::Rgba8UnormSrgb,
          usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
      };
      let texture = device.create_texture(&texture_desc);

      // Write the texture data to the texture.
      queue.write_texture(
          wgpu::ImageCopyTexture {
              texture: &texture,
              mip_level: 0,
              origin: wgpu::Origin3d::ZERO,
              aspect: wgpu::TextureAspect::All
          },
          bytemuck::cast_slice(image.as_flat_samples().as_slice()),
          wgpu::ImageDataLayout {
              offset: 0,
              bytes_per_row: std::num::NonZeroU32::new(std::mem::size_of::<u8>() as u32 * 4 * image.width()),
              rows_per_image: std::num::NonZeroU32::new(image.height())
          },
          wgpu::Extent3d {
              width: image.width(),
              height: image.height(),
              depth_or_array_layers: 1
          }
      );

    });
  }
}