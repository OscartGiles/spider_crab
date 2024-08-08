use std::{collections::HashSet, future::Future};

use reqwest::StatusCode;
use url::Url;

use crate::parser::{parse_links, AllPages, Page};

/// Contents of a page.
pub(crate) struct PageContent {
    pub(crate) url: Url,
    pub(crate) status_code: StatusCode,
    pub(crate) content: String,
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

#[cfg(test)]
mod tests {
    use super::{PageContent, SiteVisitor};
    use crate::crawler::Crawler;
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, RwLock},
    };
    use url::Url;

    /// A mock url visitor that returns a string based on the URL.
    /// Provides a count of how many times a URL has been visited, so we can assert that we
    /// only visit each URL once.
    /// Clones share the visited count by using an Arc internally.
    #[derive(Clone)]
    struct MockUrlVisitor {
        visited: Arc<RwLock<HashMap<Url, u32>>>,
    }

    impl SiteVisitor for MockUrlVisitor {
        async fn visit(&mut self, url: Url) -> PageContent {
            // Increment the number of times the URL has been visited
            {
                let mut gaurd = self.visited.write().expect("Could not acquire lock");
                let entry = gaurd.entry(url.clone()).or_insert(0);
                *entry += 1;
            }

            // Route urls to responses.
            match url.as_str() {
                "https://monzo.com/" => PageContent {
                    content: r#"<a href="/about"></a> <a href="/cost"></a>"#.into(),
                    status_code: reqwest::StatusCode::OK,
                    url,
                },
                "https://monzo.com/about" => PageContent {
                    content: r#"<a href="/about"></a> <a href="/cost"></a>"#.into(),
                    status_code: reqwest::StatusCode::ACCEPTED,
                    url,
                },
                "https://monzo.com/cost" => PageContent {
                    content: r#"<a href="/cost-inner"></a>"#.into(),
                    status_code: reqwest::StatusCode::OK,
                    url,
                },
                "https://monzo.com/cost-inner" => PageContent {
                    content: r#"<p></p>"#.into(),
                    status_code: reqwest::StatusCode::OK,
                    url,
                },
                _ => panic!("Unexpected URL: {}", url),
            }
        }
    }
    impl MockUrlVisitor {
        fn new() -> Self {
            Self {
                visited: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn visited_urls_with_counts(&self) -> HashMap<Url, u32> {
            self.visited.read().expect("Could not acquire lock").clone()
        }

        fn visited_urls(&self) -> HashSet<Url> {
            let visited = self.visited.read().expect("Could not acquire lock").clone();
            visited.keys().cloned().collect()
        }
    }

    #[tokio::test]
    async fn test_visitor() {
        // Expect the crawler to visit these URLs
        let expected_urls = HashSet::from([
            "https://monzo.com/",
            "https://monzo.com/about",
            "https://monzo.com/cost",
            "https://monzo.com/cost-inner",
        ])
        .iter()
        .map(|&url| Url::parse(url).unwrap())
        .collect();

        // Given: We crawl the (mock) Monzo website
        let mock_visitor = MockUrlVisitor::new();
        let crawler = Crawler::new(mock_visitor.clone());
        let root_url = Url::parse("https://monzo.com").unwrap();

        // When we crawl starting at the root URL
        let visited_pages = crawler.crawl(root_url).await;

        let visited_urls = visited_pages
            .0
            .iter()
            .map(|page| page.url.clone())
            .collect::<HashSet<Url>>();

        // Then: The crawler reports that it visited the expected URLs
        assert_eq!(visited_urls, expected_urls);

        // And: The mock page visitor reports that it visited the expected URLs
        assert_eq!(mock_visitor.visited_urls(), expected_urls);

        // And: The mock visitor reports that it visited each URL exactly once
        assert!(mock_visitor
            .visited_urls_with_counts()
            .iter()
            .all(|(_, &count)| count == 1));

        println!("Visited pages:\n\n{}", visited_pages);
    }
}
