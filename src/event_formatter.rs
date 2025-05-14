use crate::{
    google::LogSeverity,
    serializers::{SerializableContext, SerializableSpan, SourceLocation},
    writer::WriteAdaptor,
};
use serde::ser::{SerializeMap, Serializer as _};
use std::fmt;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_core::{field::Visit, Event, Subscriber};
use tracing_subscriber::{
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

// Helper struct to capture event fields
struct EventFieldVisitor<'a>(serde_json::Map<String, serde_json::Value>, &'a JsonFields);

impl Visit for EventFieldVisitor<'_> {
    fn record_f64(&mut self, field: &tracing_core::Field, value: f64) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap_or_else(|| {
                // tracing::debug!(target: "tracing_stackdriver::event_formatter", "f64 is not finite, using 0.0 instead");
                serde_json::Number::from(0)
            })),
        );
    }

    fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }

    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_error(
        &mut self,
        field: &tracing_core::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn fmt::Debug) {
        match field.name() {
            // Skip fields that are actually log metadata that have already been handled
            name if name.starts_with("log.") => (),
            name if name.starts_with("event.") => (),
            "message" => {
                self.0.insert(
                    "message".to_string(), // Use "message" as the key for the message field
                    serde_json::Value::String(format!("{:?}", value)),
                );
            }
            _ => {
                self.0.insert(
                    field.name().to_string(),
                    serde_json::Value::String(format!("{:?}", value)),
                );
            }
        }
    }
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

        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("severity", &severity)?;
        map.serialize_entry("time", &time)?;
        map.serialize_entry("target", meta.target())?;

        // Extract and serialize event fields
        let mut visitor = EventFieldVisitor(serde_json::Map::new(), context.field_format());
        event.record(&mut visitor);

        for (key, value) in visitor.0 {
            // The "message" field is often the primary human-readable content.
            // Google Cloud Logging expects this at the top level of jsonPayload.
            if key == "message" {
                map.serialize_entry("message", &value)?;
            } else {
                map.serialize_entry(&key, &value)?;
            }
        }

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

        if let Some(span_ref) = span {
            map.serialize_entry("span", &SerializableSpan::new(&span_ref))?;
            map.serialize_entry("spans", &SerializableContext::new(context))?;

            #[cfg(feature = "opentelemetry")]
            if let Some(config) = self.cloud_trace_configuration.as_ref() {
                // Attempt to get OpenTelemetry trace and span IDs
                let mut otel_trace_id: Option<String> = None;
                let mut otel_span_id: Option<String> = None;
                let mut otel_is_sampled: Option<bool> = None;

                // Iterate through extensions to find OtelData
                if let Some(extensions) = span_ref
                    .extensions()
                    .get::<tracing_opentelemetry::OtelData>()
                {
                    let otel_ctx = &extensions.parent_cx; // Use parent_cx as it reflects the context when span was created
                    use opentelemetry::trace::TraceContextExt;
                    if otel_ctx.has_active_span() {
                        let otel_span_ref = otel_ctx.span();
                        let otel_span_context = otel_span_ref.span_context();

                        otel_trace_id = Some(format!(
                            "projects/{}/traces/{}",
                            config.project_id,
                            otel_span_context.trace_id()
                        ));
                        otel_span_id = Some(otel_span_context.span_id().to_string());
                        otel_is_sampled = Some(otel_span_context.is_sampled());
                    }
                }

                if let Some(trace_id_val) = otel_trace_id {
                    map.serialize_entry("logging.googleapis.com/trace", &trace_id_val)?;
                }
                if let Some(span_id_val) = otel_span_id {
                    map.serialize_entry("logging.googleapis.com/spanId", &span_id_val)?;
                }
                if let Some(true) = otel_is_sampled {
                    // Only add if true
                    map.serialize_entry("logging.googleapis.com/trace_sampled", &true)?;
                }
            }
        }

        map.end()?;
        Ok(())
    }
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

