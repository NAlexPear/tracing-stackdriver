#![allow(clippy::disallowed_names)]
use std::collections::BTreeMap;

use helpers::run_with_tracing;
use mocks::{MockDefaultEvent, MockEventWithSpan};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing_stackdriver::LogSeverity;

mod helpers;
mod mocks;

#[test]
fn includes_span() {
    let events = run_with_tracing::<MockEventWithSpan>(|| {
        let span = tracing::info_span!("stackdriver_span", foo = "bar");
        let _guard = span.enter();
        tracing::info!("some stackdriver message");
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.span.name, "stackdriver_span");
    assert_eq!(event.span.foo, "bar");
}

#[test]
fn includes_correct_custom_fields() {
    let start = OffsetDateTime::now_utc();

    let events = run_with_tracing::<MockDefaultEvent>(
        || tracing::info!(target: "test target", "some stackdriver message"),
    )
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert!(event.time > start);
    assert_eq!(event.target, "test target");
    assert_eq!(event.severity, "INFO");
}

#[test]
fn includes_custom_fields_with_dot() {
    let events = run_with_tracing::<BTreeMap<String, serde_json::Value>>(|| {
        tracing::info!(foo.bar = "value", "message")
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(
        event.get("fooBar"),
        Some(&serde_json::json!("value")),
        "full event: {:?}",
        event
    );
}

#[test]
fn handles_stringly_severity_override() {
    let events = run_with_tracing::<MockDefaultEvent>(|| {
        tracing::info!(severity = "notice", "notice me, senpai!")
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.severity, "NOTICE");
}

#[test]
fn handles_enum_severity_override() {
    let events = run_with_tracing::<MockDefaultEvent>(|| {
        tracing::info!(
            severity = %LogSeverity::Notice,
            "notice me, senpai!"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.severity, "NOTICE");
}

#[test]
fn includes_correct_timestamps() {
    let mut events = run_with_tracing::<MockDefaultEvent>(|| {
        let span = tracing::info_span!("test span", foo = "bar");
        let _guard = span.enter();
        tracing::info!(target: "first target", "some stackdriver message");
        tracing::info!(target: "second target", "some stackdriver message");
    })
    .expect("Error converting test buffer to JSON")
    .into_iter();

    let first_event = events.next().expect("Error logging first event");
    let second_event = events.next().expect("Error logging second event");
    assert!(first_event.time < second_event.time);
}

#[derive(Deserialize)]
struct MockEventWithFields {
    message: String,
    baz: u16,
}

#[test]
fn includes_flattened_fields() {
    let baz = 123;

    let events =
        run_with_tracing::<MockEventWithFields>(|| tracing::info!(baz, "some stackdriver message"))
            .expect("Error converting first test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.baz, baz);
    assert_eq!(event.message, "some stackdriver message");
}
