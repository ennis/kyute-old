use crate::widget::{Widget, Node, LayoutCtx, CompositionCtx};
use crate::node::PaintCtx;
use crate::Rect;
use crate::layout::{BoxConstraints, Measurements};

pub struct Button {
    label: String,
}

impl Widget for Button {
    fn layout(&mut self, ctx: &mut LayoutCtx, children: &mut [Node], constraints: &BoxConstraints) -> Measurements {
        todo!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, children: &[Node], bounds: Rect) {
        todo!()
    }
}

#[track_caller]
fn button(cx: &mut CompositionCtx, label: &str) {
    cx.enter();
}