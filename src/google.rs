use serde::Serialize;
use std::{convert::Infallible, fmt, str::FromStr};
use tracing_core::Level;

/// The severity of the event described in a log entry, expressed as standard severity levels.
/// [See Google's LogSeverity docs here](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#LogSeverity).
#[cfg_attr(
    all(tracing_unstable, feature = "valuable"),
    derive(valuable::Valuable)
)]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogSeverity {
    /// Log entry has no assigned severity level
    #[default]
    Default,
    /// Debug or trace information
    Debug,
    /// Routine information, such as ongoing status or performance
    Info,
    /// Normal but significant events, such as start up, shut down, or a configuration change
    Notice,
    /// Warning events might cause problems
    Warning,
    /// Error events are likely to cause problems
    Error,
    /// Critical events cause more severe problems or outages
    Critical,
    /// A person must take an action immediately
    Alert,
    /// One or more systems are unusable
    Emergency,
}

impl fmt::Display for LogSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output = match self {
            Self::Default => "DEFAULT",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Notice => "NOTICE",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
            Self::Alert => "ALERT",
            Self::Emergency => "EMERGENCY",
        };

        formatter.write_str(output)
    }
}

impl From<&Level> for LogSeverity {
    fn from(level: &Level) -> Self {
        match level {
            &Level::DEBUG | &Level::TRACE => Self::Debug,
            &Level::INFO => Self::Info,
            &Level::WARN => Self::Warning,
            &Level::ERROR => Self::Error,
        }
    }
}

impl FromStr for LogSeverity {
    type Err = Infallible;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let severity = match string.to_lowercase().as_str() {
            "debug" | "trace" => Self::Debug,
            "info" => Self::Info,
            "notice" => Self::Notice,
            "warn" | "warning" => Self::Warning,
            "error" => Self::Error,
            "critical" => Self::Critical,
            "alert" => Self::Alert,
            "emergency" => Self::Emergency,
            _ => Self::Default,
        };

        Ok(severity)
    }
}

impl From<serde_json::Value> for LogSeverity {
    fn from(json: serde_json::Value) -> Self {
        // handle simple string inputs
        if let Some(str) = json.as_str() {
            return Self::from_str(str).unwrap_or(Self::Default);
        }

        // handle wacky object encoding of Valuable enums
        #[cfg(all(tracing_unstable, feature = "valuable"))]
        if let Some(map) = json.as_object() {
            if let Some(key) = map.keys().next() {
                return Self::from_str(key).unwrap_or(Self::Default);
            }
        }

        Self::Default
    }
}

/// Typechecked HttpRequest structure for stucturally logging information about a request.
/// [See Google's HttpRequest docs here](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry#HttpRequest).
#[cfg_attr(docsrs, doc(cfg(feature = "valuable")))]
#[cfg(any(docsrs, all(tracing_unstable, feature = "valuable")))]
#[derive(Default)]
pub struct HttpRequest {
    /// Valid HTTP Method for the request (e.g. GET, POST, etc)
    pub request_method: Option<http::Method>,
    /// URL from the HTTP request
    pub request_url: Option<url::Url>,
    /// Size of the HTTP request in bytes
    pub request_size: Option<u32>,
    /// Size of the HTTP response in bytes
    pub response_size: Option<u32>,
    /// Valid HTTP StatusCode for the response
    pub status: Option<http::StatusCode>,
    /// User Agent string of the request
    pub user_agent: Option<String>,
    /// IP address of the client that issued the request
    pub remote_ip: Option<std::net::IpAddr>,
    /// IP address of the server that the request was sent to
    pub server_ip: Option<std::net::IpAddr>,
    /// Referer URL of the request, as defined in HTTP/1.1 Header Field Definitions
    pub referer: Option<url::Url>,
    /// Processing latency on the server, from the time the request was received until the response was sent
    pub latency: Option<std::time::Duration>,
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

#[cfg_attr(docsrs, doc(cfg(feature = "valuable")))]
#[cfg(any(docsrs, all(tracing_unstable, feature = "valuable")))]
impl HttpRequest {
    /// Generate a new log-able HttpRequest structured log entry
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(all(tracing_unstable, feature = "valuable"))]
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

#[cfg_attr(docsrs, doc(cfg(feature = "valuable")))]
#[cfg(any(docsrs, all(tracing_unstable, feature = "valuable")))]
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

#[cfg_attr(docsrs, doc(cfg(feature = "valuable")))]
#[cfg(any(docsrs, all(tracing_unstable, feature = "valuable")))]
impl valuable::Structable for HttpRequest {
    fn definition(&self) -> valuable::StructDef<'_> {
        valuable::StructDef::new_dynamic("HttpRequest", valuable::Fields::Named(&[]))
    }
}
