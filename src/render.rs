use std::ops::{Deref, DerefMut};

use td::*;

pub trait Render {
	fn vbuf(&self) -> &[Vertex];
	fn ibuf(&self) -> &[u32];
}

pub struct RenderTargets<'a> {
	pub targets: Vec<&'a Render>
}

impl<'a> RenderTargets<'a> {
	pub fn new() -> Self {
		RenderTargets {
			targets: Vec::new()
		}
	}
}

impl<'a> Deref for RenderTargets<'a> {
	type Target = Vec<&'a Render>;
	
	fn deref(&self) -> &Self::Target {
		&self.targets
	}
}

impl<'a> DerefMut for RenderTargets<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.targets
	}
}

