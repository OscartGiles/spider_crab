use http::{Extensions, StatusCode};
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};
use std::{
    fmt::{self},
    time::{Duration, SystemTime},
};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

/// A middleware that delays the next request if a `Retry-After` header is received.
/// It does not retry the requests on its own. It can be used in conjunction with a retry middleware (see example).
///
/// # Example
/// The following combines [RetryTooManyRequestsMiddleware] with [reqwest_retry::RetryTransientMiddleware] to retry requests.
/// It will retry as per the [reqwest_retry::RetryTransientMiddleware] but will increase the delay time to respect requests by the server to slow down.
/// ```rust
/// use monzo_crawler::client_middleware::RetryTooManyRequestsMiddleware;
/// use reqwest_retry::RetryTransientMiddleware;
/// use reqwest_retry::policies::ExponentialBackoff;
/// use reqwest_middleware::ClientBuilder;
///
/// let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);
///
/// let client = ClientBuilder::new(
///  reqwest::Client::builder()
///    .user_agent("monzo_crawler")
///     .build()
///     .unwrap(),
/// )
///     .with(RetryTransientMiddleware::new_with_policy(retry_policy))
///     .with(RetryTooManyRequestsMiddleware::new())
///     .build();
/// ````
pub struct RetryTooManyRequestsMiddleware {
    retry_after: tokio::sync::RwLock<Option<SystemTime>>,
}

impl std::fmt::Debug for RetryTooManyRequestsMiddleware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryTooManyRequestsMiddleware")
            .field("retry_after", &self.retry_after)
            .finish()
    }
}

impl RetryTooManyRequestsMiddleware {
    pub fn new() -> Self {
        Self {
            retry_after: tokio::sync::RwLock::new(None),
        }
    }
}
impl Default for RetryTooManyRequestsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for RetryTooManyRequestsMiddleware {
    #[tracing::instrument(name = "slow_down_requests", skip_all)]
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let retry_after = *self.retry_after.read().await;

        if let Some(retry_after) = retry_after {
            let now = SystemTime::now();
            if let Ok(duration) = retry_after.duration_since(now) {
                info!("Sleeping for {:?}", duration);
                tokio::time::sleep(duration).await;
            } else {
                *self.retry_after.write().await = None;
            }
        }

        let result = next.clone().run(req, extensions).await;

        if let Ok(resp) = result.as_ref() {
            if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                if let Some(header) = resp.headers().get(reqwest::header::RETRY_AFTER) {
                    debug!("Server requested slowdown.");
                    let retry_after = match header.to_str() {
                        Ok(s) => match s.parse::<u64>() {
                            Ok(mut seconds) => {
                                if seconds > 60 {
                                    warn!("Retry-After header is greater than 60 seconds.");
                                    seconds = 60;
                                }
                                let retry_after = SystemTime::now() + Duration::from_secs(seconds);

                                Some(retry_after)
                            }
                            Err(e) => {
                                warn!(
                                    "Could not parse Retry-After header as integer: {}. Error: {}",
                                    s, e
                                );
                                None
                            }
                        },
                        Err(e) => {
                            warn!("Invalid Retry-After header. Contains non ASCII characters. Error: {}", e);
                            None
                        }
                    };

                    *self.retry_after.write().await = retry_after;
                }
            }
        }
        result
    }
}

pub struct MaxConcurrentMiddleware {
    semaphore: Semaphore,
}

impl std::fmt::Debug for MaxConcurrentMiddleware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaxConcurrentMiddleware")
            .field("available_permits", &self.semaphore.available_permits())
            .finish()
    }
}

impl MaxConcurrentMiddleware {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
        }
    }
}

/// A middleware that limits the number of concurrent requests being made by the client.
#[async_trait::async_trait]
impl Middleware for MaxConcurrentMiddleware {
    #[tracing::instrument(skip(req, extensions, next))]
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("Could not acquire semaphore because it was closed. This is a bug."); // Permit released on drop.
        debug!(
            "Acquired semaphore permit. Available permits: {}",
            self.semaphore.available_permits()
        );
        next.clone().run(req, extensions).await
    }
}
