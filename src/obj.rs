use std::sync::Arc;

use vulkano::buffer::{ImmutableBuffer, BufferUsage};
use vulkano::device::Queue;

use td::*;
use render::Render;

#[derive(Debug)]
pub struct Mesh {
	pub verts: Vec<Vertex>,
	pub indices: Vec<u32>,
	pub indexed: bool,
}

pub struct Object {
	pub data: Box<Mesh>,
}

impl Object {
	pub fn from_mesh(m: Mesh) -> Self {
		Object {
			data: Box::new(m),
		}
	}
	
	pub fn translate(&mut self, t: &Vec3) {
		for v in self.data.verts.iter_mut() {
			v.translate(t);
		}
	}
}

impl Mesh {
	pub fn new(verts: Vec<Vertex>, indices: Vec<u32>) -> Result<Self, ()> {
		// Make sure the indices represent triangles well
		if indices.len() % 3 != 0 {
			return Err(())
		}
		
		// Make sure none of the indices refer to a vertex that isn't available
		for i in &indices {
			if *i as usize >= verts.len() {
				return Err(())
			}
		}
		
		Ok(Mesh {
			verts,
			indices,
			indexed: true,
		})
	}
	
	pub fn new_pure(verts: Vec<Vertex>) -> Self {
		Mesh {
			verts,
			indices: Vec::new(),
			indexed: false,
		}
	}
}

impl Render for Mesh {
	fn vbuf(&self) -> &[Vertex] {
		self.verts.as_ref()
	}
	fn ibuf(&self) -> &[u32] {
		self.indices.as_ref()
	}
}

pub struct GpuMesh {
	pub vbuf: Arc<ImmutableBuffer<[Vertex]>>,
	pub ibuf: Arc<ImmutableBuffer<[u32]>>,
}

impl GpuMesh {
	pub fn from_mesh(mesh: &Mesh, queue: Arc<Queue>) -> Self {
		let (vbuf, _) = ImmutableBuffer::from_iter(mesh.verts.iter().cloned(),
		                                             BufferUsage::all(),
		                                             Arc::clone(&queue)).unwrap();
		let (ibuf, _) = ImmutableBuffer::from_iter(mesh.indices.iter().cloned(),
		                                              BufferUsage::all(),
		                                              Arc::clone(&queue)).unwrap();
		
		GpuMesh {
			vbuf,
			ibuf,
		}
	}
}

