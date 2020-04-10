use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{
    io,
    sync::{Mutex, TryLockError},
};
use tracing_stackdriver::Stackdriver;
use tracing_subscriber::{layer::SubscriberExt, Registry};

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

#[derive(Deserialize)]
struct MockDefaultEvent {
    time: DateTime<Utc>,
    target: String,
    severity: String,
}

#[test]
fn includes_correct_custom_fields() {
    lazy_static! {
        static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }

    let start = Utc::now();
    let make_writer = || MockWriter(&BUFFER);
    let stackdriver = Stackdriver::with_writer(make_writer);
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("test span", foo = "bar");
        let _ = span.enter();
        tracing::info!(target: "test target", "some stackdriver message");
    });

    let output = serde_json::from_slice::<MockDefaultEvent>(
        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec(),
    )
    .expect("Error converting test buffer to JSON");

    assert!(output.time.signed_duration_since(start) > Duration::zero());
    assert_eq!(output.target, "test target");
    assert_eq!(output.severity, "INFO");
}

#[derive(Deserialize)]
struct MockEventWithFields {
    message: String,
    baz: u16,
}

#[test]
fn includes_flattened_fields() {
    lazy_static! {
        static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }

    let baz = 123;
    let make_writer = || MockWriter(&BUFFER);
    let stackdriver = Stackdriver::with_writer(make_writer);
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("test span", foo = "bar");
        let _ = span.enter();
        tracing::info!(baz, "some stackdriver message");
    });

    let output = serde_json::from_slice::<MockEventWithFields>(
        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec(),
    )
    .expect("Error converting test buffer to JSON");

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
    lazy_static! {
        static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }

    let make_writer = || MockWriter(&BUFFER);
    let stackdriver = Stackdriver::with_writer(make_writer);
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("stackdriver_span", foo = "bar");
        let _guard = span.enter();
        tracing::info!("some stackdriver message");
    });

    let output = serde_json::from_slice::<MockEventWithSpan>(
        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec(),
    )
    .expect("Error converting test buffer to JSON");

    assert_eq!(output.span.name, "stackdriver_span");
    assert_eq!(output.span.foo, "bar");
}
