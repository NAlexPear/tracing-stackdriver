use tracing::{info, instrument, trace_span};
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;
use tracing_stackdriver_cw::layer;
use uuid::Uuid;

#[instrument]
fn database_function() {
    info!("This is the database function");
}

#[instrument]
fn business_logic_function() {
    info!("This is the business logic function");
    database_function();
}

#[instrument]
fn endpoint_function() {
    // `trace_id` can come from Google App Engine, via headers
    let trace_id = Uuid::new_v4().to_string();
    let span = trace_span!("endpoint_function", trace_id = %trace_id);
    let _enter = span.enter();

    info!(trace_id = %trace_id, "This is the endpoint function");
    business_logic_function();
}

fn main() {
    // Set up the subscriber.
    let stackdriver = layer(); // writes to std::io::Stdout
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    endpoint_function();

    dbg!(core::any::TypeId::of::<String>());
    dbg!(core::any::TypeId::of::<tracing_core::span::Attributes>());
    dbg!(core::any::TypeId::of::<tracing_subscriber::fmt::FormattedFields<tracing_subscriber::fmt::format::JsonFields>>());
}
