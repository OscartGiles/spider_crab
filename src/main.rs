use monzo_crawler::{
    client_middleware::{MaxConcurrentMiddleware, RetryTooManyRequestsMiddleware},
    Crawler, PageContent, SiteVisitor,
};
// use opentelemetry::{trace::TracerProvider as _, KeyValue};
// use opentelemetry_otlp::WithExportConfig;
// use opentelemetry_sdk::{trace::Config, Resource};
use owo_colors::OwoColorize;
// use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
// use reqwest_tracing::TracingMiddleware;
use tokio::time::Instant;
use tracing::info;

use url::Url;

// use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Clone, Debug)]
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
        let mut headers = response.headers().clone();

        let content_type = headers.remove("Content-Type");
        let content = response.text().await.unwrap();

        PageContent {
            content,
            status_code,
            url,
            content_type,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let tracer = opentelemetry_otlp::new_pipeline()
    //     .tracing()
    //     .with_exporter(
    //         opentelemetry_otlp::new_exporter()
    //             .tonic()
    //             .with_endpoint("http://localhost:4317"),
    //     )
    //     .with_trace_config(
    //         Config::default().with_resource(Resource::new(vec![KeyValue::new(
    //             "service.name",
    //             "monzo_crawler",
    //         )])),

    //     )
    //     .install_simple()
    //     .unwrap()
    //     .tracer("monzo_crawler");

    // // log level filtering here
    // let filter_layer = EnvFilter::try_from_default_env()
    //     .or_else(|_| EnvFilter::try_new("info"))
    //     .unwrap();

    // // fmt layer - printing out logs
    // let fmt_layer = fmt::layer().compact();

    // // turn our OTLP pipeline into a tracing layer
    // let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // tracing_subscriber::registry()
    //     .with(filter_layer)
    //     .with(fmt_layer)
    //     .with(otel_layer)
    //     .init();

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(2);

    let client = ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent("monzo_crawler")
            .build()
            .unwrap(),
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new())
    .with(MaxConcurrentMiddleware::new(100))
    // .with(TracingMiddleware::default())
    .build();

    let reqwest_visitor = ClientWithMiddlewareVisitor::new(client);

    let robots = r#"User-agent: *
Disallow: /docs/
Disallow: /referral/
Disallow: /-staging-referral/
Disallow: /install/
Disallow: /blog/authors/
Disallow: /-deeplinks/
"#;

    let crawler = Crawler::new(reqwest_visitor, "monzo-crawler", Some(robots));

    let start = Instant::now();
    let res = crawler
        .crawl(Url::parse("https://monzo.com").unwrap())
        .await;
    let duration = start.elapsed();
    info!("Crawling complete");

    for page in res.0.iter() {
        println!("{}", page.url);
    }
    println!(
        "Time elapsed in website.crawl() is: {:?} for total pages: {}",
        duration,
        res.0.len().green()
    );

    Ok(())
}
