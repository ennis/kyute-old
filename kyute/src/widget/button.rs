use crate::{
    align_boxes, composable,
    core2::{EventCtx, LayoutCtx, PaintCtx},
    event::PointerEventKind,
    widget::Text,
    Alignment, BoxConstraints, Cache, Environment, Event, Key, LayoutItem, Measurements, Rect,
    SideOffsets, Size, Widget, WidgetPod,
};
use kyute_shell::drawing::{Brush, Color};
use std::{cell::Cell, convert::TryFrom, sync::Arc};

#[derive(Clone)]
pub struct Button {
    label: WidgetPod<Text>,
    clicked: (bool, Key<bool>),
}

impl Button {
    /// Creates a new button with the specified label.
    #[composable]
    pub fn new(label: String) -> WidgetPod<Button> {
        WidgetPod::new(Button {
            label: Text::new(label),
            clicked: Cache::state(|| false),
        })
    }

    /// Returns whether this button has been clicked.
    pub fn clicked(&self) -> bool {
        self.clicked.0
    }
}

impl Widget for Button {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(&self, ctx: &mut EventCtx, event: &Event) {
        match event {
            Event::Pointer(p) => match p.kind {
                PointerEventKind::PointerDown => {
                    tracing::trace!("button clicked");
                    ctx.set_state(self.clicked.1, true);
                    ctx.request_focus();
                    ctx.request_redraw();
                    ctx.set_handled();
                }
                PointerEventKind::PointerOver => {
                    tracing::trace!("button PointerOver");
                    ctx.request_redraw();
                }
                PointerEventKind::PointerOut => {
                    tracing::trace!("button PointerOut");
                    ctx.request_redraw();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Measurements {
        // measure the label inside
        let padding = SideOffsets::new_all_same(4.0);
        let content_constraints = constraints.deflate(&padding);

        let label_measurements = self.label.layout(ctx, content_constraints, env);
        let mut measurements = label_measurements;

        // add padding on the sides
        measurements.size += Size::new(padding.horizontal(), padding.vertical());

        // apply minimum size
        measurements.size.width = measurements.size.width.max(10.0);
        measurements.size.height = measurements.size.height.max(10.0);

        // constrain size
        measurements.size = constraints.constrain(measurements.size);

        // center the text inside the button
        let offset = align_boxes(Alignment::CENTER, &mut measurements, label_measurements);

        self.label.set_child_offset(offset);
        measurements
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        tracing::trace!(?bounds, "button paint");
        let brush = Brush::solid_color(ctx, Color::new(0.100, 0.100, 0.100, 1.0));
        let fill = Brush::solid_color(ctx, Color::new(0.800, 0.888, 0.100, 1.0));
        if ctx.is_hovering() {
            ctx.fill_rectangle(bounds, &fill);
        }
        ctx.draw_rectangle(bounds, &brush, 2.0);
        self.label.paint(ctx, bounds, env);
    }
}
