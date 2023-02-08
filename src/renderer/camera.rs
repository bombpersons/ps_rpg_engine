use cgmath::SquareMatrix;
use winit::dpi::Position;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct LookAtCamera {
  pub eye: cgmath::Point3<f32>,
  pub target: cgmath::Point3<f32>,
  pub up: cgmath::Vector3<f32>,

  pub aspect: f32,
  pub fovy: f32,
  pub znear: f32,
  pub zfar: f32
}

impl LookAtCamera {
  pub fn get_matrix(&self) -> cgmath::Matrix4<f32> {
    let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
    let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.zfar, self.znear);

    OPENGL_TO_WGPU_MATRIX * proj * view
  }
}

pub struct PositionRotationCamera {
  pub position: cgmath::Vector3<f32>,
  pub rotation: cgmath::Vector3<f32>,

  pub aspect: f32,
  pub fovy: f32,
  pub znear: f32,
  pub zfar: f32
}

impl PositionRotationCamera {
  pub fn get_matrix(&self) -> cgmath::Matrix4<f32> {
    let mut view = cgmath::Matrix4::from_translation(self.position) *
                             cgmath::Matrix4::from_angle_x(cgmath::Deg(self.rotation.x)) *
                             cgmath::Matrix4::from_angle_y(cgmath::Deg(self.rotation.y)) *
                             cgmath::Matrix4::from_angle_z(cgmath::Deg(self.rotation.z));
    view = view.invert().unwrap();

    let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.zfar, self.znear);

    OPENGL_TO_WGPU_MATRIX * proj * view
  }
}