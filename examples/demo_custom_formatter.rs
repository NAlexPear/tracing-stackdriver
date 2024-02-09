use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;
// use tracing_stackdriver_cw::Layer;
use tracing_subscriber::fmt;
use tracing::{info, instrument};
use serde_json::json;
use std::io;
use uuid::Uuid;
use chrono::Utc;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FormatFields;

#[instrument]
fn database_function() {
    info!("This is the database function");
}

#[instrument]
fn business_logic_function() {
    info!("This is the business logic function");
    database_function();
}

#[instrument]
fn endpoint_function() {
    let trace_id = Uuid::new_v4().to_string();
    let _span = tracing::info_span!("endpoint_function", trace_id = %trace_id);

    info!(trace_id = %trace_id, "This is the endpoint function");
    business_logic_function();
}


struct CustomFormatter;

impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for CustomFormatter
    where
        S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        writer: Writer,
        event: &tracing::Event<'_>,
    ) -> Result<(), std::fmt::Error> {
        let metadata = event.metadata();
        let mut visitor = tracing_subscriber::fmt::format::JsonFields::new();
        println!("PARENT: {:?}", event.parent());
        println!("METADATA: {metadata:?}");
        for field in event.fields() {
            println!("FIELD {field:?} = {{value:?}}");
        }
         event.record(&mut |field, value| {
        // });
        // let fields = visitor.finish();

        // Using chrono for the timestamp
        let now = Utc::now();

        // let log = json!({
        //     "time": now.to_rfc3339(),
        //     "level": metadata.level().to_string(),
        //     "message": fields.message.unwrap_or_default(),
        //     // "trace_id" handling remains the same
        // });
        //
        // writeln!(writer, "{}", log)
        Ok(())
    }
}

fn main() {
    let subscriber = Registry::default()
        .with(fmt::Layer::default()
            .event_format(CustomFormatter)
            .fmt_fields(fmt::format::JsonFields::new()));

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    endpoint_function();
}
