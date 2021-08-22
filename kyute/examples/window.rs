use kyute::{
    application, get_default_application_style, theme, widget::ButtonResult, CompositionCtx,
    Environment,
};
use kyute_shell::{drawing::Color, platform::Platform};

fn gui(cx: &mut CompositionCtx) {
    use kyute::widget as w;

    cx.with_environment(get_default_application_style(), |cx| {
        cx.with_state(
            || 0.0,
            |cx, counter| {
                w::window(cx, "Kyute main window", |cx| {
                    w::vbox(cx, |cx| {
                        w::button(cx, &format!("click me: {}", counter));
                        w::button(cx, &format!("click me again: {}", counter))
                            .on_click(|| *counter += 42.0);
                        w::slider(cx, 0.0, 100.0, *counter).on_value_change(|x| *counter = x);

                        cx.with_state(String::new, |cx, str| {
                            w::text_line_edit(cx, str)
                                .on_text_changed(|new_str| *str = new_str.to_string());
                            w::text(cx, str);
                        });
                    });
                });
            },
        );
    });
}

fn main() {
    Platform::init();
    tracing_subscriber::fmt()
        .pretty()
        .with_target(false)
        //.with_level(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();
    application::run(gui);
}
