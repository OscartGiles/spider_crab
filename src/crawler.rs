use std::{
    collections::HashSet,
    future::Future,
    sync::Arc,
    time::{Duration, SystemTime},
};

use http::HeaderValue;
use reqwest::StatusCode;
use texting_robots::Robot;
use thiserror::Error;
use tokio::{sync::broadcast, task::JoinSet};
use tracing::{debug, error, info, Instrument};
use url::Url;

use crate::parser::{assume_html, parse_links, AllPages, Page};

/// An error from ths vistor. Assumes all recoverable errors have been handled and just reporting to caller.
#[derive(Error, Debug)]
#[error("failed to make a request")]
pub struct VisitorError(pub anyhow::Error);

/// Contents of a page.
pub struct PageContent {
    pub url: Url,
    pub status_code: StatusCode,
    pub content: String,
    pub content_type: Option<HeaderValue>,
}

/// A trait for visiting a URL and returning the contents of its page.
pub trait SiteVisitor: Clone + Send + 'static {
    /// Visit a URL and return the contents of the page as a [PageContent].
    fn visit(&mut self, url: Url)
        -> impl Future<Output = Result<PageContent, VisitorError>> + Send;
}

/// Web crawler.
/// Given a starting URL, the crawler should visit each URL it finds on the same domain.
/// Create a Crawler using [CrawlerBuilder].
pub struct Crawler<V>
where
    V: SiteVisitor,
{
    site_visitor: V,
    robot: Option<Robot>,
    tasks: JoinSet<Result<Page, VisitorError>>,
    channel: broadcast::Sender<Arc<Page>>,
    max_time: Option<std::time::Duration>,
    max_pages: Option<u64>,
}

impl<V> Crawler<V>
where
    V: SiteVisitor,
{
    /// Check if the crawler can visit a URL. If no [Robot] is provided assume we can visit any URL.
    fn can_visit(&self, url: &Url) -> bool {
        assume_html(url)
            && self
                .robot
                .as_ref()
                .map_or(true, |robot| robot.allowed(url.as_str()))
    }

    async fn visit_and_parse(mut site_visitor: V, url: Url) -> Result<Page, VisitorError> {
        debug!("Visiting and parsing {}", url);
        let page_response = site_visitor.visit(url).await?;

        let result = tokio::task::spawn_blocking(move || parse_links(&page_response))
            .await
            .expect("Task failed to execute to completion");

        Ok(result)
    }

    /// Subscribe to receive pages as they are crawled.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Page>> {
        self.channel.subscribe()
    }

    /// Start crawling from a given URL.
    /// Consumes the [Crawler] and returns a collection of all pages visited.
    #[tracing::instrument(skip(self))]
    pub async fn crawl(mut self, url: Url) -> AllPages {
        let mut pages: Vec<Page> = Vec::new();
        let mut visited: HashSet<Url> = HashSet::new();
        let mut page_count: u64 = 0;
        let start_time = SystemTime::now();

        debug!("Starting crawl");

        if self.can_visit(&url) {
            visited.insert(url.clone());
            let visitor = self.site_visitor.clone();

            self.tasks
                .spawn(Self::visit_and_parse(visitor, url).instrument(tracing::Span::current()));
        }

        while let Some(task_result) = self.tasks.join_next().await {
            // If there are any failures log an error and continue.
            let page = match task_result {
                Ok(page_result) => match page_result {
                    Ok(page) => page,
                    Err(request_error) => {
                        error!("Failed to reach site: {}", request_error);
                        continue;
                    }
                },
                Err(join_error) => {
                    error!("Failed to join task: {}", join_error);
                    continue;
                }
            };

            // Broadcast the page
            let _ = self.channel.send(Arc::new(page.clone())); // Ignore errors as we don't care if the receiver is gone

            let mut recovered_links = Vec::new();
            for link in page.links.iter() {
                recovered_links.push(link.clone());
            }
            pages.push(page);

            // Check if we have reached the max pages
            if Some(page_count + 1) == self.max_pages {
                info!("Max pages reached");
                break;
            }
            page_count += 1;

            // Check if we have reached the max time
            if let Some(max_time) = self.max_time {
                let now = SystemTime::now();
                if let Ok(duration) = now.duration_since(start_time) {
                    if duration > max_time {
                        info!("Max time reached");
                        break;
                    }
                }
            }

            for link in recovered_links {
                if self.can_visit(&link) {
                    let not_visited = visited.insert(link.clone());

                    if not_visited {
                        let visitor = self.site_visitor.clone();

                        self.tasks.spawn(
                            Self::visit_and_parse(visitor, link)
                                .instrument(tracing::Span::current()),
                        );
                    }
                } else {
                    debug!("Robots.txt - Ignored {} ", link);
                }
            }
        }

        AllPages(pages)
    }
}

/// Builder for [Crawler].
pub struct CrawlerBuilder<V>
where
    V: SiteVisitor,
{
    site_visitor: V,
    robot: Option<Robot>,
    max_time: Option<std::time::Duration>,
    max_pages: Option<u64>,
}

impl<V> CrawlerBuilder<V>
where
    V: SiteVisitor,
{
    /// Create a new [CrawlerBuilder] with a [SiteVisitor].
    pub fn new(site_visitor: V) -> Self {
        Self {
            site_visitor,
            robot: None,
            max_time: None,
            max_pages: None,
        }
    }

    /// Provide a robot_txt file for the crawler. The crawler will not visit pages denied in the robot_txt file.
    pub fn with_robot(mut self, robot_txt: &str, crawler_agent: &str) -> anyhow::Result<Self> {
        self.robot = Some(Robot::new(crawler_agent, robot_txt.as_bytes())?);
        Ok(self)
    }

    /// Set the maximum time the crawler will run for.
    pub fn with_max_time(mut self, max_time: u64) -> Self {
        self.max_time = Some(Duration::from_secs(max_time));
        self
    }

    /// Set the maximum number of pages the crawler will visit.
    pub fn with_max_pages(mut self, max_pages: u64) -> Self {
        self.max_pages = Some(max_pages);
        self
    }

    /// Build the crawler.
    pub fn build(self) -> Crawler<V> {
        let (tx, _) = broadcast::channel(100);
        Crawler {
            site_visitor: self.site_visitor,
            robot: self.robot,
            tasks: JoinSet::new(),
            channel: tx,
            max_time: self.max_time,
            max_pages: self.max_pages,
        }
    }
}
