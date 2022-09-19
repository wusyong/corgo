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
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    window::{WindowBuilder, WindowId},
    ContextWrapper, NotCurrent, PossiblyCurrent, WindowedContext,
};
use taffy::{
    prelude::{Number, Size},
    Taffy,
};
use webrender::{
    api::{
        units::{DeviceIntSize, LayoutSize},
        *,
    },
    DebugFlags, RenderApi, Renderer, ShaderPrecacheFlags,
};

use crate::state::{FocusState, NodeState};

pub struct Window {
    id: WindowId,
    windowed_context: ContextWrapper<PossiblyCurrent, glutin::window::Window>,
    event_tx: crossbeam_channel::Sender<Event<'static, RendererEvent>>,
    renderer: Renderer,
}

impl Window {
    /// Spawn a Window task in the background and return a Window instance.
    pub fn new(root: Component<()>, event_loop: &EventLoop<RendererEvent>) -> Self {
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
        proxy: EventLoopProxy<RendererEvent>,
    ) -> Self {
        let window = windowed_context.window();
        let inner_size = window.inner_size();
        let id = window.id();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        let windowed_context = unsafe { windowed_context.make_current().unwrap() };

        // Create gl Api
        let windowed_context = unsafe { windowed_context.treat_as_current() };
        let gl = match windowed_context.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| windowed_context.get_proc_address(symbol) as *const _)
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
        let device_size = DeviceIntSize::new(inner_size.width as i32, inner_size.height as i32);
        let notifier = Box::new(Notifier::new(id, proxy.clone()));

        // Create Webrender
        let (renderer, sender) =
            webrender::create_webrender_instance(gl.clone(), notifier, opts, None).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(device_size);
        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);
        let builder = DisplayListBuilder::new(pipeline_id);

        // Spawn and run a WindowTask
        std::thread::spawn(move || {
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
                width: Number::Defined(inner_size.width as f32),
                height: Number::Defined(inner_size.height as f32),
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

            proxy.send_event(RendererEvent::Dirty(id)).unwrap();

            let state = WindowState::default();
            let task = WindowTask {
                id,
                size: inner_size,
                event_rx,
                proxy,
                state,
                builder,
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

        Self {
            id,
            event_tx,
            windowed_context,
            renderer,
        }
    }

    pub fn id(&self) -> &WindowId {
        &self.id
    }

    // TODO: define Window wrapper struct because access range is huge
    pub fn window(&self) -> &glutin::window::Window {
        &self.windowed_context.window()
    }

    // TODO: define Renderer wrapper struct because access range is huge
    pub fn rerender(&mut self, device_size: DeviceIntSize) {
        self.renderer.update();
        self.renderer.render(device_size, 0).unwrap();
        let _ = self.renderer.flush_pipeline_info();
    }

    pub fn deinit(self) {
        self.renderer.deinit();
    }

    pub fn swap_buffers(&self) -> Result<(), glutin::ContextError> {
        self.windowed_context.swap_buffers()
    }

    pub fn send_event(&self, event: Event<RendererEvent>) {
        if let Some(event) = event.to_static() {
            self.event_tx.send(event).unwrap_or_else(|e| {
                error!("{}", e);
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RendererEvent {
    Redraw(WindowId, LayoutSize),
    Rerender(WindowId),
    Dirty(WindowId),
}

struct Notifier {
    id: WindowId,
    events_proxy: EventLoopProxy<RendererEvent>,
}

impl Notifier {
    fn new(id: WindowId, events_proxy: EventLoopProxy<RendererEvent>) -> Notifier {
        Notifier { id, events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier::new(self.id, self.events_proxy.clone()))
    }

    fn wake_up(&self, _composite_needed: bool) {
        #[cfg(not(target_os = "android"))]
        let _ = self
            .events_proxy
            .send_event(RendererEvent::Rerender(self.id));
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
    event_rx: Receiver<Event<'static, RendererEvent>>,
    proxy: EventLoopProxy<RendererEvent>,
    state: WindowState,
    builder: DisplayListBuilder,

    id: WindowId,
    size: PhysicalSize<u32>,
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
            id,
            size,
            event_rx,
            proxy,
            mut state,
            mut builder,
            pipeline_id,
            document_id,
            epoch,
            mut api,
            mut rdom,
            mut vdom,
            stretch,
            mut dirty_nodes,
        } = self;
        let mut size = size;
        let mut resize = None;

        let mut running = true;
        while running {
            if let Ok(event) = event_rx.recv() {
                match event {
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
                    Event::UserEvent(RendererEvent::Redraw(w, layout_size)) if w == id => {
                        let nodes = if state.focus.clean() {
                            DirtyNodes::All
                        } else {
                            std::mem::take(&mut dirty_nodes)
                        };

                        if !nodes.is_empty() {
                            // Stack traversed vdom to display list.
                            // And redrawing is executed in main thread.
                            // TODO: handle node.
                            crate::render::render(
                                &mut builder,
                                pipeline_id,
                                document_id,
                                epoch,
                                &mut api,
                                layout_size,
                            );
                        }

                        dirty_nodes = DirtyNodes::default();
                        proxy.send_event(RendererEvent::Rerender(id)).unwrap();
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
                    proxy.send_event(RendererEvent::Dirty(id)).unwrap();
                }
            }
        }
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
