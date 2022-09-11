#[macro_use]
extern crate log;

use dioxus::prelude::Component;
use glutin::{
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
};
use window::Window;

mod render;
mod state;
mod utils;
pub mod window;

pub fn launch(root: Component<()>) {
    // env_logger::init();

    let mut event_loop = EventLoop::with_user_event();
    let window = Window::new(root, &event_loop);

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            _ => (),
        }

        window.send_event(event);
    });
}
