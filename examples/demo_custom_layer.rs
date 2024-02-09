use tracing_subscriber::fmt::{format::JsonFields, FormatEvent, FormatFields, FormattedFields};
use tracing_subscriber::{fmt, prelude::*, registry::LookupSpan, Layer, Registry};
use std::fmt::{Debug, Write as _};
use std::io::Write;
use tracing_core::Field;

struct CustomLayer<S, N> {
    // Inner layer (e.g., for JSON formatting)
    inner: fmt::Layer<S, N, fmt::format::Format<fmt::format::Json>>,
}

impl<S, N> Layer<S> for CustomLayer<S, N>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Intercept the event here and customize the output
        let mut visitor = CustomVisitor::new();
        event.record(&mut visitor);

        // Write the customized fields to the formatter
        if let Some(mut buf) = ctx.field_buffer() {
            let mut serializer = buf.as_writer();
            for (field, value) in visitor.fields {
                if field == "trace_id" {
                    // Move trace_id to the root level
                    writeln!(serializer, "\"trace_id\": {},", value).unwrap();
                } else {
                    // Adjust or ignore fields as needed
                }
            }

            // Use the inner layer to complete formatting and output
            let fields = FormattedFields::<JsonFields>::new(String::from_utf8(buf).unwrap());
            self.inner.on_event(event, &fields, ctx);
        }
    }
}

// Define your custom visitor here to handle specific fields
struct CustomVisitor {
    fields: Vec<(String, String)>,
}

impl CustomVisitor {
    fn new() -> Self {
        CustomVisitor { fields: vec![] }
    }
}

impl tracing_subscriber::field::Visit for CustomVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        println!("RECORD_DEBUG CALLED!!");
        dbg!(field);
        dbg!(value);
        todo!()
    }

    // Implement other record methods as needed
}

fn main() {
    // Set up the custom layer
    let custom_layer = CustomLayer {
        inner: fmt::Layer::new().json(),
    };

    let subscriber = Registry::default().with(custom_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // Your application logic here
}
