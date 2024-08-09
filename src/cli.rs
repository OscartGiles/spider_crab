use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
/// Welcome to the Monzo Crawler! Try not to get rate limited!
pub struct Cli {
    /// Root URL to start crawling from
    pub url: url::Url,

    /// Root URL to start crawling from
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Hide links when displaying output in the console
    #[arg(short('l'), long)]
    pub hide_links: bool,

    /// Maximum number of concurrent connections
    #[arg(short('c'), long, default_value_t = 500)]
    pub max_concurrent_connections: usize,

    /// Maximum crawl time in seconds. Default is unlimited.
    #[arg(short('m'), long, default_value = None)]
    pub max_time: Option<u64>,

    /// Maximum number of pages to visit. Default is unlimited.
    #[arg(short('p'), long, default_value = None)]
    pub max_pages: Option<u64>,

    /// Ignore robots.txt files.
    #[arg(short, long)]
    pub ignore_robots: bool,

    /// OTL tracing endpoint.
    #[arg(short('t'), long, default_value = None)]
    pub otl_endpoint: Option<url::Url>,
}
