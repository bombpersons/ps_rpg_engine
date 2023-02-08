use std::path::Path;

use bevy_ecs::prelude::*;
use cgmath::SquareMatrix;
use gltf::{Gltf, camera::{Orthographic, Perspective, Projection}};
use wgpu::{Texture, Sampler, Device, Queue, RenderPipeline, BindGroupLayout, Buffer, TextureFormat, util::DeviceExt, TextureView, TextureViewDescriptor};

use super::fullscreen_quad;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

// Component that deterimines which texture is used for the field background.
#[derive(Resource, Debug)]
pub struct Field {
  pub background_image_name: String,
  pub background_depth_name: String,
  pub view_proj: cgmath::Matrix4<f32>
}

impl Field {
  pub fn from_gltf(background_image_name: &str, background_depth_name: &str, gltf_path: &Path) -> Self {
    let gltf = Gltf::open(gltf_path)
      .expect("Couldn't open field gltf file.");

    let aspect = super::SCREEN_WIDTH as f32 / super::SCREEN_HEIGHT as f32;


    // Look for a camera.
    // TODO: make this more rubust.
    let view_proj = {
      let mut view_proj = cgmath::Matrix4::identity();

      fn find_camera_matrix(aspect: f32, node: gltf::Node, transform: cgmath::Matrix4<f32>) -> Option<cgmath::Matrix4<f32>> {
        let transform = transform * cgmath::Matrix4::<f32>::from(node.transform().matrix());

        match node.camera() {
          Some(camera) => {
            let view: cgmath::Matrix4<f32> = transform;

            let proj = match camera.projection() {
              Projection::Orthographic(orthographic) => {
                // TODO! Figure out how to interpret the orthographic struct gltf gives us.
                cgmath::Matrix4::identity()
              },
              Projection::Perspective(perspective) => {
                cgmath::perspective(
                  cgmath::Rad(perspective.yfov()), 
                  aspect, 
                  perspective.znear(), 
                  perspective.zfar().unwrap_or(10000.0))
              }
            };

            Some(view * proj)
          },
          None => {
            let mut matrix = None;
            for node in node.children() {
              matrix = find_camera_matrix(aspect, node, transform);
            }
            matrix
          }
        }
      };

      for scene in gltf.scenes() {
        for node in scene.nodes() {
          if let Some(mat) = find_camera_matrix(aspect, node, cgmath::Matrix4::identity()) {
            view_proj = mat;
          }
        }
      }

      view_proj
    };

    Self { 
      background_image_name: background_image_name.to_string(),
      background_depth_name: background_depth_name.to_string(),
      view_proj
    }
  }
}

// Resource for the system that draws the field background.
#[derive(Resource, Debug)]
pub (super) struct FieldBackgroundRendererResource {
  render_pipeline: RenderPipeline,
  bind_group_layout: BindGroupLayout,
  vertex_buffer: Buffer,

  sampler: Sampler
}

impl FromWorld for FieldBackgroundRendererResource {
  fn from_world(world: &mut World) -> Self {
    world.resource_scope(|world, context: Mut<super::RenderContext>| {
      let shader = context.device.create_shader_module(wgpu::include_wgsl!("field_background.wgsl"));

      // Create a vertex buffer containing a quad.
      let vertex_buffer = context.device.create_buffer_init(
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
          label: Some("Field Background Renderer Bind Group Layout")
      });

      // Create a render pipeline.
      let render_pipeline_layout = context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("Field Background Render Pipeline Layout"),
          bind_group_layouts: &[&bind_group_layout],
          push_constant_ranges: &[]
      });
      let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                  format: context.post_process_texture_format,
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
        sampler
      }
    })
  }
}

pub (super) fn render(field: Res<Field>, context: Res<super::RenderContext>, resource: Local<FieldBackgroundRendererResource>, 
  mut texture_manager: ResMut<super::texture_manager::TextureManager>) {
  
  let dest_view = context.post_process_texture.create_view(&TextureViewDescriptor::default());

  // Continue to draw the background image.
  log::debug!("Rendering field background: {}", &field.background_image_name);

  let texture = texture_manager.get_texture(&context.device, &context.queue, &field.background_image_name).unwrap();
  let texture_view = texture.create_view(&TextureViewDescriptor::default());

  let bind_group = context.device.create_bind_group(
      &wgpu::BindGroupDescriptor {
          label: Some("Field Background Renderer Bind Group"),
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

  let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

      render_pass.set_pipeline(&resource.render_pipeline);

      // Bind the texture.
      render_pass.set_bind_group(0, &bind_group, &[]);
      
      // Set the vertex buffer and draw.
      render_pass.set_vertex_buffer(0, resource.vertex_buffer.slice(..));
      render_pass.draw(0..fullscreen_quad::POS_TEX_VERTICES.len() as u32, 0..1);
  }

  context.queue.submit(Some(encoder.finish()));
}