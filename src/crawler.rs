use std::{collections::HashSet, future::Future};

use http::HeaderValue;
use reqwest::StatusCode;
use texting_robots::Robot;
use tokio::task::JoinSet;
use tracing::{info, info_span, Instrument};
use url::Url;

use crate::parser::{assume_html, parse_links, AllPages, Page};

/// Contents of a page.
pub struct PageContent {
    pub url: Url,
    pub status_code: StatusCode,
    pub content: String,
    pub content_type: Option<HeaderValue>,
}

/// A trait for visiting a site and returning the contents of a page.
pub trait SiteVisitor: Clone + Send + 'static {
    fn visit(&mut self, url: Url) -> impl Future<Output = PageContent> + Send;
}

pub struct Crawler<V>
where
    V: SiteVisitor,
{
    site_visitor: V,
    robot: Option<Robot>,
    tasks: JoinSet<Page>,
}

impl<V> Crawler<V>
where
    V: SiteVisitor,
{
    pub fn new(site_visitor: V, crawler_agent: &str, robot_txt: Option<&str>) -> Self {
        let robot = robot_txt.map(|txt| Robot::new(crawler_agent, txt.as_bytes()).unwrap());

        Self {
            site_visitor,
            robot,
            tasks: JoinSet::new(),
        }
    }

    /// Check if the crawler can visit a URL. If no [Robot] is provided assume we can visit any URL.
    fn can_visit(&self, url: &Url) -> bool {
        assume_html(url)
            && self
                .robot
                .as_ref()
                .map_or(true, |robot| robot.allowed(url.as_str()))
    }

    async fn visit_and_parse(mut site_visitor: V, url: Url) -> Page {
        info!("Visiting and parsing {}", url);
        let page_response = site_visitor.visit(url).await;

        tokio::task::spawn_blocking(move || parse_links(&page_response))
            .await
            .unwrap()
    }

    #[tracing::instrument(skip(self))]
    pub async fn crawl(mut self, url: Url) -> AllPages {
        let mut pages: Vec<Page> = Vec::new();
        let mut visited: HashSet<Url> = HashSet::new();

        info!("Starting to crawl");

        if self.can_visit(&url) {
            visited.insert(url.clone());
            let visitor = self.site_visitor.clone();

            self.tasks
                .spawn(Self::visit_and_parse(visitor, url).instrument(tracing::Span::current()));
        }

        let mut counter = 0;

        while let Some(page) = self.tasks.join_next().await {
            let page = page.expect("Please handle me!!"); //ToDO: Handle errors

            counter += 1;
            println!("Counter: {}", counter);

            let mut recovered_links = Vec::new();
            for link in page.links.iter() {
                recovered_links.push(link.clone());
            }
            pages.push(page);

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
                    info!("Robots.txt - Ignored {} ", link);
                }
            }
        }

        AllPages(pages)
    }
}
