use crate::engine;
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use rand;

struct Square {
	position: (f32, f32),
	rotation: f32,
	scale: f32,
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

// impl engine::Meshable for Square {
// 	fn get_mesh(&self) -> engine::SquareTransform {
// 		engine::SquareTransform::new(self.position.0, self.position.1, self.rotation, self.scale, self.scale)
// 	}
// }

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
					position: (rand::random::<f32>() * 2.0 - 1.0, rand::random::<f32>() * 2.0 - 1.0),
					rotation: rand::random::<f32>() * std::f32::consts::PI,
					scale: rand::random::<f32>() * 0.2 + 0.2,
					time_left: 57 + (rand::random::<u32>() >> 28),
					mesh_object: Rc::clone(&mesh_object),
				};
				self.objects.push_back(new_object);
				renderer.add_mesh(mesh_object);
			}
			self.add_obj_cooldown = 16;
		}
		let mut dead_objects: Vec<usize> = Vec::new();
		for (i, square) in self.objects.iter_mut().enumerate() {
			square.time_left -= 1;
			square.update_mesh_object();
			if square.time_left == 0 { dead_objects.push(i) }
		}
		for i in dead_objects.iter().rev() {
			self.objects.remove(*i);
		}
	}
}