use helpers::run_with_tracing;
use mocks::{MockHttpEvent, MockHttpRequest};

mod helpers;
mod mocks;

#[test]
fn nests_http_request() {
    let request_method = "GET";
    let latency = "0.23s";
    let remote_ip = "192.168.1.1";
    let status = 200;

    let mock_http_request = MockHttpRequest {
        request_method: request_method.to_string(),
        latency: latency.to_string(),
        remote_ip: remote_ip.to_string(),
        status,
    };

    let events = run_with_tracing::<MockHttpEvent>(|| {
        tracing::info!(
            http_request.request_method = &request_method,
            http_request.latency = &latency,
            http_request.remote_ip = &remote_ip,
            http_request.status = &status,
            "some stackdriver message"
        )
    })
    .expect("Error converting test buffer to JSON");

    let event = events.first().expect("No event heard");
    assert_eq!(event.http_request, mock_http_request);
}
