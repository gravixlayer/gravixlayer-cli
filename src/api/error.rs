// src/api/error.rs — Typed API error hierarchy.
//
// Five variants mirror the Python SDK's exception hierarchy:
//   GravixLayerAuthenticationError → ApiError::Auth         (HTTP 401)
//   GravixLayerRateLimitError       → ApiError::RateLimit    (HTTP 429)
//   GravixLayerBadRequestError      → ApiError::BadRequest   (HTTP 4xx except 401/429)
//   GravixLayerServerError          → ApiError::Server       (HTTP 5xx)
//   GravixLayerConnectionError      → ApiError::Connection   (network / TLS errors)
//
// Occupancy quota is HTTP 403 (not retried). Create-rate is HTTP 429 (retried).
// Mapping is by status only.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    /// HTTP 401 — API key is missing, revoked, or does not match any account.
    #[error("authentication failed: {message}")]
    Auth { message: String },

    /// HTTP 429 — Create-rate or request-rate window (retried automatically).
    ///
    /// `retry_after` is the `Retry-After` header in seconds when the caller
    /// supplied it. When absent the exponential back-off formula is used.
    #[error("rate limit exceeded{}: {message}", match retry_after { Some(s) => format!(" (retry after {s:.0}s)"), None => String::new() })]
    RateLimit {
        message: String,
        retry_after: Option<f64>,
    },

    /// HTTP 4xx except 401 and 429 — caller-side error, including quota 403.
    #[error("bad request (HTTP {status}): {body}")]
    BadRequest { status: u16, body: String },

    /// HTTP 5xx — server-side error.
    #[error("server error (HTTP {status}): {body}")]
    Server { status: u16, body: String },

    /// Network-level failure: DNS, TLS, connection refused, timeout, etc.
    #[error("connection error: {0}")]
    Connection(#[source] reqwest::Error),

    /// Received a successful HTTP status but the body could not be parsed.
    #[error("failed to deserialize API response: {0}")]
    Deserialization(#[source] reqwest::Error),
}

impl ApiError {
    /// Build the appropriate `ApiError` variant from an HTTP status and body.
    pub fn from_response(status: u16, body: String) -> Self {
        Self::from_response_headers(status, body, None)
    }

    /// Same mapping as [`from_response`], with a parsed `Retry-After` on 429.
    pub fn from_response_headers(status: u16, body: String, retry_after: Option<f64>) -> Self {
        match status {
            401 => Self::Auth { message: body },
            429 => Self::RateLimit {
                message: body,
                retry_after: retry_after.filter(|s| s.is_finite() && *s >= 0.0),
            },
            400..=499 => Self::BadRequest { status, body },
            500..=599 => Self::Server { status, body },
            s => Self::Server { status: s, body },
        }
    }

    /// Numeric `Retry-After` only. HTTP-date values are ignored (backoff applies).
    pub fn retry_after_from_headers(headers: &reqwest::header::HeaderMap) -> Option<f64> {
        headers
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|s| s.is_finite() && *s >= 0.0)
    }

    /// Map a failed HTTP response. Reads `Retry-After` before consuming the body.
    pub async fn from_http(resp: reqwest::Response) -> Self {
        let status = resp.status().as_u16();
        let retry_after = Self::retry_after_from_headers(resp.headers());
        let body = resp.text().await.unwrap_or_default();
        Self::from_response_headers(status, body, retry_after)
    }

    /// Retryable: 429, 502/503/504, and connection errors.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Server { status, .. } => matches!(status, 502 | 503 | 504),
            Self::RateLimit { .. } => true,
            Self::Connection(_) => true,
            _ => false,
        }
    }

    /// Extract a retry-after hint in seconds (for rate-limit responses).
    pub fn retry_after_secs(&self) -> Option<f64> {
        match self {
            Self::RateLimit { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_status() {
            let status = e.status().map(|s| s.as_u16()).unwrap_or(0);
            Self::from_response(status, e.to_string())
        } else {
            Self::Connection(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_status_only() {
        assert!(matches!(
            ApiError::from_response(429, "{}".into()),
            ApiError::RateLimit { .. }
        ));
        assert!(ApiError::from_response(429, "anything".into()).is_retryable());
        let forbidden = ApiError::from_response(403, "anything".into());
        assert!(matches!(
            forbidden,
            ApiError::BadRequest { status: 403, .. }
        ));
        assert!(!forbidden.is_retryable());
        assert!(!ApiError::from_response(401, "no".into()).is_retryable());
        assert!(!ApiError::from_response(402, "empty wallet".into()).is_retryable());
        let limited = ApiError::from_response_headers(429, "slow".into(), Some(1.0));
        assert_eq!(limited.retry_after_secs(), Some(1.0));
        assert!(limited.is_retryable());
        let ignored = ApiError::from_response_headers(403, "no".into(), Some(1.0));
        assert!(ignored.retry_after_secs().is_none());
        assert!(!ignored.is_retryable());
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "1".parse().unwrap());
        assert_eq!(ApiError::retry_after_from_headers(&headers), Some(1.0));
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(ApiError::retry_after_from_headers(&headers), None);
        headers.insert(reqwest::header::RETRY_AFTER, "-2".parse().unwrap());
        assert_eq!(ApiError::retry_after_from_headers(&headers), None);
    }
}
