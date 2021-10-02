use crate::{align_boxes, core2::{EventCtx, LayoutCtx, PaintCtx, WidgetState}, event::PointerEventKind, widget::Text, Alignment, BoxConstraints, Environment, Event, Measurements, Rect, SideOffsets, Size, Widget, WidgetDelegate, LayoutItem};
use kyute_macros::composable;
use kyute_shell::drawing::{Brush, Color};
use std::convert::TryFrom;

#[derive(Clone)]
pub struct Button {
    label: Text,
    //action_queue: ActionQueue<ButtonAction>,
}

impl Button {
    #[composable(uncached)]
    pub fn new(label: impl Into<String>) -> Widget<Button> {
        let state = WidgetState {};
        todo!()
    }
}

impl Widget<Button> {}

pub enum ButtonAction {
    Clicked,
}

impl WidgetDelegate for Button {


    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem {
        // measure the label inside
        let padding = SideOffsets::new_all_same(4.0);
        let content_constraints = constraints.deflate(&padding);

        let label_layout = self.label.layout(ctx, content_constraints, env);
        let mut measurements = label_layout.measurements();

        // add padding on the sides
        measurements.size += Size::new(padding.horizontal(), padding.vertical());

        // apply minimum size
        measurements.size.width = measurements.size.width.max(10.0);
        measurements.size.height = measurements.size.height.max(10.0);

        // constrain size
        measurements.size = constraints.constrain(measurements.size);

        // center the text inside the button
        let offset = align_boxes(
            Alignment::CENTER,
            &mut measurements,
            label_layout.measurements(),
        );

        let mut li = LayoutItem::new(measurements);
        li.add_child(offset, label_layout);
        li
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        let brush = Brush::solid_color(ctx, Color::new(0.100, 0.100, 0.100, 1.0));
        let fill = Brush::solid_color(ctx, Color::new(0.800, 0.888, 0.100, 1.0));
        if ctx.is_hovering() {
            ctx.fill_rectangle(bounds, &fill);
        }
        ctx.draw_rectangle(bounds, &brush, 2.0);
    }
}
