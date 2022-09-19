#[macro_use]
extern crate log;

use dioxus::prelude::Component;
use glutin::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
};
use webrender::api::units::DeviceIntSize;
use window::{RendererEvent, Window};

mod render;
mod state;
mod utils;
pub mod window;

pub fn launch(root: Component<()>) {
    // env_logger::init();

    let mut event_loop = EventLoop::with_user_event();
    let mut context = Window::new(root, &event_loop);
    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let id = context.id();
        let window = context.window();
        let size = window.inner_size();

        match event {
            Event::UserEvent(RendererEvent::Dirty(w)) if &w == id => {
                let device_size = DeviceIntSize::new(size.width as i32, size.height as i32);
                let device_pixel_ratio = window.scale_factor() as f32;
                let layout_size =
                    device_size.to_f32() / webrender::euclid::Scale::new(device_pixel_ratio);
                context.send_event(Event::UserEvent(RendererEvent::Redraw(w, layout_size)));
            }
            Event::UserEvent(RendererEvent::Rerender(w)) if &w == id => {
                let device_size = DeviceIntSize::new(size.width as i32, size.height as i32);
                context.rerender(device_size);
                context.swap_buffers().ok();
            }
            _ => context.send_event(event),
        }
    });

    context.deinit();
}
