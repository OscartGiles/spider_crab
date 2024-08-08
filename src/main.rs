use monzo_crawler::{
    client_middleware::{MaxConcurrentMiddleware, RetryTooManyRequestsMiddleware},
    Crawler, PageContent, SiteVisitor,
};
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace::Config, Resource};
use owo_colors::OwoColorize;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use tokio::time::Instant;
use tracing::info;

use url::Url;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
async fn main() -> anyhow::Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "monzo_crawler",
            )])),
        )
        .install_simple()
        .unwrap()
        .tracer("monzo_crawler");

    // log level filtering here
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // fmt layer - printing out logs
    let fmt_layer = fmt::layer().compact();

    // turn our OTLP pipeline into a tracing layer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(otel_layer)
        .init();
    // .tracer("monzo_crawler");
    //
    // let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::EnvFilter::from_default_env())
    //     // .with(fmt_layer)
    //     .with(telemetry_layer)
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
    .with(MaxConcurrentMiddleware::new(10))
    .with(TracingMiddleware::default())
    .build();

    info!("Starting crawler");

    let span = tracing::span!(tracing::Level::INFO, "test_span");
    let _enter = span.enter();

    let reqwest_visitor = ClientWithMiddlewareVisitor::new(client);

    let crawler = Crawler::new(reqwest_visitor, "monzo-crawler", None);

    let start = Instant::now();
    let res = crawler
        .crawl(Url::parse("https://oscartgiles.github.io/").unwrap())
        .await;
    let duration = start.elapsed();

    println!("{}", res);
    println!(
        "Time elapsed in website.crawl() is: {:?} for total pages: {}",
        duration,
        res.0.len().green()
    );

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
