use crate::{
    google::LogSeverity,
    serializers::{SerializableContext, SerializableSpan, SourceLocation},
    visitor::Visitor,
    writer::WriteAdaptor,
};
use serde::ser::{SerializeMap, Serializer as _};
use std::fmt;
use std::fmt::Debug;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_core::field::Value;
use tracing_core::field::Visit;
use tracing_core::span::{Attributes, Record};
use tracing_core::{Event, Field, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::registry::SpanRef;
use tracing_subscriber::{
    field::VisitOutput,
    fmt::{
        format::{self, JsonFields},
        FmtContext, FormatEvent,
    },
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

/// Tracing Event formatter for Stackdriver layers
pub struct EventFormatter {
    pub(crate) include_source_location: bool,
    #[cfg(feature = "opentelemetry")]
    pub(crate) cloud_trace_configuration: Option<crate::CloudTraceConfiguration>,
}

impl EventFormatter {
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

        // FIXME: derive an accurate entry count ahead of time
        let mut map = serializer.serialize_map(None)?;

        // serialize custom fields
        map.serialize_entry("time", &time)?;
        map.serialize_entry("target", &meta.target())?;

        if self.include_source_location {
            if let Some(file) = meta.file() {
                map.serialize_entry(
                    "logging.googleapis.com/sourceLocation",
                    &SourceLocation {
                        file,
                        line: meta.line(),
                    },
                )?;
            }
        }

        // serialize the current span and its leaves
        if let Some(span) = span {
            map.serialize_entry("span", &SerializableSpan::new(&span))?;
            //map.serialize_entry("spans", &SerializableContext::new(context))?;
            let mut trace_id = TraceIdVisitor { trace_id: None };
            if let None = trace_id.trace_id {
                context
                    .visit_spans(|span| {
                        for field in span.fields() {
                            if field.name() == "trace_id" {
                                let extensions = span.extensions();
                                if let Some(json_fields) = extensions
                                    .get::<tracing_subscriber::fmt::FormattedFields<
                                    tracing_subscriber::fmt::format::JsonFields,
                                >>() {
                                    json_fields.record(&field, &mut trace_id);
                                }
                            }
                        }
                        Ok::<(), Box<dyn std::error::Error>>(())
                    })
                    .expect("ERROR visiting_spans");
            }

            if let Some(trace_id) = trace_id.trace_id {
                map.serialize_entry("traceId", &trace_id)?;
            }

            #[cfg(feature = "opentelemetry")]
            if let (Some(crate::CloudTraceConfiguration { project_id }), Some(otel_data)) = (
                self.cloud_trace_configuration.as_ref(),
                span.extensions().get::<tracing_opentelemetry::OtelData>(),
            ) {
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

                if let Some(trace_id) = trace_id {
                    map.serialize_entry(
                        "logging.googleapis.com/trace",
                        &format!("projects/{project_id}/traces/{trace_id}",),
                    )?;
                }

                if trace_sampled {
                    map.serialize_entry("logging.googleapis.com/trace_sampled", &true)?;
                }
            }
        }

        // serialize the stackdriver-specific fields with a visitor
        let mut visitor = Visitor::new(severity, map);
        event.record(&mut visitor);
        visitor.finish().map_err(Error::from)?;
        Ok(())
    }
}

/// A custom visitor that looks for the `trace_id` field and store its value.
struct TraceIdVisitor {
    trace_id: Option<String>,
}
impl TraceIdVisitor {
    fn new() -> Self {
        TraceIdVisitor { trace_id: None }
    }
}

impl Visit for TraceIdVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "trace_id" {
            // `trace_id` can be a json serialized string
            // -- if so, we unpack it
            let value = value
                .split(":")
                .skip(1)
                .map(|quoted| &quoted[1..quoted.len() - 2])
                .find(|_| true)
                .unwrap_or(value);

            self.trace_id = Some(value.to_string());
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {}
}

impl<S> FormatEvent<S, JsonFields> for EventFormatter
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

impl Default for EventFormatter {
    fn default() -> Self {
        Self {
            include_source_location: true,
            #[cfg(feature = "opentelemetry")]
            cloud_trace_configuration: None,
        }
    }
}
