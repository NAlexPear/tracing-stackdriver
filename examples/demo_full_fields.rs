use tracing::{info, instrument, trace_span};
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;
use tracing_stackdriver_cw::layer;
use uuid::Uuid;

#[instrument]
fn sync_database_function() {
    info!("This is the SYNC database function");
}

#[instrument]
fn sync_business_logic_function() {
    info!("This is the SYNC business logic function");
    sync_database_function();
}

#[instrument]
fn sync_endpoint_function() {
    // `trace_id` can come from Google App Engine, via headers.
    // Here, we generate it manually
    let trace_id = Uuid::new_v4().to_string();
    // the following 2 variables must only be dropped at the end of the function,
    // or else `traceId` won't be tracked correctly, as it is controlled by the
    // opened "spans" on each thread.
    let span = trace_span!("sync_endpoint_function", trace_id = %trace_id);
    let _enter = span.enter();

    info!(trace_id = %trace_id, "This is the SYNC endpoint function");
    sync_business_logic_function();
}

#[instrument]
async fn async_database_function() {
    info!("This is the ASYNC database function");
}

#[instrument]
async fn async_business_logic_function() {
    info!("This is the ASYNC business logic function");
    async_database_function().await;
}

#[instrument]
async fn async_endpoint_function() {
    // `trace_id` can come from Google App Engine, via headers.
    // Here, we generate it manually
    let trace_id = Uuid::new_v4().to_string();
    // the following 2 variables must only be dropped at the end of the function,
    // or else `traceId` won't be tracked correctly, as it is controlled by the
    // opened "spans" on each thread.
    let span = trace_span!("async_endpoint_function", trace_id = %trace_id);
    let _enter = span.enter();

    info!(trace_id = %trace_id, "This is the SYNC endpoint function");
    async_business_logic_function().await;
}

#[tokio::main]
async fn main() {
    // Set up the subscriber.
    let stackdriver = layer(); // writes to std::io::Stdout
    let subscriber = Registry::default().with(stackdriver);

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // For traditional sync functions, tracing-stackdriver will link spans to threads.
    sync_endpoint_function();

    // You can safely assume that tracing-stackdriver will work as expected in async scenarios when using Tokio,
    // as it will link spans to execution contexts for the async mode.
    // Ensure that all your asynchronous tasks (futures) are spawned within the Tokio runtime.
    // Avoid mixing threads and tasks directly; let Tokio manage the execution flow.
    async_endpoint_function().await;

    // observe that each log entry contains the same 'traceId' field at the root of each json,
    // like the following excerpt:
    // {"time":"2024-02-15T14:38:07.97665775Z","target":"demo_full_fields","logging.googleapis.com/sourceLocation":{"file":"examples/demo_full_fields.rs","line":"29"},"span":{"trace_id":"25075b50-d745-4d6b-9040-015be8482ad7","name":"endpoint_function"},"traceId":"25075b50-d745-4d6b-9040-015be8482ad7","severity":"INFO","message":"This is the endpoint function","traceId":"25075b50-d745-4d6b-9040-015be8482ad7"}
    // {"time":"2024-02-15T14:38:07.976721894Z","target":"demo_full_fields","logging.googleapis.com/sourceLocation":{"file":"examples/demo_full_fields.rs","line":"14"},"span":{"name":"business_logic_function"},"traceId":"25075b50-d745-4d6b-9040-015be8482ad7","severity":"INFO","message":"This is the business logic function"}
    // {"time":"2024-02-15T14:38:07.976742013Z","target":"demo_full_fields","logging.googleapis.com/sourceLocation":{"file":"examples/demo_full_fields.rs","line":"9"},"span":{"name":"database_function"},"traceId":"25075b50-d745-4d6b-9040-015be8482ad7","severity":"INFO","message":"This is the database function"}
}
