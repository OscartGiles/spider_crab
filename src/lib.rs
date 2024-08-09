pub mod client_middleware;
mod crawler;
mod parser;
pub use crawler::{Crawler, PageContent, SiteVisitor};
pub use parser::parse_links;

#[cfg(test)]
mod test {
    use std::time::Duration;

    use reqwest_middleware::ClientBuilder;
    use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
    use reqwest_tracing::TracingMiddleware;

    #[tokio::test]
    async fn test_client() {
        // Retry up to 3 times with increasing intervals between attempts.
        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(Duration::from_millis(500), Duration::from_secs(10))
            .build_with_max_retries(5);

        let client = ClientBuilder::new(
            reqwest::Client::builder()
                .user_agent("Oscar/Giles")
                .build()
                .unwrap(),
        )
        // Trace HTTP requests. See the tracing crate to make use of these traces.
        .with(TracingMiddleware::default())
        // Retry failed requests.
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        let res = client.get("https://monzo.com").send().await.unwrap();

        println!("{:?}", res.text().await.unwrap());
        // run(client).await;
    }
}
