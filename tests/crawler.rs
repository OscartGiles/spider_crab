use monzo_crawler::{Crawler, PageContent, SiteVisitor};
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
    let crawler = Crawler::new(mock_visitor.clone(), "monzo-crawler", None);
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

#[tokio::test]
async fn test_visitor_with_robots() {
    // Expect the crawler to visit these URLs
    let expected_urls = HashSet::from([
        "https://monzo.com/",
        "https://monzo.com/about",
        "https://monzo.com/cost",
        // "https://monzo.com/cost-inner",
    ])
    .iter()
    .map(|&url| Url::parse(url).unwrap())
    .collect();

    let robots_txt = r"User-Agent: *
Disallow: /cost-inner";

    // Given: We crawl the (mock) Monzo website
    let mock_visitor = MockUrlVisitor::new();
    let crawler = Crawler::new(mock_visitor.clone(), "monzo-crawler", Some(robots_txt));
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

    println!("Visited pages\n{}", visited_pages);
}
