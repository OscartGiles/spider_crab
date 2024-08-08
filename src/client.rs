use http::{Extensions, StatusCode};
use reqwest::{Request, Response};
use reqwest_middleware::{Error, Middleware, Next, Result};
use std::time::{Duration, SystemTime};

/// A middleware that retries requests based on the `Retry-After` header.
/// This middleware will sleep until the `Retry-After` time has passed.
///
/// Internally uses the `tokio::sync::RwLock` instead of the std lib `RwLock`. This is because it is write-preferring.
struct RetryTooManyRequestsMiddleware {
    retry_after: tokio::sync::RwLock<Option<SystemTime>>,
}

impl RetryTooManyRequestsMiddleware {
    fn new() -> Self {
        Self {
            retry_after: tokio::sync::RwLock::new(None),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for RetryTooManyRequestsMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let retry_after = *self.retry_after.read().await;

        if let Some(retry_after) = retry_after {
            let now = SystemTime::now();

            // Sleep until the retry_after time.
            if let Ok(duration) = retry_after.duration_since(now) {
                tokio::time::sleep(duration).await
            }
        }

        let result = next.clone().run(req, extensions).await;
        println!("TOO MANY REQUESTS");

        println!("{:?}", result);
        if let Ok(resp) = result.as_ref() {
            if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                if let Some(header) = resp.headers().get(reqwest::header::RETRY_AFTER) {
                    println!("TOO MANY REQUESTS");
                    let retry_after = match header.to_str() {
                        Ok(s) => match s.parse::<u64>() {
                            Ok(seconds) => Some(SystemTime::now() + Duration::from_secs(seconds)),
                            Err(_) => None,
                        },
                        Err(_) => None,
                    };

                    *self.retry_after.write().await = retry_after;
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod test {
    use std::{
        path::is_separator,
        sync::{Arc, Mutex},
    };

    use http::{header, StatusCode};

    use reqwest_middleware::ClientBuilder;
    use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

    use wiremock::{
        matchers::{method, path},
        Match, Mock, MockServer, ResponseTemplate,
    };

    use crate::client::RetryTooManyRequestsMiddleware;

    #[tokio::test]
    async fn test_middleware() {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);

        let client = ClientBuilder::new(
            reqwest::Client::builder()
                .user_agent("monzo_crawler")
                .build()
                .unwrap(),
        )
        .with(RetryTooManyRequestsMiddleware::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        #[derive(Clone)]
        pub struct AlreadyHit {
            count: Arc<Mutex<u32>>,
            expected: u32,
        }

        impl AlreadyHit {
            fn new() -> Self {
                Self {
                    count: Arc::new(Mutex::new(0)),
                    expected: 0,
                }
            }

            fn increment_expected(&mut self) {
                self.expected += 1;
            }
        }

        impl Match for AlreadyHit {
            fn matches(&self, request: &wiremock::Request) -> bool {
                let mut guard = self.count.lock().expect("Could not acquire lock");
                if *guard == self.expected {
                    *guard += 1;
                    true
                } else {
                    false
                }
            }
        }

        let mut hits = AlreadyHit::new();
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/go-fast"))
            .and(hits.clone())
            .respond_with(
                ResponseTemplate::new(StatusCode::TOO_MANY_REQUESTS)
                    .append_header(reqwest::header::RETRY_AFTER, "5"),
            )
            .expect(1)
            // Mounting the mock on the mock server - it's now effective!
            .mount(&mock_server)
            .await;

        hits.increment_expected();

        Mock::given(method("GET"))
            .and(path("/go-fast"))
            .and(hits.clone())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            // Mounting the mock on the mock server - it's now effective!
            .mount(&mock_server)
            .await;

        println!("{:?}", hits.count.lock().unwrap());

        let res = client
            .get(format!("{}/go-fast", &mock_server.uri()))
            .send()
            .await
            .unwrap();

        println!("{:?}", hits.count.lock().unwrap());

        // let res = client
        //     .get(format!("{}/go-fast", &mock_server.uri()))
        //     .send()
        //     .await
        //     .unwrap();

        // println!("{:?}", hits.count.lock().unwrap());

        // let res = client.get(server.url("/go-fast")).send().await.unwrap();

        // println!("{:?}", res.status());
        // mock_server.assert_hits_async(1).await;
    }
}
