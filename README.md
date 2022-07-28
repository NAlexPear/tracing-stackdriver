## `tracing-stackdriver`
### A `tracing` Subscriber for communicating Stackdriver-formatted logs

[`tracing`](https://docs.rs/tracing/0.1.13/tracing/) is a scoped, structured logging and diagnostic system based on emitting [`Event`](https://docs.rs/tracing/0.1.13/tracing/#events)s in the context of potentially-nested [`Span`](https://docs.rs/tracing/0.1.13/tracing/#spans)s across asynchronous `await` points. These properties make `tracing` ideal for use with [Google Cloud Operations Suite structured logging](https://cloud.google.com/logging/docs/structured-logging) (formerly Stackdriver).

This crate provides a [`Layer`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/fmt/struct.Layer.html) for use with a `tracing` [`Registry`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/struct.Registry.html) that formats `tracing` Spans and Events into properly-structured JSON for consumption by Google Operations Logging through the [`jsonPayload`](https://cloud.google.com/logging/docs/structured-logging) field. This includes the following behaviors and enhancements:

1. `rfc3339`-formatted timestamps for all Events
2. `severity` (in [`LogSeverity`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity) format) derived from `tracing` [`Level`](https://docs.rs/tracing/0.1.13/tracing/struct.Level.html)
3. `target` derived from the Event `target` [`Metadata`](https://docs.rs/tracing/0.1.13/tracing/struct.Metadata.html)
4. Span `name` and custom fields included under a `span` key
5. automatic nesting of `http_request.`-prefixed event fields
6. automatic camelCase-ing of all field keys (e.g. `http_request` -> `httpRequest`)
7. [`valuable`](https://docs.rs/valuable/latest/valuable/) support, including an `HttpRequest` helper `struct`
8. [OpenTelemetry](https://opentelemetry.io) integration

### Examples

#### Basic setup:

```rust
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_stackdriver::Stackdriver;

fn main() {
    let stackdriver = Stackdriver::layer(); // writes to std::io::Stdout
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
    let stackdriver = Stackdriver::layer().with_writer(make_writer); // writes to std::io::Stderr
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set up global logger");
}
```

#### With `httpRequest` fields:

See all available fields [here](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#HttpRequest).

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

### With more specific `LogSeverity` levels:

Google supports a slightly different set of severity levels than `tracing`. `tracing` levels are automatically mapped to `LogSeverity` levels, but you can customize the level beyond the intersection of `tracing` levels and `LogSeverity` levels by using the provided `LogSeverity` level with a `severity` key.

```rust
use tracing_stackdriver::LogSeverity;

fn main() {
    // requires working global setup (see above examples)

    tracing::info!(severity = %LogSeverity::Notice, "Application starting");

  // jsonPayload formatted as:
  // {
  //   "time": "some-timestamp"
  //   "severity": "NOTICE",
  //   "message": "Request received"
  // }
}
```

#### With `valuable` support:

`tracing_stackdriver` supports deeply-nested structured logging through `tracing`'s [unstable `valuable` support](https://github.com/tokio-rs/tracing/discussions/1906). In addition, `httpRequest` fields can be generated with the `HttpRequest` helper struct exported from this library for better compile-time checking of fields.

To enable `valuable` support, use the `valuable` feature flag and compile your project with `RUSTFLAGS="--cfg tracing_unstable"`.

```rust

// requires working global setup (see above examples)

use hyper::Request;
use tracing_stackdriver::HttpRequest;
use valuable::Valuable;

#[derive(Valuable)]
struct StructuredLog {
    service: &'static str,
    handler: &'static str
}


fn handle_request(request: Request) {
  let http_request = HttpRequest {
      request_method: request.method().into(),
      request_url: request.uri().into(),
      ..Default::default()
  };

  let structured_log = StructuredLog {
      service: "request_handlers",
      handler: "handle_request",
  };

  tracing::info!(
    http_request = http_request.as_value(),
    structured_log = structured_log.as_value(),
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
  //   "structuredLog": {
  //      "service": "request_handlers",
  //      "handler": "handle_request"
  //    },
  //   "message": "Request received"
  // }
}
```

#### With OpenTelemetry integration:

`tracing_stackdriver` supports integration with [OpenTelemetry](https://opentelemetry.io) via [tracing_opentelemetry](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry) and outputs [special Cloud Logging fields](https://cloud.google.com/logging/docs/agent/logging/configuration#special-fields) for trace sampling and log correlation.

To enable OpenTelemetry integration, you need to
1. use the `opentelemetry` feature flag, and
2. provide `StackdriverLayer` with your GCP project ID, and
3. add a [tracing_opentelemetry](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry) layer to the subscriber.
```rust
let stackdriver = tracing_stackdriver::layer()
    .with_project_id("my_gcp_project_id".into());
let subscriber = tracing_subscriber::Registry::default()
    // You may want to configure the `tracing_opentelemetry` layer to suit your needs.
    // See `tracing_opentelemetry`'s doc for details.
    .with(tracing_opentelemetry::layer())
    .with(stackdriver);
```

#### Roadmap:

1. distributing tracing data in [Cloud Trace](https://cloud.google.com/trace/docs) format
2. support the logging API in addition to the logging agent/std{out,err}
