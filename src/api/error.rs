// src/api/error.rs — Typed API error hierarchy.
//
// Five variants mirror the Python SDK's exception hierarchy:
//   GravixLayerAuthenticationError → ApiError::Auth         (HTTP 401)
//   GravixLayerRateLimitError       → ApiError::RateLimit    (HTTP 429, transient)
//   GravixLayerBadRequestError      → ApiError::BadRequest   (HTTP 4xx except 401/429,
//                                                              and quota-exceeded 429)
//   GravixLayerServerError          → ApiError::Server       (HTTP 5xx)
//   GravixLayerConnectionError      → ApiError::Connection   (network / TLS errors)
//
// NOTE: HTTP 429 from the backend can mean two distinct things:
//   • Transient rate limit  → retryable, mapped to ApiError::RateLimit
//   • Quota exhausted       → NOT retryable, mapped to ApiError::BadRequest(429)
//     The backend signals quota errors by including an `"exceeded"` array in the
//     JSON body (e.g. `{"exceeded":["vcpu"],...}`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    /// HTTP 401 — API key is missing, revoked, or does not match any account.
    #[error("authentication failed: {message}")]
    Auth { message: String },

    /// HTTP 429 — Transient rate limit exceeded (will be retried automatically).
    ///
    /// `retry_after` is the value of the `Retry-After` response header parsed as
    /// seconds.  When absent the exponential back-off formula is used instead.
    #[error("rate limit exceeded{}: {message}", match retry_after { Some(s) => format!(" (retry after {s:.0}s)"), None => String::new() })]
    RateLimit {
        message: String,
        retry_after: Option<f64>,
    },

    /// HTTP 4xx (except 401 and transient 429) — caller-side error.
    ///
    /// This also covers quota-exhausted 429 responses (identified by an
    /// `"exceeded"` field in the JSON body), which are permanent and not retried.
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
    /// Build the appropriate `ApiError` variant from an HTTP response status and body.
    pub fn from_response(status: u16, body: String) -> Self {
        match status {
            401 => Self::Auth { message: body },
            429 => {
                // Parse the body to distinguish quota exhaustion from a transient rate limit.
                // Quota errors contain an `"exceeded"` array (e.g. `{"exceeded":["vcpu"]}`).
                // Transient rate limits may contain a `"retry_after"` hint in seconds.
                let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();

                let is_quota_exceeded = parsed
                    .as_ref()
                    .and_then(|v| v.get("exceeded"))
                    .map(|v| v.is_array())
                    .unwrap_or(false);

                if is_quota_exceeded {
                    // Quota errors are permanent — no point retrying.
                    return Self::BadRequest { status: 429, body };
                }

                let retry_after = parsed.and_then(|v| v.get("retry_after")?.as_f64());

                Self::RateLimit {
                    message: body,
                    retry_after,
                }
            }
            400..=499 => Self::BadRequest { status, body },
            500..=599 => Self::Server { status, body },
            s => Self::Server { status: s, body },
        }
    }

    /// Returns `true` for errors that should trigger a retry attempt.
    ///
    /// Retryable: transient 429 (RateLimit), 502/503/504 (Server), connection errors.
    /// NOT retryable: quota-exceeded 429 (mapped to BadRequest), 4xx, 401, 5xx other.
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
            // reqwest gives us status errors here — but we normally construct
            // `from_response` ourselves after reading the body.  This branch is
            // a safe fallback.
            let status = e.status().map(|s| s.as_u16()).unwrap_or(0);
            Self::from_response(status, e.to_string())
        } else {
            Self::Connection(e)
        }
    }
}
