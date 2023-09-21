#![cfg(feature = "opentelemetry")]
use helpers::MockWriter;
use lazy_static::lazy_static;
use opentelemetry::{
    sdk::{testing::trace::TestSpan},
    trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
};
use rand::Rng;
use serde::{de::Error, Deserialize, Deserializer};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};
use opentelemetry::sdk::trace::TracerProvider;
use tracing_stackdriver::CloudTraceConfiguration;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt};

mod helpers;
mod mocks;

static PROJECT_ID: &str = "my_project_123";

lazy_static! {
    static ref CLOUD_TRACE_CONFIGURATION: CloudTraceConfiguration = CloudTraceConfiguration {
        project_id: PROJECT_ID.to_owned(),
    };

    // use a tracer that generates valid span IDs (unlike default NoopTracer)
    static ref TRACER: TracerProvider = TracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build();
}

#[derive(Debug, Deserialize)]
struct MockEventWithCloudTraceFields {
    #[serde(
        rename = "logging.googleapis.com/spanId",
        deserialize_with = "from_hex"
    )]
    span_id: SpanId,
    #[serde(rename = "logging.googleapis.com/trace")]
    trace_id: String,
    #[serde(rename = "logging.googleapis.com/trace_sampled", default)]
    trace_sampled: bool,
}

fn from_hex<'de, D>(deserializer: D) -> Result<SpanId, D::Error>
where
    D: Deserializer<'de>,
{
    let hex: &str = Deserialize::deserialize(deserializer)?;
    SpanId::from_hex(hex).map_err(D::Error::custom)
}

fn test_with_tracing<M>(span_id: SpanId, trace_id: TraceId, make_writer: M, callback: impl FnOnce())
where
    M: for<'writer> MakeWriter<'writer> + Sync + Send + 'static,
{
    use opentelemetry::trace::TracerProvider as _;

    // generate the tracing subscriber
    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_opentelemetry::layer()
                .with_location(false)
                .with_threads(false)
                .with_tracked_inactivity(false)
                .with_tracer(TRACER.tracer("test")),
        )
        .with(
            tracing_stackdriver::layer()
                .with_writer(make_writer)
                .with_cloud_trace(CLOUD_TRACE_CONFIGURATION.clone()),
        );

    // generate a context for events
    let context = opentelemetry::Context::current_with_span(TestSpan(SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::default(),
        false,
        TraceState::default(),
    )));

    // attach the tracing context
    let _context = context.attach();

    // run the callback in a tracing context
    tracing::subscriber::with_default(subscriber, callback);
}

#[test]
fn includes_correct_cloud_trace_fields() {
    // generate the output buffer
    let buffer = Arc::new(Mutex::new(vec![]));
    let shared = buffer.clone();
    let make_writer = move || MockWriter(shared.clone());

    // generate relevant IDs
    let mut rng = rand::thread_rng();
    let span_id = SpanId::from_u64(rng.gen());
    let trace_id = TraceId::from_u128(rng.gen());

    // generate a tracing-based event
    test_with_tracing(span_id, trace_id, make_writer, || {
        let root = tracing::debug_span!("root");
        let _root = root.enter();
        tracing::debug!("test event");
    });

    let output: MockEventWithCloudTraceFields = serde_json::from_slice(&buffer.try_lock().unwrap())
        .expect("Error converting test buffer to JSON");

    // span IDs should NOT be propagated, but generated for each span
    assert_ne!(
        output.span_id, span_id,
        "Span IDs are the same, but should not be"
    );

    // trace ID should be propagated and formatted in Cloud Trace format
    assert_eq!(
        output.trace_id,
        format!("projects/{PROJECT_ID}/traces/{trace_id}"),
        "Trace IDs are not compatible",
    );

    // trace sampling should be disabled by default
    assert!(!output.trace_sampled)
}

#[test]
fn handles_nested_spans() {
    // generate the output buffer
    let buffer = Arc::new(Mutex::new(vec![]));
    let shared = buffer.clone();
    let make_writer = move || MockWriter(shared.clone());

    // generate relevant IDs
    let mut rng = rand::thread_rng();
    let span_id = SpanId::from_u64(rng.gen());
    let trace_id = TraceId::from_u128(rng.gen());

    // generate a set of nested tracing-based events
    test_with_tracing(span_id, trace_id, make_writer, || {
        let root = tracing::debug_span!("root");
        let _root = root.enter();
        tracing::debug!("top-level test event");
        let inner = tracing::debug_span!("inner");
        let _inner = inner.enter();
        tracing::debug!("inner test event");
    });

    // parse the newline-separated messages from the test buffer
    let raw = &buffer.try_lock().unwrap();

    let mut messages = raw
        .split(|byte| byte == &b'\n')
        .filter(|segment| !segment.is_empty())
        .map(serde_json::from_slice) // FIXME: serde_json this bad boy
        .collect::<Result<Vec<MockEventWithCloudTraceFields>, _>>()
        .expect("Error converting test buffer to JSON")
        .into_iter()
        .peekable();

    // test messages at every depth for correctness
    while let Some(message) = messages.next() {
        // span IDs should NOT be propagated, but generated for each span
        assert_ne!(
            message.span_id, span_id,
            "Span IDs are the same, but should not be"
        );

        if let Some(next_message) = messages.peek() {
            // span IDs should be different between spans
            assert_ne!(
                message.span_id, next_message.span_id,
                "Span IDs between messages are the same, but should not be"
            );
        }

        // trace ID should be propagated to all messages and formatted in Cloud Trace format
        assert_eq!(
            message.trace_id,
            format!("projects/{PROJECT_ID}/traces/{trace_id}"),
            "Trace IDs are not compatible",
        );

        // trace sampling should be disabled by default
        assert!(!message.trace_sampled)
    }
}
