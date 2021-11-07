//! winit-based application wrapper.
//!
//! Provides the `run_application` function that opens the main window and translates the incoming
//! events from winit into the events expected by a kyute [`NodeTree`](crate::node::NodeTree).

use crate::{BoxConstraints, Point, WidgetPod, LayoutItem, Cache, CacheInvalidationToken};
use keyboard_types::KeyState;
use kyute_shell::{
    platform::Platform,
    winit,
    winit::{
        event::{DeviceId, ElementState, VirtualKeyCode},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::WindowId,
    },
};
use std::{
    any::Any,
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    mem,
    time::Instant,
};
use tracing::{trace_span, warn};

/*struct PendingEvent {
    source: Option<NodeId>,
    target: EventTarget,
    event: Event,
}

struct PendingAction {
    node: NodeId,
    payload: Box<dyn Any>,
}*/

/// Global application context. Contains stuff passed to all widget contexts (Event,Layout,Paint...)
pub struct AppCtx {
    /// Open windows, mapped to their corresponding widget.
    pub(crate) windows: HashMap<WindowId, WidgetPod>,
    cache: Cache,
    /*/// Events waiting to be delivered
    pending_events: Vec<PendingEvent>,
    /// Actions emitted by widgets waiting to be processed.
    pending_actions: Vec<PendingAction>,
    needs_relayout: bool,
    needs_recomposition: bool,
    needs_full_repaint: bool,*/
}

impl AppCtx {
    fn new() -> AppCtx {
        AppCtx {
            windows: HashMap::new(),
            cache: Cache::new()
            //pending_events: vec![],
            //pending_actions: vec![],
            //needs_relayout: false,
            //needs_recomposition: false,
            //needs_full_repaint: false,
        }
    }

    /// Registers a widget as a native window widget.
    /// The event loop will call `window_event` whenever an event targeting the window is received.
    pub(crate) fn register_window_widget(&mut self, window_id: WindowId, widget: WidgetPod) {
        match self.windows.entry(window_id) {
            Entry::Occupied(_) => {
                warn!("window id {:?} already registered", window_id);
            }
            Entry::Vacant(entry) => {
                entry.insert(widget);
            }
        }
    }

    pub(crate) fn find_window_widget(&self, window_id: WindowId) -> Option<WidgetPod> {
        self.windows.get(&window_id).cloned()
    }

    pub(crate) fn invalidate_cache(&mut self, token: CacheInvalidationToken) {
        self.cache.invalidate(token)
    }

    /*pub(crate) fn post_action(&mut self, node: NodeId, payload: Box<dyn Any>) {
        self.pending_actions.push(PendingAction { node, payload })
    }

    pub fn post_event(&mut self, source: Option<NodeId>, target: EventTarget, event: Event) {
        self.pending_events.push(PendingEvent {
            source,
            target,
            event,
        })
    }*/

    /*pub(crate) fn request_recomposition(&mut self) {
        self.needs_recomposition = true;
    }

    pub(crate) fn request_relayout(&mut self) {
        self.needs_relayout = true;
    }*/
}


fn get_root_widget(root_widget_fn: fn() -> WidgetPod) -> WidgetPod {
    root_widget_fn()
}

/*fn build_window_widgets_map(root: LayoutItem) -> HashMap<WindowId, LayoutItem>
{
    fn build_map_recursive(item: LayoutItem) {}
    let mut map = HashMap::new();
    Context::cache(root,
                   |root| { });
}*/

pub fn run(root_widget_fn: fn() -> WidgetPod) {

    let root_widget = root_widget_fn();

    let mut event_loop = EventLoop::new();
    let mut app_ctx = AppCtx::new();

    // run event loop
    event_loop.run(move |event, elwt, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            winit::event::Event::WindowEvent {
                window_id,
                event: winit_event,
            } => {}
            winit::event::Event::RedrawRequested(window_id) => {}
            winit::event::Event::MainEventsCleared => {}
            _ => (),
        }
    })
}
