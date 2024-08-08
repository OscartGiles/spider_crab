use std::{collections::HashSet, future::Future};

use reqwest::StatusCode;
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
    site_vistor: V,
}

impl<V> Crawler<V>
where
    V: SiteVisitor,
{
    pub fn new(site_vistor: V) -> Self {
        Self { site_vistor }
    }

    pub async fn crawl(mut self, url: Url) -> AllPages {
        let mut pages: Vec<Page> = Vec::new();
        let mut visited: HashSet<Url> = HashSet::new();
        let mut to_visit: Vec<Url> = Vec::new();

        to_visit.push(url);

        while let Some(next_url) = to_visit.pop() {
            let not_visited = visited.insert(next_url.clone());

            if not_visited {
                let mut recovered_links = Vec::new();
                let page_response = self.site_vistor.visit(next_url.clone()).await;
                let page = parse_links(page_response);

                for link in page.links.iter() {
                    recovered_links.push(link.clone());
                }
                pages.push(page);

                for link in recovered_links {
                    to_visit.push(link);
                }
            }
        }

        AllPages(pages)
    }
}
