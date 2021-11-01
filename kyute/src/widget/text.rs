//! Text elements
use crate::{env::Environment, event::{Event, LifecycleEvent}, BoxConstraints, DynLens, EventCtx, LayoutCtx, Lens, Measurements, Model, PaintCtx, Point, Rect, Widget, UpdateCtx};
use kyute_shell::{
    drawing::{Brush, Color, DrawTextOptions},
    text::{TextFormatBuilder, TextLayout},
};

pub struct Text<T: Model> {
    text: DynLens<T, String>,
    text_layout: Option<TextLayout>,
}

impl<T: Model> Text<T> {
    pub fn new() -> Text<T> {
        Text {
            text: Box::new(|| String::new()),
            text_layout: None,
        }
    }
}

/// FIXME: doesn't need to be string.
impl<T: Model> Widget<T> for Text<T> {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(
        &mut self,
        _ctx: &mut EventCtx,
        _event: &Event,
        _data: &mut T,
    ) -> Option<T::Change> {
        None
    }

    fn lifecycle(&mut self, _ctx: &mut EventCtx, _event: &LifecycleEvent, _data: &mut T) {
        // nothing
    }

    fn update(&mut self, ctx: &mut UpdateCtx, data: &mut T, change: &T::Change) {
        // nothing
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &mut T,
        _env: &Environment,
    ) -> Measurements {
        let font_name = "Consolas";
        let font_size = 12;
        let text_format = TextFormatBuilder::new()
            .size(font_size as f32)
            .family(font_name)
            .build()
            .unwrap();

        // TODO check for changes instead of re-creating from scratch every time
        let text = self.text.get(data).into_owned();
        let text_layout = TextLayout::new(&text, &text_format, constraints.biggest()).unwrap();

        // round size to nearest device pixel
        let size = text_layout.metrics().bounds.size.ceil();
        let baseline = text_layout
            .line_metrics()
            .first()
            .map(|m| m.baseline as f64);

        self.text_layout = Some(text_layout);
        Measurements { size, baseline }
    }

    fn paint(&self, ctx: &mut PaintCtx, _bounds: Rect, _data: &mut T, _env: &Environment) {
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
