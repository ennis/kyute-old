use crate::{
    align_boxes,
    core2::{EventCtx, LayoutCtx, PaintCtx, WidgetPod},
    event::{LifecycleEvent, PointerEventKind},
    widget::Text,
    Alignment, BoxConstraints, Environment, Event, Measurements, Model, Rect, SideOffsets, Size,
    UpdateCtx, Widget,
};
use kyute_shell::drawing::{Brush, Color};
use std::convert::TryFrom;

pub struct Button<T: Model> {
    label: WidgetPod<T, Text<T>>,
    on_click: Option<Box<dyn Fn(&mut EventCtx, &mut T)>>,
}

impl<T: Model> Button<T> {
    pub fn new() -> Button<T> {
        Button {
            label: WidgetPod::new(Text::new()),
            on_click: None,
        }
    }
}

impl<T: Model> Widget<T> for Button<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T) -> Option<T::Change> {
        None
    }

    fn lifecycle(&mut self, ctx: &mut EventCtx, lifecycle_event: &LifecycleEvent, data: &mut T) {
        self.label.lifecycle(ctx, lifecycle_event, data);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, data: &mut T, change: &T::Change) {
        self.label.update(ctx, data, change)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &mut T,
        env: &Environment,
    ) -> Measurements {
        // measure the label inside
        let padding = SideOffsets::new_all_same(4.0);
        let content_constraints = constraints.deflate(&padding);

        let label_measurements = self.label.layout(ctx, content_constraints, data, env);
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

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, data: &mut T, env: &Environment) {
        let brush = Brush::solid_color(ctx, Color::new(0.100, 0.100, 0.100, 1.0));
        let fill = Brush::solid_color(ctx, Color::new(0.800, 0.888, 0.100, 1.0));
        if ctx.is_hovering() {
            ctx.fill_rectangle(bounds, &fill);
        }
        ctx.draw_rectangle(bounds, &brush, 2.0);
    }
}
