#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), deny(unused_crate_dependencies))]
#![deny(missing_docs, unreachable_pub)]
#![allow(clippy::needless_doctest_main)]
/*!
`tracing` Subscriber for structuring Stackdriver-compatible
[`LogEntry`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry)

This crate provides a [`Layer`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/fmt/struct.Layer.html) for use with a `tracing` [`Registry`](https://docs.rs/tracing-subscriber/0.2.4/tracing_subscriber/struct.Registry.html) that formats `tracing` Spans and Events into properly-structured JSON for consumption by Google Operations Logging through the [`jsonPayload`](https://cloud.google.com/logging/docs/structured-logging) field. This includes the following behaviors and enhancements:

1. `rfc3339`-formatted timestamps for all Events
2. `severity` (in [`LogSeverity`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity) format) derived from `tracing` [`Level`](https://docs.rs/tracing/0.1.13/tracing/struct.Level.html)
3. `target` derived from the Event `target` [`Metadata`](https://docs.rs/tracing/0.1.13/tracing/struct.Metadata.html)
4. Span `name` and custom fields included under a `span` key
5. automatic nesting of `http_request.`-prefixed event fields
6. automatic camelCase-ing of all field keys (e.g. `http_request` -> `httpRequest`)
7. [`valuable`](https://docs.rs/valuable/latest/valuable/) support, including an `HttpRequest` helper `struct`

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
*/
mod google;
mod layer;
mod visitor;
mod writer;

pub use self::google::*;
pub use self::layer::*;
