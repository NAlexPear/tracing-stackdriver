#![allow(clippy::disallowed_names)]
#![cfg(all(tracing_unstable, feature = "valuable"))]
use helpers::run_with_tracing;
use mocks::{MockDefaultEvent, MockHttpEvent};
use serde::Deserialize;
use std::fmt::Debug;
use tracing_stackdriver::LogSeverity;
use valuable::Valuable;

mod helpers;
mod mocks;

#[test]
fn handles_valuable_severity_override() {
    let events = run_with_tracing::<MockDefaultEvent>(|| {
        tracing::info!(
            severity = LogSeverity::Notice.as_value(),
            "notice me, senpai!"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.severity, "NOTICE");
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

    let events = run_with_tracing::<MockHttpEvent>(|| {
        tracing::info!(
            http_request = http_request.as_value(),
            "http_request testing"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(
        event.http_request.request_method,
        request_method.to_string()
    );
    assert_eq!(
        event.http_request.latency,
        format!("{}s", latency.as_secs_f32())
    );
    assert_eq!(event.http_request.status, status.as_u16());
    assert_eq!(event.http_request.remote_ip, remote_ip.to_string());
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

    let events = run_with_tracing::<MockStructuredEvent>(|| {
        tracing::info!(
            structured_log = structured_log.as_value(),
            "another message"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.structured_log, structured_log);
}
