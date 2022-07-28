#![allow(clippy::blacklisted_name)]
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{fmt::Debug, sync::Mutex};
use time::OffsetDateTime;
use tracing_stackdriver::{LogSeverity, Stackdriver};
use tracing_subscriber::{layer::SubscriberExt, Registry};
#[cfg(all(tracing_unstable, feature = "valuable"))]
use valuable::Valuable;

mod common;

macro_rules! run_with_tracing {
    (|| $expression:expr) => {{
        lazy_static! {
            static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || crate::common::MockWriter(&BUFFER);
        let stackdriver = Stackdriver::layer().with_writer(make_writer);
        let subscriber = Registry::default().with(stackdriver);

        tracing::subscriber::with_default(subscriber, || $expression);

        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec()
    }};
}

#[derive(Debug, Deserialize)]
struct MockSpan {
    name: String,
    foo: String,
}

#[derive(Debug, Deserialize)]
struct MockEventWithSpan {
    span: MockSpan,
}

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

#[derive(Clone, Deserialize)]
struct MockDefaultEvent {
    #[serde(deserialize_with = "time::serde::rfc3339::deserialize")]
    time: OffsetDateTime,
    target: String,
    severity: String,
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

#[cfg(all(tracing_unstable, feature = "valuable"))]
#[test]
fn handles_valuable_severity_override() {
    let output = serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(|| tracing::info!(
        severity = LogSeverity::Notice.as_value(),
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct MockHttpRequest {
    request_method: String,
    latency: String,
    remote_ip: String,
    status: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MockHttpEvent {
    http_request: MockHttpRequest,
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

#[cfg(all(tracing_unstable, feature = "valuable"))]
#[test]
fn validates_structured_http_requests() {
    let request_method = http::Method::GET;
    let latency = std::time::Duration::from_millis(1234);
    let status = http::StatusCode::OK;
    let remote_ip = std::net::IpAddr::from([127, 0, 0, 1]);

    let http_request = tracing_stackdriver::HttpRequest {
        request_method: Some(request_method.clone()),
        latency: Some(latency),
        status: Some(status),
        remote_ip: Some(remote_ip),
        ..Default::default()
    };

    let output = serde_json::from_slice::<MockHttpEvent>(run_with_tracing!(|| tracing::info!(
        http_request = http_request.as_value(),
        "http_request testing"
    )))
    .expect("Error converting test buffer to JSON");

    assert_eq!(
        output.http_request.request_method,
        request_method.to_string()
    );

    assert_eq!(
        output.http_request.latency,
        format!("{}s", latency.as_secs_f32())
    );

    assert_eq!(output.http_request.status, status.as_u16());
    assert_eq!(output.http_request.remote_ip, remote_ip.to_string());
}

#[cfg(all(tracing_unstable, feature = "valuable"))]
#[derive(Debug, Deserialize, Valuable, PartialEq)]
struct StructuredLog {
    foo: String,
    bar: std::collections::BTreeMap<String, u16>,
}

#[cfg(all(tracing_unstable, feature = "valuable"))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MockStructuredEvent {
    structured_log: StructuredLog,
}

#[cfg(all(tracing_unstable, feature = "valuable"))]
#[test]
fn includes_valuable_structures() {
    let foo = "testing".to_string();
    let mut bar = std::collections::BTreeMap::new();
    bar.insert("baz".into(), 123);
    let structured_log = StructuredLog { foo, bar };
    let output =
        serde_json::from_slice::<MockStructuredEvent>(run_with_tracing!(|| tracing::info!(
            structured_log = structured_log.as_value(),
            "another message"
        )))
        .expect("Error converting test buffer to JSON");

    assert_eq!(output.structured_log, structured_log);
}
