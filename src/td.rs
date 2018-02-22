use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::device::Device;
use std::sync::Arc;
use cgmath::{Matrix4, Euler, Vector4, Vector3, Rad, One, Zero, Point3, Deg, InnerSpace, Quaternion, Angle, PerspectiveFov};

pub type Vec3 = Vector3<f32>;

#[derive(Clone)]
pub struct Mesh {
	verts: Vec<Vertex>,
	indices: Vec<u16>,
	position: Vec3,
}

impl Mesh {
	pub fn new(verts: Vec<Vertex>, indices: Vec<u16>) -> Mesh {
		Mesh {
			verts,
			indices,
			position: Vector3::new(0.0, 0.0, 0.0),
		}
	}
	
	pub fn get_position(&self) -> Vec3 {
		self.position.clone()
	}
	
	pub fn set_position(&mut self, pos: Vec3) {
		self.position = pos;
	}
	
	pub fn v_buf(&self, device: Arc<Device>) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
		let verts = self.verts.iter().map(|vert| {
			let mut vert = *vert;
			let new_vec = vert.vec3() + self.position;
			vert.a_Pos = [new_vec.x, new_vec.y, new_vec.z, vert.a_Pos[3]];
			vert
		}).collect::<Vec<_>>();
		CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), verts.into_iter()).unwrap()
	}
	
	pub fn i_buf(&self, device: Arc<Device>) -> Arc<CpuAccessibleBuffer<[u16]>> {
		CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), self.indices.clone().into_iter()).unwrap()
	}
	
	pub fn translate(&mut self, trans: Vec3) {
		self.position += trans;
	}
	
	pub fn rotate(&mut self, rot: Vector3<Rad<f32>>) {
		for vert in &mut self.verts {
			vert.a_Pos = *(Matrix4::from_angle_x(rot.x) * Vector4::from(vert.a_Pos)).as_ref();
			vert.a_Pos = *(Matrix4::from_angle_y(rot.y) * Vector4::from(vert.a_Pos)).as_ref();
			vert.a_Pos = *(Matrix4::from_angle_z(rot.z) * Vector4::from(vert.a_Pos)).as_ref();
		}
	}
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
	r: f32,
	g: f32,
	b: f32,
	a: f32
}

impl Color {
	pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
		Color { r, g, b, a }
	}
	
	pub fn raw(self) -> [f32; 4] {
		[self.r, self.g, self.b, self.a]
	}
	
	pub fn red() -> Color { Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 } }
	pub fn green() -> Color { Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 } }
	pub fn blue() -> Color { Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 } }
	pub fn white() -> Color { Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 } }
	pub fn black() -> Color { Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 } }
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
	pub a_Pos: [f32; 4],
	pub a_Color: [f32; 4],
}

impl Vertex {
	pub fn new(x: f32, y: f32, z: f32, w: f32, color: Color) -> Vertex {
		Vertex {
			a_Pos: [x, y, z, w],
			a_Color: color.raw()
		}
	}
	
	pub fn raw(self) -> [f32; 4] {
		self.a_Pos
	}
	
	pub fn vec3(&self) -> Vec3 {
		Vector3::new(self.a_Pos[0], self.a_Pos[1], self.a_Pos[2])
	}
	
	pub fn vec4(&self) -> Vector4<f32> {
		Vector4::new(self.a_Pos[0], self.a_Pos[1], self.a_Pos[2], self.a_Pos[3])
	}
}

impl_vertex!(Vertex, a_Pos, a_Color);

pub struct Camera {
	pos: Vec3,
	rot: Euler<Deg<f32>>,
	pub proj: PerspectiveFov<f32>,
	front: Vec3,
	right: Vec3,
	up: Vec3,
}

impl Camera {
	pub fn new(proj: PerspectiveFov<f32>) -> Camera {
		let worldup: Vec3 = Vector3::new(0.0, 1.0, 0.0);
		let rot = Euler::new(Deg(0.0), Deg(0.0), Deg(0.0));
		let front: Vec3 = Vector3::new(
			(rot.y * rot.x.cos()).cos(),
			rot.x.sin(),
			(rot.y * rot.x.cos()).sin()
		).normalize();
		let right = (front.cross(worldup)).normalize();
		let up = (right.cross(front)).normalize();
		
		Camera {
			pos: Vector3::zero(),
			rot,
			proj,
			front,
			right,
			up,
		}
	}
	
	pub fn get_pos(&self) -> Vec3 {
		self.pos
	}
	
	pub fn get_rot(&self) -> Euler<Deg<f32>> {
		self.rot
	}
	
	pub fn set_pos(&mut self, pos: Vec3) {
		self.pos = pos;
		self.update();
	}
	
	pub fn set_rot(&mut self, rot: Euler<Deg<f32>>) {
		self.rot = rot;
		self.update();
	}
	
	pub fn get_vec(&self) -> (Vec3, Vec3, Vec3) {
		(self.front, self.right, self.up)
	}
	
	fn update(&mut self) {
		let worldup: Vec3 = Vector3::new(0.0, 1.0, 0.0);
		self.front = Vector3::new(
			self.rot.y.cos() * self.rot.x.cos(),
			self.rot.x.sin(),
			self.rot.y.sin() * self.rot.x.cos()
		).normalize();
		self.right = (self.front.cross(worldup)).normalize();
		self.up = (self.right.cross(self.front)).normalize();
	}
	
	pub fn get_view(&self) -> Matrix4<f32> {
		let worldup: Vec3 = Vector3::new(0.0, 1.0, 0.0);
		let eye: [f32; 3] = *self.pos.as_ref();
		let center: [f32; 3] = *(self.pos + self.front).as_ref();
		
		Matrix4::look_at_dir(Point3::from(eye), self.front, self.up)
	}
}