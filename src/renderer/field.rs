use std::path::Path;

use bevy_ecs::prelude::Component;
use wgpu::{Texture, Sampler, Device, Queue, RenderPipeline, BindGroupLayout, Buffer, TextureFormat, util::DeviceExt, TextureView, TextureViewDescriptor};

use super::fullscreen_quad;

// #[derive(Component, Debug)]
// pub struct FieldBackground {
//   background_image: String
// }

// #[derive(Resource, Debug)]
// pub struct FieldBackgroundRendererResource {

// }

// The background for a field. Can be rendered with FieldBackgroundRenderer.
pub struct FieldBackground {
  background_texture: Texture,
  background_sampler: Sampler,
}

impl FieldBackground {
  pub fn new(device: &Device, queue: &Queue, image_path: &Path) -> Self {
      // Load the image.
      // TODO error handling.
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

      // Create a sampler.
      let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
          address_mode_u: wgpu::AddressMode::ClampToEdge,
          address_mode_v: wgpu::AddressMode::ClampToEdge,
          address_mode_w: wgpu::AddressMode::ClampToEdge,
          mag_filter: wgpu::FilterMode::Linear,
          min_filter: wgpu::FilterMode::Linear,
          mipmap_filter: wgpu::FilterMode::Linear,
          ..Default::default()
      });

      Self {
          background_texture: texture,
          background_sampler: sampler
      }
  }

  pub fn get_sampler(&self) -> &Sampler {
      &self.background_sampler
  }

  pub fn get_texture(&self) -> &Texture {
      &self.background_texture
  }

}

// // We need this for Rust to store our data correctly for the shaders
// #[repr(C)]
// // This is so we can store this in a buffer
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// struct FieldBackgroundRendererUniform {
//     screen_size: [usize; 2],
//     background_size: [usize; 2],
//     center: [f32; 2],
// }

// impl FieldBackgroundRendererUniform {
//     fn new() -> Self {
//         Self {
//             screen_size: [SCREEN_WIDTH, SCREEN_HEIGHT],
//             background_size: [SCREEN_WIDTH, SCREEN_HEIGHT],
//             center: [SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0]
//         }
//     }

//     fn set_background_size(&mut self, width: usize, height: usize) {
//         self.background_size[0] = width;
//         self.background_size[1] = height;
//     }
// }

// Draw a field background to a surface. 
pub struct FieldBackgroundRenderer {
  render_pipeline: RenderPipeline,
  bind_group_layout: BindGroupLayout,
  vertex_buffer: Buffer,
  //uniform_buffer: Buffer,
}

impl FieldBackgroundRenderer {
  pub fn new(device: &Device, output_format: TextureFormat) -> Self {
      let shader = device.create_shader_module(wgpu::include_wgsl!("field_background.wgsl"));

      // Create a vertex buffer containing a quad.
      let vertex_buffer = device.create_buffer_init(
          &wgpu::util::BufferInitDescriptor {
              label: Some("Field Background Vertex Buffer"),
              contents: bytemuck::cast_slice(fullscreen_quad::POS_TEX_VERTICES),
              usage: wgpu::BufferUsages::VERTEX
          }
      );

      // Create a buffer containing information about the camera.
      // TODO

      // Bind group layout.
      // We need to sample the background texture in our shader.
      let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[
              wgpu::BindGroupLayoutEntry {
                  binding: 0,
                  visibility: wgpu::ShaderStages::FRAGMENT,
                  ty: wgpu::BindingType::Texture {
                      multisampled: false,
                      view_dimension: wgpu::TextureViewDimension::D2,
                      sample_type: wgpu::TextureSampleType::Float { filterable: true }
                  },
                  count: None
              },
              wgpu::BindGroupLayoutEntry {
                  binding: 1,
                  visibility: wgpu::ShaderStages::FRAGMENT,
                  ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                  count: None
              }
          ],
          label: Some("Field Background Renderer Bind Group Layout")
      });

      // Create a render pipeline.
      let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("Field Background Render Pipeline Layout"),
          bind_group_layouts: &[&bind_group_layout],
          push_constant_ranges: &[]
      });
      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("Field Background Render Pipeline"),
          layout: Some(&render_pipeline_layout),
          vertex: wgpu::VertexState {
              module: &shader,
              entry_point: "vs_main",
              buffers: &[
                  fullscreen_quad::PosTexVertex::desc()
              ],
          },
          fragment: Some(wgpu::FragmentState {
              module: &shader,
              entry_point: "fs_main",
              targets: &[Some(wgpu::ColorTargetState {
                  format: output_format,
                  blend: None,
                  write_mask: wgpu::ColorWrites::ALL,
              })],
          }),
          primitive: wgpu::PrimitiveState {
              topology: wgpu::PrimitiveTopology::TriangleList,
              strip_index_format: None,
              front_face: wgpu::FrontFace::Ccw,
              cull_mode: Some(wgpu::Face::Back),
              polygon_mode: wgpu::PolygonMode::Fill,
              unclipped_depth: false,
              conservative: false,
          },
          depth_stencil: None,
          multisample: wgpu::MultisampleState {
              count: 1,
              mask: !0,
              alpha_to_coverage_enabled: false,
          },
          multiview: None
      });

      Self {
          render_pipeline,
          bind_group_layout,
          vertex_buffer
      }
  }

  pub fn render(&mut self, device: &Device, queue: &Queue, dest_view: &TextureView, field_background: &FieldBackground) {
      let texture = field_background.get_texture();
      let texture_view = texture.create_view(&TextureViewDescriptor::default());

      let bind_group = device.create_bind_group(
          &wgpu::BindGroupDescriptor {
              label: Some("Field Background Renderer Bind Group"),
              layout: &self.bind_group_layout,
              entries: &[
                  wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::TextureView(&texture_view)
                  },
                  wgpu::BindGroupEntry {
                      binding: 1,
                      resource: wgpu::BindingResource::Sampler(field_background.get_sampler())
                  }
              ]
          }
      );

      let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Field Background Renderer Encoder.")
      });

      {
          let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
              label: Some("Field Background Renderer Render Pass"),
              color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                  view: &dest_view,
                  resolve_target: None,
                  ops: wgpu::Operations {
                      load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                      store: true
                  }
              })],
              depth_stencil_attachment: None
          });

          render_pass.set_pipeline(&self.render_pipeline);

          // Bind the texture.
          render_pass.set_bind_group(0, &bind_group, &[]);
          
          // Set the vertex buffer and draw.
          render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
          render_pass.draw(0..fullscreen_quad::POS_TEX_VERTICES.len() as u32, 0..1);
      }

      queue.submit(Some(encoder.finish()));
  }
}
