use std::path::{Path};

use cgmath::{SquareMatrix, Vector3};
use gltf::Camera;
use wgpu::{util::DeviceExt, RenderPipeline, Buffer, BindGroupLayout, BindGroup};

use bevy_ecs::prelude::*;

use crate::renderer::camera;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelVertex {
    position: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
}

impl ModelVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS
        }
    }
}

// Handy full screen quad of vertices.
const CUBE_MODEL_VERTICES: &[ModelVertex] = &[
    // Front
    ModelVertex { position: [0.5, -0.5, 0.5 ], color: [1.0, 0.0, 0.0], uv: [0.0, 1.0] },
    ModelVertex { position: [0.5, -0.5, 0.5], color: [1.0, 0.0, 0.0], uv: [1.0, 1.0] },
    ModelVertex { position: [0.5, 0.5, 0.5], color: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },

    ModelVertex { position: [0.5, 0.5, 0.5], color: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },
    ModelVertex { position: [-0.5, 0.5, 0.5], color: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
    ModelVertex { position: [-0.5, -0.5, 0.5], color: [1.0, 0.0, 0.0], uv: [0.0, 1.0] },

    // Back
    ModelVertex { position: [0.5, 0.5, 0.5], color: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
    ModelVertex { position: [0.5, -0.5, 0.5], color: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
    ModelVertex { position: [-0.5, -0.5, 0.5 ], color: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },

    ModelVertex { position: [-0.5, -0.5, 0.5], color: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
    ModelVertex { position: [-0.5, 0.5, 0.5], color: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
    ModelVertex { position: [0.5, 0.5, 0.5], color: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
];

#[derive(Debug)]
pub struct ModelData {
  vertex_buffer: wgpu::Buffer,
  vertex_count: u32
}

impl ModelData {
  pub fn new(device: &wgpu::Device, filepath: &Path) -> Self {
      // // Open the file using our gltf parser.
      // let gltf = Gltf::open(filepath).unwrap();

      // info!("Loading model {}", filepath.display());

      // // Get the triangle data and put it into a vertex buffer.
      // //let vertices = Vec::new();
      // for scene in gltf.scenes() {
      //     for node in scene.nodes() {
      //         debug!("{}", node.index());

      //         // If there is a mesh...
      //         if let Some(mesh) = node.mesh() {

      //             for primitive in mesh.primitives() {
      //                 primitive.reader(|buffer| gltf.buffers()[buffer.index()]);
      //             }

      //         }
      //     }
      // }

      let vertex_buffer = device.create_buffer_init(
          &wgpu::util::BufferInitDescriptor {
              label: Some(filepath.display().to_string().as_str()),
              contents: bytemuck::cast_slice(CUBE_MODEL_VERTICES),
              usage: wgpu::BufferUsages::VERTEX
          }
      );
      let vertex_count = CUBE_MODEL_VERTICES.len() as u32;

      Self {
          vertex_buffer,
          vertex_count
      }
  }

  pub fn get_vertex_buffer(&self) -> &Buffer {
    &self.vertex_buffer
  }

  pub fn get_vertex_count(&self) -> u32 {
    self.vertex_count
  }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
  view_proj: [[f32; 4]; 4]
}

impl Default for CameraUniform {
  fn default() -> Self {
    let view_proj = cgmath::Matrix4::identity();  

    Self {
      view_proj: view_proj.into()
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
  model: [[f32; 4]; 4]
}

impl Instance {
  pub fn new(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>) -> Self {
    let model = (cgmath::Matrix4::from_translation(position) * cgmath::Matrix4::from(rotation)).into();
    
    Self {
      model
    }
  }

  fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    use std::mem;
    wgpu::VertexBufferLayout {
      array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
      // We need to switch from using a step mode of Vertex to Instance
      // This means that our shaders will only change to use the next
      // instance when the shader starts processing a new instance
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
          // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
          shader_location: 5,
          format: wgpu::VertexFormat::Float32x4,
        },
        // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
        // for each vec4. We'll have to reassemble the mat4 in
        // the shader.
        wgpu::VertexAttribute {
          offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
          shader_location: 6,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
          shader_location: 7,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
          shader_location: 8,
          format: wgpu::VertexFormat::Float32x4,
        },
      ],
    }
  }  
}

#[derive(Resource, Debug)]
pub (super) struct ModelRendererResource {
  test_model: ModelData,

  render_pipeline: RenderPipeline,
  camera_buffer: Buffer,
  camera_bind_group_layout: BindGroupLayout,
  camera_bind_group: BindGroup,

  instance_buffer: Buffer,
}

impl FromWorld for ModelRendererResource {
    fn from_world(world: &mut World) -> Self {
      world.resource_scope(|world, context: Mut<super::RenderContext>| {
        let test_model = ModelData::new(&context.device, Path::new("test"));

        // Load shader.
        let shader = context.device.create_shader_module(wgpu::include_wgsl!("model.wgsl"));

        // Create a uniform buffer for the camera
        let camera = super::camera::PositionRotationCamera {
          position: cgmath::Vector3 { x: 6.92, y: -4.0, z: 3.22 },
          rotation: cgmath::Vector3{ x: 72.4, y: 0.0, z: 79.4 },
          aspect: super::SCREEN_WIDTH as f32 / super::SCREEN_HEIGHT as f32,
          fovy: 39.6,
          znear: 0.001,
          zfar: 1000.0
        };

        let camera_uniform = CameraUniform {
          view_proj: camera.get_matrix().into()
        };

        let camera_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Model Camera Buffer"),
          contents: bytemuck::cast_slice(&[camera_uniform]),
          usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        });

        // Bind layout
        let camera_bind_group_layout = context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          label: Some("Model Camera Bind Group Layout"),
          entries: &[
            wgpu::BindGroupLayoutEntry {
              binding: 0,
              visibility: wgpu::ShaderStages::VERTEX,
              ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None
              },
              count: None
            }
          ]
        });

        let camera_bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
          label: Some("Model Camera Bind Group"),
          layout: &camera_bind_group_layout,
          entries: &[
            wgpu::BindGroupEntry {
              binding: 0,
              resource: camera_buffer.as_entire_binding()
            }
          ]
        });

        // Create an instance buffer
        let instances = {
          let mut list = Vec::new();
          list.push(Instance::new(Vector3::new(-0.34, -2.31, 0.001), cgmath::Quaternion { s: 0.781, v: Vector3::new(0.35, 0.21, 0.47)}));
          list
        };
        
        let instance_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Model Instance Buffer"),
          contents: bytemuck::cast_slice(&instances),
          usage: wgpu::BufferUsages::VERTEX
        });

        // Create the render pipeline
        let render_pipeline_layout = context.device.create_pipeline_layout(
          &wgpu::PipelineLayoutDescriptor {
            label: Some("Model Renderer Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[]
          }
        );
        let render_pipeline = context.device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
          label: Some("Model Renderer Pipeline"),
          layout: Some(&render_pipeline_layout),
          vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[ModelVertex::desc(), Instance::desc()],
          },
          fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
              format: context.post_process_texture_format,
              blend: None,
              write_mask: wgpu::ColorWrites::ALL
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
          test_model,

          render_pipeline,
          camera_buffer,
          camera_bind_group_layout,
          camera_bind_group,

          instance_buffer
        }
      })
    }
}

pub (super) fn render(field: Res<super::field::Field>, context: Res<super::RenderContext>, resource: Local<ModelRendererResource>) {
  log::debug!("Rendering models!");

  // If the field changed, re-upload the camera information.
  if field.is_changed() {
    log::debug!("Updating model renderer camera.");
    let camera_uniform = CameraUniform {
      view_proj: field.view_proj.into()
    };

    context.queue.write_buffer(&resource.camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
  }

  // Draw to the post process texture.
  let texture_view = context.post_process_texture.create_view(&wgpu::TextureViewDescriptor::default());
  let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("Model Renderer Encoder")
  });

  {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: Some("Model Renderer RenderPass"),
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
              view: &texture_view,
              resolve_target: None,
              ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
              },
          })],
          depth_stencil_attachment: None,
      });

      // Set render pipeline
      render_pass.set_pipeline(&resource.render_pipeline);

      render_pass.set_bind_group(0, &resource.camera_bind_group, &[]);
      render_pass.set_vertex_buffer(0, resource.test_model.get_vertex_buffer().slice(..));
      //render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
      
      render_pass.set_vertex_buffer(1, resource.instance_buffer.slice(..));

      render_pass.draw(0..resource.test_model.get_vertex_count(), 0..1);
  }

  context.queue.submit(Some(encoder.finish()));
}