use std::{collections::HashSet, future::Future};

use reqwest::StatusCode;
use texting_robots::Robot;
use url::Url;

use crate::parser::{parse_links, AllPages, Page};

/// Contents of a page.
pub struct PageContent {
    pub url: Url,
    pub status_code: StatusCode,
    pub content: String,
}

/// A trait for visiting a site and returning the contents of a page.
pub trait SiteVisitor {
    fn visit(&mut self, url: Url) -> impl Future<Output = PageContent> + Send;
}

pub struct Crawler<V>
where
    V: SiteVisitor,
{
    site_visitor: V,
    robot: Option<Robot>,
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
        }
    }

    /// Check if the crawler can visit a URL. If no [Robot] is provided assume we can visit any URL.
    fn can_visit(&self, url: &Url) -> bool {
        self.robot
            .as_ref()
            .map_or(true, |robot| robot.allowed(url.as_str()))
    }

    pub async fn crawl(mut self, url: Url) -> AllPages {
        let mut pages: Vec<Page> = Vec::new();
        let mut visited: HashSet<Url> = HashSet::new();
        let mut to_visit: Vec<Url> = Vec::new();

        if self.can_visit(&url) {
            to_visit.push(url);
        }

        while let Some(next_url) = to_visit.pop() {
            let not_visited = visited.insert(next_url.clone());

            if not_visited {
                let mut recovered_links = Vec::new();
                let page_response = self.site_visitor.visit(next_url.clone()).await;
                let page = parse_links(page_response);

                for link in page.links.iter() {
                    recovered_links.push(link.clone());
                }
                pages.push(page);

                for link in recovered_links {
                    if self.can_visit(&link) {
                        to_visit.push(link);
                    } else {
                        println!("DISALLOWED: {}", link);
                    }
                }
            }
        }

        AllPages(pages)
    }
}
