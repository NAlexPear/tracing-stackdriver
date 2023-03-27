use helpers::{run_with_tracing, run_with_tracing_layer};
use mocks::MockDefaultEvent;

mod helpers;
mod mocks;

#[test]
fn includes_source_location() {
    let events = run_with_tracing::<MockDefaultEvent>(|| tracing::info!("hello!"))
        .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.source_location.file.ends_with("source_location.rs"));
    assert!(!event.source_location.line.is_empty());
    assert!(event.source_location.line != "0");
}

#[test]
fn excludes_source_location() {
    let layer = tracing_stackdriver::layer().with_source_location(false);

    run_with_tracing_layer::<MockDefaultEvent>(layer, || tracing::info!("hello!"))
        .expect_err("Failed to exclude source location fields from events");
}
