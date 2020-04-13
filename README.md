## `tracing-stackdriver`
### A `tracing` Subscriber for communicating Stackdriver-formatted logs

[`tracing`](https://docs.rs/tracing/0.1.13/tracing/) is a scoped, structured logging and diagnostic system based on emitting [`Event`](https://docs.rs/tracing/0.1.13/tracing/#events)s in the context of potentially-nested [`Span`](https://docs.rs/tracing/0.1.13/tracing/#spans)s across asynchronous `await` points. These properties make `tracing` ideal for use with [Google Cloud Operations Suite structured logging](https://cloud.google.com/logging/docs/structured-logging) (formerly Stackdriver). This crate provides a [`Layer`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/fmt/struct.Layer.html) for use with a `tracing` [`Registry`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/struct.Registry.html) that formats `tracing` Spans and Events into properly-structured JSON for consumption by Google Operations Logging through the [`jsonPayload`](https://cloud.google.com/logging/docs/structured-logging) field. This includes the following behaviors and enhancements:

1. `rfc3339`-formatted timestamps for all Events
2. `severity` (in [`LogSeverity`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity) format) derived from `tracing` [`Level`](https://docs.rs/tracing/0.1.13/tracing/struct.Level.html)
3. `target` derived from the Event `target` [`Metadata`](https://docs.rs/tracing/0.1.13/tracing/struct.Metadata.html)
4. Span `name` and custom fields included under a `span` key
5. automatic nesting of `http_request.`-prefixed event fields
6. automatic camelCase-ing of all field keys (e.g. `http_request` -> `httpRequest`)

### Examples

#### Basic setup:

```rust
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_stackdriver::Stackdriver;

fn main() {
    let stackdriver = Stackdriver::default(); // writes to std::io::Stdout
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
}
```

#### Custom write location:

```rust
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_stackdriver::Stackdriver;

fn main() {
    let make_writer = || std::io::Stderr;
    let stackdriver = Stackdriver::with_writer(make_writer); // writes to std::io::Stderr
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
}
```

#### With `httpRequest` fields:

See all available fields [here](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#HttpRequest)

```rust
// requires working global setup (see above examples)

use hyper::Request;

fn handle_request(request: Request) {
  let method = &request.method();
  let uri = &request.uri();

  tracing::info!(
    http_request.request_method = %method,
    http_request.request_url = %uri,
    "Request received"
  );

  // jsonPayload formatted as:
  // {
  //   "time": "some-timestamp"
  //   "severity": "INFO",
  //   "httpRequest": {
  //     "requestMethod": "GET",
  //     "requestUrl": "/some/url/from/request"
  //    },
  //   "message": "Request received"
  // }
}
```

#### Roadmap:

1. type-safe `http_request`s derived from Google's REST v2 spec
2. distributing tracing data in [Cloud Trace](https://cloud.google.com/trace/docs) format
