use std::{collections::HashSet, future::Future};

use url::Url;

use crate::parse_links;

pub trait SiteVisitor {
    fn visit(&mut self, url: Url) -> impl Future<Output = String> + Send;
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

    pub async fn crawl(mut self, url: Url) -> HashSet<Url> {
        let mut visited: HashSet<Url> = HashSet::new();
        let mut to_visit: Vec<Url> = Vec::new();

        to_visit.push(url);

        while let Some(next_url) = to_visit.pop() {
            let not_visited = visited.insert(next_url.clone());

            if not_visited {
                let mut recovered_links = Vec::new();
                let content = self.site_vistor.visit(next_url.clone()).await;

                let page_links = parse_links(&content, &next_url);

                for link in page_links {
                    recovered_links.push(link);
                }

                for link in recovered_links {
                    to_visit.push(link);
                }
            }
        }

        visited
    }
}

#[cfg(test)]
mod tests {
    use super::SiteVisitor;
    use crate::crawler::Crawler;
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, RwLock},
    };
    use url::Url;

    /// A mock url visitor that returns a string based on the URL.
    /// Provides a count of how many times a URL has been visited, so we can assert that we
    /// only visit each URL once.
    #[derive(Clone)]
    struct MockUrlVisitor {
        visited: Arc<RwLock<HashMap<Url, u32>>>,
    }

    impl SiteVisitor for MockUrlVisitor {
        async fn visit(&mut self, url: Url) -> String {
            // Increment the number of times the URL has been visited
            {
                let mut gaurd = self.visited.write().expect("Could not acquire lock");
                let entry = gaurd.entry(url.clone()).or_insert(0);
                *entry += 1;
            }

            // Route urls to responses.
            match url.as_str() {
                "https://monzo.com/" => r#"<a href="/about"></a> <a href="/cost"></a>"#.into(),
                "https://monzo.com/about" => r#"<a href="/about"></a> <a href="/cost"></a>"#.into(),
                "https://monzo.com/cost" => r#"<a href="/cost-inner"></a>"#.into(),
                "https://monzo.com/cost-inner" => r#"<p></p>"#.into(),
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
        let visited = crawler.crawl(root_url).await;

        // Then: The crawler reports that it visited the expected URLs
        assert_eq!(visited, expected_urls);

        // And: The mock page visitor reports that it visited the expected URLs
        assert_eq!(mock_visitor.visited_urls(), expected_urls);

        // And: The mock visitor reports that it visited each URL exactly once
        assert!(mock_visitor
            .visited_urls_with_counts()
            .iter()
            .all(|(_, &count)| count == 1));
    }
}
