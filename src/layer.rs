use crate::event_formatter::EventFormatter;
#[cfg(feature = "opentelemetry")]
use std::sync::{Arc, OnceLock};
use std::{fmt, io, ops::Deref};
use tracing_core::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{format::JsonFields, MakeWriter},
    registry::LookupSpan,
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Formatting(#[from] fmt::Error),
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Time formatting error: {0}")]
    Time(#[from] time::error::Format),
}

impl From<Error> for fmt::Error {
    fn from(_: Error) -> Self {
        Self
    }
}

/// Create a configurable stackdriver-specific Layer and event formatter
pub fn layer<S>() -> Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    #[cfg(feature = "opentelemetry")]
    let dispatch = Arc::new(OnceLock::new());
    let formatter = EventFormatter {
        #[cfg(feature = "opentelemetry")]
        dispatch: dispatch.clone(),
        ..EventFormatter::default()
    };
    Layer {
        inner: tracing_subscriber::fmt::layer()
            .json()
            .event_format(formatter),
        #[cfg(feature = "opentelemetry")]
        dispatch,
    }
}

/// A tracing-compatible Layer implementation for Stackdriver
pub struct Layer<S, W = fn() -> io::Stdout>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    inner: tracing_subscriber::fmt::Layer<S, JsonFields, EventFormatter, W>,
    #[cfg(feature = "opentelemetry")]
    dispatch: Arc<OnceLock<tracing_core::dispatcher::WeakDispatch>>,
}

impl<S, W> Layer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    /// Sets the MakeWriter that the Layer being built will use to write events.
    pub fn with_writer<M>(self, make_writer: M) -> Layer<S, M>
    where
        M: for<'writer> MakeWriter<'writer> + 'static,
    {
        Layer {
            inner: self.inner.with_writer(make_writer),
            #[cfg(feature = "opentelemetry")]
            dispatch: self.dispatch,
        }
    }

    /// Configures whether or not Events will include source locations in a special LogEntry field
    pub fn with_source_location(self, include_source_location: bool) -> Self {
        Self {
            inner: self.inner.map_event_format(|mut event_formatter| {
                event_formatter.include_source_location = include_source_location;
                event_formatter
            }),
            #[cfg(feature = "opentelemetry")]
            dispatch: self.dispatch,
        }
    }

    /// Configures the Cloud Trace integration with OpenTelemetry through special LogEntry fields
    #[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
    #[cfg(any(docsrs, feature = "opentelemetry"))]
    pub fn with_cloud_trace(self, configuration: crate::CloudTraceConfiguration) -> Self {
        Self {
            inner: self.inner.map_event_format(|mut event_formatter| {
                event_formatter.cloud_trace_configuration = Some(configuration);
                event_formatter
            }),
            dispatch: self.dispatch,
        }
    }
}

/// Layer trait implementation that delegates to the inner Layer methods
impl<S, W> tracing_subscriber::layer::Layer<S> for Layer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(
        &self,
        attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_new_span(attrs, id, context)
    }

    fn on_record(
        &self,
        span: &tracing_core::span::Id,
        values: &tracing_core::span::Record<'_>,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_record(span, values, context)
    }

    fn on_enter(
        &self,
        id: &tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_enter(id, context)
    }

    fn on_exit(
        &self,
        id: &tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_exit(id, context)
    }

    fn on_close(
        &self,
        id: tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_close(id, context)
    }

    fn on_event(&self, event: &Event<'_>, context: tracing_subscriber::layer::Context<'_, S>) {
        self.inner.on_event(event, context)
    }

    fn on_register_dispatch(&self, subscriber: &tracing_core::Dispatch) {
        #[cfg(feature = "opentelemetry")]
        let _ = self.dispatch.set(subscriber.downgrade());
        self.inner.on_register_dispatch(subscriber)
    }

    unsafe fn downcast_raw(&self, id: std::any::TypeId) -> Option<*const ()> {
        self.inner.downcast_raw(id)
    }
}

impl<S, W> Deref for Layer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    type Target = tracing_subscriber::fmt::Layer<S, JsonFields, EventFormatter, W>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
