use std::sync::Arc;

use vulkano::buffer::{ImmutableBuffer, BufferUsage};
use vulkano::device::Queue;

use td::*;
use render::Render;

pub struct Mesh {
	pub verts: Vec<Vertex>,
	pub indices: Vec<u32>,
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
			indices
		})
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

struct GpuMesh {
	vbuf: Arc<ImmutableBuffer<[Vertex]>>,
	ibuf: Arc<ImmutableBuffer<[u32]>>,
}

impl GpuMesh {
	pub fn from_mesh(mesh: &Mesh, queue: Arc<Queue>) -> Self {
		let (vbuf, fut) = ImmutableBuffer::from_iter(mesh.verts.iter().cloned(),
		                                             BufferUsage::all(),
		                                             Arc::clone(&queue)).unwrap();
		let (ibuf, fut2) = ImmutableBuffer::from_iter(mesh.indices.iter().cloned(),
		                                              BufferUsage::all(),
		                                              Arc::clone(&queue)).unwrap();
		
		GpuMesh {
			vbuf: vbuf,
			ibuf: ibuf
		}
	}
}

