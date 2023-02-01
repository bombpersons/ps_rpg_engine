use std::path::Path;

use wgpu::{Device, Queue, RenderPipeline, Surface, SurfaceConfiguration, Texture, TextureDescriptor, TextureView, Sampler, BindGroupLayout, TextureViewDescriptor, util::DeviceExt, Buffer, TextureFormat};
use winit::window::Window;

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 800;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS
        }
    }
}

// Handy full screen quad of vertices.
const TEXTURED_FULL_SCREEN_QUAD_VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0, 0.0 ], uv: [0.0, 1.0] },
    Vertex { position: [1.0, -1.0, 0.0], uv: [1.0, 1.0] },
    Vertex { position: [1.0, 1.0, 0.0], uv: [1.0, 0.0] },

    Vertex { position: [1.0, 1.0, 0.0], uv: [1.0, 0.0] },
    Vertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 0.0] },
    Vertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }
];

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

// Draw a field background to a surface. 
pub struct FieldBackgroundRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    vertex_buffer: Buffer
}

impl FieldBackgroundRenderer {
    pub fn new(device: &Device, output_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("field_background.wgsl"));

        // Create a vertex buffer containing a quad.
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Field Background Vertex Buffer"),
                contents: bytemuck::cast_slice(TEXTURED_FULL_SCREEN_QUAD_VERTICES),
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
                    Vertex::desc()
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

            // Set the viewport.
            render_pass.set_viewport(0.1, 0.1, 0.5, 0.5, 0.0, 1.0);

            // Bind the texture.
            render_pass.set_bind_group(0, &bind_group, &[]);
            
            // Set the vertex buffer and draw.
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..TEXTURED_FULL_SCREEN_QUAD_VERTICES.len() as u32, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}

// Draw to the texture here and then use the render() function to draw to your output surface.
// The post_process.wgsl shader can have post processing stuff in it.
struct PostProcessRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    vertex_buffer: Buffer,

    texture: Texture,
    sampler: Sampler,
    texture_format: TextureFormat
}

impl PostProcessRenderer {
    pub fn new(device: &Device, output_format: TextureFormat) -> Self {
        // Load shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("post_process.wgsl"));

        // Create a texture.
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: SCREEN_WIDTH as u32,
                height: SCREEN_HEIGHT as u32,
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
                contents: bytemuck::cast_slice(TEXTURED_FULL_SCREEN_QUAD_VERTICES),
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
                    Vertex::desc()
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
            render_pass.draw(0..TEXTURED_FULL_SCREEN_QUAD_VERTICES.len() as u32, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}

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
        let instance = wgpu::Instance::new(wgpu::Backends::all());
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

        let post_process_renderer = PostProcessRenderer::new(&device, surface_config.format);

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
                buffers: &[
                    Vertex::desc()
                ],
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