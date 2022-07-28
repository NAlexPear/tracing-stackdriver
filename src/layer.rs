use crate::{google::LogSeverity, visitor::StackdriverEventVisitor, writer::WriteAdaptor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer as _};
use serde_json::Value;
use std::{fmt, io, ops::Deref};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_core::{Event, Subscriber};
use tracing_subscriber::{
    field::VisitOutput,
    fmt::{
        format::{self, JsonFields},
        FmtContext, FormatEvent, FormattedFields, MakeWriter,
    },
    registry::{LookupSpan, SpanRef},
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

/// Serializable tracing span for nesting formatted event fields
struct SerializableSpan<'a, 'b, S>(&'b SpanRef<'a, S>)
where
    S: for<'lookup> LookupSpan<'lookup>;

impl<'a, 'b, S> Serialize for SerializableSpan<'a, 'b, S>
where
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn serialize<R>(&self, serializer: R) -> Result<R::Ok, R::Error>
    where
        R: serde::Serializer,
    {
        let name = self.0.name();
        let extensions = self.0.extensions();

        let formatted_fields = extensions
            .get::<FormattedFields<JsonFields>>()
            .expect("No fields!");

        let span_length = formatted_fields.fields.len() + 1;
        let mut map = serializer.serialize_map(Some(span_length))?;

        match serde_json::from_str::<Value>(formatted_fields) {
            // handle string escaping "properly" (this should be fixed upstream)
            // https://github.com/tokio-rs/tracing/issues/391
            Ok(Value::Object(fields)) => {
                for (key, value) in fields {
                    map.serialize_entry(&key, &value)?;
                }
            }
            // these two options should be impossible
            Ok(value) => panic!("Invalid value: {}", value),
            Err(error) => panic!("Error parsing logs: {}", error),
        };

        map.serialize_entry("name", &name)?;
        map.end()
    }
}

struct SerializableContext<'a, 'b, S>(&'b FmtContext<'a, S, JsonFields>)
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>;

impl<'a, 'b, S> Serialize for SerializableContext<'a, 'b, S>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn serialize<R>(&self, serializer: R) -> Result<R::Ok, R::Error>
    where
        R: serde::Serializer,
    {
        let mut list = serializer.serialize_seq(None)?;

        if let Some(leaf_span) = self.0.lookup_current() {
            for span in leaf_span.scope().from_root() {
                list.serialize_element(&SerializableSpan(&span))?;
            }
        }

        list.end()
    }
}

/// Create a configurable stackdriver-specific Layer and event formatter
pub fn layer<S>() -> StackdriverLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    StackdriverLayer(
        tracing_subscriber::fmt::layer()
            .json()
            .event_format(StackdriverEventFormatter::default()),
    )
}

/// A tracing adapater for stackdriver
// FIXME: deprecate this struct-as-namespace for version 0.7.0
pub struct Stackdriver;

impl Stackdriver {
    /// Create a Layer that uses the Stackdriver format
    pub fn layer<S>() -> StackdriverLayer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        layer()
    }
}

/// A tracing-compatible Layer implementation for Stackdriver
pub struct StackdriverLayer<S, W = fn() -> io::Stdout>(
    tracing_subscriber::fmt::Layer<S, JsonFields, StackdriverEventFormatter, W>,
)
where
    S: Subscriber + for<'span> LookupSpan<'span>;

impl<S, W> StackdriverLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    // TODO: support additional Layer configuration methods as they make sense for this context

    /// Sets the MakeWriter that the Layer being built will use to write events.
    pub fn with_writer<M>(self, make_writer: M) -> StackdriverLayer<S, M>
    where
        M: for<'writer> MakeWriter<'writer> + 'static,
    {
        StackdriverLayer(self.0.with_writer(make_writer))
    }

    /// Set the Google Cloud Project ID for use with Cloud Trace
    // FIXME: set as an entire configuration option for cloud trace if there are no other uses for
    // project_id
    pub fn with_project_id(self, project_id: String) -> Self {
        Self(self.0.event_format(StackdriverEventFormatter {
            project_id: Some(project_id),
        }))
    }
}

/// Layer trait implementation that delegates to the inner Layer methods
impl<S, W> tracing_subscriber::layer::Layer<S> for StackdriverLayer<S, W>
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

impl<S, W> Deref for StackdriverLayer<S, W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    type Target = tracing_subscriber::fmt::Layer<S, JsonFields, StackdriverEventFormatter, W>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Tracing Event formatter for Stackdriver layers
#[derive(Default)]
pub struct StackdriverEventFormatter {
    project_id: Option<String>,
}

impl StackdriverEventFormatter {
    /// Internal event formatting for a given serializer
    fn format_event<S>(
        &self,
        context: &FmtContext<S, JsonFields>,
        mut serializer: serde_json::Serializer<WriteAdaptor>,
        event: &Event,
    ) -> Result<(), Error>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let time = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let meta = event.metadata();
        let severity = LogSeverity::from(meta.level());

        let span = event
            .parent()
            .and_then(|id| context.span(id))
            .or_else(|| context.lookup_current());

        let entry_count = 3 + {
            if span.is_some() {
                2
            } else {
                0
            }
        };

        let mut map = serializer.serialize_map(Some(entry_count))?;

        // serialize custom fields
        map.serialize_entry("time", &time)?;
        map.serialize_entry("target", &meta.target())?;

        // serialize the current span and its leaves
        if let Some(span) = span {
            map.serialize_entry("span", &SerializableSpan(&span))?;
            map.serialize_entry("spans", &SerializableContext(context))?;

            #[cfg(feature = "opentelemetry")]
            if let Some(otel_data) = span.extensions().get::<tracing_opentelemetry::OtelData>() {
                use opentelemetry::trace::TraceContextExt;
                let builder = &otel_data.builder;

                if let Some(span_id) = builder.span_id {
                    map.serialize_entry("logging.googleapis.com/spanId", &span_id.to_string())?;
                }

                let (trace_id, trace_sampled) = if otel_data.parent_cx.has_active_span() {
                    let span_ref = otel_data.parent_cx.span();
                    let span_context = span_ref.span_context();
                    (Some(span_context.trace_id()), span_context.is_sampled())
                } else {
                    (builder.trace_id, false)
                };

                if let (Some(trace_id), Some(project_id)) = (trace_id, self.project_id.as_ref()) {
                    map.serialize_entry(
                        "logging.googleapis.com/trace",
                        &format!("projects/{project_id}/traces/{trace_id}"),
                    )?;
                }
                if trace_sampled {
                    map.serialize_entry("logging.googleapis.com/trace_sampled", &true)?;
                }
            }
        }

        // serialize the stackdriver-specific fields with a visitor
        let mut visitor = StackdriverEventVisitor::new(severity, map);
        event.record(&mut visitor);
        visitor.finish().map_err(Error::from)?;
        Ok(())
    }
}

impl<S> FormatEvent<S, JsonFields> for StackdriverEventFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn format_event(
        &self,
        context: &FmtContext<S, JsonFields>,
        mut writer: format::Writer,
        event: &Event,
    ) -> fmt::Result
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let serializer = serde_json::Serializer::new(WriteAdaptor::new(&mut writer));
        self.format_event(context, serializer, event)?;
        writeln!(writer)
    }
}
