use crate::{
    align_boxes,
    core2::{EventCtx, LayoutCtx, PaintCtx},
    event::PointerEventKind,
    widget::Text,
    Alignment, BoxConstraints, Environment, Event, Layout, Measurements, Rect, SideOffsets, Size,
    WidgetDelegate,
};
use kyute_shell::drawing::{Brush, Color};
use std::convert::TryFrom;

#[derive(Clone)]
pub struct Button {
    label: Text,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Button {
        Button {
            label: Text::new(label.into()),
        }
    }
}

impl WidgetDelegate for Button {
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Layout {
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
        Layout::with_child_layouts(measurements, vec![(offset, label_layout)])
    }

    fn paint(&self, ctx: &mut PaintCtx, layout: Layout, env: &Environment) {
        let brush = Brush::solid_color(ctx, Color::new(0.100, 0.100, 0.100, 1.0));
        let fill = Brush::solid_color(ctx, Color::new(0.800, 0.888, 0.100, 1.0));

        if ctx.is_hovering() {
            ctx.fill_rectangle(bounds, &fill);
        }
        ctx.draw_rectangle(bounds, &brush, 2.0);

        for c in children {
            c.paint(ctx);
        }
    }
}

/*
impl WidgetDelegate for Button {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event) {
        match event {
            Event::Pointer(p) => match p.kind {
                PointerEventKind::PointerDown => {
                    tracing::trace!("button clicked");
                    ctx.emit_action(ButtonAction::Clicked);
                    ctx.request_focus();
                    ctx.request_redraw();
                    ctx.set_handled();
                }
                PointerEventKind::PointerOver => {
                    ctx.request_redraw();
                }
                PointerEventKind::PointerOut => {
                    ctx.request_redraw();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: &BoxConstraints,
        _env: &Environment,
    ) -> Measurements {
        // measure the label inside
        let padding = SideOffsets::new_all_same(4.0);
        let content_constraints = constraints.deflate(&padding);

        let mut measurements = Measurements::default();

        for c in children.iter_mut() {
            let m = c.layout(ctx, &content_constraints);
            measurements = Measurements {
                size: measurements.size.max(m.size),
                baseline: None,
            };
        }

        // add padding on the sides
        measurements.size += Size::new(padding.horizontal(), padding.vertical());

        // apply minimum size
        measurements.size.width = measurements.size.width.max(10.0);
        measurements.size.height = measurements.size.height.max(10.0);

        // constrain size
        measurements.size = constraints.constrain(measurements.size);

        // center the items inside the button
        for c in children.iter_mut() {
            let offset = align_boxes(Alignment::CENTER, &mut measurements, c.measurements());
            c.set_offset(offset);
        }

        measurements
    }

    fn paint(&mut self, ctx: &mut PaintCtx, bounds: Rect, _env: &Environment) {
        let brush = Brush::solid_color(ctx, Color::new(0.100, 0.100, 0.100, 1.0));
        let fill = Brush::solid_color(ctx, Color::new(0.800, 0.888, 0.100, 1.0));

        if ctx.is_hovering() {
            ctx.fill_rectangle(bounds, &fill);
        }
        ctx.draw_rectangle(bounds, &brush, 2.0);

        for c in children {
            c.paint(ctx);
        }
    }
}
*/
