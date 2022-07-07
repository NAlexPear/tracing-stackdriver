use crate::{
    google::LogSeverity,
    visitor::{StackdriverEventVisitor, StackdriverVisitor},
};
use serde::ser::{SerializeMap, Serializer as _};
use serde_json::Value;
use std::{
    fmt::{self, Write},
    io,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_core::{
    span::{Attributes, Id},
    Event, Subscriber,
};
use tracing_subscriber::{
    field::{MakeVisitor, RecordFields, VisitOutput},
    fmt::{
        format::{JsonFields, Writer},
        FormatFields, FormattedFields, MakeWriter,
    },
    layer::Context,
    registry::LookupSpan,
    Layer,
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Formatting error")]
    Formatting(#[from] fmt::Error),
    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Time formatting error: {0}")]
    Time(#[from] time::error::Format),
}

/// A tracing adapater for stackdriver
pub struct Stackdriver<W = fn() -> io::Stdout> {
    writer: W,
    fields: StackdriverFields,
}

impl Stackdriver {
    /// Initialize the Stackdriver Layer with the default writer (std::io::Stdout)
    pub fn new() -> Self {
        Self::default()
    }
}

impl<W> Stackdriver<W>
where
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    /// Initialize the Stackdriver Layer with a custom writer
    pub fn with_writer(writer: W) -> Self {
        Self {
            writer,
            fields: StackdriverFields::default(),
        }
    }

    fn visit<S>(&self, event: &Event, context: Context<S>) -> Result<(), Error>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let time = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let mut buffer: Vec<u8> = Default::default();
        let meta = event.metadata();
        let severity = LogSeverity::from(meta.level());
        let mut serializer = serde_json::Serializer::new(&mut buffer);
        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("time", &time)?;
        map.serialize_entry("target", &meta.target())?;

        if let Some(span) = context.lookup_current() {
            let name = &span.name();
            let extensions = span.extensions();
            let formatted_fields = extensions
                .get::<FormattedFields<StackdriverFields>>()
                .expect("No fields!");

            // TODO: include serializable data type in extensions instead of str
            let mut fields: Value = serde_json::from_str(formatted_fields)?;

            fields["name"] = serde_json::json!(name);

            map.serialize_entry("span", &fields)?;
        }

        // TODO: enable deeper structuring of keys and values across tracing
        // https://github.com/tokio-rs/tracing/issues/663
        // https://github.com/tokio-rs/tracing/discussions/1906
        let mut visitor = StackdriverEventVisitor::new(severity, map);

        event.record(&mut visitor);

        visitor.finish().map_err(Error::from)?;

        use std::io::Write;
        let mut writer = self.writer.make_writer();
        buffer.write_all(b"\n")?;
        writer.write_all(&buffer)?;
        Ok(())
    }
}

impl Default for Stackdriver {
    fn default() -> Self {
        Self {
            writer: std::io::stdout,
            fields: StackdriverFields::default(),
        }
    }
}

impl<S, W> Layer<S> for Stackdriver<W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + 'static,
{
    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        let span = context.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions
            .get_mut::<FormattedFields<StackdriverFields>>()
            .is_none()
        {
            let mut fields = FormattedFields::<StackdriverFields>::new(String::new());

            if self
                .fields
                .format_fields(fields.as_writer(), attributes)
                .is_ok()
            {
                extensions.insert(fields);
            }
        }
    }

    #[allow(unused_variables)]
    fn on_event(&self, event: &Event, context: Context<S>) {
        if let Err(error) = self.visit(event, context) {
            #[cfg(test)]
            eprintln!("{}", &error)
        }
    }
}

#[derive(Default)]
struct StackdriverFields {
    json_fields: JsonFields,
}

impl<'writer> FormatFields<'writer> for StackdriverFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        self.json_fields.format_fields(writer, fields)
    }
}

impl<'a> MakeVisitor<&'a mut dyn Write> for StackdriverFields {
    type Visitor = StackdriverVisitor<'a>;

    #[inline]
    fn make_visitor(&self, target: &'a mut dyn Write) -> Self::Visitor {
        StackdriverVisitor::new(target)
    }
}
