use std;
use std::collections::HashMap;

use {App, Renderer};
use render::{Render};
use obj::{Mesh, Object};
use td::*;
use math::{Zero, Vector3, Rad, PerspectiveFov, Deg, Euler, InnerSpace};
use window::{Event, WindowEvent, KeyboardInput, ElementState, DeviceEvent};

mod data {
	use td::Vertex;
	pub static DATA: ([Vertex; 8], [u32; 36]) = (
		[
			Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [-0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [-0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
			Vertex { a_Pos: [-0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.0, 1.0, 1.0], a_Normal: [0.0, 0.0, -1.0] },
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
	
	pub static VERTEX_DATA: [Vertex; 36] = [
		Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, -1.0] },
		Vertex { a_Pos: [0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, -1.0] },
		Vertex { a_Pos: [0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, -1.0] },
		Vertex { a_Pos: [0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, -1.0] },
		Vertex { a_Pos: [-0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, -1.0] },
		Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, -1.0] },
		
		Vertex { a_Pos: [-0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, 1.0] },
		Vertex { a_Pos: [0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, 1.0] },
		Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, 1.0] },
		Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, 1.0] },
		Vertex { a_Pos: [-0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, 1.0] },
		Vertex { a_Pos: [-0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 0.0, 1.0] },
		
		Vertex { a_Pos: [-0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [-1.0, 0.0, 0.0] },
		Vertex { a_Pos: [-0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [-1.0, 0.0, 0.0] },
		Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [-1.0, 0.0, 0.0] },
		Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [-1.0, 0.0, 0.0] },
		Vertex { a_Pos: [-0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [-1.0, 0.0, 0.0] },
		Vertex { a_Pos: [-0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [-1.0, 0.0, 0.0] },
		
		Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [1.0, 0.0, 0.0] },
		Vertex { a_Pos: [0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [1.0, 0.0, 0.0] },
		Vertex { a_Pos: [0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [1.0, 0.0, 0.0] },
		Vertex { a_Pos: [0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [1.0, 0.0, 0.0] },
		Vertex { a_Pos: [0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [1.0, 0.0, 0.0] },
		Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [1.0, 0.0, 0.0] },
		
		Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, -1.0, 0.0] },
		Vertex { a_Pos: [0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, -1.0, 0.0] },
		Vertex { a_Pos: [0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, -1.0, 0.0] },
		Vertex { a_Pos: [0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, -1.0, 0.0] },
		Vertex { a_Pos: [-0.5, -0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, -1.0, 0.0] },
		Vertex { a_Pos: [-0.5, -0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, -1.0, 0.0] },
		
		Vertex { a_Pos: [-0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 1.0, 0.0] },
		Vertex { a_Pos: [0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 1.0, 0.0] },
		Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 1.0, 0.0] },
		Vertex { a_Pos: [0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 1.0, 0.0] },
		Vertex { a_Pos: [-0.5, 0.5, 0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 1.0, 0.0] },
		Vertex { a_Pos: [-0.5, 0.5, -0.5, 1.0], a_Color: [0.0, 0.8, 0.9, 1.0], a_Normal: [0.0, 1.0, 0.0] },
	];
}

#[test]
fn basic() {
	struct Player {
		pub camera: Camera,
		pub speed: f32,
	}
	
	impl Player {
		pub fn walk(&mut self, mov: Vec3) {
			let old_pos = self.camera.get_pos();
			let vecs = self.camera.get_vec();
			let (front, right) = (vecs.0, vecs.1);
			let (forward, upward, sideward) = (mov.z, mov.y, mov.x);
			self.camera.set_pos(old_pos + Vec3::new(front.x, 0.0, front.z).normalize() * (forward * self.speed));
			let old_pos = self.camera.get_pos();
			self.camera.set_pos(old_pos + Vec3::new(right.x, 0.0, right.z).normalize() * (sideward * self.speed));
			let old_pos = self.camera.get_pos();
			self.camera.set_pos(Vec3::new(old_pos.x, old_pos.y + upward * self.speed, old_pos.z))
		}
		
		pub fn spin(&mut self, amt: Deg<f32>) {
			let old_rot = self.camera.get_rot();
			self.camera.set_rot(Euler::new(old_rot.x, (old_rot.y + amt * 15.0) % Deg(360.0), old_rot.z));
		}
		
		pub fn crane(&mut self, amt: Deg<f32>) {
			let old_rot = self.camera.get_rot();
			let mut rot: Deg<f32> = old_rot.x + amt * 15.0;
			if rot.0.partial_cmp(&89.99).unwrap() == std::cmp::Ordering::Greater {
				rot.0 = 89.99;
			} else if rot.0.partial_cmp(&-89.99).unwrap() == std::cmp::Ordering::Less {
				rot.0 = -89.99;
			}
			self.camera.set_rot(Euler::new(rot, old_rot.y, old_rot.z));
		}
	}
	
	struct TestApp {
		running: bool,
		player: Player,
		movement: ((bool, bool), (bool, bool), (bool, bool)),
	}
	
	impl App for TestApp {
		fn get_camera(&mut self) -> &mut Camera {
			&mut self.player.camera
		}
		
		fn handle_event(&mut self, event: Event, objects: &mut HashMap<String, Object>) {
			use window::VirtualKeyCode::*;
			match event {
				Event::WindowEvent { event, .. } => match event {
					WindowEvent::KeyboardInput {
						device_id: _, input: KeyboardInput { scancode: _, state: _, virtual_keycode: Some(Escape), modifiers: _ }
					} | WindowEvent::Closed => {
						self.running = false;
					},
					WindowEvent::KeyboardInput {
						device_id: _, input: KeyboardInput { scancode: _, state, virtual_keycode: key, modifiers: _mods }
					} => {
						match state {
							ElementState::Pressed => {
								match key {
									Some(W) => (self.movement.2).0 = true,
									Some(S) => (self.movement.2).1 = true,
									Some(A) => (self.movement.0).0 = true,
									Some(D) => (self.movement.0).1 = true,
									Some(Space) => (self.movement.1).0 = true,
									Some(LShift) => (self.movement.1).1 = true,
									_ => {},
								}
							},
							ElementState::Released => {
								match key {
									Some(W) => (self.movement.2).0 = false,
									Some(S) => (self.movement.2).1 = false,
									Some(A) => (self.movement.0).0 = false,
									Some(D) => (self.movement.0).1 = false,
									Some(Space) => (self.movement.1).0 = false,
									Some(LShift) => (self.movement.1).1 = false,
									_ => {},
								}
							}
						}
					},
					_ => {}
				},
				Event::DeviceEvent { event, .. } => match event {
					DeviceEvent::Motion { axis, value } => {
						match axis {
							0 => {
								self.player.spin(Deg((value / 200.0f64) as f32));
							},
							1 => {
								self.player.crane(Deg((value / 200.0f64) as f32));
							},
							_ => {}
						}
					},
					_ => {}
				},
				_ => {},
			}
		}
		
		fn update(&mut self, ms: f32, objects: &mut HashMap<String, Object>) {
			let mut movement: Vec3 = Vec3::zero();
			if (self.movement.0).0 { movement.x -= 1.0 };
			if (self.movement.0).1 { movement.x += 1.0 };
			if (self.movement.1).0 { movement.y -= 1.0 };
			if (self.movement.1).1 { movement.y += 1.0 };
			if (self.movement.2).0 { movement.z += 1.0 };
			if (self.movement.2).1 { movement.z -= 1.0 };
			
			self.player.walk(movement * ms / 200.0);
		}
		
		fn is_running(&self) -> bool {
			self.running
		}
		
		fn start(&mut self, objects: &mut HashMap<String, Object>) {
			let mesh = Mesh::new_pure(data::VERTEX_DATA.to_vec());
			let obj = Object::from_mesh(mesh);
			objects.insert("cube".to_string(), obj);
		}
	}
	
	let mut camera = Camera::new(PerspectiveFov {
		fovy: Rad(90.0),
		aspect: 1.0,
		near: 0.1,
		far: 1000.0
	});
	
	camera.set_pos(Vec3::new(-4.0, 0.75, -0.75));
	
	let player = Player {
		camera,
		speed: 1.0,
	};
	
	let running = true;
	
	let app = TestApp {
		player,
		running,
		movement: ((false, false), (false, false), (false, false)),
	};
	let mut renderer = Renderer::new(app);
	renderer.run();
}