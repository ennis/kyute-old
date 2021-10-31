//! winit-based application wrapper.
//!
//! Provides the `run_application` function that opens the main window and translates the incoming
//! events from winit into the events expected by a kyute [`NodeTree`](crate::node::NodeTree).

use crate::{
    core2::{WidgetId, WidgetPod},
    Event, InternalEvent, Model, Widget,
};
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
    // Open windows, mapped to their corresponding widget.
    pub(crate) windows: HashMap<WindowId, WidgetId>,
    pending_events: Vec<Event>,
    /*/// Events waiting to be delivered
    /// Actions emitted by widgets waiting to be processed.
    pending_actions: Vec<PendingAction>,
    needs_relayout: bool,
    needs_recomposition: bool,
    needs_full_repaint: bool,*/
}

impl AppCtx {
    fn new() -> AppCtx {
        AppCtx {
            windows: Default::default(),
            pending_events: vec![],
            //windows: HashMap::new(),
            //pending_actions: vec![],
            //needs_relayout: false,
            //needs_recomposition: false,
            //needs_full_repaint: false,
        }
    }

    /// Registers a widget as a native window widget.
    /// The event loop will call `window_event` whenever an event targeting the window is received.
    pub(crate) fn register_window_widget(&mut self, window_id: WindowId, widget: WidgetId) {
        match self.windows.entry(window_id) {
            Entry::Occupied(_) => {
                warn!("window id {:?} already registered", window_id);
            }
            Entry::Vacant(entry) => {
                entry.insert(widget);
            }
        }
    }

    pub fn post_event(&mut self, event: impl Into<Event>) {
        self.pending_events.push(event.into());
    }

    fn send_event<T: Model, W: Widget<T>>(
        &mut self,
        root_widget: &mut WidgetPod<T, W>,
        data: &mut T,
        event: impl Into<Event>,
    ) {
        self.post_event(event);
        self.flush_pending_events(root_widget, data);
    }

    fn flush_pending_events<T: Model, W: Widget<T>>(
        &mut self,
        root_widget: &mut WidgetPod<T, W>,
        data: &mut T,
    ) {
        while !self.pending_events.is_empty() {
            let events = mem::take(&mut self.pending_events);
            for event in events {
                root_widget.send_root_event(self, &event, data)
            }
        }
    }
}

pub fn run(mut root_widget: impl Widget<()> + 'static) {
    let mut event_loop = EventLoop::new();
    let mut app_ctx = AppCtx::new();
    let mut root_widget = WidgetPod::new(root_widget);

    // run event loop
    event_loop.run(move |event, elwt, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            winit::event::Event::WindowEvent {
                window_id,
                event: winit_event,
            } => {
                if let Some(&window_widget_id) = app_ctx.windows.get(&window_id) {
                    app_ctx.send_event(
                        &mut root_widget,
                        &mut (),
                        InternalEvent::RouteWindowEvent {
                            target: window_widget_id,
                            event: winit_event.to_static().unwrap(),    // TODO
                        },
                    );
                } else {
                    tracing::warn!("WindowEvent: unregistered window id: {:?}", window_id);
                }
            }
            winit::event::Event::RedrawRequested(window_id) => {
                if let Some(&window_widget_id) = app_ctx.windows.get(&window_id) {
                    app_ctx.send_event(
                        &mut root_widget,
                        &mut (),
                        InternalEvent::RouteRedrawRequest(window_widget_id),
                    );
                } else {
                    tracing::warn!("RedrawRequested: unregistered window id: {:?}", window_id);
                }
            }
            winit::event::Event::MainEventsCleared => {}
            _ => (),
        }
    })
}
