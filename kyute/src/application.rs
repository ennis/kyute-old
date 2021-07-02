//! winit-based application wrapper.
//!
//! Provides the `run_application` function that opens the main window and translates the incoming
//! events from winit into the events expected by a kyute [`NodeTree`](crate::node::NodeTree).

use crate::node::{NodeTree, NodeId};
use kyute_shell::{
    platform::Platform,
    winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowId,
    },
};
use std::collections::HashMap;

/// Application context.
pub struct AppCtx {
    /// Open windows, mapped to their corresponding node in the node tree.
    pub(crate) windows: HashMap<WindowId, NodeId>,
}

impl AppCtx {
    fn new() -> AppCtx {
        AppCtx {
            windows: HashMap::new(),
        }
    }

    fn run(mut self, mut tree: NodeTree, event_loop: EventLoop<()>) {
        // run event loop
        event_loop.run(move |event, elwt, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { window_id, event } => {
                    // deliver event to the target window in the node tree
                    if let Some(node_id) = self.windows.get(&window_id).cloned() {
                        // see RedrawRequested for more comments
                    }
                }

                Event::RedrawRequested(window_id) => {
                    // get the node corresponding to the window ID
                    if let Some(node_id) = self.windows.get(&window_id).cloned() {

                    } else {
                        tracing::warn!("repaint for unregistered window")
                    }
                }
                _ => (),
            }
        })
    }
}

pub fn run() {
    let event_loop = EventLoop::new();
    let mut tree = NodeTree::new();
    let mut app_ctx = AppCtx::new();
    // enter the main event loop
    app_ctx.run(tree, event_loop);
}
