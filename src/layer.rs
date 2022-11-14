use crate::event_formatter::EventFormatter;
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
    Layer(
        tracing_subscriber::fmt::layer()
            .json()
            .event_format(EventFormatter::default()),
    )
}

/// A tracing-compatible Layer implementation for Stackdriver
pub struct Layer<S, W = fn() -> io::Stdout>(
    tracing_subscriber::fmt::Layer<S, JsonFields, EventFormatter, W>,
)
where
    S: Subscriber + for<'span> LookupSpan<'span>;

impl<S, W> Layer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    // TODO: support additional tracing_subscriber::fmt::Layer configuration methods as they make sense for this context

    /// Sets the MakeWriter that the Layer being built will use to write events.
    pub fn with_writer<M>(self, make_writer: M) -> Layer<S, M>
    where
        M: for<'writer> MakeWriter<'writer> + 'static,
    {
        Layer(self.0.with_writer(make_writer))
    }

    /// Enable Cloud Trace integration with OpenTelemetry through special LogEntry fields
    #[cfg_attr(docsrs, doc(cfg(feature = "opentelemetry")))]
    #[cfg(any(docsrs, feature = "opentelemetry"))]
    pub fn enable_cloud_trace(self, configuration: crate::CloudTraceConfiguration) -> Self {
        Self(self.0.event_format(EventFormatter {
            cloud_trace_configuration: Some(configuration),
        }))
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
        self.0.on_new_span(attrs, id, context)
    }

    fn on_record(
        &self,
        span: &tracing_core::span::Id,
        values: &tracing_core::span::Record<'_>,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.0.on_record(span, values, context)
    }

    fn on_enter(
        &self,
        id: &tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.0.on_enter(id, context)
    }

    fn on_exit(
        &self,
        id: &tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.0.on_exit(id, context)
    }

    fn on_close(
        &self,
        id: tracing_core::span::Id,
        context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.0.on_close(id, context)
    }

    fn on_event(&self, event: &Event<'_>, context: tracing_subscriber::layer::Context<'_, S>) {
        self.0.on_event(event, context)
    }

    unsafe fn downcast_raw(&self, id: std::any::TypeId) -> Option<*const ()> {
        self.0.downcast_raw(id)
    }
}

impl<S, W> Deref for Layer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    type Target = tracing_subscriber::fmt::Layer<S, JsonFields, EventFormatter, W>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
