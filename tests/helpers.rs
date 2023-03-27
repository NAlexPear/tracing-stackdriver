#![allow(dead_code)]
use serde::Deserialize;
use std::{
    io,
    sync::{Arc, Mutex, TryLockError},
};
use tracing_stackdriver::Layer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

/// Run a traced callback against the default Layer configuration,
/// deserializing into a collection of a single event type `E`. For deserializing events
/// of more than a single type, use `serde_json::Map`.
pub fn run_with_tracing<E>(callback: impl FnOnce() -> ()) -> serde_json::Result<Vec<E>>
where
    E: for<'a> Deserialize<'a>,
{
    run_with_tracing_layer(tracing_stackdriver::layer(), callback)
}

/// Run a traced callback against a Layer configuration
// FIXME: handle composable layers (a la "with") in run_with_tracing functions
pub fn run_with_tracing_layer<E>(
    layer: Layer<Registry>,
    callback: impl FnOnce() -> (),
) -> serde_json::Result<Vec<E>>
where
    E: for<'a> Deserialize<'a>,
{
    let buffer = Arc::new(Mutex::new(vec![]));
    let shared = buffer.clone();
    let make_writer = move || MockWriter(shared.clone());
    let stackdriver: Layer<Registry, _> = layer.with_writer(make_writer);
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::with_default(subscriber, callback);

    let buffer = buffer
        .lock()
        .expect("Couldn't get lock on test write target");

    serde_json::Deserializer::from_slice(&buffer)
        .into_iter()
        .collect()
}

// FIXME: make this entirely internal
#[derive(Debug)]
pub struct MockWriter(pub Arc<Mutex<Vec<u8>>>);

impl MockWriter {
    pub fn map_err<G>(error: TryLockError<G>) -> io::Error {
        match error {
            TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
            TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
        }
    }
}

impl io::Write for MockWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.try_lock().map_err(Self::map_err)?.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.try_lock().map_err(Self::map_err)?.flush()
    }
}
