//! Retry logic with exponential backoff for Mass API HTTP calls.
//!
//! Retries only on transient transport errors (connection failures, timeouts).
//! Non-retryable errors (4xx responses, deserialization failures) are returned
//! immediately without retry.

use std::time::Duration;

/// Maximum number of retry attempts after the initial request.
const MAX_RETRIES: u32 = 3;

/// Base delay between retries (doubles each attempt: 200ms, 400ms, 800ms).
const BASE_DELAY_MS: u64 = 200;

/// Send an HTTP request with exponential backoff retry on transport errors.
///
/// The closure `f` is called up to `MAX_RETRIES + 1` times. Only
/// [`reqwest::Error`] transport failures trigger a retry — the caller is
/// responsible for inspecting the response status code.
///
/// Delays: 200ms → 400ms → 800ms between retries.
pub(crate) async fn retry_send<F, Fut>(f: F) -> Result<reqwest::Response, reqwest::Error>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    // Retry attempts with backoff, then one final attempt without retry.
    for attempt in 0..MAX_RETRIES {
        match f().await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                let delay = Duration::from_millis(BASE_DELAY_MS * 2u64.pow(attempt));
                tracing::warn!(
                    attempt = attempt + 1,
                    max_retries = MAX_RETRIES,
                    "Mass API HTTP request failed, retrying in {delay:?}: {e}"
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
    // Final attempt — no more retries.
    f().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn retry_exhausts_all_attempts_on_transport_failure() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = call_count.clone();

        let result = retry_send(|| {
            let cc = cc.clone();
            async move {
                cc.fetch_add(1, Ordering::SeqCst);
                // Request to a guaranteed-closed port → connection refused.
                reqwest::Client::builder()
                    .timeout(Duration::from_millis(50))
                    .build()
                    .unwrap()
                    .get("http://127.0.0.1:1/")
                    .send()
                    .await
            }
        })
        .await;

        assert!(result.is_err(), "request to closed port must fail");
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            MAX_RETRIES + 1,
            "should exhaust all retry attempts"
        );
    }
}
