use futures::executor::block_on;
use raw_window_handle::{HasRawWindowHandle, HasRawDisplayHandle};
use wgpu::{Device, Queue, RenderPipeline, Surface, SurfaceConfiguration, Texture, TextureDescriptor, TextureView, Sampler, BindGroupLayout, TextureViewDescriptor, util::DeviceExt, Buffer, TextureFormat};

use bevy_ecs::prelude::*;

pub mod texture_manager;

pub mod model;
pub mod field;
pub mod post_process;
pub mod fullscreen_quad;

const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 800;

// Specify the size of the viewport so that
// the rendering context can be resized with the window.
// The window can modify this resource to inform the renderer
// of any changes.
#[derive(Resource, Debug)]
pub struct Viewport {
  width: u32,
  height: u32
}

impl Default for Viewport {
  fn default() -> Self {
      Self {
        width: SCREEN_WIDTH,
        height: SCREEN_HEIGHT
      }
  }
}

impl Viewport {
  pub fn set_size(&mut self, width: u32, height: u32) {
    self.width = width;
    self.height = height;
  }
}

#[derive(Resource, Debug)]
struct RenderContext {
  device: Device,
  queue: Queue,

  surface: Surface,
  surface_config: SurfaceConfiguration,

  post_process_texture: Texture,
  post_process_texture_format: TextureFormat
}

impl RenderContext {
  fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(window: &W) -> Self {
    log::debug!("Creating Render Context");

    // The instance is a handle to our GPU
    // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
    let instance = wgpu::Instance::new(wgpu::Backends::GL);

    let surface = unsafe { instance.create_surface(window) };
    let adapter = block_on(instance.request_adapter(
      &wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      },
    )).expect("Failed to create wgpu adapter!");

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
          features: wgpu::Features::empty(),
          // WebGL doesn't support all of wgpu's features, so if
          // we're building for the web we'll have to disable some.
          limits: if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults() 
          } else {
            wgpu::Limits::default()
          },
          label: None,
        },
        None, // Trace path
    )).expect("Failed to create wgpu Device and Queue!");

    // Configure the surface.
    let surface_config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface.get_supported_formats(&adapter)[0],
      width: SCREEN_WIDTH,
      height: SCREEN_HEIGHT,
      present_mode: wgpu::PresentMode::Fifo,
      alpha_mode: wgpu::CompositeAlphaMode::Auto
    };
    surface.configure(&device, &surface_config);

    // Create a texture to render to for the post processing.
    let texture_desc = wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: SCREEN_WIDTH,
        height: SCREEN_HEIGHT,
        depth_or_array_layers: 1
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
      label: Some("Post Process Texture")
    };
    let post_process_texture = device.create_texture(&texture_desc);

      Self {
        device,
        queue,
        surface,
        surface_config,
        post_process_texture,
        post_process_texture_format: texture_desc.format
      }
    }
}

#[derive(Resource, Debug)]
struct RenderResource {
  render_pipeline: RenderPipeline
}

impl FromWorld for RenderResource {
  fn from_world(world: &mut World) -> Self {
    world.resource_scope(|world, context: Mut<RenderContext>| {
      // Load shader.
      let shader = context.device.create_shader_module(wgpu::include_wgsl!("main.wgsl"));

      // Create a render pipeline.
      let render_pipeline_layout = context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Main Window Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[]
      });
      
      let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Main Window Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader,
          entry_point: "vs_main",
          buffers: &[fullscreen_quad::PosTexVertex::desc()],
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

      Self {
          render_pipeline
      }
    })
  }
}

fn resize(mut context: ResMut<RenderContext>, viewport: Res<Viewport>) {
  if viewport.is_changed() {
    context.surface_config.width = viewport.width;
    context.surface_config.height = viewport.height;
    context.surface.configure(&context.device, &context.surface_config);
  }
}

fn render(context: ResMut<RenderContext>, resource: Local<RenderResource>) {
    log::debug!("Rendering!");

    let texture_view = context.post_process_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Main Encoder")
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("WaveSim_RenderPass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        // Set render pipeline
        render_pass.set_pipeline(&resource.render_pipeline);
    }

    context.queue.submit(Some(encoder.finish()));
}

pub fn init<W: HasRawWindowHandle + HasRawDisplayHandle>(world: &mut World, window: &W) -> SystemSet {
    // Initialize the Resources
    world.insert_resource(RenderContext::new(window));
    world.init_resource::<Viewport>();

    // Create the systems.
    SystemSet::new().label("Render Systems")
        .with_system(resize)
        .with_system(render)
        .with_system(field::render)
        .with_system(post_process::render)
}