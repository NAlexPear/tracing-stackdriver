use crate::{google::LogSeverity, visitor::StackdriverEventVisitor, writer::WriteAdaptor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer as _};
use serde_json::Value;
use std::fmt;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_core::{Event, Subscriber};
use tracing_subscriber::{
    field::VisitOutput,
    fmt::{
        format::{self, JsonFields},
        FmtContext, FormatEvent, FormattedFields,
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

/// A tracing adapater for stackdriver
pub struct Stackdriver;

impl Stackdriver {
    /// Create a Layer that uses the Stackdriver format
    pub fn layer<S>() -> tracing_subscriber::fmt::Layer<S, JsonFields, Stackdriver>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::layer()
            .json()
            .event_format(Stackdriver)
    }

    /// Internal event formatting for a given serializer
    // FIXME: respect more Layer configuration options where relevant
    fn format_event<S>(
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
        }

        // serialize the stackdriver-specific fields with a visitor
        let mut visitor = StackdriverEventVisitor::new(severity, map);
        event.record(&mut visitor);
        visitor.finish().map_err(Error::from)?;
        Ok(())
    }
}

impl<S> FormatEvent<S, JsonFields> for Stackdriver
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
        Stackdriver::format_event(context, serializer, event)?;
        writeln!(writer)
    }
}
