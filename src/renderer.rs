use std::path::Path;

use wgpu::{Device, Queue, RenderPipeline, Surface, SurfaceConfiguration, Texture, TextureDescriptor, TextureView, Sampler, BindGroupLayout, TextureViewDescriptor, util::DeviceExt, Buffer, TextureFormat};
use winit::window::Window;

use self::{model::ModelData, post_process::PostProcessRenderer, field::{FieldBackground, FieldBackgroundRenderer}};

pub mod model;
pub mod field;
pub mod post_process;
pub mod fullscreen_quad;

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 800;

pub struct Renderer {
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,

    surface: Surface,
    surface_config: SurfaceConfiguration,

    post_process_renderer: PostProcessRenderer,

    field_background: FieldBackground,
    field_background_renderer: FieldBackgroundRenderer
}

impl Renderer {
    pub async fn new(window: &Window) -> Self {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::GL);

        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
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
        ).await.unwrap();

        // Configure the surface.
        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto
        };
        surface.configure(&device, &surface_config);

        let post_process_renderer = PostProcessRenderer::new(&device, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, surface_config.format);

        let field_background = FieldBackground::new(&device, &queue, Path::new("fields/test_field.png"));
        let field_background_renderer = FieldBackgroundRenderer::new(&device, post_process_renderer.get_texture_format());

        // Load shader.
        let shader = device.create_shader_module(wgpu::include_wgsl!("main.wgsl"));

        // Create a render pipeline.
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main Window Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: post_process_renderer.get_texture_format(),
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
        
        // Uh try and load a model I guess for testing.
        let model = ModelData::new(&device, Path::new("models/base.glb"));

        Self {
            device,
            queue,
            render_pipeline,

            surface,
            surface_config,

            post_process_renderer,

            field_background,
            field_background_renderer
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Draw a background.
        let view = self.post_process_renderer.get_texture().create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Encoder")
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("WaveSim_RenderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            render_pass.set_pipeline(&self.render_pipeline);

            // Set the bind group
            //render_pass.set_bind_group(0, &bind_group, &[]);

            // Set the quad as the buffer.
            //render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            
            // Draw
            //render_pass.draw(0..FULL_SCREEN_QUAD_VERTICES.len() as u32, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Draw the background.
        self.field_background_renderer.render(&self.device, &self.queue, &view, &self.field_background);

        // Do post processing and draw to the window.
        let surface_texture = self.surface.get_current_texture()?;
        let surface_texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.post_process_renderer.render(&self.device, &self.queue, &surface_texture_view);

        surface_texture.present();

        Ok(())
    }
}