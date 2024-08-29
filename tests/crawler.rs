use http::HeaderValue;
use spider_crab::{CrawlerBuilder, PageContent, SiteVisitor, VisitorError};
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
    async fn visit(&mut self, url: Url) -> Result<PageContent, VisitorError> {
        // Increment the number of times the URL has been visited
        {
            let mut gaurd = self.visited.write().expect("Could not acquire lock");
            let entry = gaurd.entry(url.clone()).or_insert(0);
            *entry += 1;
        }

        let content_type: HeaderValue = "text/html".parse().expect("Failed to parse header");

        // Route urls to responses.
        let response = match url.as_str() {
            "https://monzo.com/" => PageContent {
                content: r#"<a href="/about"></a> <a href="/cost"></a>"#.into(),
                status_code: reqwest::StatusCode::OK,
                url,
                content_type: Some(content_type),
            },
            "https://monzo.com/about" => PageContent {
                content: r#"<a href="/about"></a> <a href="/cost"></a>"#.into(),
                status_code: reqwest::StatusCode::ACCEPTED,
                url,
                content_type: Some(content_type),
            },
            "https://monzo.com/cost" => PageContent {
                content: r#"<a href="/cost-inner"></a>"#.into(),
                status_code: reqwest::StatusCode::OK,
                url,
                content_type: Some(content_type),
            },
            "https://monzo.com/cost-inner" => PageContent {
                content: r#"<p></p>"#.into(),
                status_code: reqwest::StatusCode::OK,
                url,
                content_type: Some(content_type),
            },
            _ => panic!("Unexpected URL: {}", url),
        };

        Ok(response)
    }
}
impl MockUrlVisitor {
    fn new() -> Self {
        Self {
            visited: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn visited_urls_once(&self) -> bool {
        self.visited
            .read()
            .expect("Could not acquire lock")
            .clone()
            .iter()
            .all(|(_, &count)| count == 1)
    }

    fn visited_urls(&self) -> HashSet<Url> {
        let visited = self.visited.read().expect("Could not acquire lock").clone();
        visited.keys().cloned().collect()
    }
}

#[tokio::test]
async fn test_visitor() -> anyhow::Result<()> {
    // Expect the crawler to visit these URLs
    let expected_urls = HashSet::from([
        "https://monzo.com/",
        "https://monzo.com/about",
        "https://monzo.com/cost",
        "https://monzo.com/cost-inner",
    ])
    .iter()
    .map(|&url| Url::parse(url).expect("Failed to parse URL."))
    .collect();

    // Given: We crawl the (mock) Monzo website
    let mock_visitor = MockUrlVisitor::new();
    let crawler = CrawlerBuilder::new(mock_visitor.clone()).build();

    let root_url = Url::parse("https://monzo.com")?;

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
    assert!(mock_visitor.visited_urls_once());

    println!("Visited pages:\n\n{:?}", visited_pages);

    Ok(())
}

#[tokio::test]
async fn test_visitor_with_robots() -> anyhow::Result<()> {
    // Expect the crawler to visit these URLs
    let expected_urls = HashSet::from([
        "https://monzo.com/",
        "https://monzo.com/about",
        "https://monzo.com/cost",
        // "https://monzo.com/cost-inner",
    ])
    .iter()
    .map(|&url| Url::parse(url).expect("Failed to parse URL."))
    .collect();

    let robots_txt = r"User-Agent: *
Disallow: /cost-inner";

    // Given: We crawl the (mock) Monzo website
    let mock_visitor = MockUrlVisitor::new();
    let crawler = CrawlerBuilder::new(mock_visitor.clone())
        .with_robot(robots_txt, "test-agent")
        .expect("Could not parse robots.txt")
        .build();
    let root_url = Url::parse("https://monzo.com")?;

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
    assert!(mock_visitor.visited_urls_once());

    println!("Visited pages\n{:?}", visited_pages.0);

    Ok(())
}
