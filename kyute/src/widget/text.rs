//! Text elements
use crate::{env::Environment, event::Event, BoxConstraints, EventCtx, LayoutCtx, Measurements, PaintCtx, Point, Rect, WidgetDelegate, LayoutItem};
use kyute_shell::{
    drawing::{Brush, Color, DrawTextOptions},
    text::{TextFormatBuilder, TextLayout},
};

#[derive(Clone)]
pub struct Text {
    text: String,
    text_layout: Option<TextLayout>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Text {
        Text {
            text: text.into(),
            text_layout: None,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        // don't check if the text hasn't changed: this is done during composition
        self.text = text.into();
        self.text_layout = None;
    }
}

impl WidgetDelegate for Text {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn layout(
        &self,
        _ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        _env: &Environment,
    ) -> LayoutItem {
        let font_name = "Consolas";
        let font_size = 12;
        let text_format = TextFormatBuilder::new()
            .size(font_size as f32)
            .family(font_name)
            .build()
            .unwrap();

        // TODO check for changes instead of re-creating from scratch every time
        let text_layout = TextLayout::new(&self.text, &text_format, constraints.biggest()).unwrap();

        // round size to nearest device pixel
        let size = text_layout.metrics().bounds.size.ceil();
        let baseline = text_layout
            .line_metrics()
            .first()
            .map(|m| m.baseline as f64);

        //self.text_layout = Some(text_layout);
        LayoutItem::new(Measurements { size, baseline })
    }

    fn paint(&self, ctx: &mut PaintCtx, _bounds: Rect, _env: &Environment) {
        let text_brush = Brush::solid_color(ctx, Color::new(0.92, 0.92, 0.92, 1.0));

        if let Some(ref text_layout) = self.text_layout {
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
