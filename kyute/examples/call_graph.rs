use kyute_shell::platform::Platform;
use kyute::{Context, Data};

#[track_caller]
fn root() {
    Context::in_scope(0, || {
        eprintln!("root: {:?}", Context::current_call_key());
        window();
    });
}


#[track_caller]
fn window() {
    Context::in_scope(0, || {
        eprintln!("window: {:?}", Context::current_call_key());
        vbox();
        vbox();
    });
}

#[track_caller]
fn vbox() {

    Context::in_scope(0, || {
        eprintln!("vbox: {:?}", Context::current_call_key());

        // cached function call
        Context::cache((), |_| button());
        Context::cache((), |_| button());
    });
}

#[derive(Copy, Clone, Debug)]
struct Button;

impl Data for Button {
    fn same(&self, other: &Self) -> bool {
        true
    }
}

#[track_caller]
fn button() -> Button {
    // a state entry is created within Context::cache, so this will be added as a dependency of the cache entry
    let hovered = Context::cache((), |_| false);
    Context::dump();
    Button
}


fn main() {
    let platform = Platform::new();

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        //.with_level(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    root();
}
