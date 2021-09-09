use crate::{BoxConstraints, Context, Environment, Event, Measurements, Offset, Rect, Size};
use kyute_macros::composable;
use std::sync::{Arc, Mutex, Weak};
use std::hash::{Hash, Hasher};

/// Context passed to widgets during the layout pass.
///
/// See [`Widget::layout`].
pub struct LayoutCtx {}

pub struct PaintCtx {}

pub struct EventCtx {}

pub struct WindowPaintCtx {}

#[derive(Clone, Debug)]
struct LayoutImpl {
    measurements: Measurements,
    child_layouts: Vec<(Offset, Layout)>,
}

#[derive(Clone, Debug)]
pub struct Layout(Arc<LayoutImpl>);

impl Layout {
    pub fn new(
        measurements: Measurements,
        child_layouts: impl Into<Vec<(Offset, Layout)>>,
    ) -> Layout {
        Layout(Arc::new(LayoutImpl {
            measurements,
            child_layouts: child_layouts.into(),
        }))
    }

    pub fn size(&self) -> Size {
        self.0.measurements.size
    }

    pub fn measurements(&self) -> Measurements {
        self.0.measurements
    }

    pub fn baseline(&self) -> Option<f64> {
        self.0.measurements.baseline
    }

    pub fn child_layouts(&self) -> &[(Offset, Layout)] {
        &self.0.child_layouts
    }
}

struct WidgetImpl<T: ?Sized = dyn WidgetDelegate> {
    delegate: T,
}

struct WidgetState {}

impl Default for WidgetState {
    fn default() -> Self {
        WidgetState {}
    }
}

pub struct Widget<T: ?Sized = dyn WidgetDelegate> {
    delegate: Arc<Mutex<WidgetImpl<T>>>,
    state: Arc<WidgetState>,
}

// TODO remove this once we can do custom unsized coercions in stable
impl<T: WidgetDelegate+'static> From<Widget<T>> for Widget<dyn WidgetDelegate> {
    fn from(other: Widget<T>) -> Self {
        Widget {
            delegate: other.delegate.clone(),
            state: other.state.clone()
        }
    }
}

impl<T: ?Sized> Hash for Widget<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // reference semantics
        (&*self.delegate as *const Mutex<WidgetImpl<T>>).hash(state);
        (&*self.state as *const WidgetState).hash(state);
    }
}

impl<T: ?Sized> Clone for Widget<T> {
    fn clone(&self) -> Self {
        Widget {
            delegate: self.delegate.clone(),
            state: self.state.clone(),
        }
    }
}

impl<T: WidgetDelegate> Widget<T> {
    #[composable(uncached)]
    pub fn new(delegate: T) -> Widget<T> {
        let state = Context::state(|| Arc::new(WidgetState::default()));
        Widget {
            delegate: Arc::new(Mutex::new(WidgetImpl { delegate })),
            state: (*state).clone(),
        }
    }
}

impl <T: ?Sized+WidgetDelegate> Widget<T> {
    /// Called to measure this widget and layout the children of this widget.
    #[composable(uncached)]
    pub fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: &BoxConstraints,
        env: &Environment,
    ) -> Layout {
        Context::cache(
            (self.clone(), constraints.clone(), env.clone()),
            move |_| {
                self.delegate
                    .lock()
                    .unwrap()
                    .delegate
                    .layout(ctx, constraints, env)
            },
        )
    }
}

/// Trait that defines the behavior of a widget.
pub trait WidgetDelegate {
    /// Implement to give a debug name to your widget. Used only for debugging.
    fn debug_name(&self) -> &str {
        "WidgetDelegate"
    }

    /// Handles events and pass them down to children.
    fn event(&mut self, ctx: &mut EventCtx, event: &Event) {}

    /// Called to measure this widget and layout the children of this widget.
    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: &BoxConstraints,
        env: &Environment,
    ) -> Layout;

    /// Called to paint the widget
    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment);

    /// Called only for native window widgets.
    fn window_paint(&self, _ctx: &mut WindowPaintCtx) {}

    /// Returns `true` if the widget is fully opaque when drawn, `false` if it is semitransparent.
    /// This is mostly used as an optimization: if a semitransparent widget needs to be redrawn,
    /// its background (and thus the parent
    fn is_opaque(&self) -> bool {
        false
    }
}
