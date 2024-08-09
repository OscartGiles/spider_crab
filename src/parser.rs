use std::{collections::HashSet, fmt::Debug};

use reqwest::StatusCode;
use scraper::{Html, Selector};
use url::Url;

use crate::crawler::PageContent;

#[derive(Debug, Clone)]
pub struct Page {
    pub url: Url,
    pub status_code: StatusCode,
    pub links: HashSet<Url>,
}

/// A collection of all pages visited by the [Crawler].
#[derive(Debug)]
pub struct AllPages(pub Vec<Page>);

/// Get all unique links that are from the same domain as the `page_url`.
/// Excludes any links that do not use http or https scheme.
/// Fragments are not treated as unique links.
pub fn parse_links(page_content: &PageContent) -> Page {
    let document = Html::parse_document(&page_content.content);
    let selector = Selector::parse("a").unwrap();

    let page_url = page_content.url.clone();

    let links = document
        .select(&selector)
        .filter(|a| a.value().attr("href").is_some())
        .map(|a| {
            a.value()
                .attr("href")
                .expect("href not found. This is a bug. None's should be filtered out.")
        })
        .filter(|href| !href.starts_with('#'))
        .flat_map(|href| {
            if href.starts_with('/') {
                page_url.join(href)
            } else {
                Url::parse(href)
            }
        })
        .filter(|url| url.domain() == page_url.domain())
        .filter(|url| url.scheme() == "https" || url.scheme() == "http")
        .map(|mut href| {
            href.set_fragment(None);
            // remove_trailing_slash(href)
            href
        })
        .collect();

    Page {
        url: page_url,
        status_code: page_content.status_code,
        links,
    }
}

pub(crate) fn assume_html(url: &Url) -> bool {
    let path = url.path();
    if path.contains('.') {
        let suffix = path.split('.').last().unwrap();
        suffix == "html"
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::{crawler::PageContent, parser::assume_html};

    use super::parse_links;
    use std::{collections::HashSet, fs};
    use url::Url;

    #[test]
    fn test_link_parser() {
        let html = r#"
    <!DOCTYPE html>
    <meta charset="utf-8">
    <title>Hello, world!</title>
    <h1 class="foo">Hello, <i>world!</i></h1>
    <a href="https://monzo.com/hi">Monzo https</a>
    <a href="http://monzo.com/hi">Monzo http</a>
    <a href="ftp://monzo.com/hi">Don't include any links that don't use http or https scheme</a>
    <div>
        <p>foo</p>
        <p>bar</p>
        <p><a href="/nested-deeper">baz</a></p>
        <p><a href="/nested-deeper">duplicate</a></p>
        <p><a href="/fragments-not-unique#first">first fragment</a></p>
        <p><a href="/fragments-not-unique#second">second fragment</a></p>
        <a href="https://notmonozo.com/opps">dont include other domains</a>
        <a href="https://sudomain.monzo.com/hi">don't include subdomains</a>
    </div>
"#;
        let page = PageContent {
            url: Url::parse("https://monzo.com").unwrap(),
            status_code: reqwest::StatusCode::OK,
            content: html.to_string(),
            content_type: None,
        };

        let links = parse_links(&page).links;

        let expected_links: HashSet<Url> = HashSet::from([
            "https://monzo.com/hi",
            "http://monzo.com/hi",
            "https://monzo.com/nested-deeper",
            "https://monzo.com/fragments-not-unique",
        ])
        .iter()
        .map(|&url| Url::parse(url).unwrap())
        .collect();

        assert_eq!(links, expected_links);
    }

    #[test]
    fn test_parse_monzo() {
        let html = fs::read_to_string("./tests/test_data/monzo/home.html").unwrap();

        let page = PageContent {
            url: Url::parse("https://monzo.com").unwrap(),
            status_code: reqwest::StatusCode::OK,
            content: html,
            content_type: None,
        };

        let links = parse_links(&page).links;

        for element in links {
            println!("{}", element.as_str());
        }
    }

    #[test]
    fn test_url_parser() {
        let not_html = Url::parse("https://monzo.com/home.pdf").unwrap();
        assert!(!assume_html(&not_html));

        let not_html = Url::parse("https://monzo.com/home").unwrap();
        assert!(assume_html(&not_html));

        let not_html = Url::parse("https://monzo.com/home.html").unwrap();
        assert!(assume_html(&not_html));
    }
}
