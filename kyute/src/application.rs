//! winit-based application wrapper.
//!
//! Provides the `run_application` function that opens the main window and translates the incoming
//! events from winit into the events expected by a kyute [`NodeTree`](crate::node::NodeTree).

use crate::{
    composition,
    composition::CompositionCtx,
    core::{Dummy, EventCtx, EventTarget, Widget, NodeId, WidgetDelegate, WindowPaintCtx},
    event::{
        CompositionEvent, Event, InputState, KeyboardEvent, Modifiers, PointerButton,
        PointerButtons, PointerEvent, PointerEventKind, PointerState,
    },
    BoxConstraints, Environment, CallKey, LayoutCtx, PaintCtx, PhysicalPoint, PhysicalSize, Point,
    RepaintRequest,
};
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
use tracing::trace_span;

struct PendingEvent {
    source: Option<NodeId>,
    target: EventTarget,
    event: Event,
}

struct PendingAction {
    node: NodeId,
    payload: Box<dyn Any>,
}

// The internal event loop is managed by winit, so there will always be a `Window` object somewhere.
// It might be a `PlatformWindow` if we want a swapchain with it, or something else, but
// if will always contain a `winit::Window` internally.
// To simplify things, add a method to `Widget` to return the window?
// -> but then the widget doesn't have sole control over the window anymore.

/// Global application context. Contains stuff passed to all widget contexts (Event,Layout,Paint...)
pub struct AppCtx {
    /// Open windows, mapped to their corresponding node in the node tree.
    pub(crate) windows: HashMap<WindowId, NodeId>,
    /// Events waiting to be delivered
    pending_events: Vec<PendingEvent>,
    /// Actions emitted by widgets waiting to be processed.
    pending_actions: Vec<PendingAction>,
    needs_relayout: bool,
    needs_recomposition: bool,
    needs_full_repaint: bool,
}

impl AppCtx {
    fn new() -> AppCtx {
        AppCtx {
            windows: HashMap::new(),
            pending_events: vec![],
            pending_actions: vec![],
            needs_relayout: false,
            needs_recomposition: false,
            needs_full_repaint: false,
        }
    }

    /// Registers a node as a native window widget.
    /// The event loop will call `window_event` whenever an event targeting the window is received.
    pub(crate) fn register_native_window(&mut self, window_id: WindowId, node_id: NodeId) {
        match self.windows.entry(window_id) {
            Entry::Occupied(_) => {
                tracing::warn!("window id {:?} already registered", window_id);
            }
            Entry::Vacant(entry) => {
                entry.insert(node_id);
            }
        }
    }

    pub(crate) fn find_window_node(&self, window_id: WindowId) -> Option<NodeId> {
        self.windows.get(&window_id).cloned()
    }

    pub(crate) fn post_action(&mut self, node: NodeId, payload: Box<dyn Any>) {
        self.pending_actions.push(PendingAction { node, payload })
    }

    pub fn post_event(&mut self, source: Option<NodeId>, target: EventTarget, event: Event) {
        self.pending_events.push(PendingEvent {
            source,
            target,
            event,
        })
    }

    pub(crate) fn request_recomposition(&mut self) {
        self.needs_recomposition = true;
    }

    pub(crate) fn request_relayout(&mut self) {
        self.needs_relayout = true;
    }
}

struct RunLoop {
    app_ctx: AppCtx,
    composed_once: bool,
    root_node: Widget,
    root_fn: fn(&mut CompositionCtx),
}

impl RunLoop {
    fn handle_window_event(
        &mut self,
        window_id: winit::window::WindowId,
        event_loop: &EventLoopWindowTarget<()>,
        winit_event: &winit::event::WindowEvent,
    ) {
        // get the ID of the node corresponding to the window
        let node_id = if let Some(node_id) = self.app_ctx.find_window_node(window_id) {
            node_id
        } else {
            tracing::warn!("unregistered window");
            return;
        };

        // send raw window event to the node
        let window_node = self.root_node.find_child_mut(node_id).unwrap();
        window_node.window_event(&mut self.app_ctx, &winit_event)
    }

    /// Layouts the root node.
    fn relayout(&mut self, event_loop: &EventLoopWindowTarget<()>) {
        self.root_node
            .do_layout(&self.app_ctx, &BoxConstraints::new(.., ..));
        self.root_node.calculate_absolute_positions(Point::origin());
    }

    /// Repaints all windows.
    fn repaint(&mut self) {
        for (w, n) in self.app_ctx.windows.iter() {
            self.root_node
                .find_child_mut(*n)
                .unwrap()
                .paint_window(&self.app_ctx);
        }
    }

    /// Layouts the child nodes of the specified window.
    fn layout_window(&mut self, window_id: WindowId, event_loop: &EventLoopWindowTarget<()>) {
        let node_id = if let Some(node_id) = self.app_ctx.find_window_node(window_id) {
            node_id
        } else {
            tracing::warn!(?window_id, "unregistered window");
            return;
        };

        let window_node = self.root_node.find_child_mut(node_id).unwrap();

        // the root available space is infinite, technically, this can produce infinitely big visuals,
        // but this is never the case for visuals under windows (they are constrained by the size of the window).
        // Note that there's no window created by default. The user should create window widgets to
        // have something to render to.
        window_node.do_layout(&mut self.app_ctx, &BoxConstraints::new(.., ..));

        // after layout, recompute the absolute positions of the nodes for hit-testing
        window_node.calculate_absolute_positions(Point::origin());
    }

    /// Repaints the specified window.
    fn paint_window(&mut self, window_id: WindowId) {
        let node_id = if let Some(node_id) = self.app_ctx.find_window_node(window_id) {
            node_id
        } else {
            tracing::warn!(?window_id, "unregistered window");
            return;
        };
        self.root_node
            .find_child_mut(node_id)
            .unwrap()
            .paint_window(&mut self.app_ctx);
    }

    fn recompose(&mut self, event_loop: &EventLoopWindowTarget<()>) {
        let _span = trace_span!("recompose").entered();
        self.root_node.recompose(
            &mut self.app_ctx,
            event_loop,
            Environment::new(),
            self.root_fn,
        );
    }

    fn recompose_on_action(
        &mut self,
        event_loop: &EventLoopWindowTarget<()>,
        node: NodeId,
        action: Box<dyn Any>,
    ) {
        self.root_node.recompose_on_action(
            &mut self.app_ctx,
            event_loop,
            Environment::new(),
            node,
            action,
            self.root_fn,
        );
    }

    fn handle_event(
        &mut self,
        event: &winit::event::Event<()>,
        elwt: &EventLoopWindowTarget<()>,
        control_flow: &mut ControlFlow,
    ) {
        *control_flow = ControlFlow::Wait;

        match event {
            winit::event::Event::WindowEvent {
                window_id,
                event: winit_event,
            } => {
                self.handle_window_event(*window_id, elwt, winit_event);
            }
            winit::event::Event::RedrawRequested(window_id) => self.paint_window(*window_id),
            winit::event::Event::MainEventsCleared => {
                // --- handle posted events (loop until no more events are posted) ---
                {
                    let _span = trace_span!("processing posted events").entered();
                    loop {
                        let events = mem::replace(&mut self.app_ctx.pending_events, Vec::new());
                        for t in events {
                            self.root_node
                                .propagate_event(&mut self.app_ctx, &t.event, t.target);
                        }
                        if self.app_ctx.pending_events.is_empty() {
                            break;
                        }
                    }
                }

                // --- handle actions ---
                {
                    let pending_actions = mem::take(&mut self.app_ctx.pending_actions);
                    for action in pending_actions {
                        self.recompose_on_action(elwt, action.node, action.payload);
                    }
                }

                // --- handle recomposition requests ---
                // TODO: eventually this could be removed, since recompositions are a result of actions
                // (or more accurately, actions that result in a state change)
                {
                    if self.app_ctx.needs_recomposition {
                        self.recompose(elwt);
                        self.app_ctx.needs_recomposition = false;
                        // recomposition implies relayout
                        // TODO: if nothing changes, no relayout should be needed...
                        // -> trust the recomposition to request a relayout
                        self.app_ctx.needs_relayout = true;
                    }
                }

                // --- relayout requests ---
                {
                    let _span = trace_span!("relayout").entered();
                    if self.app_ctx.needs_relayout {
                        self.relayout(elwt);
                        self.app_ctx.needs_relayout = false;
                        self.app_ctx.needs_full_repaint = true;
                    }
                }

                // --- repaint windows ---
                {
                    if self.app_ctx.needs_full_repaint {
                        self.repaint();
                        self.app_ctx.needs_full_repaint = false;
                    }
                }
            }
            _ => (),
        }
    }
}

pub fn run(root_fn: fn(&mut CompositionCtx)) {
    let event_loop = EventLoop::new();
    let mut run_loop = RunLoop {
        app_ctx: AppCtx::new(),
        composed_once: false,
        root_node: Widget::dummy(),
        root_fn,
    };

    // first composition and layout
    run_loop.recompose(&event_loop);
    run_loop.relayout(&event_loop);

    // run event loop
    event_loop.run(move |event, elwt, control_flow| {
        run_loop.handle_event(&event, elwt, control_flow);
    })
}
