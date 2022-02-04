use crate::engine;
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use rand;

struct Square {
	position: (f32, f32),
	p_vel: (f32, f32),
	rotation: f32,
	r_vel: f32,
	scale: f32,
	s_vel: f32,
	time_left: u32,
	mesh_object: Rc<RefCell<engine::SquareTransform>>,
}

impl Square {
	fn update_mesh_object(&mut self) {
		*self.mesh_object.borrow_mut() = engine::SquareTransform::new(
			self.position.0,
			self.position.1,
			self.rotation,
			self.scale,
			self.scale,
		);
	}
}

pub struct MyApp {
	objects: VecDeque<Square>,
	add_obj_cooldown: u32,
}

impl engine::Application for MyApp {
	fn new() -> Self {
		MyApp {
			objects: VecDeque::new(),
			add_obj_cooldown: 1,
		}
	}

	fn update(&mut self, renderer: &mut engine::Renderer) {
		self.add_obj_cooldown -= 1;
		if self.add_obj_cooldown == 0 {
			self.add_obj_cooldown = rand::random::<u32>() >> 28;
			if self.objects.len() < 16 {
				let mesh_object = engine::SquareTransform::new_rc();
				let new_object = Square {
					position: (rand::random::<f32>() * 1.0 - 0.5, rand::random::<f32>() * 1.0 - 0.5),
					p_vel: (rand::random::<f32>() * 0.005 - 0.0025, rand::random::<f32>() * 0.005 - 0.0025),
					rotation: rand::random::<f32>() * std::f32::consts::PI,
					r_vel: rand::random::<f32>() * 0.02 - 0.01,
					scale: rand::random::<f32>() * 0.4 + 0.6,// * if rand::random::<f32>() > 0.5 { -1.0 } else { 1.0 },
					s_vel: rand::random::<f32>() * 0.001 - 0.0005,
					time_left: 700 + (rand::random::<u32>() >> 28),
					mesh_object: Rc::clone(&mesh_object),
				};
				self.objects.push_back(new_object);
				renderer.add_mesh(mesh_object);
			}
			self.add_obj_cooldown = 200;
		}
		let mut dead_objects: Vec<usize> = Vec::new();
		for (i, square) in self.objects.iter_mut().enumerate() {
			square.time_left -= 1;
			square.position.0 += square.p_vel.0;
			square.position.1 += square.p_vel.1;
			square.rotation += square.r_vel;
			square.scale += square.s_vel;
			square.update_mesh_object();
			if square.time_left == 0 { dead_objects.push(i) }
		}
		for i in dead_objects.iter().rev() {
			self.objects.remove(*i);
		}
	}
}