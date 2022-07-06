use http::{method::Method, status::StatusCode};
use std::{collections::BTreeMap, net::IpAddr, time::Duration};
use url::Url;

// TODO:
// 1. define the entire "simplified JSON log entry" for conversion into a real LogEntry
// 2. define a conversion from JsonValues to the simplified log entry
// 3. implement serialization for the simplified log entry
// 4. expose relevant helper structs, too
// 5. come up with a better module name

/// https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry
struct LogEntry {
    http_request: Option<HttpRequest>,
    labels: BTreeMap<String, String>,
    message: Option<String>,
    severity: LogSeverity,
    span_id: Option<String>,
}

/// https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity
enum LogSeverity {
    Default,
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Typechecked HttpRequest structure for stucturally logging information about a request
/// https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#HttpRequest
#[derive(Default)]
pub struct HttpRequest {
    /// Valid HTTP Method for the request (e.g. GET, POST, etc)
    pub request_method: Option<Method>,
    /// URL from the HTTP request
    pub request_url: Option<Url>,
    /// Size of the HTTP request in bytes
    pub request_size: Option<u32>,
    /// Size of the HTTP response in bytes
    pub response_size: Option<u32>,
    /// Valid HTTP StatusCode for the response
    pub status: Option<StatusCode>,
    /// User Agent string of the request
    pub user_agent: Option<String>,
    /// IP address of the client that issued the request
    pub remote_ip: Option<IpAddr>,
    /// IP address of the server that the request was sent to
    pub server_ip: Option<IpAddr>,
    /// Referer URL of the request, as defined in HTTP/1.1 Header Field Definitions
    pub referer: Option<Url>,
    /// Processing latency on the server, from the time the request was received until the response was sent
    pub latency: Option<Duration>,
    /// Whether or not a cache lookup was attempted
    pub cache_lookup: Option<bool>,
    /// Whether or not an entity was served from cache (with or without validation)
    pub cache_hit: Option<bool>,
    /// Whether or not the response was validated with the origin server before being served from cache
    pub cache_validated_with_origin_server: Option<bool>,
    /// Number of HTTP response bytes inserted into cache
    pub cache_fill_bytes: Option<u32>,
    /// Protocol used for the request (e.g. "HTTP/1.1", "HTTP/2", "websocket")
    pub protocol: Option<String>,
}

impl HttpRequest {
    /// Generate a new log-able HttpRequest structured log entry
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(tracing_unstable)]
static HTTP_REQUEST_FIELDS: &[valuable::NamedField<'static>] = &[
    valuable::NamedField::new("requestMethod"),
    valuable::NamedField::new("requestUrl"),
    valuable::NamedField::new("requestSize"),
    valuable::NamedField::new("responseSize"),
    valuable::NamedField::new("status"),
    valuable::NamedField::new("userAgent"),
    valuable::NamedField::new("remoteIp"),
    valuable::NamedField::new("serverIp"),
    valuable::NamedField::new("referer"),
    valuable::NamedField::new("latency"),
    valuable::NamedField::new("cacheLookup"),
    valuable::NamedField::new("cacheHit"),
    valuable::NamedField::new("cacheValidatedWithOriginServer"),
    valuable::NamedField::new("cacheFillBytes"),
    valuable::NamedField::new("protocol"),
];

#[cfg(tracing_unstable)]
impl valuable::Valuable for HttpRequest {
    fn as_value(&self) -> valuable::Value<'_> {
        valuable::Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        let request_method = self
            .request_method
            .as_ref()
            .map(|method| method.to_string());
        let request_url = self.request_url.as_ref().map(|url| url.to_string());
        let status = self.status.map(|status| status.as_u16());
        let user_agent = &self.user_agent;
        let remote_ip = self.remote_ip.map(|ip| ip.to_string());
        let server_ip = self.server_ip.map(|ip| ip.to_string());
        let referer = self.referer.as_ref().map(|url| url.to_string());
        let latency = self
            .latency
            .map(|latency| format!("{}s", latency.as_secs_f32()));

        let (fields, values): (Vec<_>, Vec<_>) = HTTP_REQUEST_FIELDS
            .iter()
            .zip(
                [
                    request_method.as_ref().map(valuable::Valuable::as_value),
                    request_url.as_ref().map(valuable::Valuable::as_value),
                    self.request_size.as_ref().map(valuable::Valuable::as_value),
                    self.response_size
                        .as_ref()
                        .map(valuable::Valuable::as_value),
                    status.as_ref().map(valuable::Valuable::as_value),
                    user_agent.as_ref().map(valuable::Valuable::as_value),
                    remote_ip.as_ref().map(valuable::Valuable::as_value),
                    server_ip.as_ref().map(valuable::Valuable::as_value),
                    referer.as_ref().map(valuable::Valuable::as_value),
                    latency.as_ref().map(valuable::Valuable::as_value),
                    self.cache_lookup.as_ref().map(valuable::Valuable::as_value),
                    self.cache_hit.as_ref().map(valuable::Valuable::as_value),
                    self.cache_validated_with_origin_server
                        .as_ref()
                        .map(valuable::Valuable::as_value),
                    self.cache_fill_bytes
                        .as_ref()
                        .map(valuable::Valuable::as_value),
                    self.protocol.as_ref().map(valuable::Valuable::as_value),
                ]
                .iter(),
            )
            .filter_map(|(field, value)| value.map(|value| (field, value)))
            .unzip();

        visit.visit_named_fields(&valuable::NamedValues::new(&fields, &values));
    }
}

#[cfg(tracing_unstable)]
impl valuable::Structable for HttpRequest {
    fn definition(&self) -> valuable::StructDef<'_> {
        valuable::StructDef::new_dynamic("HttpRequest", valuable::Fields::Named(&[]))
    }
}
