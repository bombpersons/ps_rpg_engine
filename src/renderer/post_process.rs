use wgpu::{RenderPipeline, BindGroupLayout, Buffer, Texture, Sampler, TextureFormat, Device, util::DeviceExt, Queue, TextureView, TextureViewDescriptor};
use bevy_ecs::prelude::*;
use super::{fullscreen_quad, RenderContext};

#[derive(Resource, Debug)]
pub (super) struct PostProcessResource {
  render_pipeline: RenderPipeline,
  bind_group_layout: BindGroupLayout,
  vertex_buffer: Buffer,

  sampler: Sampler,
}

impl FromWorld for PostProcessResource {
  fn from_world(world: &mut World) -> Self {
    world.resource_scope(|world, context: Mut<RenderContext>| {
      // Load shader
      let shader = context.device.create_shader_module(wgpu::include_wgsl!("post_process.wgsl"));

      // Create a texture.
      let texture_desc = wgpu::TextureDescriptor {
          size: wgpu::Extent3d {
              width: super::SCREEN_WIDTH,
              height: super::SCREEN_HEIGHT,
              depth_or_array_layers: 1
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: wgpu::TextureFormat::Rgba8UnormSrgb,
          usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
          label: Some("Post Process Texture")
      };
      let texture = context.device.create_texture(&texture_desc);

      // Vertex buffer for a screen quad.
      let vertex_buffer = context.device.create_buffer_init( 
          &wgpu::util::BufferInitDescriptor {
              label: Some("Post Process Vertex Buffer"),
              contents: bytemuck::cast_slice(fullscreen_quad::POS_TEX_VERTICES),
              usage: wgpu::BufferUsages::VERTEX
          }
      );

      // Bind group layout.
      // We need to sample the background texture in our shader.
      let bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
      let render_pipeline_layout = context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("Post Process Render Pipeline Layout"),
          bind_group_layouts: &[&bind_group_layout],
          push_constant_ranges: &[]
      });
      let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                  format: context.surface_config.format,
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
      let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
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
          sampler,
      }      
    })
  }
}

pub (super) fn render(context: ResMut<super::RenderContext>, resource: Local<PostProcessResource>) {
  log::debug!("Rendering postprocessing!");
  
  // View into the post process texture that we are going to render.
  let texture_view = context.post_process_texture.create_view(&TextureViewDescriptor::default());

  let bind_group = context.device.create_bind_group(
    &wgpu::BindGroupDescriptor {
        label: Some("Post Process Renderer Bind Group"),
        layout: &resource.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view)
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&resource.sampler)
            }
        ]
    }
  );

  // View into the destination surface texture.
  let surface_texture = context.surface.get_current_texture().unwrap(); // TODO unwrap
  let surface_texture_view = surface_texture.texture.create_view(&TextureViewDescriptor::default());

  let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Post Process Renderer Encoder.")
  });

  {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: Some("Post Process Renderer Render Pass"),
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
              view: &surface_texture_view,
              resolve_target: None,
              ops: wgpu::Operations {
                  load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                  store: true
              }
          })],
          depth_stencil_attachment: None
      });

      render_pass.set_pipeline(&resource.render_pipeline);

      // Bind the texture.
      render_pass.set_bind_group(0, &bind_group, &[]);
      
      // Set the vertex buffer and draw.
      render_pass.set_vertex_buffer(0, resource.vertex_buffer.slice(..));
      render_pass.draw(0..fullscreen_quad::POS_TEX_VERTICES.len() as u32, 0..1);
  }

  context.queue.submit(Some(encoder.finish()));

  surface_texture.present();
}