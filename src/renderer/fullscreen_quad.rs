#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PosTexVertex {
    position: [f32; 3],
    uv: [f32; 2],
}

impl PosTexVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PosTexVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS
        }
    }
}

// Handy full screen quad of vertices.
pub const POS_TEX_VERTICES: &[PosTexVertex] = &[
    PosTexVertex { position: [-1.0, -1.0, 0.0 ], uv: [0.0, 1.0] },
    PosTexVertex { position: [1.0, -1.0, 0.0], uv: [1.0, 1.0] },
    PosTexVertex { position: [1.0, 1.0, 0.0], uv: [1.0, 0.0] },

    PosTexVertex { position: [1.0, 1.0, 0.0], uv: [1.0, 0.0] },
    PosTexVertex { position: [-1.0, 1.0, 0.0], uv: [0.0, 0.0] },
    PosTexVertex { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }
];