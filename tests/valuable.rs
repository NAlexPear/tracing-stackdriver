#![allow(clippy::disallowed_names)]
#![cfg(all(tracing_unstable, feature = "valuable"))]
use lazy_static::lazy_static;
use mocks::{MockDefaultEvent, MockHttpEvent};
use serde::Deserialize;
use std::{fmt::Debug, sync::Mutex};
use tracing_stackdriver::LogSeverity;
use tracing_subscriber::{layer::SubscriberExt, Registry};
use valuable::Valuable;

mod helpers;
mod mocks;
mod writer;

#[test]
fn handles_valuable_severity_override() {
    let output = serde_json::from_slice::<MockDefaultEvent>(run_with_tracing!(|| tracing::info!(
        severity = LogSeverity::Notice.as_value(),
        "notice me, senpai!"
    )))
    .expect("Error converting test buffer to JSON");

    assert_eq!(output.severity, "NOTICE");
}

#[test]
fn validates_structured_http_requests() {
    let request_method = http::Method::GET;
    let latency = std::time::Duration::from_millis(1234);
    let status = http::StatusCode::OK;
    let remote_ip = std::net::IpAddr::from([127, 0, 0, 1]);

    let http_request = tracing_stackdriver::HttpRequest {
        request_method: Some(request_method.clone()),
        latency: Some(latency),
        status: Some(status),
        remote_ip: Some(remote_ip),
        ..Default::default()
    };

    let output = serde_json::from_slice::<MockHttpEvent>(run_with_tracing!(|| tracing::info!(
        http_request = http_request.as_value(),
        "http_request testing"
    )))
    .expect("Error converting test buffer to JSON");

    assert_eq!(
        output.http_request.request_method,
        request_method.to_string()
    );

    assert_eq!(
        output.http_request.latency,
        format!("{}s", latency.as_secs_f32())
    );

    assert_eq!(output.http_request.status, status.as_u16());
    assert_eq!(output.http_request.remote_ip, remote_ip.to_string());
}

#[derive(Debug, Deserialize, Valuable, PartialEq)]
struct StructuredLog {
    foo: String,
    bar: std::collections::BTreeMap<String, u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MockStructuredEvent {
    structured_log: StructuredLog,
}

#[test]
fn includes_valuable_structures() {
    let foo = "testing".to_string();
    let mut bar = std::collections::BTreeMap::new();
    bar.insert("baz".into(), 123);
    let structured_log = StructuredLog { foo, bar };
    let output =
        serde_json::from_slice::<MockStructuredEvent>(run_with_tracing!(|| tracing::info!(
            structured_log = structured_log.as_value(),
            "another message"
        )))
        .expect("Error converting test buffer to JSON");

    assert_eq!(output.structured_log, structured_log);
}
