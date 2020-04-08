/*!
`tracing` Subscriber for structuring Stackdriver-compatible
[`LogEntry`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry)

While `Stackdriver` will eventually be a standalone Subscriber,
it's best used in its current form as an event formatter for the existing `Json` Subscriber in `tracing_subscriber`,
e.g.:

```
use tracing_subscriber::fmt::Subscriber;

fn main() {
    let subscriber = Subscriber::builder()
        .json()
        .event_format(Stackdriver::default())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
}
```
*/
#![deny(missing_docs)]
use serde::ser::{SerializeMap, Serializer};
use serde_json::{self, json, Value};
use std::{
    fmt::{Formatter, Write},
    io,
};
use tracing::{Event, Subscriber};
use tracing_serde::{AsSerde, SerdeMapVisitor};
use tracing_subscriber::{
    fmt::{
        time::{ChronoUtc, FormatTime},
        FmtContext, FormatEvent, FormatFields, FormattedFields,
    },
    registry::LookupSpan,
};

/// A tracing adapater for stackdriver
pub struct Stackdriver {
    time: ChronoUtc,
}

impl Default for Stackdriver {
    fn default() -> Self {
        Self {
            time: ChronoUtc::rfc3339(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for Stackdriver
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        context: &FmtContext<S, N>,
        writer: &mut dyn Write,
        event: &Event,
    ) -> std::fmt::Result {
        let meta = event.metadata();
        let mut time_buffer = String::new();
        self.time.format_time(&mut time_buffer)?;

        // current ChronoUtc implementation has an extra space at the end of the timestamp
        let time = time_buffer.trim_end();

        let mut visit = || {
            let mut serializer = serde_json::Serializer::new(WriteAdaptor::new(writer));

            let mut serializer = serializer.serialize_map(None)?;

            serializer.serialize_entry("time", &time)?;
            serializer.serialize_entry("severity", &meta.level().as_serde())?;
            serializer.serialize_entry("target", &meta.target())?;

            context.visit_spans(|span| {
                let extensions = span.extensions();
                let data = extensions
                    .get::<FormattedFields<N>>()
                    .expect("Unable to find FormattedFields in extensions; this is a bug");

                // TODO: submit PR to fix this in tracing library
                let mut fields: Value = serde_json::from_str(&data)?;

                fields["name"] = json!(span.metadata().name());

                serializer.serialize_entry("span", &fields)
            })?;

            // TODO: enable deeper structuring of keys and values
            // https://github.com/tokio-rs/tracing/issues/663
            let mut visitor = SerdeMapVisitor::new(serializer);

            event.record(&mut visitor);

            visitor.finish()
        };

        visit().map_err(|_| std::fmt::Error)?;

        writeln!(writer)
    }
}

/// Utility newtype for converting between fmt::Write and io::Write
struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn Write,
}

impl<'a> WriteAdaptor<'a> {
    fn new(fmt_write: &'a mut dyn Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(&s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> std::fmt::Debug for WriteAdaptor<'a> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.pad("WriteAdaptor { .. }")
    }
}
