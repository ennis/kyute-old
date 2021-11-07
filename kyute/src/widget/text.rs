//! Text elements
use crate::{
    composable, env::Environment, event::Event, BoxConstraints, EventCtx, LayoutCtx, LayoutItem,
    Measurements, PaintCtx, Point, Rect, Widget, WidgetPod,
};
use kyute_shell::{
    drawing::{Brush, Color, DrawTextOptions},
    text::{TextFormatBuilder, TextLayout},
};
use std::cell::RefCell;

#[derive(Clone)]
pub struct Text {
    text: String,
    text_layout: RefCell<Option<TextLayout>>,
}

impl Text {
    #[composable]
    pub fn new(text: String) -> WidgetPod<Text> {
        WidgetPod::new(Text {
            text,
            text_layout: RefCell::new(None),
        })
    }
}

impl Widget for Text {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(&self, _ctx: &mut EventCtx, _event: &Event) {}

    fn layout(
        &self,
        _ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        _env: &Environment,
    ) -> Measurements {
        let font_name = "Consolas";
        let font_size = 12;
        let text_format = TextFormatBuilder::new()
            .size(font_size as f32)
            .family(font_name)
            .build()
            .unwrap();

        let text_layout = TextLayout::new(&self.text, &text_format, constraints.biggest()).unwrap();

        // round size to nearest device pixel
        let size = text_layout.metrics().bounds.size.ceil();
        let baseline = text_layout
            .line_metrics()
            .first()
            .map(|m| m.baseline as f64);

        self.text_layout.replace(Some(text_layout));
        Measurements { size, baseline }
    }

    fn paint(&self, ctx: &mut PaintCtx, _bounds: Rect, _env: &Environment) {
        let text_brush = Brush::solid_color(ctx, Color::new(0.92, 0.92, 0.92, 1.0));

        let text_layout = self.text_layout.borrow();
        if let Some(ref text_layout) = &*text_layout {
            ctx.draw_text_layout(
                Point::origin(),
                text_layout,
                &text_brush,
                DrawTextOptions::empty(),
            )
        } else {
            tracing::warn!("text layout wasn't calculated before paint")
        }
    }
}
