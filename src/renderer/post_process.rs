use wgpu::{RenderPipeline, BindGroupLayout, Buffer, Texture, Sampler, TextureFormat, Device, util::DeviceExt, Queue, TextureView, TextureViewDescriptor};

use super::fullscreen_quad;

// Draw to the texture here and then use the render() function to draw to your output surface.
// The post_process.wgsl shader can have post processing stuff in it.
pub struct PostProcessRenderer {
  render_pipeline: RenderPipeline,
  bind_group_layout: BindGroupLayout,
  vertex_buffer: Buffer,

  texture: Texture,
  sampler: Sampler,
  texture_format: TextureFormat
}

impl PostProcessRenderer {
  pub fn new(device: &Device, width: u32, height: u32, output_format: TextureFormat) -> Self {
      // Load shader
      let shader = device.create_shader_module(wgpu::include_wgsl!("post_process.wgsl"));

      // Create a texture.
      let texture_desc = wgpu::TextureDescriptor {
          size: wgpu::Extent3d {
              width,
              height,
              depth_or_array_layers: 1
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: wgpu::TextureFormat::Rgba8UnormSrgb,
          usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
          label: Some("Post Process Texture")
      };
      let texture = device.create_texture(&texture_desc);

      // Vertex buffer for a screen quad.
      let vertex_buffer = device.create_buffer_init( 
          &wgpu::util::BufferInitDescriptor {
              label: Some("Post Process Vertex Buffer"),
              contents: bytemuck::cast_slice(fullscreen_quad::POS_TEX_VERTICES),
              usage: wgpu::BufferUsages::VERTEX
          }
      );

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
          label: Some("Post Process Bind Group Layout")
      });

      // Create a render pipeline.
      let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("Post Process Render Pipeline Layout"),
          bind_group_layouts: &[&bind_group_layout],
          push_constant_ranges: &[]
      });
      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("Post Process Render Pipeline"),
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

      // Create a sampler.
      let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
          address_mode_u: wgpu::AddressMode::ClampToEdge,
          address_mode_v: wgpu::AddressMode::ClampToEdge,
          address_mode_w: wgpu::AddressMode::ClampToEdge,
          mag_filter: wgpu::FilterMode::Nearest,
          min_filter: wgpu::FilterMode::Nearest,
          mipmap_filter: wgpu::FilterMode::Nearest,
          ..Default::default()
      });

      Self {
          render_pipeline,
          bind_group_layout,
          vertex_buffer,
          texture,
          sampler,
          texture_format: texture_desc.format
      }
  }

  pub fn get_texture(&self) -> &Texture {
      &self.texture
  }

  pub fn get_texture_format(&self) -> TextureFormat {
      self.texture_format
  }

  pub fn render(&mut self, device: &Device, queue: &Queue, dest_view: &TextureView) {
      let texture_view = self.texture.create_view(&TextureViewDescriptor::default());

      let bind_group = device.create_bind_group(
          &wgpu::BindGroupDescriptor {
              label: Some("Post Process Renderer Bind Group"),
              layout: &self.bind_group_layout,
              entries: &[
                  wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::TextureView(&texture_view)
                  },
                  wgpu::BindGroupEntry {
                      binding: 1,
                      resource: wgpu::BindingResource::Sampler(&self.sampler)
                  }
              ]
          }
      );

      let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Post Process Renderer Encoder.")
      });

      {
          let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
              label: Some("Post Process Renderer Render Pass"),
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
