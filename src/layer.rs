use crate::visitor::{StackdriverEventVisitor, StackdriverVisitor};
use serde::ser::{SerializeMap, Serializer as _};
use serde_json::Value;
use std::{
    fmt::{self, Formatter, Write},
    io,
};
use tracing_core::{
    span::{Attributes, Id},
    Event, Subscriber,
};
use tracing_serde::AsSerde;
use tracing_subscriber::{
    field::{MakeVisitor, VisitOutput},
    fmt::{
        time::{ChronoUtc, FormatTime},
        FormatFields, FormattedFields, MakeWriter,
    },
    layer::Context,
    registry::LookupSpan,
    Layer,
};

/// A tracing adapater for stackdriver
pub struct Stackdriver<W = fn() -> io::Stdout>
where
    W: MakeWriter,
{
    time: ChronoUtc,
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
    W: MakeWriter,
{
    /// Initialize the Stackdriver Layer with a custom writer
    pub fn with_writer(writer: W) -> Self {
        Self {
            time: ChronoUtc::rfc3339(),
            writer,
            fields: StackdriverFields,
        }
    }

    fn visit<S>(&self, event: &Event, context: Context<S>) -> Result<(), Error>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        let writer = self.writer.make_writer();
        let meta = event.metadata();
        let mut time = String::new();

        self.time.format_time(&mut time).map_err(|_| Error::Time)?;

        let mut serializer = serde_json::Serializer::new(writer);

        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("time", &time)?;
        map.serialize_entry("severity", &meta.level().as_serde())?;
        map.serialize_entry("target", &meta.target())?;

        if let Some(span) = context.lookup_current() {
            let name = &span.name();
            let extensions = span.extensions();
            let formatted_fields = extensions
                .get::<FormattedFields<StackdriverFields>>()
                .expect("No fields!");

            // TODO: include serializable data type in extensions instead of str
            let mut fields: Value = serde_json::from_str(&formatted_fields)?;

            fields["name"] = serde_json::json!(name);

            map.serialize_entry("span", &fields)?;
        }

        // TODO: enable deeper structuring of keys and values across tracing
        // https://github.com/tokio-rs/tracing/issues/663
        let mut visitor = StackdriverEventVisitor::new(map);

        event.record(&mut visitor);

        visitor.finish().map_err(Error::from)
    }
}

impl Default for Stackdriver {
    fn default() -> Self {
        Self {
            time: ChronoUtc::rfc3339(),
            writer: || std::io::stdout(),
            fields: StackdriverFields,
        }
    }
}

impl<S, W> Layer<S> for Stackdriver<W>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: MakeWriter + 'static,
{
    fn new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        let span = context.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions
            .get_mut::<FormattedFields<StackdriverFields>>()
            .is_none()
        {
            let mut buffer = String::new();
            if self.fields.format_fields(&mut buffer, attributes).is_ok() {
                let fmt_fields: FormattedFields<StackdriverFields> = FormattedFields::new(buffer);

                extensions.insert(fmt_fields);
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

struct StackdriverFields;

impl<'a> MakeVisitor<&'a mut dyn Write> for StackdriverFields {
    type Visitor = StackdriverVisitor<'a>;

    #[inline]
    fn make_visitor(&self, target: &'a mut dyn Write) -> Self::Visitor {
        StackdriverVisitor::new(target)
    }
}

#[derive(Debug)]
enum Error {
    Formatting(fmt::Error),
    Serialization(serde_json::Error),
    Time,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::Formatting(error) => write!(formatter, "{}", &error),
            Self::Serialization(error) => write!(formatter, "{}", &error),
            Self::Time => write!(formatter, "Could not format timestamp"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Self::Formatting(error)
    }
}
