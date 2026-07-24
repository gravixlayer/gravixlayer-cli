// src/api/retry.rs — Exponential back-off with jitter.
//
// Mirrors the Python SDK formula exactly:
//
//   delay(attempt, rand_jitter, retry_after) =
//       if retry_after.is_some() { retry_after }
//       else { (1 << attempt) + rand_jitter }     // rand_jitter in [0.0, 1.0)
//
// attempt is 0-based; max_retries=3 allows attempts 0, 1, 2 (3 retries after
// the initial attempt = 4 total tries).

use std::future::Future;
use std::time::Duration;

use tracing::debug;

use super::error::ApiError;

const MAX_RETRIES: u32 = 3;
const MAX_DELAY_SECS: f64 = 32.0;

/// Compute the delay for retry attempt `attempt` (0-based).
///
/// If the server provided a `Retry-After` hint it is honoured as-is.
/// Otherwise: `delay = (1 << attempt) + uniform_jitter_in_[0,1)`.
pub fn next_retry_delay(attempt: u32, retry_after: Option<f64>) -> Duration {
    if let Some(ra) = retry_after {
        return Duration::from_secs_f64(ra.max(0.0).min(MAX_DELAY_SECS));
    }
    let base = (1u64 << attempt) as f64;
    let jitter: f64 = rand::random();
    let delay = (base + jitter).min(MAX_DELAY_SECS);
    Duration::from_secs_f64(delay)
}

/// Execute `f` with automatic retry on retryable `ApiError`s.
///
/// `f` is called with the current attempt number (0-based).  On failure the
/// delay is computed with `next_retry_delay`, the future is awaited with
/// `tokio::time::sleep`, then `f` is called again up to `max_retries` times.
pub async fn retry_with_backoff<F, Fut, T>(
    operation_name: &str,
    max_retries: u32,
    mut f: F,
) -> Result<T, ApiError>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    let mut attempt = 0u32;
    loop {
        match f(attempt).await {
            Ok(val) => return Ok(val),
            Err(err) if err.is_retryable() && attempt < max_retries => {
                let delay = next_retry_delay(attempt, err.retry_after_secs());
                debug!(
                    operation = operation_name,
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "retryable error — sleeping before retry"
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

/// Convenience wrapper using `MAX_RETRIES = 3`.
pub async fn retry<F, Fut, T>(operation_name: &str, f: F) -> Result<T, ApiError>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    retry_with_backoff(operation_name, MAX_RETRIES, f).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_increases_exponentially() {
        // Without jitter, base is 1 << attempt.
        // With jitter in [0,1), delay ∈ [base, base+1).
        for attempt in 0..4u32 {
            let base = (1u64 << attempt) as f64;
            let d = next_retry_delay(attempt, None);
            assert!(
                d.as_secs_f64() >= base,
                "attempt={attempt} delay={d:?} base={base}"
            );
            assert!(
                d.as_secs_f64() < base + 1.0,
                "attempt={attempt} delay={d:?} base={base}"
            );
        }
    }

    #[test]
    fn retry_after_header_overrides_formula() {
        let d = next_retry_delay(0, Some(5.5));
        assert!((d.as_secs_f64() - 5.5).abs() < 0.001);
    }

    #[test]
    fn delay_is_capped_at_max() {
        // 1 << 10 = 1024 — must be capped at MAX_DELAY_SECS.
        let d = next_retry_delay(10, None);
        assert!(d.as_secs_f64() <= MAX_DELAY_SECS);
    }

    #[tokio::test]
    async fn retry_succeeds_on_first_attempt() {
        let mut calls = 0u32;
        let result = retry("test", |_| {
            calls += 1;
            async { Ok::<u32, ApiError>(42) }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls, 1);
    }

    #[tokio::test]
    async fn retry_propagates_non_retryable_error_immediately() {
        let mut calls = 0u32;
        let result: Result<u32, ApiError> = retry("test", |_| {
            calls += 1;
            async {
                Err(ApiError::Auth {
                    message: "bad key".into(),
                })
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls, 1, "non-retryable should not be retried");
    }
}
