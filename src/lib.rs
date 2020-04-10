/*!
`tracing` Subscriber for structuring Stackdriver-compatible
[`LogEntry`](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry)

While `Stackdriver` will eventually be a standalone Subscriber,
it's best used in its current form as an event formatter for the existing `Json` Subscriber in `tracing_subscriber`,
e.g.:

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

