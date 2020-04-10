use serde::ser::{SerializeMap, Serializer as _};
use serde_json::Serializer;
use std::{
    collections::BTreeMap,
    fmt::{self, Formatter, Write},
    io,
};
use tracing_core::Field;
use tracing_subscriber::field::{Visit, VisitFmt, VisitOutput};

/// the Visitor implementation for Stackdriver
pub(crate) struct StackdriverVisitor<'a> {
    // TODO: consider restricting values further, perhaps?
    // this might be where we can extract httpResponse, if needed
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
                // TOOD: make httpResponse exception here
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
            .insert(&field.name(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.values.insert(
            &field.name(),
            serde_json::Value::from(format!("{:?}", value)),
        );
    }
}

impl<'a> fmt::Debug for StackdriverVisitor<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "StackdriverVisitor {{ values: {:?} }}",
            self.values
        ))
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
