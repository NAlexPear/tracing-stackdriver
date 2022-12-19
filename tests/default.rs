#![allow(clippy::disallowed_names)]
use lazy_static::lazy_static;
use mocks::{MockDefaultEvent, MockEventWithSpan, MockHttpEvent, MockHttpRequest};
use serde::Deserialize;
use std::sync::Mutex;
use time::OffsetDateTime;
use tracing_stackdriver::LogSeverity;
use tracing_subscriber::{layer::SubscriberExt, Registry};

mod helpers;
mod mocks;
mod writer;

#[test]
fn includes_span() {
    let output = serde_json::from_slice::<MockEventWithSpan>(run_with_tracing!(|| {
        let span = tracing::info_span!("stackdriver_span", foo = "bar");
        let _guard = span.enter();
        tracing::info!("some stackdriver message");
    }))
    .expect("Error converting test buffer to JSON");

    assert_eq!(output.span.name, "stackdriver_span");
    assert_eq!(output.span.foo, "bar");
}

#[test]
fn includes_correct_custom_fields() {
    let start = OffsetDateTime::now_utc();

    let output = serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(
        || tracing::info!(target: "test target", "some stackdriver message")
    ))
    .expect("Error converting test buffer to JSON");

    assert!(output.time > start);
    assert_eq!(output.target, "test target");
    assert_eq!(output.severity, "INFO");
}

#[test]
fn handles_stringly_severity_override() {
    let output = serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(|| tracing::info!(
        severity = "notice",
        "notice me, senpai!"
    )))
    .expect("Error converting test buffer to JSON");

    assert_eq!(output.severity, "NOTICE");
}

#[test]
fn handles_enum_severity_override() {
    let output = serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(|| tracing::info!(
        severity = %LogSeverity::Notice,
        "notice me, senpai!"
    )))
    .expect("Error converting test buffer to JSON");

    assert_eq!(output.severity, "NOTICE");
}

#[test]
fn includes_correct_timestamps() {
    let output = run_with_tracing!(|| {
        let span = tracing::info_span!("test span", foo = "bar");
        let _guard = span.enter();
        tracing::info!(target: "first target", "some stackdriver message");
        tracing::info!(target: "second target", "some stackdriver message");
    });

    let mut events = serde_json::Deserializer::from_slice(output).into_iter::<MockDefaultEvent>();

    let first_event = events
        .next()
        .expect("Error logging first event")
        .expect("Error converting test buffer to JSON");

    let second_event = events
        .next()
        .expect("Error logging second event")
        .expect("Error converting test buffer to JSON");

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

    let output = serde_json::from_slice::<MockEventWithFields>(run_with_tracing!(
        || tracing::info!(baz, "some stackdriver message")
    ))
    .expect("Error converting first test buffer to JSON");

    assert_eq!(&output.baz, &baz);
    assert_eq!(&output.message, "some stackdriver message");
}

#[test]
fn nests_http_request() {
    let request_method = "GET";
    let latency = "0.23s";
    let remote_ip = "192.168.1.1";
    let status = 200;

    let mock_http_request = MockHttpRequest {
        request_method: request_method.to_string(),
        latency: latency.to_string(),
        remote_ip: remote_ip.to_string(),
        status,
    };

    let output = serde_json::from_slice::<MockHttpEvent>(run_with_tracing!(|| tracing::info!(
        http_request.request_method = &request_method,
        http_request.latency = &latency,
        http_request.remote_ip = &remote_ip,
        http_request.status = &status,
        "some stackdriver message"
    )))
    .expect("Error converting test buffer to JSON");

    assert_eq!(&output.http_request, &mock_http_request);
}

#[test]
fn includes_source_location() {
    let output =
        serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(|| tracing::info!("hello!")))
            .expect("Error converting test buffer to JSON");
    assert!(output.source_location.file.ends_with("default.rs"));
    assert!(!output.source_location.line.is_empty());
    assert!(output.source_location.line != "0");
}
