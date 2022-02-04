use std::borrow::Cow;
use winit::{
	event::{Event, WindowEvent},
	event::{VirtualKeyCode, KeyboardInput},
	event_loop::{ControlFlow, EventLoop},
	window::Window,
};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::time::{Instant, Duration};
use game_loop::game_loop;

const SQUARE_VERTEX_ARRAY: [f32; 12] = [
	-1.0, -1.0,
	 1.0, -1.0,
	 1.0,  1.0,

	-1.0, -1.0,
	 1.0,  1.0,
	-1.0,  1.0,
];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct SquareTransform {
	position_x: f32,
	position_y: f32,
	rotation: f32,
	scale_x: f32,
	scale_y: f32,
	// tex_coord_1_x: f32,
	// tex_coord_1_y: f32,
	// tex_coord_2_x: f32,
	// tex_coord_2_y: f32,
}

impl SquareTransform {
	pub fn new(position_x: f32, position_y: f32, rotation: f32, scale_x: f32, scale_y: f32,) -> Self {
		Self {
			position_x,
			position_y,
			rotation,
			scale_x,
			scale_y,
		}
	}

	pub fn new_rc() -> Rc<RefCell<Self>> {
		Rc::new(RefCell::new(SquareTransform {position_x: 0.0,position_y: 0.1,rotation: 0.0,scale_x: 0.0,scale_y: 0.0,}))
	}
}

pub struct Renderer {
	meshes: Vec<Weak<RefCell<SquareTransform>>>,
}

impl Renderer {
	fn new() -> Self {
		Renderer {
			meshes: Vec::new(),
		}
	}

	pub fn add_mesh(&mut self, mesh: Rc<RefCell<SquareTransform>>) {
		self.meshes.push(Rc::downgrade(&mesh));
	}

	fn build_buffer_data(&mut self) -> (Box<[u8]>, usize) {
		let meshes = self.meshes.iter().filter_map(|mesh| {
			if let Some(mesh_rc) = mesh.upgrade() {
				Some(mesh_rc.borrow().clone())
			} else { None }
		});
		let mesh_count = self.meshes.len();
		let meshes = meshes.fold(Vec::<u8>::new(), |mut accum, mesh| {
			accum.extend_from_slice(bytemuck::cast_slice(&[mesh]));
			accum
		});
		(meshes.into_boxed_slice(), mesh_count)
	}

	fn clear_old_rcs(&mut self) {
		let mut dead_objects: Vec<usize> = Vec::new();
		for (i, mesh) in self.meshes.iter().enumerate() {
			if mesh.upgrade().is_none() { dead_objects.push(i) }
		}
		for i in dead_objects.iter().rev() {
			self.meshes.remove(*i);
		}
	}
}

pub trait Application {
	fn new() -> Self;
	fn update(&mut self, renderer: &mut Renderer);
}

impl Gpu {
	async fn init(window: &Window) -> Self {
		let size = window.inner_size();
		let instance = wgpu::Instance::new(wgpu::Backends::all());
		let surface = unsafe { instance.create_surface(window) };
		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				force_fallback_adapter: false,
				// Request an adapter which can render to our surface
				compatible_surface: Some(&surface),
			})
			.await
			.expect("Failed to find an appropriate adapter");

		// Create the logical device and command queue
		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					label: None,
					features: wgpu::Features::empty(),
					// Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
					limits: wgpu::Limits::downlevel_webgl2_defaults()
						.using_resolution(adapter.limits()),
				},
				None,
			)
			.await
			.expect("Failed to create device");

		// Load the shaders from disk
		let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
		});

		let mut texture_data = [[[0u8; 4]; 32]; 32];
		for y in 0..32 {
			for x in 0..32 {
				let z = (x as u8 + y as u8) * 4;
				texture_data[y][x] = [z, z, z, 255];
			}
		}

		let texture_size = wgpu::Extent3d {
			width: 32,
			height: 32,
			depth_or_array_layers: 1,
		};
		let diffuse_texture = device.create_texture(
			&wgpu::TextureDescriptor {
				label: None,
				size: texture_size,
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format: wgpu::TextureFormat::Rgba8UnormSrgb,
				usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			}
		);

		queue.write_texture(
			wgpu::ImageCopyTexture {
				texture: &diffuse_texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
				aspect: wgpu::TextureAspect::All,
			},
			// The actual pixel data
			bytemuck::cast_slice(&texture_data),
			// The layout of the texture
			wgpu::ImageDataLayout {
				offset: 0,
				bytes_per_row: std::num::NonZeroU32::new(4 * texture_size.width),
				rows_per_image: std::num::NonZeroU32::new(texture_size.height),
			},
			texture_size,
		);

		let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});

		let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: None,
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: wgpu::BufferSize::new(0),
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler(
						wgpu::SamplerBindingType::Filtering,
					),
					count: None,
				},
			]
		});

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[
				&bind_group_layout,
			],
			push_constant_ranges: &[],
		});

		let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

		let square_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&SQUARE_VERTEX_ARRAY),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let surface_config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: swapchain_format,
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Mailbox,
		};

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: None,
			layout: Some(&pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",
				buffers: &[
					wgpu::VertexBufferLayout {
						array_stride: 4 * 2 as wgpu::BufferAddress,
						step_mode: wgpu::VertexStepMode::Vertex,
						attributes: &[
							wgpu::VertexAttribute {
								format: wgpu::VertexFormat::Float32x2,
								offset: 0,
								shader_location: 0,
							},
						],
					},
					wgpu::VertexBufferLayout {
						array_stride: 4 * 5 as wgpu::BufferAddress,
						step_mode: wgpu::VertexStepMode::Instance,
						attributes: &[
							wgpu::VertexAttribute {
								format: wgpu::VertexFormat::Float32x2,
								offset: 0,
								shader_location: 1,
							},
							wgpu::VertexAttribute {
								format: wgpu::VertexFormat::Float32,
								offset: 4 * 2,
								shader_location: 2,
							},
							wgpu::VertexAttribute {
								format: wgpu::VertexFormat::Float32x2,
								offset: 4 * 2 + 4 * 1,
								shader_location: 3,
							},
						],
					},
				],
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: "fs_main",
				targets: &[
					wgpu::ColorTargetState {
	                    format: surface_config.format,
	                    blend: None,
	                    write_mask: wgpu::ColorWrites::ALL,
	                },
					wgpu::ColorTargetState {
						format: wgpu::TextureFormat::Rgba8UnormSrgb,
						blend: None,
						write_mask: wgpu::ColorWrites::ALL,
					},
				],
			}),
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview: None,
		});

		let draw_texture = device.create_texture(
			&wgpu::TextureDescriptor {
				label: None,
				size: texture_size,
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format: wgpu::TextureFormat::Rgba8UnormSrgb,
				usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
			}
		);

		surface.configure(&device, &surface_config);

		Gpu {
			device,
			surface,
			queue,
			surface_config,

			render_pipeline,
			bind_group_layout,
			draw_texture,
			diffuse_sampler,
			square_vertex_buffer,
		}
	}
}

struct Gpu {
	device: wgpu::Device,
	surface: wgpu::Surface,
	queue: wgpu::Queue,
	surface_config: wgpu::SurfaceConfiguration,

	render_pipeline: wgpu::RenderPipeline,
	bind_group_layout: wgpu::BindGroupLayout,
	draw_texture: wgpu::Texture,
	diffuse_sampler: wgpu::Sampler,
	square_vertex_buffer: wgpu::Buffer,
}

impl Gpu {
	fn draw<A: Application>(&mut self, application: &A, renderer: &mut Renderer) {
		renderer.clear_old_rcs();
		let (instance_mesh_data, mesh_count) = renderer.build_buffer_data();
		let square_instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: &instance_mesh_data,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
		});
		self.queue.write_buffer(&square_instance_buffer, 0, &instance_mesh_data);

		let texture_size = wgpu::Extent3d {
			width: self.surface_config.width,
			height: self.surface_config.height,
			depth_or_array_layers: 1,
		};
		let screen_cpy_texture = self.device.create_texture(
			&wgpu::TextureDescriptor {
				label: None,
				size: texture_size,
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format: wgpu::TextureFormat::Rgba8UnormSrgb,
				usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
			}
		);
		let screen_cpy_texture_view = screen_cpy_texture.create_view(&wgpu::TextureViewDescriptor::default());

		let draw_texture_view = self.draw_texture.create_view(&wgpu::TextureViewDescriptor::default());


		let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&[self.surface_config.height as f32 / self.surface_config.width as f32]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});
		let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &self.bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: uniform_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&draw_texture_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&self.diffuse_sampler),
				}
			],
		});

		let frame = self.surface
			.get_current_texture()
			.expect("Failed to acquire next swap chain texture");
		let view = frame
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
		{
			let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: None,
				color_attachments: &[
					wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
							store: true,
						},
					},
					wgpu::RenderPassColorAttachment {
						view: &screen_cpy_texture_view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color::RED),
							store: true,
						},
					},
				],
				depth_stencil_attachment: None,
			});
			rpass.set_bind_group(0, &bind_group, &[]);
			rpass.set_pipeline(&self.render_pipeline);
			rpass.set_vertex_buffer(0, self.square_vertex_buffer.slice(..));
			rpass.set_vertex_buffer(1, square_instance_buffer.slice(..));
			rpass.draw(0..6, 0..mesh_count as u32);
		}

		self.draw_texture = self.device.create_texture(
			&wgpu::TextureDescriptor {
				label: None,
				size: texture_size,
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format: wgpu::TextureFormat::Rgba8UnormSrgb,
				usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
			}
		);

		encoder.copy_texture_to_texture(
			screen_cpy_texture.as_image_copy(),
			self.draw_texture.as_image_copy(),
			texture_size,
		);

		self.queue.submit(Some(encoder.finish()));
		frame.present();
	}

	fn resize(&mut self, width: u32, height: u32) {
		self.surface_config.width = width;
		self.surface_config.height = height;
		let aspect_ratio = height as f32 / width as f32;
		// g.game.2.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[aspect_ratio]));
		self.surface.configure(&self.device, &self.surface_config);
	}
}

async fn run<A: 'static + Application>(event_loop: EventLoop<()>, window: Window) {
	let gpu = Gpu::init(&window).await;

	let renderer = Renderer::new();

	let application = A::new();

	game_loop(event_loop, window, (gpu, renderer, application), 60, 0.1, |g| {
		g.game.2.update(&mut g.game.1);
	}, move |g| {
		g.game.0.draw(&g.game.2, &mut g.game.1);
	}, move |g, event| {
		match event {
			Event::WindowEvent {
				event: WindowEvent::Resized(size),
				..
			} => {
				g.game.0.resize(size.width, size.height);
				g.window.request_redraw();
			}
			Event::WindowEvent {
				event: WindowEvent::CloseRequested |
					WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. }, .. },
				..
			} => g.exit(),
			_ => (),
		}
	});
}

pub fn main<A: 'static +  Application>() {
	let event_loop = EventLoop::new();
	let window_builder = winit::window::WindowBuilder::new()
		.with_inner_size(winit::dpi::PhysicalSize::new(1600, 1600));
		// .with_resizable(false);
	let window = window_builder.build(&event_loop).unwrap();
	#[cfg(not(target_arch = "wasm32"))]
	{
		// Temporarily avoid srgb formats for the swapchain on the web
		pollster::block_on(run::<A>(event_loop, window));
	}
	#[cfg(target_arch = "wasm32")]
	{
		std::panic::set_hook(Box::new(console_error_panic_hook::hook));
		console_log::init().expect("could not initialize logger");
		use winit::platform::web::WindowExtWebSys;
		// On wasm, append the canvas to the document body
		web_sys::window()
			.and_then(|win| win.document())
			.and_then(|doc| doc.body())
			.and_then(|body| {
				body.append_child(&web_sys::Element::from(window.canvas()))
					.ok()
			})
			.expect("couldn't append canvas to document body");
		wasm_bindgen_futures::spawn_local(run::<A>(event_loop, window));
	}
}