use crate::engine;

pub struct MyApp {

}

impl engine::Application for MyApp {
	fn new() -> Self {
		MyApp {

		}
	}

	fn update(&mut self, renderer: &mut engine::Renderer) {

	}
}