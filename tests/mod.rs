#![allow(clippy::blacklisted_name)]
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{
    io,
    sync::{Mutex, TryLockError},
};
use time::OffsetDateTime;
use tracing_stackdriver::Stackdriver;
use tracing_subscriber::{layer::SubscriberExt, Registry};

macro_rules! run_with_tracing {
    (|| $expression:expr) => {{
        lazy_static! {
            static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || MockWriter(&BUFFER);
        let stackdriver = Stackdriver::with_writer(make_writer);
        let subscriber = Registry::default().with(stackdriver);

        tracing::subscriber::with_default(subscriber, || $expression);

        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec()
    }};
}

struct MockWriter<'a>(&'a Mutex<Vec<u8>>);

impl<'a> MockWriter<'a> {
    fn map_err<G>(error: TryLockError<G>) -> io::Error {
        match error {
            TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
            TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
        }
    }
}

impl<'a> io::Write for MockWriter<'a> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.try_lock().map_err(Self::map_err)?.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.try_lock().map_err(Self::map_err)?.flush()
    }
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

    let output = serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(|| {
        let span = tracing::info_span!("test span", foo = "bar");
        let _guard = span.enter();
        tracing::info!(target: "test target", "some stackdriver message");
    }))
    .expect("Error converting test buffer to JSON");

    assert!(output.time > start);
    assert_eq!(output.target, "test target");
    assert_eq!(output.severity, "INFO");
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

    let output = serde_json::from_slice::<MockEventWithFields>(run_with_tracing!(|| {
        let span = tracing::info_span!("test span", foo = "bar");
        let _ = span.enter();
        tracing::info!(baz, "some stackdriver message");
    }))
    .expect("Error converting first test buffer to JSON");

    assert_eq!(&output.baz, &baz);
    assert_eq!(&output.message, "some stackdriver message");
}

#[derive(Deserialize)]
struct MockSpan {
    name: String,
    foo: String,
}

#[derive(Deserialize)]
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct MockHttpRequest {
    latency: String,
    remote_ip: String,
    status: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MockHttpEvent {
    http_request: MockHttpRequest,
}

#[test]
fn nests_http_request() {
    let latency = "0.23s";
    let remote_ip = "192.168.1.1";
    let status = 200;

    let mock_http_request = MockHttpRequest {
        latency: latency.to_string(),
        remote_ip: remote_ip.to_string(),
        status,
    };

    let output = serde_json::from_slice::<MockHttpEvent>(run_with_tracing!(|| {
        let span = tracing::info_span!("stackdriver_span");
        let _guard = span.enter();
        tracing::info!(
            http_request.latency = &latency,
            http_request.remote_ip = &remote_ip,
            http_request.status = &status,
            "some stackdriver message"
        );
    }))
    .expect("Error converting test buffer to JSON");

    assert_eq!(&output.http_request, &mock_http_request);
}
