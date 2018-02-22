use App;
use render::{RenderTargets, Render};
use obj::{Mesh};
use td::*;
use math::{Zero, Vector3, Rad, PerspectiveFov};
use window::{Event, WindowEvent};

static DATA: ([Vertex; 8], [u32; 36]) = (
	[
		Vertex { a_Pos: [-0.25, -0.25, -0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [ 0.25, -0.25, -0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [ 0.25, -0.25,  0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [-0.25, -0.25,  0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [-0.25,  0.25, -0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [ 0.25,  0.25, -0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [ 0.25,  0.25,  0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
		Vertex { a_Pos: [-0.25,  0.25,  0.25, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0] },
	],
	[
		0, 1, 2, 2, 3, 0, // top
		0, 1, 4, 4, 5, 1, // front
		1, 2, 5, 5, 6, 2, // right
		2, 3, 6, 6, 7, 3, // back
		3, 0, 7, 7, 4, 0, // left
		4, 5, 6, 6, 7, 4, // bottom
	]
);


#[test]
fn basic() {
	struct TestApp {
		cube: Mesh,
		running: bool,
		camera: Camera,
	}
	
	impl App for TestApp {
		fn get_data(&self) -> RenderTargets {
			let mut targets = RenderTargets::new();
			targets.push(&self.cube);
			targets
		}
		
		fn get_camera(&mut self) -> &mut Camera {
			&mut self.camera
		}
		
		fn handle_event(&mut self, event: Event) {
			match event {
				Event::WindowEvent { event: WindowEvent::Closed, .. } => {
					*self.is_running() = false;
				},
				_ => {}
			}
		}
		
		fn update(&mut self, ms: f32) {
		
		}
		
		fn is_running(&mut self) -> &mut bool {
			&mut self.running
		}
	}
	
	let mut camera = Camera::new(PerspectiveFov {
		fovy: Rad(90.0),
		aspect: 1.0,
		near: 0.1,
		far: 1000.0
	});
	camera.set_pos(Vec3::new(-1.0, 0.0, 0.0));
	let cube = Mesh::new(DATA.0.to_vec(), DATA.1.to_vec()).unwrap();
	let running = true;
	
	TestApp {
		cube,
		camera,
		running
	}.run();
}