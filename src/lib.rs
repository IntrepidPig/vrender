#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate winit;
extern crate vulkano_win;
extern crate cgmath;

pub mod render;
pub mod obj;
pub mod td;
pub mod math {
	pub use cgmath::*;
}
pub mod window {
	pub use winit::*;
}
#[cfg(test)]
mod tests;

use td::{Color, Vertex, Camera};
use render::{Render, RenderTarget};
use obj::Object;

use std::borrow::Borrow;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::cell::RefCell;
use std::mem;
use std::collections::HashMap;
use std::slice;

use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice, Features};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::buffer::{CpuAccessibleBuffer, ImmutableBuffer, DeviceLocalBuffer, BufferUsage, CpuBufferPool};
use vulkano::buffer::sys::UnsafeBuffer;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::shader::ShaderModule;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::command_buffer::{CommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::sync::{now, GpuFuture};
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::format::Format;
use vulkano::swapchain::{self, Surface, Swapchain, PresentMode, SurfaceTransform, SwapchainCreationError, AcquireError};
use vulkano_win::VkSurfaceBuild;
use winit::{EventsLoop, WindowBuilder, Window, Event};
use cgmath::{Matrix4, Quaternion, PerspectiveFov, Deg, Rotation, Vector3, Rad, One, Zero};

pub struct Renderer<A: App> {
	app: A,
	instance: Arc<Instance>,
	events_loop: EventsLoop,
	window: Arc<Surface<Window>>,
	targets: HashMap<String, Object>,
}

pub struct Context<'a> {
	pub window: &'a Window,
	pub targets: &'a mut HashMap<String, Object>,
}

impl<A: App> Renderer<A> {
	pub fn new(app: A) -> Self {
		let instance = Instance::new(None, &vulkano_win::required_extensions(), None)
			.expect("Failed to create instance");
		
		let mut events_loop = EventsLoop::new();
		
		let surface = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();
		
		let mut renderer = Renderer {
			app,
			instance,
			events_loop,
			window: Arc::clone(&surface),
			targets: HashMap::new(),
		};
		
		renderer.app.start(Context {
			window: &renderer.window.window(),
			targets: &mut renderer.targets,
		});
		renderer
	}
	
	pub fn add_target(&mut self, name: &str, t: Object) {
		self.targets.insert(name.to_string(), t);
	}
	
	pub fn run(&mut self) {
		let instance = Arc::clone(&self.instance);
		
		let mut dimensions = {
			let (width, height) = self.window.window().get_inner_size_pixels().unwrap();
			[width, height]
		};
		
		self.app.get_camera().proj = cgmath::PerspectiveFov {
			fovy: Deg(45.0 as f32).into(),
			aspect: dimensions[0] as f32 / dimensions[1] as f32,
			near: 0.1,
			far: 1000.0
		};
		
		let physical = PhysicalDevice::enumerate(&instance).next()
				.expect("No physical device available");
		let queue_family = physical.queue_families()
				.find(|&q| q.supports_graphics())
				.expect("Couldn't find a graphical queue family");
		let (device, mut queues) = {
			Device::new(physical,
			            &Features::none(),
			            &DeviceExtensions {
				            khr_swapchain: true,
				            ..DeviceExtensions::none()
			            },
			            [(queue_family, 0.5)].iter().cloned())
					.expect("Failed to create Device")
		};
		let queue = queues.next().unwrap();
		
		let (mut swapchain, mut images) = {
			let caps = self.window
					.capabilities(physical)
					.expect("Failed to get surface capabilites");
			
			let alpha = caps.supported_composite_alpha.iter().next().unwrap();
			let format = caps.supported_formats[0].0;
			
			Swapchain::new(device.clone(),
			               self.window.clone(),
			               caps.min_image_count,
			               format,
			               dimensions,
			               1,
			               caps.supported_usage_flags,
			               &queue,
			               SurfaceTransform::Identity,
			               alpha,
			               PresentMode::Fifo,
			               true,
			               None)
					.expect("Failed to create swapchain")
		};
		
		let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
			attachments: {
				color: {
					load: Clear,
					store: Store,
					format: swapchain.format(),
					samples: 1,
				},
				depth: {
					load: Clear,
					store: DontCare,
					format: Format::D16Unorm,
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {depth}
			}
		).unwrap());
		
		let vs = vs::Shader::load(device.clone()).expect("Failed to create shader");
		let fs = fs::Shader::load(device.clone()).expect("Failed to create shader");
		
		let pipeline = Arc::new(GraphicsPipeline::start()
				.vertex_input_single_buffer::<Vertex>()
				.vertex_shader(vs.main_entry_point(), ())
				//.triangle_list()
				.viewports_dynamic_scissors_irrelevant(1)
				.fragment_shader(fs.main_entry_point(), ())
				.depth_stencil_simple_depth()
				.render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
				.build(device.clone())
				.unwrap());
		
		let mut framebuffers: Option<Vec<Arc<Framebuffer<_, _>>>> = None;
		let mut recreate_swapchain = false;
		let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;
		let mut depth_buffer = vulkano::image::attachment::AttachmentImage::transient(device.clone(), dimensions, vulkano::format::D16Unorm).unwrap();
		
		let uniform_buffer = CpuBufferPool::<vs::ty::Data>::uniform_buffer(device.clone());
		
		while self.app.is_running() {
			let start = Instant::now();
			
			previous_frame_end.cleanup_finished();
			
			if recreate_swapchain {
				dimensions = {
					let (width, height) = self.window.window().get_inner_size_pixels().unwrap();
					[width, height]
				};
				
				let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
					Ok(r) => r,
					Err(SwapchainCreationError::UnsupportedDimensions) => {
						continue;
					},
					Err(e) => panic!("{:?}", e)
				};
				
				mem::replace(&mut swapchain, new_swapchain);
				mem::replace(&mut images, new_images);
				
				self.app.get_camera().proj = cgmath::PerspectiveFov {
					fovy: Deg(45.0 as f32).into(),
					aspect: dimensions[0] as f32 / dimensions[1] as f32,
					near: 0.1,
					far: 1000.0
				};
				
				depth_buffer = vulkano::image::attachment::AttachmentImage::transient(device.clone(), dimensions, vulkano::format::D16Unorm).unwrap();
				
				framebuffers = None;
				recreate_swapchain = false;
			}
			
			if framebuffers.is_none() {
				let new_framebuffers = Some(images.iter().map(|image| {
					Arc::new(Framebuffer::start(render_pass.clone())
							.add(image.clone()).unwrap()
							.add(depth_buffer.clone()).unwrap()
							.build().unwrap())
				}).collect::<Vec<_>>());
				mem::replace(&mut framebuffers, new_framebuffers);
			}
			
			let (image_num, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
				Ok(r) => r,
				Err(AcquireError::OutOfDate) => {
					recreate_swapchain = true;
					continue;
				},
				Err(e) => panic!("{:?}", e)
			};
			
			let uniform_buffer_sub = {
				let uniform_data = vs::ty::Data {
					proj: *Matrix4::from(self.app.get_camera().proj).as_ref(),
					view: *self.app.get_camera().get_view().as_ref(),
					viewPos: *self.app.get_camera().get_pos().as_ref(),
				};
				
				uniform_buffer.next(uniform_data).unwrap()
			};
			
			let set = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
				.add_buffer(uniform_buffer_sub).unwrap()
				.build().unwrap());
			
			let mut cmd_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
					.begin_render_pass(framebuffers.as_ref().unwrap()[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()])
					.unwrap();
			
			{
				// Add a command for each object in the object
				for (_, data) in &self.targets {
					let dynamic_state = DynamicState {
						viewports: Some(vec![Viewport {
							origin: [0.0, 0.0],
							dimensions: [dimensions[0] as f32, dimensions[1] as f32],
							depth_range: 0.0..1.0,
						}]),
						..DynamicState::none()
					};
					
					let viter = data.data.vbuf();
					let (vbuf, vbuf_fut) = ImmutableBuffer::from_iter(viter.iter().cloned(), BufferUsage::all(), queue.clone()).unwrap();
					
					// Draw indexed call if the mesh has an index buffer
					if data.data.indexed {
						let iiter = data.data.ibuf();
						let (ibuf, ibuf_fut) = ImmutableBuffer::from_iter(iiter.iter().cloned(), BufferUsage::all(), queue.clone()).unwrap();
						cmd_buffer = cmd_buffer
							.draw_indexed(pipeline.clone(), dynamic_state, vbuf, ibuf, set.clone(), ())
							.unwrap();
					} else {
						// Draw the vertices as usual
						cmd_buffer = cmd_buffer
							.draw(pipeline.clone(), dynamic_state, vbuf, set.clone(), ())
							.unwrap();
					}
				}
			}
			
			let cmd_buffer = cmd_buffer
					.end_render_pass()
					.unwrap()
					
					.build()
					.unwrap();
			
			let future = previous_frame_end.join(acquire_future)
					.then_execute(queue.clone(), cmd_buffer).unwrap()
					.then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
					.then_signal_fence_and_flush().unwrap();
			
			previous_frame_end = Box::new(future) as Box<_>;
			
			let app = &mut self.app;
			let events_loop = &mut self.events_loop;
			let window = &self.window;
			let targets = &mut self.targets;
			
			events_loop.poll_events(|event| {
				app.handle_event(event.clone(), Context {
					window: window.window(),
					targets,
				});
				
				use winit::WindowEvent::*;
				match event {
					Event::WindowEvent { event, .. } => {
						match event {
							Resized(_, _) => recreate_swapchain = true,
							_ => {}
						}
					},
					_ => {}
				}
			});
			
			let elapsed = start.elapsed();
			let ms = (elapsed.as_secs() as f64 * 1000.0f64 + elapsed.subsec_nanos() as f64 / 1_000_000.0f64) as f32;
			app.update(ms, Context {
				window: window.window(),
				targets,
			});
		}
	}
}

pub trait App {
	fn get_camera(&mut self) -> &mut Camera;
	fn handle_event(&mut self, event: Event, context: Context);
	fn update(&mut self, ms: f32, context: Context);
	fn is_running(&self) -> bool;
	fn start(&mut self, context: Context) { }
}

mod vs {
	#[derive(VulkanoShader)]
	#[ty = "vertex"]
	#[src = "#version 450 core

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Color;
layout(location = 2) in vec3 a_Normal;

layout(location = 0) out vec4 v_Color;
layout(location = 1) out vec3 v_Normal;
layout(location = 2) out vec3 v_Pos;
layout(location = 3) out vec3 viewPos;

layout(set = 0, binding = 0) uniform Data {
	mat4 proj;
	mat4 view;
	vec3 viewPos;
} uniforms;

void main() {
	v_Color = a_Color;
    gl_Position = uniforms.proj * uniforms.view * vec4(a_Pos.xyz, 1.0);
	v_Pos = a_Pos.xyz;
	v_Normal = a_Normal;
	viewPos = uniforms.viewPos;
}
"]
	#[allow(dead_code)]
	struct Dummy;
}

mod fs {
	#[derive(VulkanoShader)]
	#[ty = "fragment"]
	#[src = "#version 450 core

layout(location = 0) in vec4 v_Color;
layout(location = 1) in vec3 v_Normal;
layout(location = 2) in vec3 v_Pos;
layout(location = 3) in vec3 viewPos;

layout(location = 0) out vec4 f_Color;

void main() {
	float ambientStrength = 0.1;
	vec3 ambient = ambientStrength * vec3(1.0, 1.0, 1.0);
	
	vec3 lightColor = vec3(1.0, 1.0, 1.0);
	vec3 lightPos = vec3(4.0, 3.0, 2.0);
	vec3 norm = normalize(v_Normal);
	vec3 lightDir = normalize(lightPos - v_Pos);

	float diff = max(dot(norm, lightDir), 0.0);
	vec3 diffuse = diff * lightColor;

	float specularStrength = 0.5;
	vec3 viewDir = normalize(viewPos - v_Pos);
	vec3 reflectDir = reflect(-lightDir, norm);
	float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32);
	vec3 specular = specularStrength * spec * lightColor;

	vec4 result = vec4((ambient + diffuse + specular) * v_Color.xyz, v_Color.w);
    f_Color = result;
}
"]
	#[allow(dead_code)]
	struct Dummy;
}
