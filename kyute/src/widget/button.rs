use crate::{
    align_boxes,
    composition::CompositionCtx,
    core::{EventCtx, LayoutCtx, Widget, PaintCtx, WidgetDelegate},
    env::Environment,
    event::{Event, PointerEventKind},
    layout::{BoxConstraints, Measurements},
    widget::text::text,
    Alignment, Rect, SideOffsets, Size,
};
use kyute_shell::drawing::{Brush, Color};
use std::convert::TryFrom;
use crate::composition::ActionResult;

#[derive(Copy,Clone)]
enum ButtonAction {
    Clicked,
}

#[derive(Copy,Clone)]
pub struct ButtonResult(Option<ButtonAction>);

impl ButtonResult {
    pub fn on_click(&self, f: impl FnOnce()) {
        match self.0 {
            None => {}
            Some(ButtonAction::Clicked) => f(),
        }
    }
}

struct Button;

impl WidgetDelegate for Button {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(
        &mut self,
        ctx: &mut EventCtx,
        children: &mut [Widget],
        event: &Event,
    ) {
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
        children: &mut [Widget],
        constraints: &BoxConstraints,
        _env: &Environment
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

    fn paint(
        &mut self,
        ctx: &mut PaintCtx,
        children: &mut [Widget],
        bounds: Rect,
        _env: &Environment
    ) {
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

pub fn button(cx: &mut CompositionCtx, label: &str) -> ButtonResult {
    cx.enter(0);
    let action =
        cx.emit_node(|_| Button, |cx, button| {}, |cx| text(cx, label));
    cx.exit();
    ButtonResult(action.cast())
}
