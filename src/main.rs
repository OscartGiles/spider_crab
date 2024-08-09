use std::time::Duration;

use monzo_crawler::{
    client_middleware::{MaxConcurrentMiddleware, RetryTooManyRequestsMiddleware},
    ClientWithMiddlewareVisitor, Crawler,
};
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime::Tokio, trace::Config, Resource};

use owo_colors::{self, OwoColorize};
use reqwest::redirect;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use texting_robots::get_robots_url;
use tokio::time::Instant;
use url::Url;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// A client with middleware for obtaining Robots.txt files.
/// Roughly follows https://github.com/Smerity/texting_robots?tab=readme-ov-file#crawling-considerations
fn robots_client() -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);

    ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .redirect(redirect::Policy::limited(10))
            .build()
            .unwrap(),
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new(Duration::from_secs(5)))
    .with(TracingMiddleware::default())
    .build()
}

fn crawler_client(
    max_retries: u32,
    too_many_requests_delay: Duration,
    max_concurrent_connections: usize,
) -> ClientWithMiddleware {
    let retry_policy = ExponentialBackoff::builder()
        .jitter(reqwest_retry::Jitter::Bounded)
        .build_with_max_retries(max_retries);

    ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .redirect(redirect::Policy::limited(10))
            .build()
            .unwrap(),
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new(too_many_requests_delay))
    .with(MaxConcurrentMiddleware::new(max_concurrent_connections))
    .with(TracingMiddleware::default())
    .build()
}

/// Try to get a robots.txt file for a given URL, returning None if it doesn't exist.
async fn get_robots(root_url: &Url) -> anyhow::Result<Option<String>> {
    let rclient = robots_client();
    let robots_url = get_robots_url(root_url.as_str())?;

    let res = rclient.get(robots_url.as_str()).send().await?;
    let robots = res.text().await;
    Ok(robots.ok())
}

fn configure_tracing() {
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
                APP_USER_AGENT,
            )])),
        )
        // .install_simple()
        .install_batch(Tokio)
        .unwrap()
        .tracer(APP_USER_AGENT);

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    configure_tracing();

    let client = crawler_client(5, Duration::from_secs(5), 1000);
    let reqwest_visitor = ClientWithMiddlewareVisitor::new(client);

    let root_url = Url::parse("https://monzo.com")?;
    let robots_txt = get_robots(&root_url).await?;
    let crawler = Crawler::new(reqwest_visitor, APP_USER_AGENT, robots_txt.as_deref());

    let start = Instant::now();
    let res = crawler.crawl(root_url).await;

    let duration = start.elapsed();
    for page in res.0.iter() {
        println!("{}", page.url);
    }
    println!(
        "Time elapsed in website.crawl() is: {:?} for total pages: {}",
        duration,
        res.0.len().green()
    );

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}
