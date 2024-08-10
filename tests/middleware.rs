use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use http::StatusCode;

use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use tracing::{debug, error};

use monzo_crawler::client_middleware::RetryTooManyRequestsMiddleware;
use tracing_test::traced_test;
use wiremock::{
    matchers::{method, path},
    Match, Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
#[traced_test(enable = true)]
async fn test_too_many_request_middleware() -> anyhow::Result<()> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);

    let client = ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent("monzo_crawler")
            .build()?,
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new(Duration::from_secs(1)))
    .build();

    struct AlreadyHitInner {
        count: u32,
        not_before: Option<SystemTime>,
        delay: Duration,
    }
    /// A match that counts the number of hits and only matches when the expected number of hits is reached.
    #[derive(Clone)]
    struct AlreadyHit {
        inner: Arc<Mutex<AlreadyHitInner>>,
        expected: u32,
    }

    impl AlreadyHit {
        fn new(n_hits: u32, delay: Duration) -> Self {
            Self {
                inner: Arc::new(Mutex::new(AlreadyHitInner {
                    count: 0,
                    not_before: None,
                    delay,
                })),
                expected: n_hits,
            }
        }

        /// Update the number of hits that [AlreadyHit] with match on.
        fn update_expected(&self, n_hits: u32) -> Self {
            let mut new = self.clone();
            new.expected = n_hits;
            new
        }
    }

    impl Match for AlreadyHit {
        fn matches(&self, _request: &wiremock::Request) -> bool {
            let mut guard = self.inner.lock().expect("Could not acquire lock");
            if guard.count == self.expected {
                guard.count += 1;

                // Make sure the request respects the Retry-After header.
                if let Some(not_before) = guard.not_before {
                    if let Ok(duration) = not_before.duration_since(SystemTime::now()) {
                        error!(
                            "Request did not respect Retry-After. Too early by: {:?}",
                            duration
                        );
                        return false;
                    } else {
                        debug!("Request respected Retry-After.");
                    }
                }
                guard.not_before = Some(SystemTime::now() + guard.delay);
                true
            } else {
                false
            }
        }
    }

    let hits = AlreadyHit::new(0, Duration::from_secs(1));
    let mock_server = MockServer::start().await;

    // First the server returns a 429, asking the client to slow down.
    Mock::given(method("GET"))
        .and(path("/go-fast"))
        .and(hits.clone())
        .respond_with(
            ResponseTemplate::new(StatusCode::TOO_MANY_REQUESTS)
                .append_header(reqwest::header::RETRY_AFTER, "1"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // One the second try the server returns a 200, asking the client to slow down.
    let hits_1 = hits.update_expected(1);
    Mock::given(method("GET"))
        .and(path("/go-fast"))
        .and(hits_1)
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock_server)
        .await;

    let _res = client
        .get(format!("{}/go-fast", &mock_server.uri()))
        .send()
        .await?;

    // ToDo: Assert that the Retry-After header was respected.
    Ok(())
}
