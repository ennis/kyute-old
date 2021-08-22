use kyute::{application, theme, CompositionCtx, Environment, get_default_application_style};
use kyute_shell::{drawing::Color, platform::Platform};
use kyute::widget::ButtonAction;

fn gui(cx: &mut CompositionCtx) {
    use kyute::widget as w;

    // equivalent with custom DSL:
    /*let counter = 0;
    Window {
        Button {
        .label = format!("click me: {}", counter)
        }
    }*/

    //let env = Environment::new().add();

    cx.with_environment(get_default_application_style(), |cx| {
        cx.with_state(
            || 0.0,
            |cx, counter| {
                // this closure will be called in a loop until the state doesn't change
                w::window(cx, "Kyute main window", |cx| {
                    w::vbox(cx, |cx| {
                        w::button(cx, &format!("click me: {}", counter));
                        if let Some(ButtonAction::Clicked) = w::button(cx, &format!("click me again: {}", counter)) {
                            *counter += 42.0;
                        }
                        w::slider(cx, 0.0, 100.0, counter);
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
