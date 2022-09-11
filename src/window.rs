use std::{cell::RefCell, rc::Rc, sync::Arc};

use anymap::AnyMap;
use crossbeam_channel::Receiver;
use dioxus::{
    core::{ElementId, EventPriority, SchedulerMsg, UserEvent},
    events::KeyboardData,
    html::input_data::keyboard_types::{Code, Key, Location, Modifiers},
    prelude::{Component, VirtualDom},
};
use dioxus_native_core::real_dom::RealDom;
use gleam::gl;
use glutin::{
    event::{ElementState, Event, StartCause, VirtualKeyCode, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    window::{WindowBuilder, WindowId},
    NotCurrent, PossiblyCurrent, WindowedContext,
};
use taffy::{
    prelude::{Number, Size},
    Taffy,
};
use webrender::{
    api::{units::DeviceIntSize, *},
    DebugFlags, RenderApi, Renderer, ShaderPrecacheFlags, Transaction,
};

use crate::state::{FocusState, NodeState};

#[derive(Debug)]
pub struct Window {
    id: WindowId,
    event_tx: crossbeam_channel::Sender<Event<'static, Redraw>>,
}

impl Window {
    /// Spawn a Window task in the background and return a Window instance.
    pub fn new(root: Component<()>, event_loop: &EventLoop<Redraw>) -> Self {
        // Create glutin's WindowedContext
        let window_builder = WindowBuilder::new()
            // .with_decorations(false)
            .with_transparent(true);
        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::GlThenGles {
                opengl_version: (3, 2),
                opengles_version: (3, 0),
            })
            .build_windowed(window_builder, &event_loop)
            .unwrap();
        let proxy = event_loop.create_proxy();

        Window::spawn(root, windowed_context, proxy)
    }

    /// Spawn a Window task in the background and return a Window instance.
    pub fn spawn(
        root: Component<()>,
        windowed_context: WindowedContext<NotCurrent>,
        proxy: EventLoopProxy<Redraw>,
    ) -> Self {
        let id = windowed_context.window().id();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        // Spawn and run a WindowTask
        std::thread::spawn(move || {
            // Create gl Api
            let windowed_context = unsafe { windowed_context.make_current().unwrap() };
            let gl = match windowed_context.get_api() {
                glutin::Api::OpenGl => unsafe {
                    gl::GlFns::load_with(|symbol| {
                        windowed_context.get_proc_address(symbol) as *const _
                    })
                },
                glutin::Api::OpenGlEs => unsafe {
                    gl::GlesFns::load_with(|symbol| {
                        windowed_context.get_proc_address(symbol) as *const _
                    })
                },
                glutin::Api::WebGl => unimplemented!(),
            };

            info!("OpenGL version {}", gl.get_string(gl::VERSION));
            let device_pixel_ratio = windowed_context.window().scale_factor() as f32;
            info!("Device pixel ratio: {}", device_pixel_ratio);

            // Setup options for Webrender
            let debug_flags = DebugFlags::ECHO_DRIVER_MESSAGES | DebugFlags::TEXTURE_CACHE_DBG;
            let opts = webrender::WebRenderOptions {
                resource_override_path: None,
                precache_flags: ShaderPrecacheFlags::FULL_COMPILE,
                clear_color: ColorF::new(0.3, 0.0, 0.0, 0.5),
                debug_flags,
                //allow_texture_swizzling: false,
                ..Default::default()
            };
            let size = windowed_context.window().inner_size();
            let device_size = DeviceIntSize::new(size.width as i32, size.height as i32);
            let notifier = Box::new(Notifier::new(id, proxy.clone()));

            // Create Webrender
            let (renderer, sender) =
                webrender::create_webrender_instance(gl.clone(), notifier, opts, None).unwrap();
            let mut api = sender.create_api();
            let document_id = api.add_document(device_size);
            let epoch = Epoch(0);
            let pipeline_id = PipelineId(0, 0);

            let layout_size = device_size.to_f32() / euclid::Scale::new(device_pixel_ratio);
            let mut txn = Transaction::new();
            let mut builder = DisplayListBuilder::new(pipeline_id);
            builder.begin();

            txn.set_display_list(epoch, None, layout_size, builder.end());
            txn.set_root_pipeline(pipeline_id);
            txn.generate_frame(0, RenderReasons::empty());
            api.send_transaction(document_id, txn);

            // Create Real DOM
            let mut rdom: RealDom<NodeState> = RealDom::new();

            // Create Virtual DOM
            let mut vdom = VirtualDom::new(root);
            let mutations = vdom.rebuild();

            // Update real dom's nodes
            let to_update = rdom.apply_mutations(vec![mutations]);
            let stretch = Rc::new(RefCell::new(Taffy::new()));
            let mut ctx = AnyMap::new();
            ctx.insert(stretch.clone());

            // Update the style and layout
            let to_rerender = rdom.update_state(&vdom, to_update, ctx);
            let size = Size {
                width: Number::Defined(size.width as f32),
                height: Number::Defined(size.height as f32),
            };
            stretch
                .borrow_mut()
                .compute_layout(
                    rdom[ElementId(rdom.root_id())].state.layout.node.unwrap(),
                    size,
                )
                .unwrap();
            rdom.traverse_depth_first_mut(|n| {
                if let Some(node) = n.state.layout.node {
                    n.state.layout.layout = Some(*stretch.borrow().layout(node).unwrap());
                }
            });
            let dirty_nodes = DirtyNodes::Some(to_rerender.into_iter().collect());

            proxy.send_event(Redraw(id)).unwrap();

            let state = WindowState::default();
            let task = WindowTask {
                event_rx,
                proxy,
                state,
                windowed_context,
                renderer,
                pipeline_id,
                document_id,
                epoch,
                api,
                rdom,
                vdom,
                stretch,
                dirty_nodes,
            };

            task.run();
        });

        Self { id, event_tx }
    }

    pub fn id(&self) -> &WindowId {
        &self.id
    }

    pub fn send_event(&self, event: Event<Redraw>) {
        if let Some(event) = event.to_static() {
            self.event_tx.send(event).unwrap_or_else(|e| {
                error!("{}", e);
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Redraw(WindowId);

struct Notifier {
    id: WindowId,
    events_proxy: EventLoopProxy<Redraw>,
}

impl Notifier {
    fn new(id: WindowId, events_proxy: EventLoopProxy<Redraw>) -> Notifier {
        Notifier { id, events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier::new(self.id, self.events_proxy.clone()))
    }

    fn wake_up(&self, _composite_needed: bool) {
        #[cfg(not(target_os = "android"))]
        let _ = self.events_proxy.send_event(Redraw(self.id));
    }

    fn new_frame_ready(&self, _: DocumentId, _scrolled: bool, composite_needed: bool) {
        self.wake_up(composite_needed);
    }
}

#[derive(Default)]
struct WindowState {
    modifiers: Modifiers,
    focus: FocusState,
}

struct WindowTask {
    event_rx: Receiver<Event<'static, Redraw>>,
    proxy: EventLoopProxy<Redraw>,
    state: WindowState,

    windowed_context: WindowedContext<PossiblyCurrent>,
    renderer: Renderer,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    epoch: Epoch,
    api: RenderApi,

    rdom: RealDom<NodeState>,
    vdom: VirtualDom,
    stretch: Rc<RefCell<Taffy>>,
    dirty_nodes: DirtyNodes,
}

impl WindowTask {
    // Run the WindowTask. This should control either a thread or an async task.
    fn run(self) {
        let Self {
            event_rx,
            proxy,
            mut state,
            windowed_context,
            mut renderer,
            pipeline_id,
            document_id,
            epoch,
            mut api,
            mut rdom,
            mut vdom,
            stretch,
            mut dirty_nodes,
        } = self;
        let window = windowed_context.window();
        let id = window.id();
        let mut size = window.inner_size();
        let mut resize = None;

        let mut running = true;
        while running {
            if let Ok(event) = event_rx.recv() {
                match event {
                    Event::NewEvents(event) => match event {
                        StartCause::Init => window.request_redraw(),
                        _ => (),
                    },
                    Event::MainEventsCleared => window.request_redraw(),
                    Event::WindowEvent { window_id, event } if window_id == id => match event {
                        WindowEvent::CloseRequested => running = false,
                        WindowEvent::KeyboardInput { input, .. } => {
                            if let Some(key) = input.virtual_keycode {
                                // TODO parse the right key
                                let data = KeyboardData::new(
                                    Key::F10,
                                    Code::KeyA,
                                    Location::Standard,
                                    false,
                                    state.modifiers,
                                );

                                // keypress events are only triggered when a key that has text is pressed
                                if let ElementState::Pressed = input.state {
                                    WindowTask::send_event(
                                        &vdom,
                                        UserEvent {
                                            scope_id: None,
                                            priority: EventPriority::Medium,
                                            element: Some(ElementId(1)),
                                            name: "keypress",
                                            data: Arc::new(data.clone()),
                                            bubbles: true,
                                        },
                                    );

                                    if key == VirtualKeyCode::Tab {
                                        state.focus.progress(
                                            &mut rdom,
                                            !state.modifiers.contains(Modifiers::SHIFT),
                                        );
                                    }
                                }

                                WindowTask::send_event(
                                    &vdom,
                                    UserEvent {
                                        scope_id: None,
                                        priority: EventPriority::Medium,
                                        element: state.focus.last_focused_id,
                                        name: match input.state {
                                            ElementState::Pressed => "keydown",
                                            ElementState::Released => "keyup",
                                        },
                                        data: Arc::new(data),
                                        bubbles: true,
                                    },
                                );
                            }
                        }
                        WindowEvent::ModifiersChanged(mods) => {
                            let mut modifiers = Modifiers::empty();
                            if mods.alt() {
                                modifiers |= Modifiers::ALT;
                            }
                            if mods.ctrl() {
                                modifiers |= Modifiers::CONTROL;
                            }
                            if mods.logo() {
                                modifiers |= Modifiers::META;
                            }
                            if mods.shift() {
                                modifiers |= Modifiers::SHIFT;
                            }
                            state.modifiers = modifiers;
                        }
                        WindowEvent::Resized(s) => resize = Some(s),
                        // TODO mouse state
                        // WindowEvent::CursorMoved {
                        // WindowEvent::MouseInput {
                        _ => (),
                    },
                    Event::UserEvent(Redraw(w)) if w == id => window.request_redraw(),
                    Event::RedrawRequested(w) if w == id => {
                        let nodes = if state.focus.clean() {
                            DirtyNodes::All
                        } else {
                            std::mem::take(&mut dirty_nodes)
                        };

                        if !nodes.is_empty() {
                            let device_size =
                                DeviceIntSize::new(size.width as i32, size.height as i32);
                            let device_pixel_ratio = window.scale_factor() as f32;
                            let layout_size =
                                device_size.to_f32() / euclid::Scale::new(device_pixel_ratio);

                            crate::render::render(
                                pipeline_id,
                                document_id,
                                epoch,
                                &mut api,
                                layout_size,
                            );

                            renderer.update();
                            renderer.render(device_size, 0).unwrap();
                            let _ = renderer.flush_pipeline_info();
                            windowed_context.swap_buffers().ok();
                        }

                        dirty_nodes = DirtyNodes::default();
                    }
                    _ => (),
                }
            }

            vdom.process_all_messages();
            if resize.is_some() || vdom.has_work() {
                let mutations = { vdom.work_with_deadline(|| false) };

                for m in mutations.iter() {
                    // TODO self.prune(m);
                    state.focus.prune(m, &rdom);
                }

                // Update the real dom's nodes
                let to_update = rdom.apply_mutations(mutations);
                let mut ctx = AnyMap::new();
                ctx.insert(stretch.clone());

                // Update the style and layout
                let to_rerender = rdom.update_state(&vdom, to_update, ctx);

                if !to_rerender.is_empty() || resize.is_some() {
                    if let Some(s) = resize.take() {
                        dirty_nodes = DirtyNodes::All;
                        size = s;
                    }

                    stretch
                        .borrow_mut()
                        .compute_layout(
                            rdom[ElementId(rdom.root_id())].state.layout.node.unwrap(),
                            Size {
                                width: Number::Defined(size.width as f32),
                                height: Number::Defined(size.height as f32),
                            },
                        )
                        .unwrap();

                    rdom.traverse_depth_first_mut(|n| {
                        if let Some(node) = n.state.layout.node {
                            n.state.layout.layout = Some(*stretch.borrow().layout(node).unwrap());
                        }
                    });

                    if let DirtyNodes::Some(nodes) = &mut dirty_nodes {
                        nodes.extend(to_rerender.into_iter());
                    }
                    proxy.send_event(Redraw(id)).unwrap();
                }
            }
        }

        renderer.deinit();
    }

    // Send UserEvent to vdom's schedular
    fn send_event(vdom: &VirtualDom, event: UserEvent) {
        vdom.get_scheduler_channel()
            .unbounded_send(SchedulerMsg::Event(event))
            .unwrap_or_else(|e| error!("{}", e));
    }
}

pub enum DirtyNodes {
    All,
    Some(Vec<ElementId>),
}

impl DirtyNodes {
    pub fn is_empty(&self) -> bool {
        match self {
            DirtyNodes::All => false,
            DirtyNodes::Some(v) => v.is_empty(),
        }
    }
}

// TODO it should have a empty variant
impl Default for DirtyNodes {
    fn default() -> Self {
        DirtyNodes::Some(vec![])
    }
}
