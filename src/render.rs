use std::time::{Duration, Instant};
use std::sync::Arc;
use std::marker::{Send, Sync};

use vulkano;
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice, Features};
use vulkano::device::{Device, Queue, DeviceExtensions};
use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage, CpuBufferPool};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::shader::ShaderModule;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::descriptor::{PipelineLayoutAbstract};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::command_buffer::{CommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::sync::{now, GpuFuture};
use vulkano::framebuffer::{RenderPass, RenderPassDesc, RenderPassAbstract, Framebuffer, FramebufferAbstract, Subpass};
use vulkano::format::{self, FormatDesc, Format};
use vulkano::memory::pool::{StdMemoryPool};
use vulkano::swapchain::{self, Swapchain, PresentMode, SurfaceTransform, SwapchainCreationError, AcquireError};
use vulkano::image::{SwapchainImage, AttachmentImage, ImageViewAccess};
use vulkano_win::{self, VkSurfaceBuild, Window};
use winit::{EventsLoop, WindowBuilder};
use cgmath::{Matrix4, Deg, Vector3, Rad, PerspectiveFov, One, Zero, Quaternion};
use td::*;

pub struct Renderer {
	instance: Arc<Instance>,
	device: Arc<Device>,
	swapchain: Arc<Swapchain>,
	images: Vec<Arc<SwapchainImage>>,
	pipeline: Arc<GraphicsPipelineAbstract + Send + Sync>,
	render_pass: Arc<RenderPass<Arc<RenderPassDesc>>>,
	queue: Arc<Queue>,
	window: Window,
	camera: Camera,
	uniform_buffer: CpuBufferPool<vs::ty::Data, Arc<StdMemoryPool>>,
	shaders: (vs::Shader, fs::Shader),
	framebuffers: Option<Vec<Arc<FramebufferAbstract>>>,
	previous_frame_end: Arc<GpuFuture>,
	depth_buffer: Arc<AttachmentImage<format::D16Unorm>>,
	dimensions: [u32; 2],
	recreate_swapchain: bool,
}

impl Renderer {
	pub fn new() -> Renderer {
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
				            .. DeviceExtensions::none()
			            },
			            [(queue_family, 0.5)].iter().cloned())
					.expect("Failed to create Device")
		};
		let queue = queues.next().unwrap();
		
		let (mut swapchain, mut images) = {
			let caps = window.surface()
					.capabilities(physical)
					.expect("Failed to get surface capabilites");
			
			let alpha  = caps.supported_composite_alpha.iter().next().unwrap();
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
		).unwrap().build_render_pass(device.clone()).unwrap());
		
		let vs = vs::Shader::load(device.clone()).expect("Failed to create shader");
		let fs = fs::Shader::load(device.clone()).expect("Failed to create shader");
		
		let pipeline = Arc::new(GraphicsPipeline::start()
				.vertex_input_single_buffer::<Vertex>()
				.vertex_shader(vs.main_entry_point(), ())
				.viewports_dynamic_scissors_irrelevant(1)
				.fragment_shader(fs.main_entry_point(), ())
				.depth_stencil_simple_depth()
				.render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
				.build(device.clone())
				.unwrap());
		
		let uniform_buffer = CpuBufferPool::<vs::ty::Data>::new(device.clone(), BufferUsage::all());
		
		let mut framebuffers: Option<Vec<Arc<Framebuffer<_, _>>>> = None;
		let mut recreate_swapchain = false;
		let mut previous_frame_end = Arc::new(now(device.clone())) as Arc<GpuFuture>;
		let mut depth_buffer = vulkano::image::attachment::AttachmentImage::transient(device.clone(), dimensions, vulkano::format::D16Unorm).unwrap();
		
		let camera = Camera::new(PerspectiveFov {
			fovy: Deg(45.0).into(),
			aspect: dimensions[0] as f32 / dimensions[1] as f32,
			near: 0.1,
			far: 1000.0
		});
		
		Renderer {
			instance,
			device,
			swapchain,
			images,
			pipeline,
			render_pass,
			queue,
			window,
			camera,
			uniform_buffer,
			shaders: (vs, fs),
			framebuffers,
			previous_frame_end,
			depth_buffer,
			dimensions,
			recreate_swapchain: false,
		}
	}
	
	pub fn render(&mut self, targets: Vec<Mesh>) {
		self.previous_frame_end.cleanup_finished();
		
		if self.recreate_swapchain {
			self.dimensions = {
				let (width, height) = self.window.window().get_inner_size_pixels().unwrap();
				[width, height]
			};
			
			let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimension(self.dimensions) {
				Ok(r) => r,
				Err(SwapchainCreationError::UnsupportedDimensions) => {
					return;
				},
				Err(e) => panic!("{:?}", e)
			};
			
			self.swapchain = new_swapchain;
			self.images = new_images;
			
			self.camera.proj = PerspectiveFov {
				fovy: Deg(45.0 as f32).into(),
				aspect: self.dimensions[0] as f32 / self.dimensions[1] as f32,
				near: 0.1,
				far: 1000.0
			};
			
			self.depth_buffer = vulkano::image::attachment::AttachmentImage::transient(self.device.clone(), self.dimensions, vulkano::format::D16Unorm).unwrap();
			
			self.framebuffers = None;
			self.recreate_swapchain = false;
		}
		
		if self.framebuffers.is_none() {
			let new_framebuffers = Some(self.images.iter().map(|image| {
				Arc::new(Framebuffer::start(self.render_pass.clone())
						.add(image.clone()).unwrap()
						.add(self.depth_buffer.clone()).unwrap()
						.build().unwrap())
			}).collect::<Vec<_>>());
			self.framebuffers = new_framebuffers;
		}
		
		let (image_num, acquire_future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
			Ok(r) => r,
			Err(AcquireError::OutOfDate) => {
				self.recreate_swapchain = true;
				return;
			},
			Err(e) => panic!("{:?}", e)
		};
		
		let dynamic_state = DynamicState {
			viewports: Some(vec![Viewport {
				origin: [0.0, 0.0],
				dimensions: [self.dimensions[0] as f32, self.dimensions[1] as f32],
				depth_range: 0.0..1.0,
			}]),
			..DynamicState::none()
		};
		
		let uniform_buffer_sub = {
			let uniform_data = vs::ty::Data {
				proj: *Matrix4::from(self.camera.proj).as_ref(),
				view: *Matrix4::from_translation(self.camera.pos).as_ref(),
			};
			
			self.uniform_buffer.next(uniform_data).unwrap()
		};
		
		let set = Arc::new(PersistentDescriptorSet::start(self.pipeline.clone(), 0)
				.add_buffer(uniform_buffer_sub).unwrap()
				.build().unwrap());
		
		let cmd_buffer = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
				.begin_render_pass(self.framebuffers.as_ref().unwrap()[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()])
				.unwrap();
		
		for target in targets {
			cmd_buffer
					.draw_indexed(self.pipeline.clone(),
					              dynamic_state,
					              target.v_buf(self.device.clone()),
					              target.i_buf(self.device.clone()),
					              set.clone(),
					              ())
					.unwrap();
		}
		
		cmd_buffer
				.end_render_pass()
				.unwrap()
				
				.build()
				.unwrap();
		
		let future = self.previous_frame_end.join(acquire_future)
				.then_execute(self.queue.clone(), cmd_buffer).unwrap()
				.then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
				.then_signal_fence_and_flush().unwrap();
		
		self.previous_frame_end = Arc::new(future) as Arc<GpuFuture>;
	}
}

pub trait Render {
	fn v_buf() -> Arc<CpuAccessibleBuffer<[Vertex]>>;
	fn i_buf() -> Arc<CpuAccessibleBuffer<[u16]>>;
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