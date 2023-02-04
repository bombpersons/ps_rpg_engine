use std::path::Path;

use gltf::Gltf;
use log::{debug, info, log, trace};

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

pub struct ModelData {

}

impl ModelData {
    pub fn new(device: &wgpu::Device, filepath: &Path) -> Self {
        // Open the file using our gltf parser.
        let gltf = Gltf::open(filepath).unwrap();

        info!("Loading model {}", filepath.display());

        // Get the triangle data and put it into a vertex buffer.
        //let vertices = Vec::new();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                debug!("{}", node.index());

                // If there is a mesh...
                if let Some(mesh) = node.mesh() {

                    for primitive in mesh.primitives() {
                        primitive.reader(|buffer| gltf.buffers()[buffer.index()]);
                    }

                }
            }
        }

        Self {
            
        }
    }
}