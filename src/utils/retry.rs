// Retry utilities
// TODO: Implement retry with exponential backoff

use std::time::Duration;
use tokio::time::sleep;

pub async fn with_retry<F, T, E>(
    mut operation: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(error);
                }

                let delay = Duration::from_secs(2u64.pow(attempt.min(5)));
                sleep(delay).await;
            }
        }
    }
}
