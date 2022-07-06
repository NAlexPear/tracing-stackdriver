use inflector::Inflector;
use serde::ser::{SerializeMap, Serializer as _};
use serde_json::Serializer;
use std::{
    collections::BTreeMap,
    fmt::{self, Formatter, Write},
    io,
};
use tracing_core::Field;
use tracing_subscriber::field::{Visit, VisitFmt, VisitOutput};

/// the EventVisitor implementation for Stackdriver
pub(crate) struct StackdriverEventVisitor<'a, S: SerializeMap> {
    values: BTreeMap<&'a str, serde_json::Value>,
    serializer: S,
}

impl<'a, S> StackdriverEventVisitor<'a, S>
where
    S: SerializeMap,
{
    /// Returns a new default visitor using the provided writer
    pub(crate) fn new(serializer: S) -> Self {
        Self {
            values: BTreeMap::new(),
            serializer,
        }
    }
}

impl<'a, S> VisitOutput<fmt::Result> for StackdriverEventVisitor<'a, S>
where
    S: SerializeMap,
{
    fn finish(mut self) -> fmt::Result {
        let inner = || {
            let mut http_request = BTreeMap::new();

            for (key, value) in self.values {
                if key.starts_with("http_request.") {
                    if let Some(request_key) = key.splitn(2, '.').last() {
                        http_request.insert(request_key.to_camel_case(), value);
                    }
                } else {
                    self.serializer
                        .serialize_entry(&key.to_camel_case(), &value)?;
                }
            }

            if !http_request.is_empty() {
                self.serializer
                    .serialize_entry("httpRequest", &http_request)?;
            }

            self.serializer.end()
        };

        if inner().is_err() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl<'a, S> Visit for StackdriverEventVisitor<'a, S>
where
    S: SerializeMap,
{
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.values.insert(
            field.name(),
            serde_json::Value::from(format!("{:?}", value)),
        );
    }

    #[cfg(tracing_unstable)]
    fn record_value(&mut self, field: &Field, value: valuable::Value<'_>) {
        let value = serde_json::to_value(valuable_serde::Serializable::new(value)).unwrap();

        self.values.insert(field.name(), value);
    }
}

impl<'a, S> fmt::Debug for StackdriverEventVisitor<'a, S>
where
    S: SerializeMap,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("StackdriverEventVisitor")
            .field("values", &self.values)
            .finish()
    }
}

/// the Visitor implementation for Stackdriver
pub(crate) struct StackdriverVisitor<'a> {
    values: BTreeMap<&'a str, serde_json::Value>,
    writer: &'a mut dyn Write,
}

impl<'a> StackdriverVisitor<'a> {
    /// Returns a new default visitor using the provided writer
    pub(crate) fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            values: BTreeMap::new(),
            writer,
        }
    }
}

impl<'a> VisitFmt for StackdriverVisitor<'a> {
    fn writer(&mut self) -> &mut dyn Write {
        self.writer
    }
}

impl<'a> VisitOutput<fmt::Result> for StackdriverVisitor<'a> {
    fn finish(self) -> fmt::Result {
        let inner = || {
            let mut serializer = Serializer::new(WriteAdaptor::new(self.writer));
            let mut map = serializer.serialize_map(None)?;

            for (key, value) in self.values {
                map.serialize_entry(key, &value)?;
            }

            map.end()
        };

        if inner().is_err() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl<'a> Visit for StackdriverVisitor<'a> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name(), serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.values.insert(
            field.name(),
            serde_json::Value::from(format!("{:?}", value)),
        );
    }

    #[cfg(tracing_unstable)]
    fn record_value(&mut self, field: &Field, value: valuable::Value<'_>) {
        let value = serde_json::to_value(valuable_serde::Serializable::new(value)).unwrap();

        self.values.insert(field.name(), value);
    }
}

impl<'a> fmt::Debug for StackdriverVisitor<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("StackdriverVisitor")
            .field("values", &self.values)
            .finish()
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
            .write_str(s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> std::fmt::Debug for WriteAdaptor<'a> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        // FIXME: use the struct builders
        formatter.pad("WriteAdaptor { .. }")
    }
}
