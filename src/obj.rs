use std::sync::Arc;
use std::fmt;

use vulkano::buffer::{ImmutableBuffer, BufferUsage};

use RenderInternal;
use td::*;

pub struct Mesh {
	pub(crate) verts: Arc<ImmutableBuffer<[Vertex]>>,
	pub(crate) indices: Option<Arc<ImmutableBuffer<[u32]>>>,
}

impl fmt::Debug for Mesh {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Mesh {{ verts: ImmutableBuffer, indices: {} }}", if self.indices.is_some() {
			"Some(ImmutableBuffer)"
		} else {
			"None"
		})
	}
}

pub struct Object {
	pub mesh: Box<Mesh>,
}

impl Object {
	pub fn from_mesh(m: Mesh) -> Self {
		Object {
			mesh: Box::new(m),
		}
	}
}

impl Mesh {
	pub fn new(internal: &RenderInternal, verts: Vec<Vertex>, indices: Vec<u32>) -> Result<Self, ()> {
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
		
		let (vbuf, _) = ImmutableBuffer::from_iter(verts.into_iter(), BufferUsage::all(), internal.queue.clone()).unwrap();
		let (ibuf, _) = ImmutableBuffer::from_iter(indices.into_iter(), BufferUsage::all(), internal.queue.clone()).unwrap();
		
		Ok(Mesh {
			verts: vbuf,
			indices: Some(ibuf),
		})
	}
	
	pub fn new_pure(internal: &RenderInternal, verts: Vec<Vertex>) -> Self {
		let (vbuf, _) = ImmutableBuffer::from_iter(verts.into_iter(), BufferUsage::all(), internal.queue.clone()).unwrap();
		
		Mesh {
			verts: vbuf,
			indices: None,
		}
	}
}

