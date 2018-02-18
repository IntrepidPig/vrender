#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate winit;
extern crate vulkano_win;
extern crate cgmath;

pub mod td;
pub mod math {
	pub use cgmath::*;
}
pub mod window {
	pub use winit::*;
}

use td::{Color, Vertex, Mesh, Camera, Render};

use std::time::{Duration, Instant};
use std::sync::Arc;
use std::mem;
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
use vulkano::swapchain::{self, Swapchain, PresentMode, SurfaceTransform, SwapchainCreationError, AcquireError};
use vulkano_win::VkSurfaceBuild;
use winit::{EventsLoop, WindowBuilder, Window, Event};
use cgmath::{Matrix4, Quaternion, PerspectiveFov, Deg, Rotation, Vector3, Rad, One, Zero};

pub trait App {
	fn get_data(&self) -> &Vec<(Vec<Vertex>, Vec<u32>)>;
	fn get_camera(&mut self) -> &mut Camera;
	fn handle_event(&mut self, event: Event);
	fn update(&mut self, ms: f32);
	fn is_running(&mut self) -> &mut bool;
	
	fn run(&mut self) {
		let instance = Instance::new(None, &vulkano_win::required_extensions(), None)
				.expect("Failed to create instance");
		
		let mut events_loop = EventsLoop::new();
		let window = WindowBuilder::new()
				.build_vk_surface(&events_loop, instance.clone())
				.unwrap();
		let mut dimensions = {
			let (width, height) = window.window().get_inner_size_pixels().unwrap();
			[width, height]
		};
		
		self.get_camera().proj = cgmath::PerspectiveFov {
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
			let caps = window.surface()
					.capabilities(physical)
					.expect("Failed to get surface capabilites");
			
			let alpha = caps.supported_composite_alpha.iter().next().unwrap();
			let format = caps.supported_formats[0].0;
			
			Swapchain::new(device.clone(),
			               window.surface().clone(),
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
		
		while *self.is_running() {
			let start = Instant::now();
			
			previous_frame_end.cleanup_finished();
			
			if recreate_swapchain {
				dimensions = {
					let (width, height) = window.window().get_inner_size_pixels().unwrap();
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
				
				self.get_camera().proj = cgmath::PerspectiveFov {
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
					proj: *Matrix4::from(self.get_camera().proj).as_ref(),
					view: *self.get_camera().get_view().as_ref(),
				};
				
				uniform_buffer.next(uniform_data).unwrap()
			};
			
			let set = Arc::new(PersistentDescriptorSet::start(pipeline.clone(), 0)
					.add_buffer(uniform_buffer_sub).unwrap()
					.build().unwrap());
			
			let mut cmd_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
					.begin_render_pass(framebuffers.as_ref().unwrap()[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()])
					.unwrap();
			
			for data in self.get_data() {
				let dynamic_state = DynamicState {
					viewports: Some(vec![Viewport {
						origin: [0.0, 0.0],
						dimensions: [dimensions[0] as f32, dimensions[1] as f32],
						depth_range: 0.0..1.0,
					}]),
					..DynamicState::none()
				};
				
				let data: (&[Vertex], &[u32]) = (data.0.as_slice(), data.1.as_slice());
				
				//let data = unsafe { (slice::from_raw_parts(data.0, data.1), slice::from_raw_parts(data.2, data.3)) };
			
				let (vbuf, vbuf_fut) = ImmutableBuffer::from_iter(data.0.iter().cloned(), BufferUsage::all(), queue.clone()).unwrap();
				let (ibuf, ibuf_fut) = ImmutableBuffer::from_iter(data.1.iter().cloned(), BufferUsage::all(), queue.clone()).unwrap();
				
				//let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), data.0.iter().cloned()).unwrap();
				//let indices = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), data.1.iter().cloned()).unwrap();
				//let (vertex_buffer, _) = ImmutableBuffer::from_iter(data.0.into_iter(), BufferUsage::all(), queue.clone()).unwrap();
				//let (indices, _) = ImmutableBuffer::from_iter(data.1.into_iter(), BufferUsage::all(), queue.clone()).unwrap();
				
				cmd_buffer = cmd_buffer
						.draw_indexed(pipeline.clone(), dynamic_state, vbuf, ibuf, set.clone(), ())
						.unwrap();
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
			
			events_loop.poll_events(|event| {
				self.handle_event(event.clone());
				
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
			self.update(ms);
			//mesh.rotate(Vector3::new(rot, rot, rot));
			//mesh.translate(Vector3::new(0.0, 0.0, -ms / 1000.0));
		}
	}
}

mod vs {
	#[derive(VulkanoShader)]
	#[ty = "vertex"]
	#[src = "#version 450 core

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec4 a_Color;

layout(location = 0) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Data {
	mat4 proj;
	mat4 view;
} uniforms;

void main() {
	v_Color = a_Color;
    gl_Position = uniforms.proj * uniforms.view * vec4(a_Pos.xyz, 1.0);
}
"]
	struct Dummy;
}

mod fs {
	#[derive(VulkanoShader)]
	#[ty = "fragment"]
	#[src = "#version 450 core

layout(location = 0) in vec4 v_Color;

layout(location = 0) out vec4 f_Color;

void main() {
    f_Color = v_Color;
}
"]
	struct Dummy;
}