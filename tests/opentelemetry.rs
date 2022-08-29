#![cfg(feature = "opentelemetry")]
use lazy_static::lazy_static;
use opentelemetry::trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState};
use serde::{de::Error, Deserialize, Deserializer};
use std::{fmt::Debug, sync::Mutex};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;

mod writer;

static PROJECT_ID: &str = "my_project_123";

macro_rules! run_with_tracing {
    (|| $expression:expr) => {{
        lazy_static! {
            static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || writer::MockWriter(&BUFFER);

        let stackdriver = tracing_stackdriver::layer()
            .with_writer(make_writer)
            .with_project_id(PROJECT_ID.into());

        let subscriber = tracing_subscriber::registry()
            .with(
                tracing_opentelemetry::layer()
                    .with_location(false)
                    .with_threads(false)
                    .with_tracked_inactivity(false),
            )
            .with(stackdriver);

        tracing::subscriber::with_default(subscriber, || {
            let parent = opentelemetry::Context::new();
            let root_span = tracing::info_span!("root_span");
            root_span.set_parent(parent);
            let _root_span = root_span.enter();
            $expression
        });

        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec()
    }};
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

#[test]
fn includes_cloud_trace_fields() {
    let raw = run_with_tracing!(|| tracing::info!("Should have cloud trace fields"));

    let output = serde_json::from_slice::<MockEventWithCloudTraceFields>(raw)
        .expect("Error converting test buffer to JSON");

    assert_ne!(output.span_id.to_string(), "0000000000000000");
    assert!(output
        .trace_id
        .starts_with(&format!("projects/{PROJECT_ID}/traces/")));
    assert!(!output.trace_sampled);
}

#[test]
fn includes_explicit_opentelemetry_context_fields() {
    let span_id = SpanId::from_hex("b7ad6b7169203331").expect("Error converting hex to SpanId");
    let trace_id = TraceId::from_hex("0af7651916cd43dd8448eb211c80319c")
        .expect("Error converting hex to TraceId");

    let raw = run_with_tracing!(|| {
        let parent = opentelemetry::Context::new().with_remote_span_context(SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::default(),
            true,
            TraceState::from_key_value([("", ""); 0]).unwrap(),
        ));

        let root_span = tracing::info_span!("root_span");
        root_span.set_parent(parent);
        root_span.in_scope(|| {
            let inner_span = tracing::info_span!("inner_span");
            inner_span.in_scope(|| tracing::info!("Should have cloud trace fields"));
        })
    });

    println!("nested output -> {}", String::from_utf8_lossy(raw));

    let output = serde_json::from_slice::<MockEventWithCloudTraceFields>(raw)
        .expect("Error converting test buffer to JSON");

    assert_eq!(output.span_id, span_id, "Spans are not the same");
    assert_eq!(
        output.trace_id,
        format!("projects/{PROJECT_ID}/traces/{trace_id}")
    );
    assert!(output.trace_sampled)
}
