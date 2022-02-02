use std::borrow::Cow;
use winit::{
	event::{Event, WindowEvent},
	event::{VirtualKeyCode, KeyboardInput},
	event_loop::{ControlFlow, EventLoop},
	window::Window,
};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

const SQUARE_VERTEX_ARRAY: [f32; 12] = [
	-1.0, -1.0,
	 1.0, -1.0,
	 1.0,  1.0,

	-1.0, -1.0,
	 1.0,  1.0,
	-1.0,  1.0,
];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SquareTransform {
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



async fn run(event_loop: EventLoop<()>, window: Window) {
	let size = window.inner_size();
	let instance = wgpu::Instance::new(wgpu::Backends::all());
	let surface = unsafe { instance.create_surface(&window) };
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

	let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
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

	let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: bytemuck::cast_slice(&[size.height as f32 / size.width as f32]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});

	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		label: None,
		layout: &bind_group_layout,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: uniform_buffer.as_entire_binding(),
			},
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
			},
			wgpu::BindGroupEntry {
				binding: 2,
				resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
			}
		],
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

	let square_transform = SquareTransform {
		position_x: 0.0,
		position_y: 0.0,
		rotation: 0.0,
		scale_x: 1.0,
		scale_y: 1.0,
	};

	// PX, PY, RXY, SX, SY
	let square_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: bytemuck::cast_slice(&[square_transform]),
		usage: wgpu::BufferUsages::VERTEX,
	});

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
			targets: &[swapchain_format.into()],
		}),
		primitive: wgpu::PrimitiveState::default(),
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		multiview: None,
	});

	let mut config = wgpu::SurfaceConfiguration {
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
		format: swapchain_format,
		width: size.width,
		height: size.height,
		present_mode: wgpu::PresentMode::Mailbox,
	};

	surface.configure(&device, &config);

	event_loop.run(move |event, _, control_flow| {
		// Have the closure take ownership of the resources.
		// `event_loop.run` never returns, therefore we must do this to ensure
		// the resources are properly cleaned up.
		let _ = (&instance, &adapter, &shader, &pipeline_layout);

		*control_flow = ControlFlow::Wait;
		match event {
			Event::WindowEvent {
				event: WindowEvent::Resized(size),
				..
			} => {
				// Reconfigure the surface with the new size
				config.width = size.width;
				config.height = size.height;
				let aspect_ratio = size.height as f32 / size.width as f32;
				queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[aspect_ratio]));
				surface.configure(&device, &config);
				window.request_redraw();
			}
			Event::RedrawRequested(_) => {
				let frame = surface
					.get_current_texture()
					.expect("Failed to acquire next swap chain texture");
				let view = frame
					.texture
					.create_view(&wgpu::TextureViewDescriptor::default());
				let mut encoder =
					device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
				{
					let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
						label: None,
						color_attachments: &[wgpu::RenderPassColorAttachment {
							view: &view,
							resolve_target: None,
							ops: wgpu::Operations {
								load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
								store: true,
							},
						}],
						depth_stencil_attachment: None,
					});
					rpass.set_bind_group(0, &bind_group, &[]);
					rpass.set_pipeline(&render_pipeline);
					rpass.set_vertex_buffer(0, square_vertex_buffer.slice(..));
					rpass.set_vertex_buffer(1, square_instance_buffer.slice(..));
					rpass.draw(0..6, 0..1);
				}

				queue.submit(Some(encoder.finish()));
				frame.present();
			}
			Event::WindowEvent {
				event: WindowEvent::CloseRequested |
					WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. }, .. },
				..
			} => *control_flow = ControlFlow::Exit,
			_ => {}
		}
	});
}

pub fn main() {
	let event_loop = EventLoop::new();
	let window = winit::window::Window::new(&event_loop).unwrap();
	#[cfg(not(target_arch = "wasm32"))]
	{
		// Temporarily avoid srgb formats for the swapchain on the web
		pollster::block_on(run(event_loop, window));
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
		wasm_bindgen_futures::spawn_local(run(event_loop, window));
	}
}