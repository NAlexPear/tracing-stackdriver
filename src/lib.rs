/*!
`tracing` Subscriber for structuring Stackdriver-compatible
[`LogEntry`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry)

This crate provides a [`Layer`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/fmt/struct.Layer.html)
for use with a `tracing` [`Registry`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/struct.Registry.html)
that formats `tracing` Spans and Events into properly-structured JSON for consumption by Google Operations Logging
through the [`jsonPayload`](https://cloud.google.com/logging/docs/structured-logging) field.

This includes the following behaviors and enhancements:

1. `rfc3339`-formatted timestamps for all Events
2. `severity` (in [`LogSeverity`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity) format) derived from `tracing` [`Level`](https://docs.rs/tracing/0.1.13/tracing/struct.Level.html)
3. `target` derived from the Event `target` [`Metadata`](https://docs.rs/tracing/0.1.13/tracing/struct.Metadata.html)
4. Span `name` and custom fields included under a `span` key
5. automatic nesting of `http_request.`-prefixed event fields
6. automatic camelCase-ing of all field keys (e.g. `http_request` -> `httpRequest`)

```
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_stackdriver::Stackdriver;

fn main() {
    let stackdriver = Stackdriver::default(); // writes to std::io::Stdout
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
}
```
*/
#![deny(missing_docs, unreachable_pub)]
mod layer;
mod visitor;

pub use self::layer::*;
