use monzo_crawler::{
    client_middleware::{MaxConcurrentMiddleware, RetryTooManyRequestsMiddleware},
    Crawler, PageContent, SiteVisitor,
};
use owo_colors::OwoColorize;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use tokio::time::Instant;
use url::Url;

struct ClientWithMiddlewareVisitor {
    client: ClientWithMiddleware,
}

impl ClientWithMiddlewareVisitor {
    fn new(client: ClientWithMiddleware) -> Self {
        Self { client }
    }
}

impl SiteVisitor for ClientWithMiddlewareVisitor {
    async fn visit(&mut self, url: url::Url) -> PageContent {
        let response = self.client.get(url.as_str()).send().await.unwrap();

        let status_code = response.status();
        let content = response.text().await.unwrap();

        PageContent {
            content,
            status_code,
            url,
        }
    }
}

#[tokio::main]
async fn main() {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);

    let client = ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent("monzo_crawler")
            .build()
            .unwrap(),
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new())
    .with(MaxConcurrentMiddleware::new(10))
    .build();

    let reqwest_visitor = ClientWithMiddlewareVisitor::new(client);

    let crawler = Crawler::new(reqwest_visitor, "monzo-crawler", None);

    let start = Instant::now();
    let res = crawler
        .crawl(Url::parse("https://rsseau.fr").unwrap())
        .await;
    let duration = start.elapsed();

    println!("{}", res);
    println!(
        "Time elapsed in website.crawl() is: {:?} for total pages: {}",
        duration,
        res.0.len().green()
    );
}
