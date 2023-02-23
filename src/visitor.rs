use crate::google::LogSeverity;
use inflector::Inflector;
use serde::ser::SerializeMap;
use std::{collections::BTreeMap, fmt};
use tracing_core::Field;
use tracing_subscriber::field::{Visit, VisitOutput};

/// Visitor for Stackdriver events that formats custom fields
pub(crate) struct Visitor<'a, S>
where
    S: SerializeMap,
{
    values: BTreeMap<&'a str, serde_json::Value>,
    severity: LogSeverity,
    serializer: S,
}

impl<'a, S> Visitor<'a, S>
where
    S: SerializeMap,
{
    /// Returns a new default visitor using the provided writer
    pub(crate) fn new(severity: LogSeverity, serializer: S) -> Self {
        Self {
            values: BTreeMap::new(),
            severity,
            serializer,
        }
    }
}

impl<'a, S> VisitOutput<fmt::Result> for Visitor<'a, S>
where
    S: SerializeMap,
{
    fn finish(mut self) -> fmt::Result {
        let inner = || {
            let severity = self
                .values
                .remove("severity")
                .map(LogSeverity::from)
                .unwrap_or(self.severity);

            self.serializer.serialize_entry("severity", &severity)?;

            let mut http_request = BTreeMap::new();
            let mut google_labels = BTreeMap::new();

            for (key, value) in self.values {
                if key.starts_with("google.labels.") {
                    if let Some(google_labels_key) = key.splitn(3, '.').last() {
                        google_labels.insert(google_labels_key.to_camel_case(), value);
                    }
                } else if key.starts_with("google.") {
                    if let Some(google_field) = key.splitn(3, '.').last() {
                        self.serializer
                            .serialize_entry(&format!("logging.googleapis.com/{}", google_field.to_camel_case()), &value)?;
                    }
                } else if key.starts_with("http_request.") {
                    if let Some(request_key) = key.splitn(2, '.').last() {
                        http_request.insert(request_key.to_camel_case(), value);
                    }
                } else {
                    self.serializer
                        .serialize_entry(&key.to_camel_case(), &value)?;
                }
            }

            if !google_labels.is_empty() {
                self.serializer
                    .serialize_entry("logging.googleapis.com/labels", &google_labels)?;
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

impl<'a, S> Visit for Visitor<'a, S>
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

    #[cfg(all(tracing_unstable, feature = "valuable"))]
    fn record_value(&mut self, field: &Field, value: valuable::Value<'_>) {
        let value = serde_json::to_value(valuable_serde::Serializable::new(value)).unwrap();

        self.values.insert(field.name(), value);
    }
}

impl<'a, S> fmt::Debug for Visitor<'a, S>
where
    S: SerializeMap,
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_struct("Visitor")
            .field("values", &self.values)
            .finish()
    }
}
