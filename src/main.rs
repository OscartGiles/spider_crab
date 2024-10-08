mod cli;
use std::{path::Path, time::Duration};

use clap::Parser;
use cli::Cli;
use indicatif::{MultiProgress, ProgressBar};
use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{Config, TracerProvider},
    Resource,
};
use spider_crab::{
    client_middleware::{MaxConcurrentMiddleware, RetryTooManyRequestsMiddleware},
    AllPages, ClientWithMiddlewareVisitor, CrawlerBuilder,
};

use owo_colors::{self, OwoColorize};
use reqwest::redirect;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use reqwest_tracing::TracingMiddleware;
use texting_robots::get_robots_url;
use tokio::{io::AsyncWriteExt, time::Instant};
use url::Url;

use tracing_subscriber::{prelude::*, EnvFilter};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// A client with middleware for obtaining Robots.txt files.
/// Roughly follows https://github.com/Smerity/texting_robots?tab=readme-ov-file#crawling-considerations
fn robots_client() -> anyhow::Result<ClientWithMiddleware> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);

    Ok(ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .redirect(redirect::Policy::limited(10))
            .build()?,
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new(Duration::from_secs(5)))
    .with(TracingMiddleware::default())
    .build())
}

fn crawler_client(
    max_retries: u32,
    too_many_requests_delay: Duration,
    max_concurrent_connections: usize,
) -> anyhow::Result<ClientWithMiddleware> {
    let retry_policy = ExponentialBackoff::builder()
        .jitter(reqwest_retry::Jitter::Bounded)
        .build_with_max_retries(max_retries);

    Ok(ClientBuilder::new(
        reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .redirect(redirect::Policy::limited(10))
            .build()?,
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .with(RetryTooManyRequestsMiddleware::new(too_many_requests_delay))
    .with(MaxConcurrentMiddleware::new(max_concurrent_connections))
    .with(TracingMiddleware::default())
    .build())
}

/// Try to get a robots.txt file for a given URL, returning None if it doesn't exist.
async fn get_robots(root_url: &Url) -> anyhow::Result<String> {
    let rclient = robots_client()?;
    let robots_url = get_robots_url(root_url.as_str())?;

    let res = rclient.get(robots_url.as_str()).send().await?;
    let robots = res.text().await;
    robots.map_err(Into::into)
}

fn print_links(all_pages: &AllPages, hide_links: bool) {
    for page in all_pages.0.iter() {
        println!("{}", page.url.green());

        if !hide_links {
            for link in page.links.iter() {
                println!("  --> {}", link.cyan());
            }
        }
    }
}

async fn write_links_to_file(
    all_pages: &AllPages,
    file: &Path,
    hide_links: bool,
) -> anyhow::Result<()> {
    let mut file = tokio::fs::File::create(file).await?;
    for page in all_pages.0.iter() {
        file.write_all(format!("{}\n", page.url).as_bytes()).await?;
        if !hide_links {
            for link in page.links.iter() {
                file.write_all(format!("  --> {}\n", link).as_bytes())
                    .await?;
            }
        }
    }
    Ok(())
}

/// Configure tracing with the given OTL endpoint.
fn configure_tracing(otl_endpoint: Url) -> anyhow::Result<TracerProvider> {
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otl_endpoint),
        )
        .with_trace_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                APP_USER_AGENT,
            )])),
        )
        .install_batch(Tokio)?;

    let tracer = provider.tracer(APP_USER_AGENT);

    let filter_layer = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(otel_layer)
        .init();

    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let trace_provider = cli.otl_endpoint.map(configure_tracing);
    let trace_provider = if let Some(provider) = trace_provider {
        Some(provider?)
    } else {
        None
    };

    let client = crawler_client(5, Duration::from_secs(5), cli.max_concurrent_connections)?;
    let reqwest_visitor = ClientWithMiddlewareVisitor::new(client);

    // Build a crawler
    let mut crawler_builder = CrawlerBuilder::new(reqwest_visitor);
    if let Ok(robots_txt) = get_robots(&cli.url).await {
        if !cli.ignore_robots {
            crawler_builder = crawler_builder.with_robot(&robots_txt, APP_USER_AGENT)?;
        }
    }
    if let Some(max_pages) = cli.max_pages {
        crawler_builder = crawler_builder.with_max_pages(max_pages);
    }
    if let Some(max_time_seconds) = cli.max_time {
        crawler_builder = crawler_builder.with_max_time(max_time_seconds);
    }

    let crawler = crawler_builder.build();

    // Subscribe to the crawler's broadcast channel. This will allow us to receive progress updates
    let mut rx = crawler.subscribe();
    let url_string = cli.url.clone();
    // Spawn a task to manage progress bar updates
    let progress_handle = tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let mut count = 0;

        let multi_progress = MultiProgress::new();
        let header = multi_progress.add(ProgressBar::new_spinner());
        let current_url = multi_progress.add(ProgressBar::new_spinner());
        let visit_stats = multi_progress.add(ProgressBar::new_spinner());

        header.enable_steady_tick(Duration::from_millis(120));
        current_url.enable_steady_tick(Duration::from_millis(120));
        visit_stats.enable_steady_tick(Duration::from_millis(120));

        header.set_message(format!("Crawling: {}", url_string.as_str().green()));

        while let Ok(page) = rx.blocking_recv() {
            count += 1;
            let duration = start.elapsed();
            let seconds = duration.as_secs() % 60;
            let minutes = (duration.as_secs() / 60) % 60;
            visit_stats.set_message(format!(
                "  Visited {} pages in {:0>2}:{:0>2}",
                count.cyan(),
                minutes.to_string().cyan(),
                seconds.to_string().cyan()
            ));
            current_url.set_message(format!("  Current url: {}", page.url.as_str().green()));
        }
        header.finish_and_clear();
        current_url.finish_and_clear();
        visit_stats.finish_and_clear();
    });

    let res = crawler.crawl(cli.url).await;
    progress_handle.await?;

    match &cli.output {
        Some(path) => write_links_to_file(&res, path, cli.hide_links).await?,
        None => print_links(&res, cli.hide_links),
    };

    // Shutdown tracing
    if let Some(provider_builder) = trace_provider {
        provider_builder.shutdown()?;
    }
    Ok(())
}
