use serde::ser::{Serialize, SerializeMap, SerializeSeq};
use serde_json::Value;
use tracing_core::Subscriber;
use tracing_subscriber::{
    fmt::{format::JsonFields, FmtContext, FormattedFields},
    registry::{LookupSpan, SpanRef},
};

/// Serializable tracing span for nesting formatted event fields
pub(crate) struct SerializableSpan<'a, 'b, S>(&'b SpanRef<'a, S>)
where
    S: for<'lookup> LookupSpan<'lookup>;

impl<'a, 'b, S> SerializableSpan<'a, 'b, S>
where
    S: for<'lookup> LookupSpan<'lookup>,
{
    pub(crate) fn new(span: &'b SpanRef<'a, S>) -> Self {
        Self(span)
    }
}

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

/// Serializable tracing context for serializing a collection of spans
pub(crate) struct SerializableContext<'a, 'b, S>(&'b FmtContext<'a, S, JsonFields>)
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>;

impl<'a, 'b, S> SerializableContext<'a, 'b, S>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    #[allow(dead_code)]
    pub(crate) fn new(context: &'b FmtContext<'a, S, JsonFields>) -> Self {
        Self(context)
    }
}

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
                list.serialize_element(&SerializableSpan::new(&span))?;
            }
        }

        list.end()
    }
}

pub(crate) struct SourceLocation<'a> {
    pub(crate) file: &'a str,
    pub(crate) line: Option<u32>,
}

impl<'a> Serialize for SourceLocation<'a> {
    fn serialize<R>(&self, serializer: R) -> Result<R::Ok, R::Error>
    where
        R: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(if self.line.is_some() { 2 } else { 1 }))?;
        map.serialize_entry("file", self.file)?;
        if let Some(line) = self.line {
            // Stackdriver expects the line number to be serialised as a string:
            // https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogEntrySourceLocation
            map.serialize_entry("line", &line.to_string())?;
        }
        map.end()
    }
}
