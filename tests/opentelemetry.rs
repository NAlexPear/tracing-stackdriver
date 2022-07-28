#![cfg(feature = "opentelemetry")]

use lazy_static::lazy_static;
use serde::Deserialize;
use std::{fmt::Debug, sync::Mutex};
use tracing_subscriber::layer::SubscriberExt;

mod common;

macro_rules! run_with_tracing {
    (|| $expression:expr) => {{
        lazy_static! {
            static ref BUFFER: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || crate::common::MockWriter(&BUFFER);
        let stackdriver = tracing_stackdriver::layer()
            .with_writer(make_writer)
            .with_project_id("my_project_123".into());
        let subscriber = tracing_subscriber::Registry::default()
            .with(tracing_opentelemetry::layer())
            .with(stackdriver);

        tracing::subscriber::with_default(subscriber, || $expression);

        &BUFFER
            .try_lock()
            .expect("Couldn't get lock on test write target")
            .to_vec()
    }};
}

#[derive(Debug, Deserialize)]
struct MockEventWithCloudTraceFields {
    #[serde(rename = "logging.googleapis.com/spanId")]
    span_id: Option<String>,
    #[serde(rename = "logging.googleapis.com/trace")]
    trace_id: Option<String>,
    #[serde(rename = "logging.googleapis.com/trace_sampled")]
    trace_sampled: Option<bool>,
}

#[test]
fn includes_cloud_trace_fields() {
    use opentelemetry::trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState,
    };

    use tracing_opentelemetry::OpenTelemetrySpanExt;
    let output = run_with_tracing!(|| {
        let cx = opentelemetry::Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex("0af7651916cd43dd8448eb211c80319c").unwrap(),
            SpanId::from_hex("b7ad6b7169203331").unwrap(),
            TraceFlags::SAMPLED,
            true,
            TraceState::from_key_value([("", ""); 0]).unwrap(),
        ));
        let root_span = tracing::info_span!("root_span");
        root_span.set_parent(cx);
        root_span.in_scope(|| {
            tracing::info!("Should have cloud trace fields");
        });
    });

    let output = serde_json::from_slice::<MockEventWithCloudTraceFields>(output)
        .expect("Error converting test buffer to JSON");

    assert!(matches!(
        output.span_id.as_deref().map(SpanId::from_hex),
        Some(Ok(_))
    ));
    assert_eq!(
        output.trace_id.as_deref(),
        Some("projects/my_project_123/traces/0af7651916cd43dd8448eb211c80319c")
    );
    assert_eq!(output.trace_sampled, Some(true))
}

#[test]
fn includes_cloud_trace_fields_in_nested_span() {
    use opentelemetry::trace::{
        SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState,
    };
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let output = run_with_tracing!(|| {
        let cx = opentelemetry::Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex("0af7651916cd43dd8448eb211c80319c").unwrap(),
            SpanId::from_hex("b7ad6b7169203331").unwrap(),
            TraceFlags::default(),
            true,
            TraceState::from_key_value([("", ""); 0]).unwrap(),
        ));
        let root_span = tracing::info_span!("root_span");
        root_span.set_parent(cx);
        root_span.in_scope(|| {
            let inner_span = tracing::info_span!("inner_span");
            inner_span.in_scope(|| {
                tracing::info!("Should have cloud trace fields");
            });
        })
    });

    let output = serde_json::from_slice::<MockEventWithCloudTraceFields>(output)
        .expect("Error converting test buffer to JSON");

    assert!(matches!(
        output.span_id.as_deref().map(SpanId::from_hex),
        Some(Ok(_))
    ));
    assert_eq!(
        output.trace_id.as_deref(),
        Some("projects/my_project_123/traces/0af7651916cd43dd8448eb211c80319c")
    );
    assert!(!matches!(output.trace_sampled, Some(true)))
}
